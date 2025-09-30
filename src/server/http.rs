// HTTP server implementation
// Per spec-kit/003-backend-spec.md section 1.1

use std::sync::Arc;

use actix_files::Files;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web_actors::ws;

use crate::config::Config;
use crate::handlers;
use crate::pty::PtyManager;
use crate::security::jwks_client::JwksClient;
use crate::security::jwt_validator::JwtValidator;
use crate::server::middleware::auth::{JwtAuthMiddleware, UserContext};
use crate::server::middleware::{
    cors::CorsConfig as CorsMiddlewareConfig,
    security_headers::{
        SecurityHeadersConfig as SecurityHeadersMiddlewareConfig, SecurityHeadersMiddleware,
    },
};
use crate::server::websocket::WebSocketSession;
use crate::session::{SessionId, SessionManager};

#[cfg(feature = "tls")]
use crate::config::server::TlsConfig;
#[cfg(feature = "tls")]
use crate::server::tls::{load_tls_config, validate_tls_files};

/// HTTP server instance
/// Per spec-kit/003-backend-spec.md: Single-port architecture
/// All HTTP, WebSocket, and static assets served from ONE port (default 8080)
pub struct Server {
    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
    jwt_validator: Arc<JwtValidator>,
}

impl Server {
    /// Create a new HTTP server instance
    /// Per spec-kit/003-backend-spec.md section 1.1
    /// Per spec-kit/011-authentication-spec.md: JWT authentication
    pub fn new(config: Config, session_manager: SessionManager) -> Self {
        tracing::info!(
            "Initializing HTTP server on {}:{}",
            config.server.host,
            config.server.port
        );

        // Initialize JWKS client and JWT validator
        // Per spec-kit/011-authentication-spec.md section 2.1
        let jwks_client = Arc::new(JwksClient::new(config.auth.clone()));
        let jwt_validator = Arc::new(JwtValidator::new(jwks_client, config.auth.clone()));

        Self {
            config: Arc::new(config),
            session_manager: Arc::new(session_manager),
            jwt_validator,
        }
    }

