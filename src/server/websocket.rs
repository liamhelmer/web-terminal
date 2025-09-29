// WebSocket handler for terminal sessions
// Per spec-kit/003-backend-spec.md section 1.2

use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web_actors::ws;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::error::Error;
use crate::protocol::{ClientMessage, ConnectionStatus, ServerMessage, Signal};
use crate::pty::PtyManager;
use crate::session::{SessionId, SessionManager};

/// WebSocket session actor
///
/// Per FR-3.3: Real-time streaming via WebSocket
/// Per spec-kit/007-websocket-spec.md
pub struct WebSocketSession {
    /// Session ID for this WebSocket connection
    session_id: SessionId,
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// PTY manager
    pty_manager: Arc<PtyManager>,
    /// PTY process ID
    pty_id: Option<String>,
    /// Last heartbeat timestamp
    last_heartbeat: Instant,
    /// Output receiver channel
    output_rx: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl WebSocketSession {
    /// Create a new WebSocket session
    pub fn new(
        session_id: SessionId,
        session_manager: Arc<SessionManager>,
        pty_manager: Arc<PtyManager>,
    ) -> Self {
        Self {
            session_id,
            session_manager,
            pty_manager,
            pty_id: None,
            last_heartbeat: Instant::now(),
            output_rx: None,
        }
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
    async fn initialize_pty(&mut self, ctx: &mut ws::WebsocketContext<Self>) -> crate::Result<()> {
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

        // Start streaming output (using actix_web::rt::spawn for Send requirements)
        let pty_manager = self.pty_manager.clone();
        let pty_id_clone = pty_id.clone();
        actix_web::rt::spawn(async move {
            if let Err(e) = pty_manager.stream_output(&pty_id_clone, tx).await {
                tracing::error!("Failed to stream PTY output: {}", e);
            }
        });

        self.pty_id = Some(pty_id);

        tracing::info!("Initialized PTY for session {}", self.session_id);
        Ok(())
    }

    /// Handle client command
    fn handle_command(&mut self, data: String, ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error("PTY not initialized", ctx);
                return;
            }
        };

        // Write command to PTY
        let pty_manager = self.pty_manager.clone();
        let fut = async move {
            match pty_manager.create_writer(&pty_id) {
                Ok(mut writer) => {
                    if let Err(e) = writer.write(data.as_bytes()).await {
                        tracing::error!("Failed to write to PTY: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create PTY writer: {}", e);
                }
            }
        };

        ctx.spawn(actix::fut::wrap_future(fut));
    }

    /// Handle terminal resize
    fn handle_resize(&self, cols: u16, rows: u16, ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error("PTY not initialized", ctx);
                return;
            }
        };

        let pty_manager = self.pty_manager.clone();
        let fut = async move {
            if let Err(e) = pty_manager.resize(&pty_id, cols, rows).await {
                tracing::error!("Failed to resize PTY: {}", e);
            }
        };

        ctx.spawn(actix::fut::wrap_future(fut));
    }

    /// Handle signal
    fn handle_signal(&self, signal: Signal, ctx: &mut ws::WebsocketContext<Self>) {
        let pty_id = match &self.pty_id {
            Some(id) => id.clone(),
            None => {
                self.send_error("PTY not initialized", ctx);
                return;
            }
        };

        let pty_manager = self.pty_manager.clone();
        let fut = async move {
            match signal {
                Signal::SIGINT | Signal::SIGTERM | Signal::SIGKILL => {
                    if let Err(e) = pty_manager.kill(&pty_id).await {
                        tracing::error!("Failed to kill PTY: {}", e);
                    }
                }
            }
        };

        ctx.spawn(actix::fut::wrap_future(fut));
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

        // Clean up PTY (using actix_web::rt::spawn)
        if let Some(pty_id) = &self.pty_id {
            let pty_manager = self.pty_manager.clone();
            let pty_id = pty_id.clone();
            actix_web::rt::spawn(async move {
                if let Err(e) = pty_manager.kill(&pty_id).await {
                    tracing::error!("Failed to kill PTY on session close: {}", e);
                }
            });
        }
    }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                // Parse client message
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Command { data }) => {
                        self.handle_command(data, ctx);
                    }
                    Ok(ClientMessage::Resize { cols, rows }) => {
                        self.handle_resize(cols, rows, ctx);
                    }
                    Ok(ClientMessage::Signal { signal }) => {
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
                    let pty_manager = self.pty_manager.clone();
                    let pty_id = pty_id.clone();
                    let fut = async move {
                        match pty_manager.create_writer(&pty_id) {
                            Ok(mut writer) => {
                                if let Err(e) = writer.write(&bin).await {
                                    tracing::error!("Failed to write binary to PTY: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to create PTY writer: {}", e);
                            }
                        }
                    };
                    ctx.spawn(actix::fut::wrap_future(fut));
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