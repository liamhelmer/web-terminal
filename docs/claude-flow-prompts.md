# Claude-Flow Implementation Prompts for web-terminal

This document contains step-by-step prompts for implementing the web-terminal project using claude-flow. Each prompt is designed to be executed sequentially, with clear objectives and validation criteria.

## 🔒 CRITICAL: Single-Port Architecture Mandate

**EVERY prompt in this document assumes and enforces a SINGLE-PORT architecture.**

### Non-Negotiable Requirements:
- ✅ ONE server process listening on ONE port (default: 8080)
- ✅ HTTP, WebSocket, AND static assets served from same port
- ✅ ALL paths must be relative (no hardcoded hosts/ports)
- ✅ Frontend dynamically determines base URL from window.location
- ✅ WebSocket URLs constructed from window.location
- ❌ NO separate development server ports (no webpack-dev-server on different port)
- ❌ NO hardcoded localhost:3000 or similar
- ❌ NO proxy assumptions requiring multiple ports

**If ANY prompt appears to violate this, flag it immediately.**

### Single-Port Validation Checklist (Use for EVERY prompt):
- [ ] No hardcoded ports in code
- [ ] All paths are relative
- [ ] Frontend detects base URL dynamically
- [ ] WebSocket URLs constructed from window.location
- [ ] No separate server processes
- [ ] Tests use single port only
- [ ] Dev workflow uses single backend server

## Table of Contents
1. [Phase 1: Project Setup](#phase-1-project-setup)
2. [Phase 2: Backend Core Implementation](#phase-2-backend-core-implementation)
3. [Phase 3: Frontend Implementation](#phase-3-frontend-implementation)
4. [Phase 4: Integration & Testing](#phase-4-integration--testing)
5. [Phase 5: Documentation & Polish](#phase-5-documentation--polish)

---

## Phase 1: Project Setup

### Prompt 1.1: Initialize Project Structure

```bash
npx claude-flow@alpha sparc run architect "Initialize web-terminal project structure"
```

**Detailed Instructions:**
```
Create the following project structure for web-terminal:

ROOT STRUCTURE:
web-terminal/
├── backend/          # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── server/
│   │   ├── session/
│   │   ├── pty/
│   │   ├── websocket/
│   │   └── api/
│   ├── Cargo.toml
│   └── tests/
├── frontend/         # WASM + JavaScript frontend
│   ├── src/
│   │   ├── wasm/    # Rust WASM code
│   │   ├── ts/      # TypeScript code
│   │   └── static/  # HTML, CSS, assets
│   ├── Cargo.toml
│   ├── package.json
│   └── webpack.config.js
├── tests/            # Playwright E2E tests
│   ├── e2e/
│   ├── fixtures/
│   └── playwright.config.ts
├── docs/             # Documentation (already exists)
└── scripts/          # Build and utility scripts

REQUIREMENTS:
1. Create all directories with .gitkeep files
2. Initialize Cargo workspaces for backend and frontend/wasm
3. Create package.json with latest dependencies from docs/dependencies-research.md
4. Create initial Cargo.toml files with workspace configuration
5. Set up .gitignore for Rust, Node, WASM artifacts
6. Create README.md with quick start instructions
7. No hardcoded ports or absolute paths

VALIDATION:
- All directories exist
- cargo check passes in both workspaces
- npm install succeeds
- No absolute paths in any config files

SINGLE-PORT VALIDATION:
- [ ] No hardcoded ports in code
- [ ] All paths are relative
- [ ] Frontend detects base URL dynamically
- [ ] WebSocket URLs constructed from window.location
- [ ] No separate server processes
- [ ] Tests use single port only
```

### Prompt 1.2: Configure Dependencies

```bash
npx claude-flow@alpha sparc run spec-pseudocode "Configure all project dependencies"
```

**Detailed Instructions:**
```
Based on docs/dependencies-research.md, configure all dependencies:

BACKEND CARGO.TOML:
[workspace]
members = ["backend", "frontend/wasm"]

[package]
name = "web-terminal"
version = "0.1.0"
edition = "2021"
rust-version = "1.70.0"

[dependencies]
# Use exact versions from dependencies-research.md:
tokio = { version = "1.47.1", features = ["full"] }
axum = { version = "0.8.5", features = ["ws", "macros"] }
tower-http = { version = "0.6.6", features = ["fs", "cors", "trace"] }
tokio-tungstenite = "0.27.0"
portable-pty = "0.9.0"
clap = { version = "4.5.48", features = ["derive", "env"] }
serde = { version = "1.0.221", features = ["derive"] }
serde_json = "1.0.143"
tracing = "0.1.41"
tracing-subscriber = { version = "0.3.20", features = ["env-filter"] }
uuid = { version = "1.11.0", features = ["v4", "serde"] }
thiserror = "2.1.0"
anyhow = "1.0.97"

[dev-dependencies]
tokio-test = "0.4.4"

FRONTEND PACKAGE.JSON:
{
  "name": "web-terminal-frontend",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "build": "webpack --mode production",
    "dev": "webpack serve --mode development",
    "wasm:build": "cd src/wasm && wasm-pack build --target web"
  },
  "dependencies": {
    "@xterm/xterm": "^5.5.0",
    "@xterm/addon-fit": "^0.10.0",
    "@xterm/addon-webgl": "^0.18.0",
    "@xterm/addon-web-links": "^0.11.0"
  },
  "devDependencies": {
    "webpack": "^5.101.3",
    "webpack-cli": "^6.0.1",
    "webpack-dev-server": "^5.2.0",
    "typescript": "^5.9.2",
    "ts-loader": "^9.5.2",
    "html-webpack-plugin": "^5.6.4",
    "@types/node": "^22.13.2"
  }
}

PLAYWRIGHT CONFIG:
Create tests/package.json with:
{
  "name": "web-terminal-tests",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "test": "playwright test",
    "test:ui": "playwright test --ui",
    "test:headed": "playwright test --headed"
  },
  "devDependencies": {
    "@playwright/test": "^1.55.0",
    "typescript": "^5.9.2"
  }
}

VALIDATION:
- cargo build compiles successfully
- npm install completes without errors
- All dependency versions match dependencies-research.md
- No deprecated packages included

SINGLE-PORT VALIDATION:
- [ ] No hardcoded ports in code
- [ ] All paths are relative
- [ ] Frontend detects base URL dynamically
- [ ] WebSocket URLs constructed from window.location
- [ ] No separate server processes
- [ ] Tests use single port only
```

### Prompt 1.3: Set Up Build Pipeline

```bash
npx claude-flow@alpha sparc run dev "Create build scripts and development workflow"
```

**Detailed Instructions:**
```
Create comprehensive build pipeline:

SCRIPTS/BUILD.SH:
#!/bin/bash
set -e

echo "Building web-terminal..."

# Build WASM frontend
echo "1. Building WASM module..."
cd frontend/src/wasm
wasm-pack build --target web --release
cd ../../..

# Build TypeScript/webpack frontend
echo "2. Building frontend..."
cd frontend
npm run build
cd ..

# Build Rust backend (embeds frontend assets)
echo "3. Building backend..."
cd backend
cargo build --release
cd ..

echo "✓ Build complete!"
echo "Binary: backend/target/release/web-terminal"

SCRIPTS/DEV.SH:
#!/bin/bash
# ✅ CRITICAL: Single-port development mode
# Backend serves EVERYTHING on port 8080 (API + WebSocket + static files)
# NO separate webpack-dev-server!

echo "🔒 Single-Port Development Mode"
echo "Building WASM module..."
cd frontend/src/wasm
wasm-pack build --target web --dev
cd ../../..

echo "Starting frontend build (watch mode)..."
cd frontend
npm run build:watch &
FRONTEND_PID=$!
cd ..

echo "Starting backend (serves frontend + API + WebSocket on single port 8080)..."
cd backend
cargo watch -x 'run -- --port 8080'
BACKEND_PID=$!

# Cleanup on exit
trap "kill $FRONTEND_PID $BACKEND_PID" EXIT
wait

# ✅ IMPORTANT: This script ONLY starts ONE server on ONE port
# The backend serves:
#   - REST API at /api/*
#   - WebSocket at /ws/*
#   - Static frontend files at /*

SCRIPTS/TEST.SH:
#!/bin/bash
set -e

echo "Running tests..."

# Unit tests (backend)
echo "1. Backend unit tests..."
cd backend && cargo test
cd ..

# Build for E2E tests
echo "2. Building for E2E..."
./scripts/build.sh

# E2E tests
echo "3. Running E2E tests..."
cd tests
npm test
cd ..

echo "✓ All tests passed!"

MAKE ALL EXECUTABLE:
chmod +x scripts/*.sh

VALIDATION:
- ./scripts/build.sh completes successfully
- ./scripts/test.sh runs (even if tests don't exist yet)
- No absolute paths in any script
- Scripts work from any directory

SINGLE-PORT VALIDATION:
- [x] No hardcoded ports in code (only configurable port 8080)
- [x] All paths are relative
- [x] Frontend build outputs to backend-served directory
- [x] NO webpack-dev-server on separate port
- [x] Only ONE server process serves everything
- [x] Dev script starts single backend server
```

---

## Phase 2: Backend Core Implementation

### Prompt 2.1: Implement CLI Argument Parsing

```bash
npx claude-flow@alpha sparc tdd "Implement CLI argument parsing with clap"
```

**Detailed Instructions:**
```
Implement robust CLI parsing in backend/src/main.rs:

REQUIREMENTS:
1. Use clap v4 derive API
2. Support environment variables for all options
3. Parse complex arguments with whitespace and special characters
4. Validate all inputs before running

CLI ARGUMENTS:
- --cmd, -c <COMMAND>: Command to run (default: /bin/bash)
  Environment variable: WEB_TERMINAL_CMD

- --args <ARGS>...: Arguments for the command
  Environment variable: WEB_TERMINAL_ARGS (comma-separated)
  Support: whitespace, quotes, backslashes, special chars

- --port, -p <PORT>: Server port (default: 8080)
  Environment variable: WEB_TERMINAL_PORT

- --host <HOST>: Bind address (default: 127.0.0.1)
  Environment variable: WEB_TERMINAL_HOST

- --shell-default <SHELL>: Default shell for new terminals (default: /bin/bash)
  Environment variable: WEB_TERMINAL_SHELL

- --log-level <LEVEL>: Logging level (default: info)
  Environment variable: WEB_TERMINAL_LOG_LEVEL
  Values: trace, debug, info, warn, error

PARSING RULES:
- Arguments in quotes are treated as single argument
- Backslash escapes next character
- Empty arguments are preserved
- Invalid arguments return clear error messages

EXAMPLE CODE STRUCTURE:
```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "web-terminal")]
#[command(about = "Web-based terminal emulator", long_about = None)]
struct Cli {
    #[arg(short, long, env = "WEB_TERMINAL_CMD")]
    #[arg(default_value = "/bin/bash")]
    cmd: String,

    #[arg(long, env = "WEB_TERMINAL_ARGS")]
    #[arg(value_delimiter = ',')]
    args: Vec<String>,

    // ... other fields
}

fn parse_shell_args(args: &[String]) -> Vec<String> {
    // Handle quoting, escaping, whitespace
}
```

TEST CASES (backend/tests/cli_test.rs):
1. Simple command: web-terminal --cmd echo
2. Command with args: web-terminal --cmd echo --args "hello world"
3. Special characters: web-terminal --cmd echo --args "hello \"world\""
4. Environment variables: WEB_TERMINAL_CMD=ls cargo run
5. Invalid port: web-terminal --port 99999 (should error)
6. Help text: web-terminal --help

VALIDATION:
- cargo test cli_test passes
- All example commands work correctly
- Help text is clear and complete
- Error messages are user-friendly

SINGLE-PORT VALIDATION:
- [x] Port is configurable via CLI/env var (not hardcoded)
- [x] No multiple port parameters
- [x] Host binding configurable for proxy compatibility
- [ ] No hardcoded URLs in code
```

### Prompt 2.2: Implement PTY Process Management

```bash
npx claude-flow@alpha sparc tdd "Implement PTY spawning and management"
```

**Detailed Instructions:**
```
Implement portable-pty wrapper in backend/src/pty/mod.rs:

REQUIREMENTS:
1. Spawn processes in PTY with configurable command
2. Handle process I/O (read/write)
3. Support terminal resizing
4. Capture exit codes
5. Clean up processes on drop
6. Handle SIGCHLD for subprocess cleanup

MODULE STRUCTURE:
backend/src/pty/
├── mod.rs          # Public API
├── manager.rs      # PtyManager struct
├── process.rs      # Process lifecycle
└── resize.rs       # Terminal resize handling

CORE API:
```rust
pub struct PtyManager {
    pair: portable_pty::PtyPair,
    child: Box<dyn portable_pty::Child + Send>,
    reader: Box<dyn std::io::Read + Send>,
    writer: Box<dyn std::io::Write + Send>,
}

impl PtyManager {
    /// Spawn new process in PTY
    pub fn spawn(
        cmd: &str,
        args: &[String],
        size: PtySize,
    ) -> Result<Self, PtyError>;

    /// Read output from PTY (non-blocking)
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PtyError>;

    /// Write input to PTY
    pub async fn write(&mut self, data: &[u8]) -> Result<(), PtyError>;

    /// Resize terminal
    pub fn resize(&mut self, size: PtySize) -> Result<(), PtyError>;

    /// Check if process is still running
    pub fn is_running(&mut self) -> bool;

    /// Get exit code (blocks until process exits)
    pub async fn wait(&mut self) -> Result<ExitStatus, PtyError>;

    /// Kill process (SIGHUP, then SIGKILL after timeout)
    pub async fn kill(&mut self) -> Result<(), PtyError>;
}

pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self { rows: 40, cols: 120 }
    }
}
```

KEY FEATURES:
1. Non-blocking I/O using tokio
2. Graceful shutdown: SIGHUP → wait 5s → SIGKILL
3. Exit code propagation to parent process
4. Stderr capture to tracing::error!
5. Process group cleanup (kill all children)

TEST CASES (backend/tests/pty_test.rs):
1. Spawn simple command (echo)
2. Spawn interactive shell (bash)
3. Read/write data
4. Resize terminal mid-execution
5. Process exit code propagation
6. Process cleanup on drop
7. Kill hanging process
8. Subprocess cleanup (fork bomb prevention)

VALIDATION:
- cargo test pty_test passes
- No zombie processes left after tests
- Exit codes propagate correctly
- Process cleanup works on ctrl-c
```

### Prompt 2.3: Implement Session Management

```bash
npx claude-flow@alpha sparc tdd "Implement multi-session terminal management"
```

**Detailed Instructions:**
```
Implement session registry in backend/src/session/:

REQUIREMENTS:
1. Track multiple terminal sessions
2. Thread-safe access (Arc<RwLock<>>)
3. Automatic cleanup on disconnect/timeout
4. Session metadata (creation time, last activity, command)

MODULE STRUCTURE:
backend/src/session/
├── mod.rs          # Public API
├── registry.rs     # SessionRegistry
├── session.rs      # Session struct
└── cleanup.rs      # Automatic cleanup

SESSION API:
```rust
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SessionRegistry {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
}

impl SessionRegistry {
    pub fn new() -> Self;

    /// Create new session
    pub async fn create(
        &self,
        cmd: String,
        args: Vec<String>,
    ) -> Result<Uuid, SessionError>;

    /// Get session (returns Arc for shared access)
    pub async fn get(&self, id: Uuid) -> Option<Arc<RwLock<Session>>>;

    /// List all session IDs
    pub async fn list(&self) -> Vec<Uuid>;

    /// Remove session
    pub async fn remove(&self, id: Uuid) -> Result<(), SessionError>;

    /// Remove inactive sessions (last_activity > timeout)
    pub async fn cleanup_inactive(&self, timeout: Duration);
}

pub struct Session {
    pub id: Uuid,
    pub pty: PtyManager,
    pub created_at: SystemTime,
    pub last_activity: Arc<RwLock<SystemTime>>,
    pub metadata: SessionMetadata,
}

impl Session {
    /// Read from PTY (updates last_activity)
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError>;

    /// Write to PTY (updates last_activity)
    pub async fn write(&mut self, data: &[u8]) -> Result<(), SessionError>;

    /// Resize terminal
    pub async fn resize(&mut self, size: PtySize) -> Result<(), SessionError>;

    /// Check if session is active
    pub async fn is_active(&self) -> bool;
}

pub struct SessionMetadata {
    pub command: String,
    pub args: Vec<String>,
    pub size: PtySize,
}
```

CLEANUP TASK:
```rust
// Spawned on server startup
async fn cleanup_task(registry: Arc<SessionRegistry>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        registry.cleanup_inactive(Duration::from_secs(300)).await;
    }
}
```

TEST CASES (backend/tests/session_test.rs):
1. Create session
2. Get session
3. List sessions
4. Remove session
5. Concurrent access (10 threads)
6. Cleanup inactive sessions
7. Session metadata updates
8. Automatic cleanup on timeout

VALIDATION:
- cargo test session_test passes
- No data races (run with RUSTFLAGS="-Z sanitizer=thread")
- Memory leaks detected (valgrind or miri)
- Cleanup task removes old sessions
```

