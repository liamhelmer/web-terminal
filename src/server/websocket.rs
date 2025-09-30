// WebSocket handler for terminal sessions
// Per spec-kit/007-websocket-spec.md
// Per spec-kit/011-authentication-spec.md: WebSocket authentication

use actix::{Actor, ActorContext, ActorFutureExt, AsyncContext, StreamHandler, WrapFuture};
use actix_web_actors::ws;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::protocol::{error_codes, ClientMessage, ConnectionStatus, ServerMessage, Signal};
use crate::pty::PtyManager;
use crate::server::middleware::auth::UserContext;
use crate::security::jwt_validator::JwtValidator;
use crate::session::{SessionId, SessionManager};

/// Heartbeat interval: 5 seconds
/// Per spec-kit/007-websocket-spec.md: Heartbeat mechanism
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// Client timeout: 30 seconds
/// Per spec-kit/007-websocket-spec.md: Connection timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum message size: 1 MB
/// Per spec-kit/007-websocket-spec.md: Message validation
const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// WebSocket session actor
///
/// Per FR-3.3: Real-time streaming via WebSocket
/// Per spec-kit/007-websocket-spec.md: WebSocket protocol
/// Per spec-kit/011-authentication-spec.md: WebSocket authentication
pub struct WebSocketSession {
    /// Session ID for this WebSocket connection
    session_id: SessionId,
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// PTY manager (owned per-connection)
    pty_manager: PtyManager,
    /// PTY process ID
    pty_id: Option<String>,
    /// Last heartbeat timestamp
    last_heartbeat: Instant,
    /// User context from authenticated JWT
    /// Per spec-kit/011-authentication-spec.md: Authentication required before processing
    user_context: Option<UserContext>,
    /// JWT validator for token authentication
    jwt_validator: Arc<JwtValidator>,
    /// Authentication timeout flag
    auth_timeout_scheduled: bool,
}

impl WebSocketSession {
    /// Create a new WebSocket session
    /// Per spec-kit/011-authentication-spec.md: Authentication required
    pub fn new(
        session_id: SessionId,
        session_manager: Arc<SessionManager>,
        pty_manager: PtyManager,
        jwt_validator: Arc<JwtValidator>,
    ) -> Self {
        Self {
            session_id,
            session_manager,
            pty_manager,
            pty_id: None,
            last_heartbeat: Instant::now(),
            user_context: None,
            jwt_validator,
            auth_timeout_scheduled: false,
        }
    }

    /// Authenticate WebSocket connection with JWT token
    /// Per spec-kit/011-authentication-spec.md: WebSocket authentication flow
    fn authenticate(&mut self, token: String, ctx: &mut ws::WebsocketContext<Self>) {
        let validator = self.jwt_validator.clone();
        let session_id_clone = self.session_id.clone();

        // Spawn async validation task
        ctx.spawn(
            async move { validator.validate(&token).await }
                .into_actor(self)
                .map(move |result, actor, ctx| {
                    match result {
                        Ok(validated_token) => {
                            let user_context = UserContext::from_claims(
                                validated_token.claims,
                                validated_token.provider,
                            );
                            tracing::info!(
                                "WebSocket authenticated: user={}, session={}",
                                user_context.user_id.as_str(),
                                session_id_clone
                            );

                            // Send authenticated message
                            let msg = ServerMessage::Authenticated {
                                user_id: user_context.user_id.as_str().to_string(),
                                email: user_context.email.clone(),
                                groups: Some(
                                    user_context
                                        .groups
                                        .iter()
                                        .map(|g| g.as_str().to_string())
                                        .collect(),
                                ),
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                ctx.text(json);
                            }

                            actor.user_context = Some(user_context);
                        }
                        Err(e) => {
                            tracing::warn!("WebSocket authentication failed: {}", e);
                            let msg = ServerMessage::Error {
                                code: error_codes::AUTHENTICATION_FAILED.to_string(),
                                message: "Authentication failed: Invalid or expired token"
                                    .to_string(),
                                details: None,
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                ctx.text(json);
                            }
                            ctx.close(Some(ws::CloseCode::Policy.into()));
                        }
                    }
                }),
        );
    }

