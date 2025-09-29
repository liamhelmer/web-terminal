// HTTP server implementation
// Per spec-kit/003-backend-spec.md section 1.1

use std::sync::Arc;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web_actors::ws;
use actix_files::Files;

use crate::config::Config;
use crate::pty::PtyManager;
use crate::session::{SessionId, SessionManager};
use crate::server::websocket::WebSocketSession;

/// HTTP server instance
/// Per spec-kit/003-backend-spec.md: Single-port architecture
/// All HTTP, WebSocket, and static assets served from ONE port (default 8080)
pub struct Server {
    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
}

impl Server {
    /// Create a new HTTP server instance
    /// Per spec-kit/003-backend-spec.md section 1.1
    pub fn new(
        config: Config,
        session_manager: SessionManager,
    ) -> Self {
        tracing::info!(
            "Initializing HTTP server on {}:{}",
            config.server.host,
            config.server.port
        );

        Self {
            config: Arc::new(config),
            session_manager: Arc::new(session_manager),
        }
    }

    /// Start the HTTP server
    /// Per spec-kit/003-backend-spec.md: Single-port deployment
    /// CRITICAL: All services (HTTP, WebSocket, static files) on ONE port
    pub async fn run(self) -> std::io::Result<()> {
        let bind_addr = format!("{}:{}", self.config.server.host, self.config.server.port);

        tracing::info!("Starting server on {}", bind_addr);
        tracing::info!("WebSocket endpoint: ws://{}/ws", bind_addr);
        tracing::info!("Health check: http://{}/api/v1/health", bind_addr);

        let session_manager = self.session_manager.clone();

        HttpServer::new(move || {
            App::new()
                // Shared application state
                .app_data(web::Data::new(session_manager.clone()))
                // Middleware
                .wrap(tracing_actix_web::TracingLogger::default())
                // API routes
                .service(
                    web::scope("/api/v1")
                        .route("/health", web::get().to(health_check))
                        .route("/sessions", web::post().to(create_session))
                        .route("/sessions/{id}", web::get().to(get_session))
                        .route("/sessions/{id}", web::delete().to(delete_session))
                        .route("/sessions/{id}/history", web::get().to(get_session_history))
                )
                // WebSocket endpoint (relative path per spec)
                .route("/ws", web::get().to(websocket_handler))
                // Static files served from same port
                .service(Files::new("/", "./static").index_file("index.html"))
        })
        .bind(&bind_addr)?
        .workers(self.config.server.worker_threads)
        .run()
        .await
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
async fn create_session(
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse> {
    // TODO: Extract user_id from JWT token in Authorization header
    // For now, use a dummy user_id
    let user_id = "test_user".to_string().into();

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
        Ok(session) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "session_id": session.id.as_str(),
                "user_id": session.user_id.as_str(),
            })))
        }
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
/// CRITICAL: Relative path /ws (no hardcoded host/port)
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse> {
    // TODO: Extract session_id from query parameter or JWT token
    // For now, create a new session
    let user_id = "test_user".to_string().into();

    match session_manager.create_session(user_id).await {
        Ok(session) => {
            let session_id = session.id.clone();
            tracing::info!("WebSocket connection for session: {}", session_id);

            // Create PTY manager per-connection (avoids Sync issues)
            let pty_manager = PtyManager::with_defaults();

            let ws_session = WebSocketSession::new(
                session_id,
                session_manager.into_inner(),
                pty_manager,
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
    use crate::session::SessionConfig;
    use crate::pty::PtyConfig;

    #[test]
    fn test_server_creation() {
        let config = Config::default();
        let session_manager = SessionManager::new(SessionConfig::default());

        let server = Server::new(config, session_manager);
        assert!(Arc::strong_count(&server.config) >= 1);
    }
}