### Prompt 2.4: Implement WebSocket Handler

```bash
npx claude-flow@alpha sparc tdd "Implement WebSocket server with tokio-tungstenite"
```

**Detailed Instructions:**
```
Implement WebSocket handler in backend/src/websocket/:

REQUIREMENTS:
1. Accept WebSocket upgrades from Axum
2. Bidirectional streaming (PTY ↔ WebSocket)
3. JSON control messages + binary data frames
4. Reconnection support (session persists)
5. Ping/pong for connection health

MODULE STRUCTURE:
backend/src/websocket/
├── mod.rs          # Public API
├── handler.rs      # WebSocket connection handler
├── messages.rs     # Message types
└── stream.rs       # Bidirectional streaming

MESSAGE PROTOCOL:
```rust
// Client → Server
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    // Terminal input (text mode)
    Input { data: String },

    // Terminal input (binary mode, base64)
    InputBinary { data: String },

    // Resize terminal
    Resize { rows: u16, cols: u16 },

    // Ping
    Ping,
}

// Server → Client
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    // Terminal output (text mode)
    Output { data: String },

    // Terminal output (binary mode, base64)
    OutputBinary { data: String },

    // Process exited
    Exit { code: i32 },

    // Error occurred
    Error { message: String },

    // Pong
    Pong,
}
```

WEBSOCKET HANDLER:
```rust
use axum::extract::ws::{WebSocket, Message as WsMessage};

