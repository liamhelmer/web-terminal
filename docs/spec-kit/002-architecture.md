# Web-Terminal: System Architecture Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29
**References:** [000-overview.md](./000-overview.md), [001-requirements.md](./001-requirements.md)

---

## Table of Contents

1. [High-Level Architecture](#high-level-architecture)
2. [Component Architecture](#component-architecture)
3. [Data Flow](#data-flow)
4. [Technology Stack](#technology-stack)
5. [Security Architecture](#security-architecture)
6. [Deployment Architecture](#deployment-architecture)
7. [Scalability Considerations](#scalability-considerations)
8. [Architecture Decision Records](#architecture-decision-records)

---

## High-Level Architecture

### System Overview

Web-Terminal follows a client-server architecture with WebSocket-based real-time communication:

```
┌─────────────────────────────────────────────────────────────────┐
│                          Client Layer                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Browser (Chrome, Firefox, Safari)           │   │
│  │  ┌────────────────┐  ┌──────────────────────────────┐  │   │
│  │  │  xterm.js UI   │  │   WebSocket Client          │  │   │
│  │  │  (Terminal     │◄─┤   (Bidirectional           │  │   │
│  │  │   Emulator)    │  │    Communication)           │  │   │
│  │  └────────────────┘  └──────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────────────────────┘
                       │ WebSocket/HTTPS (Single Port)
┌──────────────────────▼──────────────────────────────────────────┐
│                        Server Layer                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │               Actix-Web Server (Rust)                   │   │
│  │  ┌────────────┐  ┌──────────────┐  ┌───────────────┐  │   │
│  │  │   HTTP     │  │  WebSocket   │  │  Session      │  │   │
│  │  │  Handler   │  │   Handler    │  │  Manager      │  │   │
│  │  └────────────┘  └──────────────┘  └───────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Execution Layer (Rust)                     │   │
│  │  ┌────────────┐  ┌──────────────┐  ┌───────────────┐  │   │
│  │  │  Command   │  │   Process    │  │  Virtual      │  │   │
│  │  │  Executor  │  │   Manager    │  │  File System  │  │   │
│  │  └────────────┘  └──────────────┘  └───────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │               Security Layer (Rust)                     │   │
│  │  ┌────────────┐  ┌──────────────┐  ┌───────────────┐  │   │
│  │  │   Auth     │  │   Sandbox    │  │   Resource    │  │   │
│  │  │  Service   │  │   Manager    │  │   Limiter     │  │   │
│  │  └────────────┘  └──────────────┘  └───────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### Key Architectural Principles

1. **Separation of Concerns**: Clear boundaries between layers
2. **Single Responsibility**: Each component has one primary purpose
3. **Dependency Inversion**: High-level modules don't depend on low-level details
4. **Security by Default**: All operations sandboxed and validated
5. **Performance First**: Async I/O throughout, minimal copying
6. **Fail Fast**: Input validation at boundaries

---

## Component Architecture

### 1. Client Layer

#### 1.1 Terminal UI Component
**Technology:** xterm.js + TypeScript
**Responsibilities:**
- Render terminal output in browser
- Handle keyboard and mouse input
- Manage terminal state (cursor position, buffer, etc.)
- Apply ANSI color codes and escape sequences
- Provide terminal API for application logic

**Key Interfaces:**
```typescript
interface TerminalUI {
  write(data: string): void;
  onData(callback: (data: string) => void): void;
  onResize(callback: (cols: number, rows: number) => void): void;
  clear(): void;
  resize(cols: number, rows: number): void;
  dispose(): void;
}
```

**Key Dependencies:**
- xterm.js library (v5.0+)
- xterm-addon-fit (terminal resizing)
- xterm-addon-web-links (clickable URLs)

#### 1.2 WebSocket Client Component
**Technology:** Native WebSocket API + TypeScript
**Responsibilities:**
- Establish WebSocket connection to server
- Send user input to server
- Receive command output from server
- Handle connection lifecycle (connect, disconnect, reconnect)
- Implement message protocol

**Key Interfaces:**
```typescript
interface WebSocketClient {
  connect(url: string, token: string): Promise<void>;
  disconnect(): void;
  send(message: Message): void;
  onMessage(callback: (message: Message) => void): void;
  onConnectionChange(callback: (status: ConnectionStatus) => void): void;
}

type Message =
  | { type: 'command', data: string }
  | { type: 'output', data: string }
  | { type: 'error', data: string }
  | { type: 'control', data: ControlMessage };
```

**Key Dependencies:**
- Native WebSocket API
- Exponential backoff for reconnection

---

### 2. Server Layer

#### 2.1 HTTP Server Component
**Technology:** Actix-Web (Rust)
**Responsibilities:**
- Serve static assets (HTML, CSS, JS)
- Handle HTTP API requests
- Upgrade HTTP connections to WebSocket
- Implement authentication middleware
- Enforce rate limiting

**Key Interfaces:**
```rust
pub trait HttpServer {
    async fn start(config: ServerConfig) -> Result<Self>;
    async fn stop(self) -> Result<()>;
    fn add_route(&mut self, path: &str, handler: RouteHandler);
}

pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub max_connections: usize,
}
```

**Key Dependencies:**
- actix-web (web framework)
- actix-files (static file serving)
- rustls (TLS support)

#### 2.2 WebSocket Handler Component
**Technology:** Actix-Web WebSocket + Tokio
**Responsibilities:**
- Accept WebSocket connections
- Parse incoming messages
- Route messages to appropriate handlers
- Send outgoing messages to clients
- Manage connection state
- Implement backpressure handling

**Key Interfaces:**
```rust
pub trait WebSocketHandler {
    async fn on_connect(&mut self, session_id: SessionId) -> Result<()>;
    async fn on_message(&mut self, session_id: SessionId, msg: Message) -> Result<()>;
    async fn on_disconnect(&mut self, session_id: SessionId) -> Result<()>;
    async fn send(&self, session_id: SessionId, msg: Message) -> Result<()>;
}

pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<CloseReason>),
}
```

**Key Dependencies:**
- actix-web-actors (WebSocket support)
- tokio (async runtime)
- futures (stream processing)

#### 2.3 Session Manager Component
**Technology:** Rust + Tokio
**Responsibilities:**
- Create and destroy sessions
- Maintain session registry (in-memory DashMap)
- Assign unique session IDs
- Enforce session limits
- Implement session timeout
- Store session state in memory (ephemeral)

**Key Interfaces:**
```rust
pub trait SessionManager: Send + Sync {
    async fn create_session(&self, user_id: UserId) -> Result<Session>;
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>>;
    async fn destroy_session(&self, session_id: &SessionId) -> Result<()>;
    async fn list_sessions(&self, user_id: &UserId) -> Result<Vec<SessionId>>;
    async fn cleanup_expired_sessions(&self) -> Result<usize>;
}

pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub state: SessionState,
}
```

**Key Dependencies:**
- dashmap (concurrent HashMap)
- tokio::time (timeouts)

---

### 3. Execution Layer

#### 3.1 Command Executor Component
**Technology:** Rust + Tokio Process
**Responsibilities:**
- Parse command strings
- Validate command security
- Execute commands as child processes
- Capture stdout/stderr
- Handle process signals
- Enforce execution timeouts

**Key Interfaces:**
```rust
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, cmd: Command) -> Result<CommandResult>;
    async fn kill(&self, process_id: ProcessId) -> Result<()>;
    fn list_running(&self) -> Vec<ProcessInfo>;
}

pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: PathBuf,
    pub timeout: Option<Duration>,
}

pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub execution_time: Duration,
}
```

**Key Dependencies:**
- tokio::process (async process spawning)
- shell-words (command parsing)

#### 3.2 Process Manager Component
**Technology:** Rust + Tokio
**Responsibilities:**
- Track running processes
- Monitor resource usage (CPU, memory)
- Enforce process limits
- Handle process cleanup
- Stream process output in real-time

**Key Interfaces:**
```rust
pub trait ProcessManager: Send + Sync {
    async fn spawn(&self, cmd: Command, limits: ResourceLimits) -> Result<ProcessHandle>;
    async fn kill(&self, handle: &ProcessHandle) -> Result<()>;
    async fn wait(&self, handle: &ProcessHandle) -> Result<ExitStatus>;
    fn get_info(&self, handle: &ProcessHandle) -> Option<ProcessInfo>;
}