    /// Check if WebSocket is authenticated
    /// Per spec-kit/011-authentication-spec.md: Require authentication before processing
    fn require_auth(&self, ctx: &mut ws::WebsocketContext<Self>) -> bool {
        if self.user_context.is_none() {
            tracing::warn!("Unauthenticated WebSocket message rejected");
            self.send_error(
                error_codes::AUTHENTICATION_REQUIRED,
                "Authentication required. Send authenticate message first.",
                ctx,
            );
            return false;
        }
        true
    }

    /// Start heartbeat task
    /// Per spec-kit/007-websocket-spec.md: Heartbeat mechanism (5s interval, 30s timeout)
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check if client timed out
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                tracing::warn!(
                    "WebSocket heartbeat timeout for session {}",
                    act.session_id
                );
                ctx.stop();
                return;
            }

            // Send ping
            ctx.ping(b"");
        });
    }

    /// Schedule authentication timeout
    /// Per spec-kit/007-websocket-spec.md: Must authenticate within 30 seconds
    fn schedule_auth_timeout(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        if !self.auth_timeout_scheduled {
            self.auth_timeout_scheduled = true;
            ctx.run_later(CLIENT_TIMEOUT, |act, ctx| {
                if act.user_context.is_none() {
                    tracing::warn!(
                        "WebSocket authentication timeout for session {}",
                        act.session_id
                    );
                    act.send_error(
                        error_codes::AUTHENTICATION_REQUIRED,
                        "Authentication timeout. Please authenticate within 30 seconds.",
                        ctx,
                    );
                    ctx.close(Some(ws::CloseCode::Policy.into()));
                }
            });
        }
    }

    /// Handle client command
    /// Per spec-kit/007-websocket-spec.md: Command execution
    fn handle_command(&mut self, data: String, ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error(error_codes::INTERNAL_ERROR, "PTY not initialized", ctx);
                return;
            }
        };

        // Write command to PTY
        match self.pty_manager.create_writer(&pty_id) {
            Ok(mut writer) => {
                // Spawn async write task
                actix_web::rt::spawn(async move {
                    if let Err(e) = writer.write(data.as_bytes()).await {
                        tracing::error!("Failed to write to PTY: {}", e);
                    }
                });
            }
            Err(e) => {
                tracing::error!("Failed to create PTY writer: {}", e);
                self.send_error(error_codes::INTERNAL_ERROR, &e.to_string(), ctx);
            }
        }
    }

    /// Handle terminal resize
    /// Per spec-kit/007-websocket-spec.md: Terminal resize
    fn handle_resize(&mut self, cols: u16, rows: u16, ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error(error_codes::INTERNAL_ERROR, "PTY not initialized", ctx);
                return;
            }
        };

        let pty_manager = &self.pty_manager;
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                pty_manager.resize(&pty_id, cols, rows).await
            })
        });

        if let Err(e) = result {
            tracing::error!("Failed to resize PTY: {}", e);
            self.send_error(error_codes::INTERNAL_ERROR, &e.to_string(), ctx);
        }
    }

    /// Handle signal
    /// Per spec-kit/007-websocket-spec.md: Send signal to process
    fn handle_signal(&mut self, signal: Signal, ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error(error_codes::INTERNAL_ERROR, "PTY not initialized", ctx);
                return;
            }
        };

        // Handle signal directly by killing the PTY
        let pty_manager = &self.pty_manager;
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match signal {
                    Signal::SIGINT | Signal::SIGTERM | Signal::SIGKILL => {
                        pty_manager.kill(&pty_id).await
                    }
                }
            })
        });

        if let Err(e) = result {
            tracing::error!("Failed to send signal to PTY: {}", e);
            self.send_error(error_codes::COMMAND_KILLED, &e.to_string(), ctx);
        }
    }

    /// Handle environment variable set
    /// Per spec-kit/007-websocket-spec.md: Environment variable management
    fn handle_env_set(&mut self, key: String, value: String, ctx: &mut ws::WebsocketContext<Self>) {
        let session_manager = self.session_manager.clone();
        let session_id = self.session_id.clone();

        ctx.spawn(
            async move {
                if let Ok(session) = session_manager.get_session(&session_id).await {
                    session.set_env(key.clone(), value.clone()).await;
                    Ok((key, value))
                } else {
                    Err(crate::error::Error::SessionNotFound(session_id.to_string()))
                }
            }
            .into_actor(self)
            .map(move |result, _actor, ctx| match result {
                Ok((key, value)) => {
                    let msg = ServerMessage::EnvUpdated { key, value };
                    if let Ok(json) = serde_json::to_string(&msg) {
                        ctx.text(json);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to set environment variable: {}", e);
                }
            }),
        );
    }

    /// Handle change directory
    /// Per spec-kit/007-websocket-spec.md: Working directory management
    fn handle_chdir(&mut self, path: String, ctx: &mut ws::WebsocketContext<Self>) {
        let session_manager = self.session_manager.clone();
        let session_id = self.session_id.clone();

        ctx.spawn(
            async move {
                if let Ok(session) = session_manager.get_session(&session_id).await {
                    let path_buf = std::path::PathBuf::from(&path);
                    session.update_working_dir(path_buf.clone()).await?;
                    Ok(path)
                } else {
                    Err(crate::error::Error::SessionNotFound(session_id.to_string()))
                }
            }
            .into_actor(self)
            .map(move |result, actor, ctx| match result {
                Ok(path) => {
                    let msg = ServerMessage::CwdChanged { path };
                    if let Ok(json) = serde_json::to_string(&msg) {
                        ctx.text(json);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to change directory: {}", e);
                    actor.send_error(error_codes::PATH_INVALID, &e.to_string(), ctx);
                }
            }),
        );
    }

    /// Handle echo test message
    /// Per spec-kit/007-websocket-spec.md: Testing protocol
    fn handle_echo(&self, data: String, ctx: &mut ws::WebsocketContext<Self>) {
        let msg = ServerMessage::Echo { data };
        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    /// Send error message to client
    /// Per spec-kit/007-websocket-spec.md: Error responses
    fn send_error(&self, code: &str, message: &str, ctx: &mut ws::WebsocketContext<Self>) {
        let msg = ServerMessage::Error {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    /// Send acknowledgment
    /// Per spec-kit/007-websocket-spec.md: Message acknowledgment
    fn send_ack(&self, message_id: Option<String>, ctx: &mut ws::WebsocketContext<Self>) {
        let msg = ServerMessage::Ack { message_id };
        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    /// Validate message size
    /// Per spec-kit/007-websocket-spec.md: Maximum 1 MB per message
    fn validate_message_size(&self, size: usize, ctx: &mut ws::WebsocketContext<Self>) -> bool {
        if size > MAX_MESSAGE_SIZE {
            tracing::warn!("Message size {} exceeds maximum {}", size, MAX_MESSAGE_SIZE);
            self.send_error(
                error_codes::INVALID_MESSAGE,
                &format!("Message size exceeds maximum of {} bytes", MAX_MESSAGE_SIZE),
                ctx,
            );
            return false;
        }
        true
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("WebSocket session started: {}", self.session_id);

        // Start heartbeat
        self.start_heartbeat(ctx);

        // Schedule authentication timeout
        self.schedule_auth_timeout(ctx);

        // Send connection status
        let msg = ServerMessage::ConnectionStatus {
            status: ConnectionStatus::Connected,
            session_id: Some(self.session_id.to_string()),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("WebSocket session stopped: {}", self.session_id);

        // Clean up PTY
        if let Some(pty_id) = &self.pty_id {
            let pty_id = pty_id.clone();
            let pty_manager = &self.pty_manager;
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    if let Err(e) = pty_manager.kill(&pty_id).await {
                        tracing::error!("Failed to kill PTY on session close: {}", e);
                    }
                })
            });
        }
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(
        &mut self,
        msg: std::result::Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                // Validate message size
                if !self.validate_message_size(text.len(), ctx) {
                    return;
                }

                // Parse client message
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        // Validate message
                        if let Err(e) = client_msg.validate() {
                            tracing::warn!("Message validation failed: {}", e);
                            self.send_error(
                                error_codes::INVALID_MESSAGE,
                                &format!("Message validation failed: {}", e),
                                ctx,
                            );
                            return;
                        }

                        // Handle message
                        match client_msg {
                            ClientMessage::Authenticate { token } => {
                                self.authenticate(token, ctx);
                            }
                            ClientMessage::Command { data } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                self.handle_command(data, ctx);
                            }
                            ClientMessage::Resize { cols, rows } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                self.handle_resize(cols, rows, ctx);
                            }
                            ClientMessage::Signal { signal } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                self.handle_signal(signal, ctx);
                            }
                            ClientMessage::EnvSet { key, value } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                self.handle_env_set(key, value, ctx);
                            }
                            ClientMessage::Chdir { path } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                self.handle_chdir(path, ctx);
                            }
                            ClientMessage::FileUploadStart { .. } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                // TODO: Implement file upload
                                self.send_error(
                                    error_codes::INTERNAL_ERROR,
                                    "File upload not yet implemented",
                                    ctx,
                                );
                            }
                            ClientMessage::FileUploadComplete { .. } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                // TODO: Implement file upload
                            }
                            ClientMessage::FileDownload { .. } => {
                                if !self.require_auth(ctx) {
                                    return;
                                }
                                // TODO: Implement file download
                                self.send_error(
                                    error_codes::INTERNAL_ERROR,
                                    "File download not yet implemented",
                                    ctx,
                                );
                            }
                            ClientMessage::Ping => {
                                self.last_heartbeat = Instant::now();
                                let msg = ServerMessage::Pong {
                                    timestamp: Some(
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_millis() as u64,
                                    ),
                                    latency_ms: None,
                                };
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    ctx.text(json);
                                }
                            }
                            ClientMessage::Echo { data } => {
                                self.handle_echo(data, ctx);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse client message: {}", e);
                        self.send_error(
                            error_codes::INVALID_MESSAGE,
                            &format!("Invalid message format: {}", e),
                            ctx,
                        );
                    }
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                // Validate message size
                if !self.validate_message_size(bin.len(), ctx) {
                    return;
                }

                if !self.require_auth(ctx) {
                    return;
                }

                // Handle binary data (write directly to PTY)
                if let Some(pty_id) = &self.pty_id {
                    let pty_id = pty_id.clone();
                    match self.pty_manager.create_writer(&pty_id) {
                        Ok(mut writer) => {
                            actix_web::rt::spawn(async move {
                                if let Err(e) = writer.write(&bin).await {
                                    tracing::error!("Failed to write binary to PTY: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to create PTY writer: {}", e);
                        }
                    }
                }
            }
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                tracing::info!("WebSocket close: {:?}", reason);
                ctx.close(reason);
                ctx.stop();
            }
            Err(e) => {
                tracing::error!("WebSocket protocol error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::jwks_client::JwksClient;
    use crate::session::manager::SessionConfig;

    #[test]
    fn test_websocket_session_creation() {
        let session_id = SessionId::generate();
        let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));
        let pty_manager = PtyManager::with_defaults();
        let config = crate::config::Config::default();
        let jwks_client = Arc::new(JwksClient::new(config.auth.clone()));
        let jwt_validator = Arc::new(JwtValidator::new(jwks_client, config.auth));

        let _ws_session =
            WebSocketSession::new(session_id, session_manager, pty_manager, jwt_validator);
    }
}