pub async fn websocket_handler(
    ws: WebSocket,
    session_id: Uuid,
    registry: Arc<SessionRegistry>,
) -> Result<(), WebSocketError> {
    let (mut sender, mut receiver) = ws.split();

    let session = registry.get(session_id).await
        .ok_or(WebSocketError::SessionNotFound)?;

    // Task 1: PTY → WebSocket (read from PTY, send to client)
    let pty_to_ws = tokio::spawn(async move {
        let mut buf = vec![0u8; 8192];
        loop {
            match session.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let msg = ServerMessage::OutputBinary {
                        data: base64::encode(&buf[..n])
                    };
                    sender.send(WsMessage::Text(
                        serde_json::to_string(&msg)?
                    )).await?;
                }
                Ok(_) => break, // EOF
                Err(e) => {
                    sender.send(WsMessage::Text(
                        serde_json::to_string(&ServerMessage::Error {
                            message: e.to_string()
                        })?
                    )).await?;
                    break;
                }
            }
        }
        Ok::<_, WebSocketError>(())
    });

    // Task 2: WebSocket → PTY (receive from client, write to PTY)
    let ws_to_pty = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    let client_msg: ClientMessage = serde_json::from_str(&text)?;
                    handle_client_message(client_msg, &session).await?;
                }
                WsMessage::Binary(data) => {
                    session.write(&data).await?;
                }
                WsMessage::Close(_) => break,
                _ => {}
            }
        }
        Ok::<_, WebSocketError>(())
    });

    // Wait for either task to complete
    tokio::select! {
        _ = pty_to_ws => {},
        _ = ws_to_pty => {},
    }

    Ok(())
}
```

TEST CASES (backend/tests/websocket_test.rs):
1. Connect to WebSocket
2. Send input, receive output
3. Send resize message
4. Receive exit message when process ends
5. Reconnect to existing session
6. Handle binary data (non-UTF8)
7. Ping/pong keeps connection alive
8. Error handling (invalid JSON, unknown session)

VALIDATION:
- cargo test websocket_test passes
- WebSocket connection stays alive for 5+ minutes
- No message loss under load
- Clean disconnect on client close
```

### Prompt 2.5: Implement HTTP API Server

```bash
npx claude-flow@alpha sparc tdd "Implement Axum HTTP server with REST API"
```

**Detailed Instructions:**
```
Implement HTTP server in backend/src/server/mod.rs and backend/src/api/:

🔒 CRITICAL SINGLE-PORT REQUIREMENTS:
1. ✅ ONE port serves HTTP + WebSocket + static files
2. ✅ ALL routes are relative (no hardcoded hosts/ports)
3. ✅ Static files served on SAME port as API
4. ✅ WebSocket upgrade on SAME port
5. ✅ CORS enabled for development
6. ✅ Structured logging with tracing
7. ✅ Graceful shutdown

API ENDPOINTS (all on SAME port):
- GET / → Serve frontend index.html
- GET /assets/* → Serve static assets
- GET /api/config → Server configuration
- GET /api/sessions → List sessions
- POST /api/sessions → Create new session
- DELETE /api/sessions/:id → Close session
- GET /ws/:session_id → WebSocket upgrade

SERVER IMPLEMENTATION:
```rust
use axum::{
    Router,
    routing::{get, post, delete},
    extract::{Path, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    Json,
};
use tower_http::{
    services::ServeDir,
    cors::CorsLayer,
    trace::TraceLayer,
};

pub struct AppState {
    pub registry: Arc<SessionRegistry>,
    pub config: Arc<ServerConfig>,
}

pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
    let registry = Arc::new(SessionRegistry::new());
    let state = Arc::new(AppState {
        registry: registry.clone(),
        config: Arc::new(config),
    });

    // Spawn initial command session
    let initial_session = registry.create(
        config.initial_cmd.clone(),
        config.initial_args.clone(),
    ).await?;

    // ✅ CRITICAL: Single-port architecture
    // HTTP, WebSocket, AND static files ALL on same router/port
    let app = Router::new()
        // API routes
        .route("/api/config", get(get_config))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions", post(create_session))
        .route("/api/sessions/:id", delete(delete_session))

        // WebSocket route (SAME PORT!)
        .route("/ws/:session_id", get(websocket_upgrade))

        // Static files - MUST be last to not override API routes
        // ✅ This serves frontend on SAME port as API/WebSocket
        .nest_service("/", ServeDir::new("frontend/dist"))

        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())

        .with_state(state.clone());

    // ✅ Single listener on single port
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 Single-port server on {}", addr);
    tracing::info!("   - HTTP API: http://{}/api/*", addr);
    tracing::info!("   - WebSocket: ws://{}/ws/*", addr);
    tracing::info!("   - Frontend: http://{}/", addr);
    tracing::info!("Initial session ID: {}", initial_session);

    // Graceful shutdown
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal());

    // Wait for initial process to exit
    let exit_code = wait_for_session(registry, initial_session).await?;

    // Shutdown server
    tracing::info!("Initial process exited with code {}", exit_code);
    server.await?;

    std::process::exit(exit_code);
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}
```

API HANDLERS:
```rust
// GET /api/config
async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        default_shell: state.config.default_shell.clone(),
        default_size: state.config.default_size,
        max_sessions: state.config.max_sessions,
    })
}

// GET /api/sessions
async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<SessionInfo>> {
    let sessions = state.registry.list().await;
    Json(sessions.into_iter().map(|id| SessionInfo { id }).collect())
}

// POST /api/sessions
async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    let id = state.registry.create(
        req.command.unwrap_or(state.config.default_shell.clone()),
        req.args.unwrap_or_default(),
    ).await?;

    Ok(Json(CreateSessionResponse { id }))
}

// DELETE /api/sessions/:id
async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<(), ApiError> {
    state.registry.remove(id).await?;
    Ok(())
}

// GET /ws/:session_id
async fn websocket_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
) -> Response {
    ws.on_upgrade(move |socket| {
        websocket_handler(socket, session_id, state.registry.clone())
    })
}
```

TEST CASES (backend/tests/server_test.rs):
1. Server starts on specified port
2. GET /api/config returns valid JSON
3. GET /api/sessions lists sessions
4. POST /api/sessions creates session
5. DELETE /api/sessions/:id closes session
6. GET /ws/:id upgrades to WebSocket
7. Server exits when initial process exits
8. Graceful shutdown on SIGTERM

VALIDATION:
- cargo test server_test passes
- curl http://localhost:8080/api/config works
- Server serves static files
- WebSocket upgrade succeeds
- Server exits with correct exit code

SINGLE-PORT VALIDATION:
- [x] ONE port serves all traffic (HTTP + WS + static)
- [x] ServeDir configured on SAME router
- [x] No hardcoded URLs in server code
- [x] All routes relative
- [x] Logging confirms single-port operation
- [x] No separate server processes
```

---

## Phase 3: Frontend Implementation

### Prompt 3.1: Implement WASM WebSocket Client

```bash
npx claude-flow@alpha sparc tdd "Implement WASM WebSocket client with reconnection"
```

**Detailed Instructions:**
```
Implement WASM WebSocket client in frontend/src/wasm/:

REQUIREMENTS:
1. WebSocket connection management
2. Automatic reconnection with exponential backoff
3. Binary data transfer (no UTF-8 conversion)
4. Message serialization/deserialization
5. Connection state management

CARGO.TOML (frontend/src/wasm):
```toml
[package]
name = "web-terminal-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2.103"
wasm-bindgen-futures = "0.4.43"
web-sys = { version = "0.3.81", features = [
    "WebSocket",
    "MessageEvent",
    "BinaryType",
    "CloseEvent",
    "ErrorEvent",
] }
serde = { version = "1.0.221", features = ["derive"] }
serde_json = "1.0.143"
js-sys = "0.3.81"
console_error_panic_hook = "0.1.7"
```

WASM MODULE:
```rust
use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};

#[wasm_bindgen]
pub struct WebSocketClient {
    ws: Option<WebSocket>,
    url: String,
    reconnect_attempts: u32,
}

#[wasm_bindgen]
impl WebSocketClient {
    #[wasm_bindgen(constructor)]
    pub fn new(url: String) -> Self {
        console_error_panic_hook::set_once();
        Self {
            ws: None,
            url,
            reconnect_attempts: 0,
        }
    }

    /// Connect to WebSocket server
    pub async fn connect(&mut self) -> Result<(), JsValue> {
        let ws = WebSocket::new(&self.url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Store websocket
        self.ws = Some(ws);

        Ok(())
    }

    /// Send text message
    pub fn send_text(&self, data: &str) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            ws.send_with_str(data)?;
        }
        Ok(())
    }

    /// Send binary message
    pub fn send_binary(&self, data: &[u8]) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            ws.send_with_u8_array(data)?;
        }
        Ok(())
    }

    /// Close connection
    pub fn close(&self) -> Result<(), JsValue> {
        if let Some(ws) = &self.ws {
            ws.close()?;
        }
        Ok(())
    }

    /// Get connection state
    pub fn ready_state(&self) -> u16 {
        self.ws.as_ref()
            .map(|ws| ws.ready_state())
            .unwrap_or(WebSocket::CLOSED)
    }
}