pub struct ResourceLimits {
    pub max_memory: usize,        // bytes
    pub max_cpu_percent: u8,      // 0-100
    pub max_file_descriptors: u32,
    pub timeout: Option<Duration>,
}

pub struct ProcessInfo {
    pub pid: u32,
    pub memory_usage: usize,
    pub cpu_percent: f32,
    pub status: ProcessStatus,
}
```

**Key Dependencies:**
- sysinfo (system resource monitoring)
- tokio (async task management)

#### 3.3 Virtual File System Component
**Technology:** Rust
**Responsibilities:**
- Provide isolated file system per session
- Enforce relative path constraints
- Track file system usage
- Implement file operations
- Apply permission model
- Enforce storage quotas

**Key Interfaces:**
```rust
pub trait VirtualFileSystem: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>>;
    fn create_dir(&self, path: &Path) -> Result<()>;
    fn remove(&self, path: &Path) -> Result<()>;
    fn metadata(&self, path: &Path) -> Result<Metadata>;
    fn disk_usage(&self) -> Result<DiskUsage>;
}

pub struct DiskUsage {
    pub used_bytes: u64,
    pub quota_bytes: u64,
    pub file_count: usize,
}
```

**Key Dependencies:**
- std::fs (file system operations)
- tempfile (temporary workspace directories)

---

### 4. Security Layer

#### 4.1 Authentication Service Component
**Technology:** Rust + JWT
**Responsibilities:**
- Validate authentication tokens
- Issue session tokens
- Enforce authentication policies
- Handle token expiration
- Support multiple auth methods

**Key Interfaces:**
```rust
pub trait AuthService: Send + Sync {
    async fn authenticate(&self, credentials: Credentials) -> Result<AuthToken>;
    async fn validate_token(&self, token: &str) -> Result<UserId>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken>;
    async fn revoke_token(&self, token: &str) -> Result<()>;
}

pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Instant,
    pub user_id: UserId,
}
```

**Key Dependencies:**
- jsonwebtoken (JWT handling)
- argon2 (password hashing)

#### 4.2 Sandbox Manager Component
**Technology:** Rust + OS sandboxing
**Responsibilities:**
- Isolate session execution environments
- Enforce security boundaries
- Prevent privilege escalation
- Monitor sandbox violations
- Apply security policies

**Key Interfaces:**
```rust
pub trait SandboxManager: Send + Sync {
    async fn create_sandbox(&self, config: SandboxConfig) -> Result<Sandbox>;
    async fn destroy_sandbox(&self, sandbox: Sandbox) -> Result<()>;
    fn is_allowed(&self, sandbox: &Sandbox, operation: Operation) -> bool;
}

pub struct SandboxConfig {
    pub allowed_commands: Option<Vec<String>>,
    pub blocked_commands: Vec<String>,
    pub network_enabled: bool,
    pub max_processes: usize,
}
```

**Key Dependencies:**
- nix (OS-level sandboxing primitives)

#### 4.3 Resource Limiter Component
**Technology:** Rust
**Responsibilities:**
- Enforce resource limits per session
- Track resource usage
- Terminate violating sessions
- Report resource metrics
- Implement rate limiting

**Key Interfaces:**
```rust
pub trait ResourceLimiter: Send + Sync {
    fn check_limit(&self, session_id: &SessionId, resource: Resource) -> Result<()>;
    fn track_usage(&self, session_id: &SessionId, resource: Resource, amount: u64);
    fn get_usage(&self, session_id: &SessionId) -> ResourceUsage;
    fn reset_limits(&self, session_id: &SessionId);
}

pub enum Resource {
    Memory,
    Cpu,
    Disk,
    Network,
    FileDescriptors,
}