    /// Start the HTTP server
    /// Per spec-kit/003-backend-spec.md: Single-port deployment
    /// CRITICAL: All services (HTTP, WebSocket, static files) on ONE port
    /// Per spec-kit/009-deployment-spec.md: TLS and security headers
    pub async fn run(self) -> std::io::Result<()> {
        let bind_addr = format!("{}:{}", self.config.server.host, self.config.server.port);

        // Check if TLS is enabled
        let tls_enabled = self.config.server.tls.is_some();
        let protocol = if tls_enabled { "https" } else { "http" };
        let ws_protocol = if tls_enabled { "wss" } else { "ws" };

        tracing::info!(
            "Starting server on {} ({})",
            bind_addr,
            protocol.to_uppercase()
        );
        tracing::info!("WebSocket endpoint: {}://{}/ws", ws_protocol, bind_addr);
        tracing::info!("Health check: {}://{}/api/v1/health", protocol, bind_addr);

        let session_manager = self.session_manager.clone();
        let jwt_validator = self.jwt_validator.clone();

        // Create JWT auth middleware
        // Per spec-kit/011-authentication-spec.md: HTTP auth middleware
        let auth_middleware = JwtAuthMiddleware::new(jwt_validator.clone());

        // Build CORS middleware config
        // Per spec-kit/002-architecture.md Layer 1: Network Security
        let cors_config = CorsMiddlewareConfig {
            allowed_origins: self.config.server.cors.allowed_origins.clone(),
            allowed_methods: self.config.server.cors.allowed_methods.clone(),
            allowed_headers: self.config.server.cors.allowed_headers.clone(),
            max_age: self.config.server.cors.max_age,
            supports_credentials: self.config.server.cors.supports_credentials,
        };

        // Build security headers middleware from config
        // Per spec-kit/002-architecture.md: Defense in depth strategy
        let security_headers_config = SecurityHeadersMiddlewareConfig {
            enable_hsts: self.config.server.security_headers.enable_hsts,
            hsts_max_age: self.config.server.security_headers.hsts_max_age,
            enable_csp: self.config.server.security_headers.enable_csp,
            csp_policy: self.config.server.security_headers.csp_policy.clone(),
            enable_frame_options: self.config.server.security_headers.enable_frame_options,
            frame_options: self.config.server.security_headers.frame_options.clone(),
        };
        let security_headers = SecurityHeadersMiddleware::new(security_headers_config);

        let server = HttpServer::new(move || {
            // Build CORS middleware inside closure (Cors is not Clone)
            let cors = cors_config.build();

            App::new()
                // Shared application state
                .app_data(web::Data::new(session_manager.clone()))
                .app_data(web::Data::new(jwt_validator.clone()))
                // Middleware (applied in order)
                .wrap(tracing_actix_web::TracingLogger::default())
                .wrap(security_headers.clone())
                .wrap(cors)
                // API routes (per docs/spec-kit/006-api-spec.md)
                .service(
                    web::scope("/api/v1")
                        // Public endpoints (no auth required)
                        .route("/health", web::get().to(handlers::health_check))
                        // Protected endpoints (require JWT auth)
                        .service(
                            web::scope("")
                                .wrap(auth_middleware.clone())
                                .route("/sessions", web::post().to(handlers::create_session))
                                .route("/sessions", web::get().to(handlers::list_sessions))
                                .route("/sessions/{id}", web::get().to(handlers::get_session))
                                .route("/sessions/{id}", web::delete().to(handlers::delete_session))
                                .route(
                                    "/sessions/{id}/history",
                                    web::get().to(handlers::get_session_history),
                                ),
                        ),
                )
                // WebSocket endpoint (authentication via Authenticate message)
                // Per spec-kit/007-websocket-spec.md: WebSocket authentication
                .route("/ws", web::get().to(websocket_handler))
                // Static files served from same port
                .service(Files::new("/", "./static").index_file("index.html"))
        })
        .workers(self.config.server.worker_threads);

        // Configure TLS if enabled
        // Per spec-kit/009-deployment-spec.md: TLS 1.2+ enforcement
        #[cfg(feature = "tls")]
        if let Some(ref tls_config) = self.config.server.tls {
            tracing::info!("TLS enabled - loading certificates");
            validate_tls_files(&tls_config.cert_path, &tls_config.key_path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

            let rustls_config = load_tls_config(tls_config)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

            return server
                .bind_rustls_0_23(&bind_addr, rustls_config)?
                .run()
                .await;
        }

        // Non-TLS binding
        server.bind(&bind_addr)?.run().await
    }
}

/// Health check endpoint
/// Per spec-kit/006-api-spec.md: GET /api/v1/health
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Create a new session
/// Per spec-kit/006-api-spec.md: POST /api/v1/sessions
/// Per spec-kit/011-authentication-spec.md: Extract user from JWT
async fn create_session(
    req: HttpRequest,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse> {
    // Extract user context from authenticated request
    // Per spec-kit/011-authentication-spec.md: UserContext in request extensions
    use actix_web::HttpMessage;
    let user_id = req
        .extensions()
        .get::<UserContext>()
        .map(|ctx| ctx.user_id.clone())
        .unwrap_or_else(|| "unknown_user".to_string().into());

    match session_manager.create_session(user_id).await {
        Ok(session) => {
            tracing::info!("Created session: {}", session.id);
            Ok(HttpResponse::Created().json(serde_json::json!({
                "session_id": session.id.as_str(),
            })))
        }
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

/// Get session information
/// Per spec-kit/006-api-spec.md: GET /api/v1/sessions/{id}
async fn get_session(
    session_manager: web::Data<SessionManager>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id: SessionId = path.into_inner().into();

    match session_manager.get_session(&session_id).await {
        Ok(session) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "session_id": session.id.as_str(),
            "user_id": session.user_id.as_str(),
        }))),
        Err(e) => {
            tracing::error!("Failed to get session: {}", e);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

/// Delete a session
/// Per spec-kit/006-api-spec.md: DELETE /api/v1/sessions/{id}
async fn delete_session(
    session_manager: web::Data<SessionManager>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id: SessionId = path.into_inner().into();

    match session_manager.destroy_session(&session_id).await {
        Ok(_) => {
            tracing::info!("Deleted session: {}", session_id);
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            tracing::error!("Failed to delete session: {}", e);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

/// Get session command history
/// Per spec-kit/006-api-spec.md: GET /api/v1/sessions/{id}/history
async fn get_session_history(
    session_manager: web::Data<SessionManager>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id: SessionId = path.into_inner().into();

    match session_manager.get_session(&session_id).await {
        Ok(session) => {
            let history = session.get_history().await;
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "session_id": session.id.as_str(),
                "history": history,
            })))
        }
        Err(e) => {
            tracing::error!("Failed to get session history: {}", e);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

/// WebSocket handler
/// Per spec-kit/007-websocket-spec.md: WebSocket endpoint at /ws
/// Per spec-kit/011-authentication-spec.md: WebSocket authentication via Authenticate message
/// CRITICAL: Relative path /ws (no hardcoded host/port)
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    session_manager: web::Data<SessionManager>,
    jwt_validator: web::Data<Arc<JwtValidator>>,
) -> Result<HttpResponse> {
    // Create temporary session for WebSocket
    // User will authenticate via Authenticate message
    // Per spec-kit/011-authentication-spec.md: Authentication required before processing
    let user_id = "pending_auth".to_string().into();

    match session_manager.create_session(user_id).await {
        Ok(session) => {
            let session_id = session.id.clone();
            tracing::info!(
                "WebSocket connection for session: {} (pending auth)",
                session_id
            );

            // Create PTY manager per-connection (avoids Sync issues)
            let pty_manager = PtyManager::with_defaults();

            let ws_session = WebSocketSession::new(
                session_id,
                session_manager.clone().into_inner(),
                pty_manager,
                (**jwt_validator).clone(),
            );

            ws::start(ws_session, &req, stream)
        }
        Err(e) => {
            tracing::error!("Failed to create session for WebSocket: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyConfig;
    use crate::session::SessionConfig;

    #[test]
    fn test_server_creation() {
        let config = Config::default();
        let session_manager = SessionManager::new(SessionConfig::default());
        let jwt_secret = b"test_secret_key_at_least_32_bytes_long";

        let server = Server::new(config, session_manager, jwt_secret);
        assert!(Arc::strong_count(&server.config) >= 1);
    }
}