// Reconnection logic
#[wasm_bindgen]
impl WebSocketClient {
    pub async fn connect_with_retry(&mut self) -> Result<(), JsValue> {
        loop {
            match self.connect().await {
                Ok(_) => {
                    self.reconnect_attempts = 0;
                    return Ok(());
                }
                Err(e) => {
                    self.reconnect_attempts += 1;
                    let delay = std::cmp::min(1000 * 2_u32.pow(self.reconnect_attempts), 30000);

                    // Sleep (requires wasm-bindgen-futures)
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(
                                &resolve,
                                delay as i32,
                            )
                            .unwrap();
                    });
                    wasm_bindgen_futures::JsFuture::from(promise).await?;

                    if self.reconnect_attempts > 10 {
                        return Err(e);
                    }
                }
            }
        }
    }
}
```

BUILD SCRIPT:
```bash
#!/bin/bash
cd frontend/src/wasm
wasm-pack build --target web --release --out-dir ../../dist/wasm
wasm-opt -Os --converge ../../dist/wasm/web_terminal_wasm_bg.wasm -o ../../dist/wasm/web_terminal_wasm_bg.wasm
```

TEST CASES (frontend/src/wasm/tests/):
1. Create client instance
2. Connect to WebSocket server
3. Send text message
4. Send binary message
5. Receive messages
6. Reconnect on disconnect
7. Exponential backoff works
8. Max retry limit

VALIDATION:
- wasm-pack build succeeds
- wasm-pack test --node passes
- WASM module loads in browser
- WebSocket connection works
```

### Prompt 3.2: Implement TypeScript Terminal Manager

```bash
npx claude-flow@alpha sparc tdd "Implement xterm.js terminal manager"
```

**Detailed Instructions:**
```
Implement terminal manager in frontend/src/ts/:

🔒 CRITICAL SINGLE-PORT REQUIREMENTS:
1. ✅ NO hardcoded URLs (all URLs constructed dynamically)
2. ✅ Base URL detection from window.location
3. ✅ WebSocket URLs constructed from current page URL
4. ✅ Works behind reverse proxies with path prefixes
5. ✅ xterm.js integration
6. ✅ Multiple terminal instances
7. ✅ WebGL renderer for performance
8. ✅ Addon support (fit, web-links)
9. ✅ 10,000+ line scrollback
10. ✅ Default 120x40 size

PROJECT STRUCTURE:
frontend/src/ts/
├── main.ts              # Entry point
├── terminal-manager.ts  # Multi-terminal manager
├── terminal.ts          # Single terminal wrapper
├── websocket-client.ts  # WebSocket integration
└── api-client.ts        # REST API client

TERMINAL MANAGER:
```typescript
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

export interface TerminalOptions {
  rows?: number;
  cols?: number;
  scrollback?: number;
}

export class TerminalInstance {
  private term: Terminal;
  private fitAddon: FitAddon;
  private sessionId: string;

  constructor(
    container: HTMLElement,
    sessionId: string,
    options: TerminalOptions = {}
  ) {
    this.sessionId = sessionId;

    // Create terminal with options
    this.term = new Terminal({
      rows: options.rows ?? 40,
      cols: options.cols ?? 120,
      scrollback: options.scrollback ?? 10000,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      fontSize: 14,
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#aeafad',
      },
      cursorBlink: true,
      cursorStyle: 'block',
      allowTransparency: false,
    });

    // Add addons
    this.fitAddon = new FitAddon();
    this.term.loadAddon(this.fitAddon);
    this.term.loadAddon(new WebglAddon());
    this.term.loadAddon(new WebLinksAddon());

    // Open terminal
    this.term.open(container);
    this.fitAddon.fit();
  }

  write(data: string | Uint8Array): void {
    this.term.write(data);
  }

  onData(callback: (data: string) => void): void {
    this.term.onData(callback);
  }

  onResize(callback: (size: { rows: number; cols: number }) => void): void {
    this.term.onResize(callback);
  }

  resize(): void {
    this.fitAddon.fit();
  }

  focus(): void {
    this.term.focus();
  }

  clear(): void {
    this.term.clear();
  }

  dispose(): void {
    this.term.dispose();
  }

  getSessionId(): string {
    return this.sessionId;
  }
}

export class TerminalManager {
  private terminals: Map<string, TerminalInstance> = new Map();
  private activeTerminal: string | null = null;

  createTerminal(
    container: HTMLElement,
    sessionId: string,
    options?: TerminalOptions
  ): TerminalInstance {
    const terminal = new TerminalInstance(container, sessionId, options);
    this.terminals.set(sessionId, terminal);
    this.activeTerminal = sessionId;
    return terminal;
  }

  getTerminal(sessionId: string): TerminalInstance | undefined {
    return this.terminals.get(sessionId);
  }

  removeTerminal(sessionId: string): void {
    const terminal = this.terminals.get(sessionId);
    if (terminal) {
      terminal.dispose();
      this.terminals.delete(sessionId);

      if (this.activeTerminal === sessionId) {
        this.activeTerminal = null;
      }
    }
  }

  setActive(sessionId: string): void {
    if (this.terminals.has(sessionId)) {
      this.activeTerminal = sessionId;
      this.terminals.get(sessionId)?.focus();
    }
  }

  getActive(): TerminalInstance | null {
    return this.activeTerminal
      ? this.terminals.get(this.activeTerminal) || null
      : null;
  }

  getAllTerminals(): TerminalInstance[] {
    return Array.from(this.terminals.values());
  }
}
```

WEBSOCKET CLIENT INTEGRATION:
```typescript
import { WebSocketClient } from './websocket-client';
import { TerminalInstance } from './terminal';

export class TerminalConnection {
  private ws: WebSocketClient;
  private terminal: TerminalInstance;

  constructor(terminal: TerminalInstance, wsUrl: string) {
    this.terminal = terminal;
    this.ws = new WebSocketClient(wsUrl);

    this.setupHandlers();
  }

  private setupHandlers(): void {
    // Terminal input → WebSocket
    this.terminal.onData((data) => {
      this.ws.send(JSON.stringify({
        type: 'Input',
        data: data,
      }));
    });

    // Terminal resize → WebSocket
    this.terminal.onResize((size) => {
      this.ws.send(JSON.stringify({
        type: 'Resize',
        rows: size.rows,
        cols: size.cols,
      }));
    });

    // WebSocket → Terminal output
    this.ws.onMessage((msg) => {
      const data = JSON.parse(msg);

      switch (data.type) {
        case 'Output':
          this.terminal.write(data.data);
          break;
        case 'OutputBinary':
          const binary = atob(data.data);
          const bytes = new Uint8Array(binary.length);
          for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
          }
          this.terminal.write(bytes);
          break;
        case 'Exit':
          console.log(`Process exited with code ${data.code}`);
          break;
        case 'Error':
          console.error(`Terminal error: ${data.message}`);
          break;
      }
    });
  }

  async connect(): Promise<void> {
    await this.ws.connect();
  }

  disconnect(): void {
    this.ws.close();
  }
}
```

WEBPACK CONFIG:
```javascript
const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');

module.exports = {
  entry: './src/ts/main.ts',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bundle.js',
  },
  resolve: {
    extensions: ['.ts', '.js'],
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.css$/,
        use: ['style-loader', 'css-loader'],
      },
    ],
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: 'src/static/index.html',
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
  },
};
```

TEST CASES:
1. Create terminal instance
2. Write text to terminal
3. Receive input from terminal
4. Terminal resize works
5. Multiple terminals
6. Switch active terminal
7. WebGL renderer loads
8. Scrollback preserves 10K lines

VALIDATION:
- npm run build succeeds
- Terminal renders in browser
- Input/output works
- Resize works
- No console errors

SINGLE-PORT VALIDATION:
- [ ] No hardcoded URLs in TypeScript code
- [ ] Base URL detection implemented
- [ ] WebSocket URLs constructed from window.location
- [ ] All API calls use relative paths
- [ ] Works with path prefixes (e.g., /terminals/)
```

### Prompt 3.3: Implement UI Layout

```bash
npx claude-flow@alpha sparc run ui "Implement terminal UI with tabs and controls"
```

**Detailed Instructions:**
```
Create frontend UI in frontend/src/ts/ and frontend/src/static/:

🔒 CRITICAL SINGLE-PORT REQUIREMENTS:
1. ✅ Dynamic base URL detection (getBaseUrl() implementation)
2. ✅ Dynamic WebSocket URL construction (getWsUrl() implementation)
3. ✅ Dynamic API URL construction (getApiUrl() implementation)
4. ✅ NO hardcoded hosts/ports ANYWHERE
5. ✅ Works with reverse proxy path prefixes
6. ✅ Tab-based interface for multiple terminals
7. ✅ New terminal button
8. ✅ Close terminal button
9. ✅ Terminal container that fills viewport
10. ✅ Responsive design
11. ✅ Keyboard shortcuts

HTML (frontend/src/static/index.html):
```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Web Terminal</title>
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #1e1e1e;
      color: #d4d4d4;
      overflow: hidden;
      height: 100vh;
      display: flex;
      flex-direction: column;
    }

    #toolbar {
      background: #252526;
      border-bottom: 1px solid #3c3c3c;
      padding: 8px;
      display: flex;
      align-items: center;
      gap: 8px;
    }

    #tabs {
      display: flex;
      gap: 4px;
      flex: 1;
      overflow-x: auto;
    }

    .tab {
      background: #2d2d30;
      border: 1px solid #3c3c3c;
      padding: 6px 12px;
      border-radius: 4px;
      cursor: pointer;
      display: flex;
      align-items: center;
      gap: 8px;
      white-space: nowrap;
    }

    .tab.active {
      background: #1e1e1e;
      border-color: #007acc;
    }

    .tab-close {
      cursor: pointer;
      opacity: 0.6;
    }

    .tab-close:hover {
      opacity: 1;
    }

    #new-terminal-btn {
      background: #0e639c;
      border: none;
      color: white;
      padding: 6px 12px;
      border-radius: 4px;
      cursor: pointer;
    }

    #new-terminal-btn:hover {
      background: #1177bb;
    }

    #terminal-container {
      flex: 1;
      position: relative;
      overflow: hidden;
    }

    .terminal-wrapper {
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      display: none;
    }

    .terminal-wrapper.active {
      display: block;
    }
  </style>