pub struct ResourceUsage {
    pub memory: u64,
    pub cpu_percent: f32,
    pub disk_bytes: u64,
    pub network_bytes: u64,
}
```

**Key Dependencies:**
- sysinfo (resource monitoring)
- parking_lot (efficient locks)

---

## Data Flow

### Command Execution Flow

```
┌──────────┐     1. Input     ┌─────────────┐
│  Client  ├─────────────────►│ WebSocket   │
│ (Browser)│                  │  Handler    │
└──────────┘                  └──────┬──────┘
     ▲                               │ 2. Parse & Validate
     │                               ▼
     │                        ┌──────────────┐
     │                        │   Session    │
     │                        │   Manager    │
     │                        └──────┬───────┘
     │                               │ 3. Get Session
     │                               ▼
     │                        ┌──────────────┐
     │                        │    Auth      │
     │                        │   Service    │
     │                        └──────┬───────┘
     │                               │ 4. Check Permissions
     │                               ▼
     │                        ┌──────────────┐
     │                        │   Sandbox    │
     │                        │   Manager    │
     │                        └──────┬───────┘
     │                               │ 5. Check Allowed
     │                               ▼
     │                        ┌──────────────┐
     │                        │   Command    │
     │                        │   Executor   │
     │                        └──────┬───────┘
     │                               │ 6. Execute
     │                               ▼
     │                        ┌──────────────┐
     │                        │   Process    │
     │                        │   Manager    │
     │                        └──────┬───────┘
     │                               │ 7. Stream Output
     │                               ▼
     │                        ┌──────────────┐
     │                        │  WebSocket   │
     │                        │   Handler    │
     │                        └──────┬───────┘
     │ 8. Output                     │
     └───────────────────────────────┘
```

### Session Lifecycle Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Session Lifecycle                         │
└─────────────────────────────────────────────────────────────┘

1. Connection Request
   ├─► HTTP Upgrade to WebSocket
   ├─► Auth Token Validation
   └─► Session Creation

2. Active Session
   ├─► Command Execution
   ├─► Output Streaming
   ├─► Resource Monitoring
   └─► Activity Tracking

3. Disconnection (Network Issue)
   ├─► Buffer Output
   ├─► Pause Processes
   └─► Wait for Reconnection (5 min)

4. Reconnection
   ├─► Resume Session
   ├─► Replay Buffered Output
   └─► Resume Processes

5. Session Termination
   ├─► Kill All Processes
   ├─► Clean Up Resources
   ├─► Save Session Logs
   └─► Remove from Registry
```

---

## Technology Stack

### Backend Stack

| Component | Technology | Version | Justification |
|-----------|-----------|---------|---------------|
| Language | Rust | 1.75+ | Memory safety, performance, WASM support |
| Web Framework | Actix-Web | 4.x | Highest performance, WebSocket support |
| Async Runtime | Tokio | 1.x | Industry standard, mature ecosystem |
| Serialization | serde + serde_json | 1.x | Fast, type-safe JSON handling |
| WebSocket | actix-web-actors | 4.x | Native WebSocket integration |
| Authentication | jsonwebtoken | 9.x | JWT standard implementation |
| Logging | tracing + tracing-subscriber | 0.1.x | Structured logging, performance |
| Metrics | prometheus | 0.13.x | Industry standard monitoring |
| Testing | tokio-test, mockall | latest | Async testing, mocking |

### Frontend Stack

| Component | Technology | Version | Justification |
|-----------|-----------|---------|---------------|
| Language | TypeScript | 5.x | Type safety, tooling support |
| Terminal Emulator | xterm.js | 5.x | Industry standard, feature-rich |
| Build Tool | Vite | 5.x | Fast builds, modern features |
| Package Manager | pnpm | 8.x | Fast, disk-efficient |
| Testing | Vitest | 1.x | Fast, Vite-native |

### DevOps Stack

| Component | Technology | Version | Justification |
|-----------|-----------|---------|---------------|
| Containerization | Docker | 24.x | Standard deployment format |
| CI/CD | GitHub Actions | N/A | Integrated with repository |
| Monitoring | Prometheus + Grafana | latest | Industry standard |
| Logging | Loki | latest | Grafana stack integration |

---

## Security Architecture

### Defense in Depth Strategy

