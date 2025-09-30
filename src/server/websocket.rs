// WebSocket handler for terminal sessions
// Per spec-kit/003-backend-spec.md section 1.2

use actix::{Actor, ActorContext, ActorFutureExt, AsyncContext, StreamHandler, WrapFuture};
use actix_web_actors::ws;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::block_in_place;
use crate::protocol::{ClientMessage, ConnectionStatus, ServerMessage, Signal};
use crate::pty::PtyManager;
use crate::server::middleware::auth::UserContext;
use crate::security::jwt_validator::JwtValidator;
use crate::session::{SessionId, SessionManager};

/// WebSocket session actor
///
/// Per FR-3.3: Real-time streaming via WebSocket
/// Per spec-kit/007-websocket-spec.md
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
    /// Output receiver channel
    output_rx: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
    /// User context from authenticated JWT
    /// Per spec-kit/011-authentication-spec.md: Authentication required before processing
    user_context: Option<UserContext>,
    /// JWT validator for token authentication
    jwt_validator: Arc<JwtValidator>,
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
            output_rx: None,
            user_context: None,
            jwt_validator,
        }
    }

    /// Authenticate WebSocket connection with JWT token
    /// Per spec-kit/011-authentication-spec.md: WebSocket authentication flow
    fn authenticate(&mut self, token: String, ctx: &mut ws::WebsocketContext<Self>) {
        let validator = self.jwt_validator.clone();
        let session_id_clone = self.session_id.clone();

        // Spawn async validation task
        ctx.spawn(
            async move {
                validator.validate(&token).await
            }
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
                        };
                        if let Ok(json) = serde_json::to_string(&msg) {
                            ctx.text(json);
                        }

                        actor.user_context = Some(user_context);
                    }
                    Err(e) => {
                        tracing::warn!("WebSocket authentication failed: {}", e);
                        let msg = ServerMessage::Error {
                            message: "Authentication failed: Invalid or expired token".to_string(),
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
            let msg = ServerMessage::Error {
                message: "Authentication required. Send authenticate message first.".to_string(),
            };
            if let Ok(json) = serde_json::to_string(&msg) {
                ctx.text(json);
            }
            return false;
        }
        true
    }

    /// Start heartbeat task
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            // Check if client timed out
            if Instant::now().duration_since(act.last_heartbeat) > Duration::from_secs(30) {
                tracing::warn!("WebSocket heartbeat timeout for session {}", act.session_id);
                ctx.stop();
                return;
            }

            // Send ping
            ctx.ping(b"");
        });
    }

    /// Initialize PTY for this session
    async fn initialize_pty(&mut self, _ctx: &mut ws::WebsocketContext<Self>) -> crate::Result<()> {
        // Spawn PTY process
        let handle = self.pty_manager.spawn(None)?;
        let pty_id = handle.id().to_string();

        // Set PTY ID in session
        if let Ok(session) = self.session_manager.get_session(&self.session_id).await {
            session.set_pty(pty_id.clone()).await;
        }

        // Create output channel
        let (tx, rx) = mpsc::unbounded_channel();
        self.output_rx = Some(rx);

        // Start streaming output
        // TODO: Implement streaming without cloning manager
        // For now, we'll poll output in poll_output method

        self.pty_id = Some(pty_id);

        tracing::info!("Initialized PTY for session {}", self.session_id);
        Ok(())
    }

    /// Handle client command
    fn handle_command(&mut self, data: String, _ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error("PTY not initialized", _ctx);
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
            }
        }
    }

    /// Handle terminal resize
    fn handle_resize(&mut self, cols: u16, rows: u16, _ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error("PTY not initialized", _ctx);
                return;
            }
        };

        // Call resize directly (it's sync in current impl)
        if let Err(e) = block_in_place(|| {
            Handle::current().block_on(self.pty_manager.resize(&pty_id, cols, rows))
        }) {
            tracing::error!("Failed to resize PTY: {}", e);
        }
    }

    /// Handle signal
    fn handle_signal(&mut self, signal: Signal, _ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error("PTY not initialized", _ctx);
                return;
            }
        };

        // Handle signal directly
        match signal {
            Signal::SIGINT | Signal::SIGTERM | Signal::SIGKILL => {
                if let Err(e) = block_in_place(|| {
                    Handle::current().block_on(self.pty_manager.kill(&pty_id))
                }) {
                    tracing::error!("Failed to kill PTY: {}", e);
                }
            }
        }
    }

    /// Send error message to client
    fn send_error(&self, message: &str, ctx: &mut ws::WebsocketContext<Self>) {
        let msg = ServerMessage::Error {
            message: message.to_string(),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            ctx.text(json);
        }
    }

    /// Poll output receiver for PTY output
    fn poll_output(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        if let Some(ref mut rx) = self.output_rx {
            ctx.run_interval(Duration::from_millis(10), |act, ctx| {
                if let Some(ref mut rx) = act.output_rx {
                    // Try to receive up to 100 messages
                    for _ in 0..100 {
                        match rx.try_recv() {
                            Ok(data) => {
                                // Convert bytes to string
                                let output = String::from_utf8_lossy(&data).to_string();
                                let msg = ServerMessage::Output { data: output };

                                if let Ok(json) = serde_json::to_string(&msg) {
                                    ctx.text(json);
                                }
                            }
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                tracing::info!("PTY output channel disconnected");
                                ctx.stop();
                                return;
                            }
                        }
                    }
                }
            });
        }
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("WebSocket session started: {}", self.session_id);

        // Start heartbeat
        self.start_heartbeat(ctx);

        // Initialize PTY
        let session_id = self.session_id.clone();
        let fut = async move {
            // Initialization happens here
        };

        // Initialize PTY in started method
        // This will be done via a message or direct initialization

        // Send connection status
        let msg = ServerMessage::ConnectionStatus {
            status: ConnectionStatus::Connected,
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
            if let Err(e) = block_in_place(|| {
                Handle::current().block_on(self.pty_manager.kill(&pty_id))
            }) {
                tracing::error!("Failed to kill PTY on session close: {}", e);
            }
        }
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                // Parse client message
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Authenticate { token }) => {
                        // Per spec-kit/011-authentication-spec.md: Process authenticate message
                        self.authenticate(token, ctx);
                    }
                    Ok(ClientMessage::Command { data }) => {
                        // Per spec-kit/011-authentication-spec.md: Require auth before commands
                        if !self.require_auth(ctx) {
                            return;
                        }
                        self.handle_command(data, ctx);
                    }
                    Ok(ClientMessage::Resize { cols, rows }) => {
                        // Per spec-kit/011-authentication-spec.md: Require auth before resize
                        if !self.require_auth(ctx) {
                            return;
                        }
                        self.handle_resize(cols, rows, ctx);
                    }
                    Ok(ClientMessage::Signal { signal }) => {
                        // Per spec-kit/011-authentication-spec.md: Require auth before signals
                        if !self.require_auth(ctx) {
                            return;
                        }
                        self.handle_signal(signal, ctx);
                    }
                    Ok(ClientMessage::Ping) => {
                        self.last_heartbeat = Instant::now();
                        let msg = ServerMessage::Pong;
                        if let Ok(json) = serde_json::to_string(&msg) {
                            ctx.text(json);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse client message: {}", e);
                        self.send_error("Invalid message format", ctx);
                    }
                }
            }
            Ok(ws::Message::Binary(bin)) => {
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