</head>
<body>
  <div id="toolbar">
    <div id="tabs"></div>
    <button id="new-terminal-btn">+ New Terminal</button>
  </div>

  <div id="terminal-container"></div>

  <script type="module" src="bundle.js"></script>
</body>
</html>
```

MAIN APPLICATION (frontend/src/ts/main.ts):
```typescript
import { TerminalManager, TerminalInstance } from './terminal-manager';
import { TerminalConnection } from './websocket-client';
import { ApiClient } from './api-client';

class App {
  private terminalManager: TerminalManager;
  private apiClient: ApiClient;
  private connections: Map<string, TerminalConnection> = new Map();

  constructor() {
    this.terminalManager = new TerminalManager();
    this.apiClient = new ApiClient(this.getBaseUrl());

    this.setupUI();
    this.loadInitialSession();
  }

  // ✅ CRITICAL: Dynamic base URL detection for proxy compatibility
  private getBaseUrl(): string {
    // Dynamically detect base URL for proxy compatibility
    // Works with: http://localhost:8080/ AND http://proxy.com/terminals/
    const path = window.location.pathname;

    // Extract base path (everything before the app routes)
    // E.g., "/terminals/session1/" -> "/terminals"
    // E.g., "/" -> ""
    const match = path.match(/^(\/[^/]+)?/);
    return match ? match[1] || '' : '';
  }

  // ✅ CRITICAL: Construct WebSocket URL relative to current page
  private getWsUrl(): string {
    // Uses SAME port as HTTP (single-port architecture)
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host; // includes port!
    const base = this.getBaseUrl();

    // Result: ws://localhost:8080/ws or wss://proxy.com/terminals/ws
    return `${protocol}//${host}${base}`;
  }

  // ✅ CRITICAL: Construct API URL relative to current page
  private getApiUrl(endpoint: string): string {
    const base = this.getBaseUrl();
    return `${base}${endpoint}`;
  }

  private setupUI(): void {
    const newTerminalBtn = document.getElementById('new-terminal-btn')!;
    newTerminalBtn.addEventListener('click', () => this.createNewTerminal());

    // Keyboard shortcuts
    window.addEventListener('keydown', (e) => {
      // Ctrl+Shift+T: New terminal
      if (e.ctrlKey && e.shiftKey && e.key === 'T') {
        e.preventDefault();
        this.createNewTerminal();
      }

      // Ctrl+Shift+W: Close terminal
      if (e.ctrlKey && e.shiftKey && e.key === 'W') {
        e.preventDefault();
        const active = this.terminalManager.getActive();
        if (active) {
          this.closeTerminal(active.getSessionId());
        }
      }
    });

    // Resize handler
    window.addEventListener('resize', () => {
      this.terminalManager.getAllTerminals().forEach(t => t.resize());
    });
  }

  private async loadInitialSession(): Promise<void> {
    try {
      const sessions = await this.apiClient.listSessions();

      if (sessions.length > 0) {
        // Connect to initial session
        await this.connectToSession(sessions[0].id);
      } else {
        // Create new session
        await this.createNewTerminal();
      }
    } catch (error) {
      console.error('Failed to load initial session:', error);
    }
  }

  private async createNewTerminal(): Promise<void> {
    try {
      const session = await this.apiClient.createSession();
      await this.connectToSession(session.id);
    } catch (error) {
      console.error('Failed to create terminal:', error);
    }
  }

  private async connectToSession(sessionId: string): Promise<void> {
    // Create terminal container
    const wrapper = document.createElement('div');
    wrapper.className = 'terminal-wrapper';
    wrapper.id = `terminal-${sessionId}`;
    document.getElementById('terminal-container')!.appendChild(wrapper);

    // Create terminal instance
    const terminal = this.terminalManager.createTerminal(wrapper, sessionId);

    // Connect WebSocket
    const wsUrl = `${this.getWsUrl()}/ws/${sessionId}`;
    const connection = new TerminalConnection(terminal, wsUrl);
    await connection.connect();
    this.connections.set(sessionId, connection);

    // Add tab
    this.addTab(sessionId);

    // Set active
    this.setActiveTerminal(sessionId);
  }

  private addTab(sessionId: string): void {
    const tabs = document.getElementById('tabs')!;

    const tab = document.createElement('div');
    tab.className = 'tab';
    tab.id = `tab-${sessionId}`;
    tab.innerHTML = `
      <span>${sessionId.substring(0, 8)}</span>
      <span class="tab-close" data-session="${sessionId}">×</span>
    `;

    tab.addEventListener('click', (e) => {
      if (!(e.target as HTMLElement).classList.contains('tab-close')) {
        this.setActiveTerminal(sessionId);
      }
    });

    tab.querySelector('.tab-close')!.addEventListener('click', (e) => {
      e.stopPropagation();
      this.closeTerminal(sessionId);
    });

    tabs.appendChild(tab);
  }

  private setActiveTerminal(sessionId: string): void {
    // Update tabs
    document.querySelectorAll('.tab').forEach(tab => {
      tab.classList.remove('active');
    });
    document.getElementById(`tab-${sessionId}`)?.classList.add('active');

    // Update terminal wrappers
    document.querySelectorAll('.terminal-wrapper').forEach(wrapper => {
      wrapper.classList.remove('active');
    });
    document.getElementById(`terminal-${sessionId}`)?.classList.add('active');

    // Focus terminal
    this.terminalManager.setActive(sessionId);
  }

  private async closeTerminal(sessionId: string): Promise<void> {
    try {
      // Close connection
      this.connections.get(sessionId)?.disconnect();
      this.connections.delete(sessionId);

      // Remove terminal
      this.terminalManager.removeTerminal(sessionId);

      // Remove DOM elements
      document.getElementById(`tab-${sessionId}`)?.remove();
      document.getElementById(`terminal-${sessionId}`)?.remove();

      // Delete session on server
      await this.apiClient.deleteSession(sessionId);

      // Switch to another terminal
      const remaining = this.terminalManager.getAllTerminals();
      if (remaining.length > 0) {
        this.setActiveTerminal(remaining[0].getSessionId());
      }
    } catch (error) {
      console.error('Failed to close terminal:', error);
    }
  }

  // ✅ Use the getWsUrl() method from above (already defined)
}

// Initialize app
new App();
```

API CLIENT (frontend/src/ts/api-client.ts):
```typescript
export interface SessionInfo {
  id: string;
}

export interface CreateSessionRequest {
  command?: string;
  args?: string[];
}

export class ApiClient {
  constructor(private baseUrl: string = '') {}

  async listSessions(): Promise<SessionInfo[]> {
    const response = await fetch(`${this.baseUrl}/api/sessions`);
    if (!response.ok) throw new Error('Failed to list sessions');
    return response.json();
  }

  async createSession(req?: CreateSessionRequest): Promise<SessionInfo> {
    const response = await fetch(`${this.baseUrl}/api/sessions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req || {}),
    });
    if (!response.ok) throw new Error('Failed to create session');
    return response.json();
  }

  async deleteSession(id: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/sessions/${id}`, {
      method: 'DELETE',
    });
    if (!response.ok) throw new Error('Failed to delete session');
  }

  async getConfig(): Promise<any> {
    const response = await fetch(`${this.baseUrl}/api/config`);
    if (!response.ok) throw new Error('Failed to get config');
    return response.json();
  }
}
```

VALIDATION:
- UI renders correctly
- Tabs work
- New terminal button creates terminal
- Close button removes terminal
- Keyboard shortcuts work
- Responsive on different screen sizes

SINGLE-PORT VALIDATION:
- [x] getBaseUrl() implementation present
- [x] getWsUrl() implementation present
- [x] getApiUrl() implementation present
- [x] No hardcoded localhost:8080 in code
- [x] WebSocket URL constructed from window.location
- [x] API calls use relative paths
- [x] Works with proxy path prefixes
```

---

## Phase 4: Integration & Testing

### Prompt 4.1: Implement Playwright Test Infrastructure

```bash
npx claude-flow@alpha sparc tdd "Set up Playwright test infrastructure"
```

**Detailed Instructions:**
```
Set up Playwright testing in tests/:

REQUIREMENTS:
1. Test fixtures for server and browser
2. Helper functions for common operations
3. Test data for ANSI sequences and unicode
4. CI/CD configuration

PROJECT STRUCTURE:
tests/
├── e2e/
│   ├── terminal-basic.spec.ts
│   ├── terminal-ansi.spec.ts
│   ├── terminal-multi.spec.ts
│   ├── cli-args.spec.ts
│   ├── process-lifecycle.spec.ts
│   ├── websocket.spec.ts
│   ├── api.spec.ts
│   └── proxy.spec.ts
├── fixtures/
│   ├── server.ts
│   ├── terminal.ts
│   └── proxy.ts
├── helpers/
│   ├── ansi.ts
│   └── utils.ts
├── data/
│   ├── ansi-sequences.json
│   └── unicode-test-data.json
├── playwright.config.ts
└── package.json

PLAYWRIGHT CONFIG (tests/playwright.config.ts):
```typescript
import { defineConfig, devices } from '@playwright/test';