```
┌─────────────────────────────────────────────────────────────┐
│              Layer 1: Network Security                       │
│  • TLS 1.3 Encryption                                       │
│  • HTTPS Only (HSTS)                                        │
│  • CORS Policy                                              │
│  • Rate Limiting                                            │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│           Layer 2: Authentication & Authorization            │
│  • JWT Token Validation                                     │
│  • Session Token Expiry                                     │
│  • Role-Based Access Control                                │
│  • Multi-Factor Authentication (Optional)                   │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              Layer 3: Input Validation                       │
│  • Command Syntax Validation                                │
│  • Path Traversal Prevention                                │
│  • Command Injection Prevention                             │
│  • Message Schema Validation                                │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│             Layer 4: Sandbox Isolation                       │
│  • Process Isolation                                        │
│  • File System Restrictions                                 │
│  • Network Isolation                                        │
│  • Resource Limits                                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│           Layer 5: Monitoring & Auditing                     │
│  • Security Event Logging                                   │
│  • Anomaly Detection                                        │
│  • Audit Trail                                              │
│  • Alerting                                                 │
└─────────────────────────────────────────────────────────────┘
```

### Security Controls

1. **Encryption**: All data in transit encrypted with TLS 1.3
2. **Authentication**: JWT tokens with expiration and refresh
3. **Authorization**: RBAC with principle of least privilege
4. **Sandboxing**: Process isolation with resource limits
5. **Input Validation**: All inputs validated and sanitized
6. **Audit Logging**: All security-relevant events logged
7. **Rate Limiting**: Prevent abuse and DoS attacks
8. **CSP Headers**: Prevent XSS attacks
9. **CORS Policy**: Restrict cross-origin requests
10. **Secrets Management**: No hardcoded secrets, environment variables only

---

## Deployment Architecture

### Single-Server Deployment

```
┌───────────────────────────────────────────────────────────────┐
│                        Docker Host                             │
│                                                                │
│  ┌──────────────────────────────────────────────────────┐    │
│  │           web-terminal Container                     │    │
│  │                                                       │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │    Actix-Web Server (Port 8080)             │    │    │
│  │  │  • HTTP/HTTPS Handler                       │    │    │
│  │  │  • WebSocket Handler                        │    │    │
│  │  │  • Session Manager                          │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  │                                                       │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │    Execution Layer                          │    │    │
│  │  │  • Command Executor                         │    │    │
│  │  │  • Process Manager                          │    │    │
│  │  │  • Virtual File System                      │    │    │
│  │  └─────────────────────────────────────────────┘    │    │
│  │                                                       │    │
│  │  Volumes:                                             │    │
│  │  • /data/sessions (session persistence)              │    │
│  │  • /data/workspaces (user workspaces)                │    │
│  │  • /data/logs (application logs)                     │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                                │
│  Port Mapping: 8080:8080 (Single Port for All Traffic)       │
│  • HTTP/HTTPS                                                  │
│  • WebSocket                                                   │
│  • Static Assets                                               │
│  • API Endpoints                                               │
└───────────────────────────────────────────────────────────────┘
```

### Multi-Server Deployment (Future)

```
┌────────────────────────────────────────────────────────────────┐
│                       Load Balancer                             │
│                    (nginx / HAProxy)                            │
└─────────────┬─────────────────────────────┬────────────────────┘
              │                             │
      ┌───────▼────────┐           ┌───────▼────────┐
      │   Server 1     │           │   Server 2     │
      │  web-terminal  │           │  web-terminal  │
      └───────┬────────┘           └───────┬────────┘
              │                             │
              └──────────┬──────────────────┘
                         │
                  ┌──────▼───────┐
                  │    Redis     │
                  │ (Shared State)│
                  └──────────────┘
```

---

## Scalability Considerations

### Horizontal Scaling Strategy

1. **In-Memory Session Storage**
   - Session state stored in DashMap (per ADR 012-data-storage-decision.md)
   - No persistent database required
   - Use sticky sessions for load balancing (session affinity)
   - Future: Optional Redis backend for shared session state

2. **Resource Pooling**
   - Process pool for command execution
   - Thread pool for I/O operations
   - In-memory data structures (DashMap for sessions)

3. **Async I/O Throughout**
   - Non-blocking I/O operations
   - Efficient use of system threads
   - High concurrency support

4. **Caching Strategy**
   - Cache frequently accessed data
   - Reduce redundant computations
   - TTL-based cache invalidation

### Vertical Scaling Limits

