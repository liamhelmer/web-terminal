# Web-Terminal: Rust Backend Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29
**References:** [002-architecture.md](./002-architecture.md)

---

## Table of Contents

1. [Module Structure](#module-structure)
2. [Core Components](#core-components)
3. [API Endpoints](#api-endpoints)
4. [Data Models](#data-models)
5. [Error Handling](#error-handling)
6. [Configuration](#configuration)
7. [Performance Optimizations](#performance-optimizations)

---

## Module Structure

```
src/
├── main.rs                    # Application entry point
├── lib.rs                     # Library root
├── config/
│   ├── mod.rs
│   ├── server.rs             # Server configuration
│   └── security.rs           # Security configuration
├── server/
│   ├── mod.rs
│   ├── http.rs               # HTTP server setup
│   ├── websocket.rs          # WebSocket handler
│   └── middleware.rs         # Auth, logging middleware
├── session/
│   ├── mod.rs
│   ├── manager.rs            # Session lifecycle
│   ├── state.rs              # Session state
│   └── registry.rs           # Session registry
├── execution/
│   ├── mod.rs
│   ├── executor.rs           # Command execution
│   ├── process.rs            # Process management
│   └── environment.rs        # Environment variables
├── filesystem/
│   ├── mod.rs
│   ├── vfs.rs                # Virtual file system
│   ├── quota.rs              # Storage quotas
│   └── operations.rs         # File operations
├── security/
│   ├── mod.rs
│   ├── auth.rs               # Authentication
│   ├── sandbox.rs            # Sandboxing
│   ├── limits.rs             # Resource limits
│   └── validator.rs          # Input validation
├── protocol/
│   ├── mod.rs
│   ├── messages.rs           # Message types
│   └── codec.rs              # Serialization
├── monitoring/
│   ├── mod.rs
│   ├── metrics.rs            # Prometheus metrics
│   └── logging.rs            # Structured logging
└── error.rs                   # Error types
```

---

## Core Components

### 1. Server Module

#### 1.1 HTTP Server (src/server/http.rs)

```rust
use actix_web::{web, App, HttpServer};
use rustls::ServerConfig;

pub struct Server {
    config: ServerConfig,
    sessions: Arc<SessionManager>,
    auth: Arc<AuthService>,
}

impl Server {
    pub async fn new(config: ServerConfig) -> Result<Self> {
        let sessions = Arc::new(SessionManager::new());
        let auth = Arc::new(AuthService::new());

        Ok(Self {
            config,
            sessions,
            auth,
        })
    }

    pub async fn run(self) -> Result<()> {
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(self.sessions.clone()))
                .app_data(web::Data::new(self.auth.clone()))
                .wrap(middleware::Logger::default())
                .wrap(middleware::Auth::new(self.auth.clone()))
                .wrap(middleware::RateLimit::default())
                .service(routes::health)
                .service(routes::websocket)
                .service(actix_files::Files::new("/", "./static"))
        })
        .bind((self.config.host.as_str(), self.config.port))?
        .run()
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub max_connections: usize,
    pub worker_threads: usize,
}
```

#### 1.2 WebSocket Handler (src/server/websocket.rs)

```rust
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;

pub struct WebSocketSession {
    id: SessionId,
    manager: Arc<SessionManager>,
    executor: Arc<CommandExecutor>,
    last_heartbeat: Instant,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        tracing::info!("WebSocket session started: {}", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("WebSocket session stopped: {}", self.id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                self.handle_command(text, ctx);
            }
            Ok(ws::Message::Binary(bin)) => {
                self.handle_binary(bin, ctx);
            }
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl WebSocketSession {
    fn handle_command(&mut self, text: String, ctx: &mut ws::WebsocketContext<Self>) {
        let msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to parse message: {}", e);
                return;
            }
        };

        match msg {
            ClientMessage::Command { data } => {
                self.execute_command(data, ctx);
            }
            ClientMessage::Resize { cols, rows } => {
                self.resize_terminal(cols, rows);
            }
            ClientMessage::Signal { signal } => {
                self.send_signal(signal);
            }
        }
    }

    fn execute_command(&mut self, cmd: String, ctx: &mut ws::WebsocketContext<Self>) {
        let executor = self.executor.clone();
        let session_id = self.id.clone();

        let fut = async move {
            let result = executor.execute(session_id, cmd).await;
            match result {
                Ok(output) => ServerMessage::Output { data: output },
                Err(e) => ServerMessage::Error { message: e.to_string() },
            }
        };

        let fut = actix::fut::wrap_future(fut);
        ctx.spawn(fut.map(|msg, _act, ctx| {
            ctx.text(serde_json::to_string(&msg).unwrap());
        }));
    }

    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > Duration::from_secs(30) {
                tracing::warn!("WebSocket heartbeat timeout");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}
```

---

### 2. Session Module

#### 2.1 Session Manager (src/session/manager.rs)

```rust
use dashmap::DashMap;
use tokio::sync::RwLock;

pub struct SessionManager {
    sessions: DashMap<SessionId, Arc<Session>>,
    user_sessions: DashMap<UserId, Vec<SessionId>>,
    config: SessionConfig,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            user_sessions: DashMap::new(),
            config: SessionConfig::default(),
        }
    }

    pub async fn create_session(&self, user_id: UserId) -> Result<Session> {
        // Check user session limit
        if let Some(sessions) = self.user_sessions.get(&user_id) {
            if sessions.len() >= self.config.max_sessions_per_user {
                return Err(Error::SessionLimitExceeded);
            }
        }

        let session = Session::new(user_id, self.config.clone())?;
        let session_id = session.id.clone();

        // Store session
        self.sessions.insert(session_id.clone(), Arc::new(session.clone()));

        // Track user sessions
        self.user_sessions
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(session_id);

        tracing::info!("Created session {} for user {}", session.id, user_id);
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &SessionId) -> Result<Arc<Session>> {
        self.sessions
            .get(session_id)
            .map(|s| s.clone())
            .ok_or(Error::SessionNotFound)
    }

    pub async fn destroy_session(&self, session_id: &SessionId) -> Result<()> {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            // Kill all processes
            session.kill_all_processes().await?;

            // Clean up file system
            session.cleanup_filesystem().await?;

            // Remove from user sessions
            if let Some(mut user_sessions) = self.user_sessions.get_mut(&session.user_id) {
                user_sessions.retain(|id| id != session_id);
            }

            tracing::info!("Destroyed session {}", session_id);
            Ok(())
        } else {
            Err(Error::SessionNotFound)
        }
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let now = Instant::now();
        let mut expired = Vec::new();

        for entry in self.sessions.iter() {
            let session = entry.value();
            if now.duration_since(session.last_activity) > self.config.timeout {
                expired.push(entry.key().clone());
            }
        }

        let count = expired.len();
        for session_id in expired {
            self.destroy_session(&session_id).await?;
        }

        tracing::info!("Cleaned up {} expired sessions", count);
        Ok(count)
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub timeout: Duration,
    pub max_sessions_per_user: usize,
    pub workspace_quota: u64,
    pub max_processes: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30 * 60), // 30 minutes
            max_sessions_per_user: 10,
            workspace_quota: 1024 * 1024 * 1024, // 1GB
            max_processes: 10,
        }
    }
}
```

#### 2.2 Session State (src/session/state.rs)

```rust
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub created_at: Instant,
    pub last_activity: Instant,
    state: RwLock<SessionState>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
    pub command_history: Vec<String>,
    pub processes: HashMap<ProcessId, ProcessHandle>,
}

impl Session {
    pub fn new(user_id: UserId, config: SessionConfig) -> Result<Self> {
        let id = SessionId::generate();
        let working_dir = PathBuf::from(format!("/workspace/{}", id));

        // Create workspace directory
        std::fs::create_dir_all(&working_dir)?;

        let state = SessionState {
            working_dir,
            environment: Self::default_environment(),
            command_history: Vec::new(),
            processes: HashMap::new(),
        };

        Ok(Self {
            id,
            user_id,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            state: RwLock::new(state),
        })
    }

    pub async fn update_working_dir(&self, path: PathBuf) -> Result<()> {
        let mut state = self.state.write().await;

        // Validate path is within workspace
        if !path.starts_with(&state.working_dir) {
            return Err(Error::InvalidPath);
        }

        state.working_dir = path;
        Ok(())
    }

    pub async fn add_to_history(&self, command: String) {
        let mut state = self.state.write().await;
        state.command_history.push(command);

        // Limit history size
        if state.command_history.len() > 1000 {
            state.command_history.remove(0);
        }
    }

    pub async fn get_history(&self) -> Vec<String> {
        let state = self.state.read().await;
        state.command_history.clone()
    }

    fn default_environment() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env.insert("HOME".to_string(), "/workspace".to_string());
        env.insert("SHELL".to_string(), "/bin/bash".to_string());
        env
    }
}
```

---

### 3. Execution Module

#### 3.1 Command Executor (src/execution/executor.rs)

```rust
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct CommandExecutor {
    sandbox: Arc<SandboxManager>,
    limits: Arc<ResourceLimiter>,
}

impl CommandExecutor {
    pub fn new(sandbox: Arc<SandboxManager>, limits: Arc<ResourceLimiter>) -> Self {
        Self { sandbox, limits }
    }

    pub async fn execute(&self, session_id: SessionId, cmd: String) -> Result<CommandOutput> {
        // Validate command
        self.sandbox.validate_command(&cmd)?;

        // Check resource limits
        self.limits.check_limit(&session_id, Resource::Processes)?;

        // Parse command
        let parts = shell_words::split(&cmd)?;
        if parts.is_empty() {
            return Err(Error::EmptyCommand);
        }

        let program = &parts[0];
        let args = &parts[1..];

        // Create process
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream output
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        let mut output = CommandOutput::new();

        // Read output
        loop {
            tokio::select! {
                line = stdout_lines.next_line() => {
                    match line? {
                        Some(line) => output.stdout.push_str(&line),
                        None => break,
                    }
                }
                line = stderr_lines.next_line() => {
                    match line? {
                        Some(line) => output.stderr.push_str(&line),
                        None => {},
                    }
                }
            }
        }

        // Wait for completion
        let status = child.wait().await?;
        output.exit_code = status.code().unwrap_or(-1);

        Ok(output)
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    fn new() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }
    }
}
```

---

### 4. Security Module

#### 4.1 Authentication Service (src/security/auth.rs)

```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl AuthService {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation: Validation::default(),
        }
    }

    pub async fn authenticate(&self, credentials: Credentials) -> Result<AuthToken> {
        // Validate credentials (implement your auth logic)
        let user_id = self.validate_credentials(&credentials).await?;

        // Create JWT claims
        let claims = Claims {
            sub: user_id.to_string(),
            exp: (Utc::now() + Duration::hours(8)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        // Generate token
        let token = encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(AuthToken {
            access_token: token,
            expires_at: Instant::now() + Duration::from_secs(8 * 3600),
            user_id,
        })
    }

    pub async fn validate_token(&self, token: &str) -> Result<UserId> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &self.validation,
        )?;

        Ok(UserId::from_str(&token_data.claims.sub)?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}
```

---

## Data Models

### Protocol Messages

```rust
// src/protocol/messages.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Command {
        data: String,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Signal {
        signal: Signal,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Output {
        data: String,
    },
    Error {
        message: String,
    },
    ProcessExited {
        exit_code: i32,
    },
    ConnectionStatus {
        status: ConnectionStatus,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Signal {
    SIGINT = 2,
    SIGTERM = 15,
    SIGKILL = 9,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
}
```

---

## Error Handling

```rust
// src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Session not found")]
    SessionNotFound,

    #[error("Session limit exceeded")]
    SessionLimitExceeded,

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Command not allowed: {0}")]
    CommandNotAllowed(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Invalid token")]
    InvalidToken,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

---

## Configuration

```rust
// src/config/server.rs

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server: ServerConfig::from_env()?,
            session: SessionConfig::default(),
            security: SecurityConfig::from_env()?,
            logging: LoggingConfig::default(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,

    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_max_connections() -> usize {
    10000
}
```

---

## Performance Optimizations

### 1. Async I/O Throughout
- All I/O operations use Tokio async runtime
- No blocking operations in hot paths
- Efficient use of system threads

### 2. Zero-Copy Where Possible
- Use `Bytes` for buffer management
- Avoid unnecessary string allocations
- Stream output instead of buffering

### 3. Connection Pooling
- Reuse WebSocket connections
- Pool process executors
- Cache frequently accessed data

### 4. Memory Management
- Use `Arc` for shared data
- Minimize cloning with references
- Drop resources eagerly

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial backend specification |