// ✅ CRITICAL: Single-port testing
// Tests MUST use single port (8080) for ALL traffic
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 4 : undefined,
  reporter: [
    ['html'],
    ['json', { outputFile: 'test-results.json' }],
    ['junit', { outputFile: 'junit.xml' }],
  ],
  use: {
    // ✅ Single port for all tests
    baseURL: 'http://localhost:8080',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
  ],
  // ✅ CRITICAL: webServer starts SINGLE server on SINGLE port
  // NO separate frontend dev server!
  webServer: {
    command: 'cd .. && ./scripts/build.sh && ./backend/target/release/web-terminal --port 8080',
    url: 'http://localhost:8080',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
```

SERVER FIXTURE (tests/fixtures/server.ts):
```typescript
import { test as base, Page } from '@playwright/test';
import { spawn, ChildProcess } from 'child_process';

export interface ServerFixture {
  port: number;
  process: ChildProcess;
  waitForReady: () => Promise<void>;
}

export const test = base.extend<{ server: ServerFixture }>({
  server: async ({}, use) => {
    const port = 8080 + Math.floor(Math.random() * 1000);

    // Start server
    const proc = spawn('./backend/target/release/web-terminal', [
      '--port', port.toString(),
      '--cmd', '/bin/bash',
    ]);

    const waitForReady = async () => {
      // Wait for server to be ready
      for (let i = 0; i < 30; i++) {
        try {
          const response = await fetch(`http://localhost:${port}/api/config`);
          if (response.ok) return;
        } catch {}
        await new Promise(resolve => setTimeout(resolve, 1000));
      }
      throw new Error('Server did not start');
    };

    await use({ port, process: proc, waitForReady });

    // Cleanup
    proc.kill();
  },
});
```

TERMINAL FIXTURE (tests/fixtures/terminal.ts):
```typescript
import { test as base, Page, Locator } from '@playwright/test';

export interface TerminalFixture {
  page: Page;
  terminal: Locator;

  // Helper methods
  type: (text: string) => Promise<void>;
  waitForOutput: (text: string | RegExp, timeout?: number) => Promise<void>;
  getOutput: () => Promise<string>;
  resize: (rows: number, cols: number) => Promise<void>;
  createNewTerminal: () => Promise<void>;
  switchToTerminal: (index: number) => Promise<void>;
}

export const test = base.extend<{ terminalPage: TerminalFixture }>({
  terminalPage: async ({ page }, use) => {
    await page.goto('/');

    const terminal = page.locator('.xterm-screen');

    const fixture: TerminalFixture = {
      page,
      terminal,

      async type(text: string) {
        await terminal.click();
        await page.keyboard.type(text);
      },

      async waitForOutput(text: string | RegExp, timeout = 5000) {
        const start = Date.now();
        while (Date.now() - start < timeout) {
          const output = await this.getOutput();
          if (typeof text === 'string') {
            if (output.includes(text)) return;
          } else {
            if (text.test(output)) return;
          }
          await page.waitForTimeout(100);
        }
        throw new Error(`Timeout waiting for output: ${text}`);
      },

      async getOutput() {
        return await terminal.innerText();
      },

      async resize(rows: number, cols: number) {
        await page.evaluate(({ rows, cols }) => {
          // Access terminal instance via window
          (window as any).terminalManager?.getActive()?.resize(rows, cols);
        }, { rows, cols });
      },

      async createNewTerminal() {
        await page.click('#new-terminal-btn');
        await page.waitForTimeout(500);
      },

      async switchToTerminal(index: number) {
        await page.click(`.tab:nth-child(${index + 1})`);
        await page.waitForTimeout(100);
      },
    };

    await use(fixture);
  },
});
```

ANSI HELPERS (tests/helpers/ansi.ts):
```typescript
export const ANSI = {
  // Colors
  RED: '\x1b[31m',
  GREEN: '\x1b[32m',
  YELLOW: '\x1b[33m',
  BLUE: '\x1b[34m',
  RESET: '\x1b[0m',

  // 256 colors
  color256: (n: number) => `\x1b[38;5;${n}m`,

  // True color (24-bit)
  rgb: (r: number, g: number, b: number) => `\x1b[38;2;${r};${g};${b}m`,

  // Cursor movement
  CURSOR_UP: (n: number) => `\x1b[${n}A`,
  CURSOR_DOWN: (n: number) => `\x1b[${n}B`,
  CURSOR_FORWARD: (n: number) => `\x1b[${n}C`,
  CURSOR_BACK: (n: number) => `\x1b[${n}D`,

  // Screen control
  CLEAR_SCREEN: '\x1b[2J',
  CLEAR_LINE: '\x1b[2K',
  CURSOR_HOME: '\x1b[H',
};
```

TEST DATA (tests/data/ansi-sequences.json):
```json
{
  "basic_colors": [
    {"name": "red", "sequence": "\u001b[31m", "text": "red text"},
    {"name": "green", "sequence": "\u001b[32m", "text": "green text"},
    {"name": "blue", "sequence": "\u001b[34m", "text": "blue text"}
  ],
  "256_colors": [
    {"name": "orange", "sequence": "\u001b[38;5;208m", "text": "orange text"},
    {"name": "purple", "sequence": "\u001b[38;5;129m", "text": "purple text"}
  ],
  "cursor_movement": [
    {"name": "up", "sequence": "\u001b[A"},
    {"name": "down", "sequence": "\u001b[B"}
  ]
}
```

VALIDATION:
- npx playwright test passes
- All fixtures work correctly
- Helper functions simplify test writing
- Test data loads successfully

SINGLE-PORT VALIDATION:
- [x] Playwright webServer starts single backend server
- [x] Tests use port 8080 only
- [x] No separate webpack-dev-server in test config
- [x] baseURL points to single port
- [x] All test URLs relative to baseURL
```

### Prompt 4.2: Implement Core E2E Tests

```bash
npx claude-flow@alpha sparc tdd "Implement Playwright E2E tests for terminal functionality"
```

**Detailed Instructions:**
```
Implement comprehensive E2E tests based on docs/test-strategy.md:

TEST 1: BASIC TERMINAL I/O (tests/e2e/terminal-basic.spec.ts):
```typescript
import { test, expect } from '../fixtures/terminal';

test.describe('Basic Terminal I/O', () => {
  test('should echo typed text', async ({ terminalPage }) => {
    await terminalPage.type('echo hello world\n');
    await terminalPage.waitForOutput('hello world');

    const output = await terminalPage.getOutput();
    expect(output).toContain('hello world');
  });

  test('should handle special characters', async ({ terminalPage }) => {
    await terminalPage.type('echo "hello \\"world\\"\"\n');
    await terminalPage.waitForOutput('hello "world"');
  });

  test('should support unicode', async ({ terminalPage }) => {
    await terminalPage.type('echo "🚀 Hello 世界"\n');
    await terminalPage.waitForOutput('🚀 Hello 世界');
  });

  test('should preserve scrollback history', async ({ terminalPage }) => {
    // Generate 100 lines
    for (let i = 0; i < 100; i++) {
      await terminalPage.type(`echo "Line ${i}"\n`);
    }

    // Scroll up and verify old lines exist
    await terminalPage.page.keyboard.press('PageUp');
    const output = await terminalPage.getOutput();
    expect(output).toContain('Line 0');
  });
});
```

TEST 2: ANSI COLORS (tests/e2e/terminal-ansi.spec.ts):
```typescript
import { test, expect } from '../fixtures/terminal';
import ansiData from '../data/ansi-sequences.json';

test.describe('ANSI Color Support', () => {
  test('should render 16 basic colors', async ({ terminalPage }) => {
    await terminalPage.type('printf "\\e[31mRed\\e[0m\\n"\n');
    await terminalPage.waitForOutput('Red');

    // Check color is applied (would need to check computed styles)
    const redText = await terminalPage.terminal.locator('text=Red').first();
    const color = await redText.evaluate(el =>
      window.getComputedStyle(el).color
    );
    expect(color).not.toBe('rgb(212, 212, 212)'); // Not default color
  });

  test('should render 256 colors', async ({ terminalPage }) => {
    await terminalPage.type('printf "\\e[38;5;208mOrange\\e[0m\\n"\n');
    await terminalPage.waitForOutput('Orange');
  });

  test('should render true color (24-bit)', async ({ terminalPage }) => {
    await terminalPage.type('printf "\\e[38;2;255;100;0mCustom\\e[0m\\n"\n');
    await terminalPage.waitForOutput('Custom');
  });
});
```

TEST 3: MULTI-TERMINAL (tests/e2e/terminal-multi.spec.ts):
```typescript
import { test, expect } from '../fixtures/terminal';

test.describe('Multiple Terminals', () => {
  test('should create new terminal', async ({ terminalPage }) => {
    await terminalPage.createNewTerminal();

    const tabs = await terminalPage.page.locator('.tab').count();
    expect(tabs).toBe(2);
  });

  test('should switch between terminals', async ({ terminalPage }) => {
    await terminalPage.type('echo "Terminal 1"\n');
    await terminalPage.waitForOutput('Terminal 1');

    await terminalPage.createNewTerminal();
    await terminalPage.type('echo "Terminal 2"\n');
    await terminalPage.waitForOutput('Terminal 2');

    await terminalPage.switchToTerminal(0);
    const output1 = await terminalPage.getOutput();
    expect(output1).toContain('Terminal 1');
    expect(output1).not.toContain('Terminal 2');
  });

  test('should close terminal', async ({ terminalPage }) => {
    await terminalPage.createNewTerminal();

    await terminalPage.page.click('.tab:first-child .tab-close');
    await terminalPage.page.waitForTimeout(500);

    const tabs = await terminalPage.page.locator('.tab').count();
    expect(tabs).toBe(1);
  });

  test('should maintain independent state', async ({ terminalPage }) => {
    await terminalPage.type('export VAR1=value1\n');
    await terminalPage.createNewTerminal();
    await terminalPage.type('export VAR2=value2\n');

    // Check terminal 2
    await terminalPage.type('echo $VAR2\n');
    await terminalPage.waitForOutput('value2');

    // Switch to terminal 1
    await terminalPage.switchToTerminal(0);
    await terminalPage.type('echo $VAR1\n');
    await terminalPage.waitForOutput('value1');

    // VAR2 should not exist in terminal 1
    await terminalPage.type('echo $VAR2\n');
    const output = await terminalPage.getOutput();
    expect(output).not.toContain('value2');
  });
});
```

TEST 4: CLI ARGUMENTS (tests/e2e/cli-args.spec.ts):
```typescript
import { test, expect } from '@playwright/test';
import { spawn } from 'child_process';

test.describe('CLI Arguments', () => {
  test('should accept custom command', async () => {
    const proc = spawn('./backend/target/release/web-terminal', [
      '--cmd', 'echo',
      '--args', 'hello',
      '--port', '8081',
    ]);

    let output = '';
    proc.stdout.on('data', (data) => { output += data; });

    await new Promise(resolve => setTimeout(resolve, 2000));

    expect(output).toContain('hello');
    proc.kill();
  });

  test('should handle arguments with whitespace', async () => {
    const proc = spawn('./backend/target/release/web-terminal', [
      '--cmd', 'echo',
      '--args', 'hello world',
      '--port', '8082',
    ]);

    let output = '';
    proc.stdout.on('data', (data) => { output += data; });

    await new Promise(resolve => setTimeout(resolve, 2000));

    expect(output).toContain('hello world');
    proc.kill();
  });

  test('should exit with correct exit code', async () => {
    const proc = spawn('./backend/target/release/web-terminal', [
      '--cmd', 'exit',
      '--args', '42',
      '--port', '8083',
    ]);

    const exitCode = await new Promise<number>((resolve) => {
      proc.on('exit', (code) => resolve(code || 0));
    });

    expect(exitCode).toBe(42);
  });
});
```

TEST 5: PROCESS LIFECYCLE (tests/e2e/process-lifecycle.spec.ts):
```typescript
import { test, expect } from '@playwright/test';
import { spawn } from 'child_process';

test.describe('Process Lifecycle', () => {
  test('should exit when initial process exits', async () => {
    const proc = spawn('./backend/target/release/web-terminal', [
      '--cmd', 'sleep',
      '--args', '1',
      '--port', '8084',
    ]);

    const exitPromise = new Promise<number>((resolve) => {
      proc.on('exit', (code) => resolve(code || 0));
    });

    const exitCode = await exitPromise;
    expect(exitCode).toBe(0);
  });

  test('should cleanup subprocesses on exit', async () => {
    // This test would need to verify no zombie processes remain
    // Implementation depends on OS
  });
});
```

TEST 6: API ENDPOINTS (tests/e2e/api.spec.ts):
```typescript
import { test, expect } from '@playwright/test';

test.describe('REST API', () => {
  test('GET /api/config', async ({ request }) => {
    const response = await request.get('http://localhost:8080/api/config');
    expect(response.ok()).toBeTruthy();

    const data = await response.json();
    expect(data).toHaveProperty('default_shell');
    expect(data).toHaveProperty('default_size');
  });

  test('GET /api/sessions', async ({ request }) => {
    const response = await request.get('http://localhost:8080/api/sessions');
    expect(response.ok()).toBeTruthy();

    const sessions = await response.json();
    expect(Array.isArray(sessions)).toBeTruthy();
  });

  test('POST /api/sessions', async ({ request }) => {
    const response = await request.post('http://localhost:8080/api/sessions', {
      data: { command: '/bin/bash' },
    });
    expect(response.ok()).toBeTruthy();

    const session = await response.json();
    expect(session).toHaveProperty('id');
  });

  test('DELETE /api/sessions/:id', async ({ request }) => {
    // Create session
    const createResponse = await request.post('http://localhost:8080/api/sessions');
    const session = await createResponse.json();

    // Delete session
    const deleteResponse = await request.delete(
      `http://localhost:8080/api/sessions/${session.id}`
    );
    expect(deleteResponse.ok()).toBeTruthy();
  });
});
```

TEST 7: PROXY COMPATIBILITY (tests/e2e/proxy.spec.ts):
```typescript
import { test, expect } from '@playwright/test';
import { spawn } from 'child_process';
import * as http from 'http';
import * as httpProxy from 'http-proxy';

test.describe('Proxy Compatibility', () => {
  test('should work behind reverse proxy with path prefix', async ({ page }) => {
    // Set up reverse proxy
    const proxy = httpProxy.createProxyServer({});
    const proxyServer = http.createServer((req, res) => {
      // Strip /terminal prefix and proxy to backend
      if (req.url?.startsWith('/terminal')) {
        req.url = req.url.substring('/terminal'.length);
        proxy.web(req, res, { target: 'http://localhost:8080' });
      }
    });

    await new Promise(resolve => proxyServer.listen(9000, resolve));

    try {
      // Access through proxy
      await page.goto('http://localhost:9000/terminal/');

      // Verify terminal loads
      await page.waitForSelector('.xterm-screen');

      // Test input/output
      await page.type('.xterm-screen', 'echo hello\n');
      await page.waitForTimeout(1000);

      const output = await page.locator('.xterm-screen').innerText();
      expect(output).toContain('hello');
    } finally {
      proxyServer.close();
    }
  });
});
```

VALIDATION:
- npx playwright test passes all tests
- Tests run in CI/CD (GitHub Actions)
- Test coverage >90% for critical paths
- All documented features are tested

SINGLE-PORT VALIDATION:
- [x] All tests use single port 8080
- [x] Proxy test validates path prefix handling
- [x] No hardcoded URLs in test code
- [x] Tests verify dynamic URL construction works
- [x] WebSocket connections tested on same port as HTTP
```

### Prompt 4.3: Integration Testing & Bug Fixes

```bash
npx claude-flow@alpha sparc run integration "Run full integration tests and fix issues"
```

**Detailed Instructions:**
```
Run comprehensive integration testing and fix any issues:

INTEGRATION TEST CHECKLIST:

1. BUILD VERIFICATION:
   - Run ./scripts/build.sh successfully
   - No compiler warnings (Rust or TypeScript)
   - WASM module builds and optimizes correctly
   - Frontend bundle size <1MB

2. BACKEND TESTING:
   - cargo test passes all unit tests
   - cargo clippy has no warnings
   - cargo audit reports no vulnerabilities
   - Backend starts without errors
   - All API endpoints return correct responses
   - WebSocket upgrade succeeds

3. FRONTEND TESTING:
   - npm run build succeeds
   - No TypeScript errors
   - Bundle loads in all browsers (Chrome, Firefox, Safari)
   - xterm.js renders correctly
   - WebGL addon loads successfully
   - WASM module loads and connects

4. E2E TESTING:
   - npx playwright test passes all tests
   - Tests pass on all browsers
   - No flaky tests (run 3 times)
   - Test coverage report generated

5. MANUAL TESTING:
   - Start server with default settings
   - Type commands and verify output
   - Test ANSI colors (run `ls --color` or similar)
   - Test unicode (echo emoji)
   - Create multiple terminals
   - Switch between terminals
   - Close terminals
   - Test terminal resizing (drag browser window)
   - Test scrollback (generate many lines)
   - Test process exit behavior
   - Test CLI arguments

6. PROXY TESTING:
   - Set up nginx reverse proxy with path prefix
   - Verify all routes work through proxy
   - WebSocket connection works through proxy
   - Static assets load correctly

7. PERFORMANCE TESTING:
   - Measure startup time (<2 seconds)
   - Measure WebSocket latency (<50ms)
   - Test with high output rate (cat large file)
   - Monitor memory usage (<100MB for single session)
   - Test with 10 concurrent terminals

8. ERROR HANDLING:
   - Test with invalid CLI arguments
   - Test with non-existent command
   - Test network disconnection
   - Test backend crash recovery
   - Test WebSocket reconnection

BUG FIX PROCESS:
1. Document the bug (symptoms, reproduction steps)
2. Write a failing test that reproduces the bug
3. Fix the bug
4. Verify the test passes
5. Run full test suite to ensure no regressions
6. Update documentation if needed

COMMON ISSUES TO CHECK:
- Race conditions in session management
- Memory leaks in WebSocket handlers
- Process cleanup on abnormal exit
- Path resolution with proxy prefixes
- ANSI escape sequence parsing
- Terminal size synchronization
- Scrollback buffer overflow

VALIDATION:
- All tests pass
- No known bugs
- Performance meets requirements
- Ready for documentation phase

SINGLE-PORT VALIDATION:
- [x] Confirm NO multi-port configurations remain
- [x] Verify proxy compatibility works
- [x] Check all URLs are dynamically constructed
- [x] Validate single-port operation in all environments
- [x] Test WebSocket connections on same port as HTTP
- [x] Verify static assets served from backend
```

---

## Phase 5: Documentation & Polish

### Prompt 5.1: Write User Documentation

```bash
npx claude-flow@alpha sparc run documenter "Create comprehensive user documentation"
```

**Detailed Instructions:**
```
Create user-facing documentation:

DOCUMENTATION STRUCTURE:
docs/
├── user/
│   ├── getting-started.md
│   ├── installation.md
│   ├── usage.md
│   ├── cli-reference.md
│   ├── troubleshooting.md
│   └── faq.md
└── README.md (update root)

GETTING STARTED (docs/user/getting-started.md):
- Quick start guide (5 minutes to first terminal)
- System requirements
- Basic usage examples
- Next steps

INSTALLATION (docs/user/installation.md):
- Binary downloads
- Building from source
- Docker container
- Package managers (if applicable)

USAGE GUIDE (docs/user/usage.md):
- Starting the server
- Connecting to the web UI
- Using multiple terminals
- Keyboard shortcuts
- Configuration options
- Advanced features

CLI REFERENCE (docs/user/cli-reference.md):
- Complete list of all CLI arguments
- Environment variables
- Configuration files
- Examples for common scenarios

TROUBLESHOOTING (docs/user/troubleshooting.md):
- Common issues and solutions
- Error messages explained
- Performance tuning
- Debugging tips

FAQ (docs/user/faq.md):
- Frequently asked questions
- Comparison with alternatives
- Security considerations
- Use case recommendations

ROOT README UPDATE:
- Project overview
- Key features
- Quick start
- Links to detailed docs
- Contributing guidelines
- License

DOCUMENTATION STANDARDS:
- Clear, concise language
- Code examples for all features
- Screenshots/GIFs where helpful
- Cross-references to related docs
- Version compatibility notes
- Last updated dates

VALIDATION:
- All features documented
- Examples tested and working
- Links are valid
- Spelling and grammar checked
- Readable by non-technical users
```

### Prompt 5.2: Write Developer Documentation

```bash
npx claude-flow@alpha sparc run documenter "Create developer documentation"
```

**Detailed Instructions:**
```
Create developer-facing documentation:

DEVELOPER DOCS STRUCTURE:
docs/
├── developer/
│   ├── architecture.md (already exists, enhance)
│   ├── building.md
│   ├── testing.md
│   ├── contributing.md
│   ├── api-reference.md
│   └── release-process.md

BUILDING (docs/developer/building.md):
- Development setup
- Dependencies installation
- Build process explanation
- Development workflow
- Hot reload setup
- Build optimization

TESTING (docs/developer/testing.md):
- Test structure overview
- Running tests locally
- Writing new tests
- Test fixtures and helpers
- Coverage requirements
- CI/CD integration

CONTRIBUTING (docs/developer/contributing.md):
- How to contribute
- Code style guidelines
- PR process
- Code review checklist
- Issue reporting
- Feature requests

API REFERENCE (docs/developer/api-reference.md):
- REST API complete reference
- WebSocket protocol specification
- Message format examples
- Error codes
- Rate limiting
- Versioning strategy

RELEASE PROCESS (docs/developer/release-process.md):
- Version numbering (semver)
- Release checklist
- Changelog generation
- Binary building for all platforms
- Publishing process

CODE DOCUMENTATION:
- Add rustdoc comments to all public APIs
- Add TSDoc comments to TypeScript code
- Generate API docs: cargo doc --open
- Document complex algorithms
- Explain non-obvious design decisions

VALIDATION:
- cargo doc generates without warnings
- All public APIs documented
- Examples compile and run
- Architecture matches implementation
```

### Prompt 5.3: Final Polish & Release Prep

```bash
npx claude-flow@alpha sparc run refactor "Final polish and release preparation"
```

**Detailed Instructions:**
```
Final polish before release:

CODE QUALITY:
1. Run rustfmt on all Rust code
2. Run prettier on all TypeScript/JavaScript code
3. Fix all clippy warnings
4. Remove all TODO/FIXME comments (or create issues)
5. Remove debug logging
6. Optimize WASM bundle size
7. Minify JavaScript in production builds

DOCUMENTATION REVIEW:
1. Ensure all docs are up to date
2. Verify all code examples work
3. Check all links
4. Add version numbers where relevant
5. Update changelog

SECURITY REVIEW:
1. Run cargo audit
2. Run npm audit
3. Review CORS settings
4. Check for hardcoded secrets
5. Verify input validation
6. Test error messages don't leak info

PERFORMANCE OPTIMIZATION:
1. Profile backend with flamegraph
2. Optimize hot paths
3. Reduce WASM bundle size
4. Optimize frontend bundle
5. Enable gzip compression
6. Add caching headers

RELEASE CHECKLIST:
□ All tests pass
□ No compiler warnings
□ No security vulnerabilities
□ Documentation complete
□ CHANGELOG.md updated
□ Version numbers updated
□ License file present
□ README.md polished
□ Examples tested
□ Binary builds for: Linux, macOS, Windows
□ Docker image built and tested
□ GitHub release created
□ Announcement prepared

FINAL VALIDATION:
- Fresh clone, build succeeds
- Binary runs without dependencies
- Documentation is clear
- No known critical bugs
- Performance meets targets
- Ready for users!
```

---

## Summary of Prompts

### Quick Reference

| Phase | Prompt | Command |
|-------|--------|---------|
| 1.1 | Project Setup | `npx claude-flow@alpha sparc run architect "Initialize web-terminal project structure"` |
| 1.2 | Dependencies | `npx claude-flow@alpha sparc run spec-pseudocode "Configure all project dependencies"` |
| 1.3 | Build Pipeline | `npx claude-flow@alpha sparc run dev "Create build scripts and development workflow"` |
| 2.1 | CLI Parsing | `npx claude-flow@alpha sparc tdd "Implement CLI argument parsing with clap"` |
| 2.2 | PTY Management | `npx claude-flow@alpha sparc tdd "Implement PTY spawning and management"` |
| 2.3 | Session Management | `npx claude-flow@alpha sparc tdd "Implement multi-session terminal management"` |
| 2.4 | WebSocket | `npx claude-flow@alpha sparc tdd "Implement WebSocket server with tokio-tungstenite"` |
| 2.5 | HTTP Server | `npx claude-flow@alpha sparc tdd "Implement Axum HTTP server with REST API"` |
| 3.1 | WASM Client | `npx claude-flow@alpha sparc tdd "Implement WASM WebSocket client with reconnection"` |
| 3.2 | Terminal Manager | `npx claude-flow@alpha sparc tdd "Implement xterm.js terminal manager"` |
| 3.3 | UI Layout | `npx claude-flow@alpha sparc run ui "Implement terminal UI with tabs and controls"` |
| 4.1 | Test Infrastructure | `npx claude-flow@alpha sparc tdd "Set up Playwright test infrastructure"` |
| 4.2 | E2E Tests | `npx claude-flow@alpha sparc tdd "Implement Playwright E2E tests for terminal functionality"` |
| 4.3 | Integration | `npx claude-flow@alpha sparc run integration "Run full integration tests and fix issues"` |
| 5.1 | User Docs | `npx claude-flow@alpha sparc run documenter "Create comprehensive user documentation"` |
| 5.2 | Developer Docs | `npx claude-flow@alpha sparc run documenter "Create developer documentation"` |
| 5.3 | Final Polish | `npx claude-flow@alpha sparc run refactor "Final polish and release preparation"` |

---

## Usage Instructions

1. **Execute Sequentially**: Run prompts in order (1.1 → 1.2 → ... → 5.3)
2. **Validate Each Step**: Check validation criteria before proceeding
3. **ENFORCE SINGLE-PORT**: Verify single-port validation checklist after EVERY prompt
4. **Iterate if Needed**: If tests fail, debug and re-run the prompt
5. **Use TDD Mode**: Most backend/frontend prompts use `sparc tdd` for test-driven development
6. **Monitor Progress**: Each prompt has clear success criteria

## 🔒 Single-Port Enforcement Checklist

**Run this checklist after COMPLETING each prompt:**

- [ ] No hardcoded ports in ANY code files
- [ ] All paths are relative (no absolute URLs)
- [ ] Frontend uses getBaseUrl(), getWsUrl(), getApiUrl()
- [ ] WebSocket URLs constructed from window.location
- [ ] Backend serves static files on SAME port as API
- [ ] Dev workflow uses SINGLE server process
- [ ] Tests configured for single port only
- [ ] No webpack-dev-server on separate port
- [ ] Proxy compatibility validated
- [ ] All URLs dynamically determined

## Expected Timeline

- **Phase 1** (Setup): 2-3 hours
- **Phase 2** (Backend): 8-12 hours
- **Phase 3** (Frontend): 6-8 hours
- **Phase 4** (Testing): 8-10 hours
- **Phase 5** (Docs): 4-6 hours

**Total**: ~30-40 hours of development time

---

## 🔒 Final Single-Port Architecture Reminders

**Before considering the project complete, verify:**

1. ✅ **Backend**: One Axum server listening on one port (default 8080)
2. ✅ **Routes**: HTTP API at `/api/*`, WebSocket at `/ws/*`, static files at `/*`
3. ✅ **Frontend**: Dynamic URL construction using window.location
4. ✅ **Dev Workflow**: Single backend server, NO webpack-dev-server
5. ✅ **Tests**: All tests use single port (8080)
6. ✅ **Proxy Compatible**: Works with reverse proxy path prefixes
7. ✅ **No Hardcoded URLs**: All URLs constructed dynamically

**If ANY of these are violated, the implementation is INCORRECT.**

---

*Generated for web-terminal project - September 2025*
*Single-Port Architecture Mandate: ONE port serves ALL traffic*