| Resource | Limit per Server | Notes |
|----------|-----------------|-------|
| Concurrent Sessions | 10,000 | Depends on session activity |
| Memory | 16-32 GB | Allows 512MB per session avg |
| CPU | 8-16 cores | Async I/O minimizes CPU usage |
| Disk I/O | SSD required | Fast storage for workspaces |
| Network | 1 Gbps+ | WebSocket bandwidth intensive |

---

## Architecture Decision Records

### ADR-000: In-Memory Storage Only

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Need data storage strategy for session state, command history, and user context
**Decision:** Use in-memory storage only (DashMap), no persistent database
**Consequences:**
- ✅ Simple deployment (single binary, no database)
- ✅ Fast performance (direct memory access)
- ✅ Security (no persistent sensitive data)
- ✅ Crash recovery is simple (clean slate)
- ❌ Session loss on server restart
- ❌ No command history across sessions
- ❌ Requires sticky sessions for load balancing

**Full Documentation:** See [012-data-storage-decision.md](./012-data-storage-decision.md) for comprehensive details

---

### ADR-001: Rust for Backend

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Need high-performance backend with memory safety
**Decision:** Use Rust for backend implementation
**Consequences:**
- ✅ Memory safety without GC
- ✅ Excellent async support (Tokio)
- ✅ Strong type system
- ✅ WASM compilation support
- ❌ Steeper learning curve
- ❌ Slower compile times

**Alternatives Considered:**
- Go: Rejected due to GC pauses, larger binaries
- Node.js: Rejected due to performance concerns
- C++: Rejected due to memory safety issues

---

### ADR-002: Actix-Web Framework

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Need high-performance web framework with WebSocket support
**Decision:** Use Actix-Web for HTTP and WebSocket handling
**Consequences:**
- ✅ Highest performance Rust web framework
- ✅ Native WebSocket support
- ✅ Actor model for concurrency
- ✅ Mature and stable
- ❌ Actor model learning curve

**Alternatives Considered:**
- Axum: Less mature, though growing rapidly
- Rocket: Less async support, slower
- Warp: Steeper learning curve

---

### ADR-003: xterm.js for Terminal Emulation

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Need robust browser-based terminal emulator
**Decision:** Use xterm.js as terminal emulation library
**Consequences:**
- ✅ Industry standard, battle-tested
- ✅ Rich feature set (ANSI, colors, etc.)
- ✅ Active development and community
- ✅ Plugin ecosystem
- ❌ Dependency on external library

**Alternatives Considered:**
- Custom implementation: Too complex, reinventing wheel
- terminal.js: Less mature, fewer features
- Hyper terminal: Electron-based, not web-native

---

### ADR-004: Single Port Architecture

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Simplify deployment and firewall configuration
**Decision:** Serve HTTP, WebSocket, and static assets on single configurable port
**Consequences:**
- ✅ Simplified firewall rules (one port to open)
- ✅ Easier deployment (no port coordination needed)
- ✅ No port conflicts
- ✅ Simplified client configuration (automatic URL detection)
- ✅ Better for containerization (single exposed port)
- ✅ Frontend uses relative paths automatically
- ❌ All services share same port (not an issue with modern async servers)

**Implementation Details:**
- Frontend detects base URL from `window.location`
- WebSocket URLs constructed dynamically with correct protocol (ws:// or wss://)
- All API requests use relative paths
- Actix-Web routes configured to serve static files on same server

**Alternatives Considered:**
- Separate ports for frontend/backend: Rejected, violates single-port principle
- Separate ports for HTTP/WebSocket: Rejected, adds unnecessary complexity
- Reverse proxy with multiple backends: Rejected, over-engineering for simple use case

---

### ADR-005: Virtual File System with Relative Paths

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Enforce security boundary for file operations
**Decision:** Implement virtual file system with relative path constraints
**Consequences:**
- ✅ Strong security boundary
- ✅ Prevents path traversal
- ✅ Easy quota enforcement
- ❌ Limited file system access (by design)

**Alternatives Considered:**
- Chroot: OS-dependent, complex setup
- Full file system access: Security risk

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial architecture specification |

---

## Approvals

- [ ] Technical Lead
- [ ] Security Architect
- [ ] Architecture Review Board
- [ ] DevOps Lead