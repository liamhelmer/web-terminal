# Web-Terminal

[![CI](https://github.com/liamhelmer/web-terminal/actions/workflows/ci-rust.yml/badge.svg)](https://github.com/liamhelmer/web-terminal/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A modern, browser-based terminal emulator with local execution capabilities. Built with Rust and WebAssembly for native-level performance and enterprise-grade security.

## Overview

Web-Terminal provides a fully functional command-line interface accessible from any modern browser. It combines the power of native terminal functionality with the accessibility of web applications, using a sandboxed execution environment for security.

### Key Features

- **Full Terminal Emulation**: ANSI escape sequences, 256-color support, VT100/xterm compatibility
- **Real-Time Communication**: WebSocket-based streaming with <20ms latency
- **Security First**: WASM sandboxing, JWT/JWKS authentication, process isolation
- **High Performance**: Sub-100ms command execution, supports 10,000+ concurrent sessions
- **Single-Port Architecture**: All traffic (HTTP, WebSocket, static assets) on one configurable port
- **Enterprise Authentication**: JWT/JWKS integration with Backstage, Auth0, Okta, and other OAuth2/OIDC providers
- **Multi-Session Support**: Persistent sessions with state management
- **File Operations**: Virtual file system with upload/download capabilities

## Quick Start

### Prerequisites

- **Rust**: 1.83+ (MSRV)
- **Node.js**: 20+ LTS
- **pnpm**: 8+

### Build and Run

#### Backend (Rust)

```bash
# Build release binary
cargo build --release

# Run tests
cargo test

# Start server (default: http://localhost:8080)
cargo run --release

# Or with custom configuration
cargo run --release -- --port 8080 --config config/production.toml
```

#### Frontend (TypeScript + Vite)

```bash
cd frontend

# Install dependencies
pnpm install

# Build for production
pnpm run build

# Run tests
pnpm run test

# Type checking
pnpm run typecheck
```

#### Integration Testing

```bash
# Requires backend running on port 8080
cd frontend
pnpm run test:e2e
```

#### Benchmarks

```bash
# Run performance benchmarks
cargo bench
```

### Docker Deployment

```bash
# Build multi-arch image
docker build -t web-terminal:latest .

# Run container (single port)
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/config:/app/config:ro \
  --name web-terminal \
  web-terminal:latest

# With docker-compose
docker-compose up -d

# View logs
docker-compose logs -f web-terminal
```

## Architecture

### Single-Port Design

Web-Terminal uses a **single configurable port** for all traffic:

- **HTTP/HTTPS**: Static assets, API endpoints, health checks
- **WebSocket**: Real-time terminal I/O streaming
- **Authentication**: JWT bearer tokens via Authorization header

**Default Port**: 8080 (configurable via `--port` flag or `WEB_TERMINAL_PORT` environment variable)

### Technology Stack

**Backend:**
- Rust 1.83+ (memory safety, performance)
- Actix-Web 4.11 (web framework)
- Tokio 1.47 (async runtime)
- JWKS + jsonwebtoken 9 (authentication)

**Frontend:**
- TypeScript 5.9
- xterm.js 5.3 (terminal emulator)
- Vite 7.1 (build tool)

**Authentication:**
- JWT/JWKS (public key cryptography)
- No shared secrets required
- Seamless key rotation support

### System Architecture

```
┌─────────────────────────────────────────────┐
│           Browser (Client)                  │
│  ┌─────────────┐    ┌─────────────────┐    │
│  │  xterm.js   │◄───┤   WebSocket     │    │
│  │  Terminal   │    │   Client        │    │
│  └─────────────┘    └─────────────────┘    │
└──────────────┬──────────────────────────────┘
               │ Single Port (8080)
               │ WebSocket + JWT Token
┌──────────────▼──────────────────────────────┐
│        Actix-Web Server (Rust)              │
│  ┌──────────────────────────────────────┐  │
│  │  Authentication & Authorization      │  │
│  │  • JWKS Client (Backstage, Auth0)    │  │
│  │  • JWT Verification (RS256/RS384)    │  │
│  │  • User/Group Authorization          │  │
│  │  • Audit Logging                     │  │
│  └──────────────────────────────────────┘  │
│  ┌──────────────────────────────────────┐  │
│  │  Session Manager (In-Memory)         │  │
│  │  • DashMap (concurrent HashMap)      │  │
│  │  • Session State Management          │  │
│  └──────────────────────────────────────┘  │
│  ┌──────────────────────────────────────┐  │
│  │  Execution Layer                     │  │
│  │  • PTY Spawning                      │  │
│  │  • Process Management                │  │
│  │  • Virtual File System               │  │
│  └──────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## Configuration

### Environment Variables

```bash
# Server Configuration
WEB_TERMINAL_PORT=8080              # Single port for all traffic
WEB_TERMINAL_HOST=0.0.0.0           # Bind address
WEB_TERMINAL_LOG_LEVEL=info         # Logging level

# Authentication (Optional - see Authentication section)
AUTH_ENABLED=true                   # Enable JWT/JWKS authentication
AUTH_JWKS_PROVIDERS='[{"name":"backstage","url":"https://backstage.example.com/.well-known/jwks.json","issuer":"https://backstage.example.com"}]'
AUTH_ALLOWED_USERS='["user:default/admin"]'
AUTH_ALLOWED_GROUPS='["group:default/platform-team"]'
AUTH_JWKS_CACHE_TTL=3600            # JWKS cache TTL (seconds)
AUTH_AUDIT_LOG=true                 # Enable audit logging

# Session Configuration
WEB_TERMINAL_SESSION_TIMEOUT=3600   # Session timeout (seconds)
WEB_TERMINAL_MAX_SESSIONS=10000     # Max concurrent sessions

# Resource Limits
WEB_TERMINAL_MAX_MEMORY_MB=512      # Memory per session (MB)
WEB_TERMINAL_MAX_CPU_PERCENT=50     # CPU per session (%)
```

### Configuration File

Create `config/production.toml`:

```toml
[server]
port = 8080                    # Single port for all traffic
host = "0.0.0.0"
workers = 8                    # Auto-detected from CPU cores

[session]
timeout_seconds = 3600
max_sessions = 10000

[limits]
max_memory_mb = 512
max_cpu_percent = 50
max_disk_mb = 1024

[logging]
level = "info"
format = "json"
```

Load configuration:

```bash
cargo run --release -- --config config/production.toml
```

## Authentication

Web-Terminal supports **JWT/JWKS authentication** for enterprise integration with Backstage, Auth0, Okta, and other OAuth2/OIDC providers.

### JWKS Authentication Setup

**1. Configure JWKS Provider (e.g., Backstage):**

```bash
# Set environment variables
export AUTH_ENABLED=true
export AUTH_JWKS_PROVIDERS='[
  {
    "name": "backstage",
    "url": "https://backstage.example.com/.well-known/jwks.json",
    "issuer": "https://backstage.example.com"
  }
]'
```

**2. Configure Authorization:**

```bash
# Allow specific users (Backstage entity format)
export AUTH_ALLOWED_USERS='["user:default/admin","user:default/platform-ops"]'

# Allow groups (any member can access)
export AUTH_ALLOWED_GROUPS='["group:default/platform-team","group:default/sre-team"]'

# Enable audit logging
export AUTH_AUDIT_LOG=true
```

**3. Test JWKS Endpoint Accessibility:**

```bash
# CRITICAL: JWKS endpoint must be reachable from web-terminal server
curl -v https://backstage.example.com/.well-known/jwks.json

# Expected: HTTP 200 with JSON containing "keys" array
# If this fails, authentication will not work
```

**4. Client Connection:**

```javascript
// Frontend sends JWT token in Authorization header
const ws = new WebSocket('ws://localhost:8080/ws');
ws.addEventListener('open', () => {
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'Bearer eyJhbGciOiJSUzI1NiIs...'
  }));
});
```

### Authentication Flow

```
1. User authenticates with identity provider (Backstage, Auth0, etc.)
2. Provider issues JWT token with user/group claims
3. Client connects to web-terminal WebSocket with JWT
4. Web-terminal fetches JWKS public keys (cached)
5. JWT signature verified using public key
6. User/group claims extracted and checked against allowed lists
7. Session created with authenticated user context
8. All actions audit logged with user identity
```

### Key Features

- **No Shared Secrets**: Uses public key cryptography (JWKS)
- **Seamless Key Rotation**: Automatic key refresh from provider
- **Multi-Provider Support**: Multiple JWKS providers simultaneously
- **Backstage Integration**: Native support for Backstage entity references
- **Audit Logging**: All auth events logged with user context

### Troubleshooting Authentication

**Common Issues:**

1. **JWKS endpoint unreachable**:
   - Ensure DNS resolution works from container
   - Check firewall rules allow HTTPS to JWKS endpoint
   - Test with `curl` from inside container

2. **JWT verification fails**:
   - Check token expiration (`exp` claim)
   - Verify issuer matches JWKS provider config
   - Ensure `kid` (key ID) exists in JWKS

3. **Authorization denied**:
   - Check user ID format (e.g., "user:default/john.doe")
   - Verify group memberships in JWT claims
   - Review audit logs for authorization decisions

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_session_creation

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Start backend
cargo run --release &

# Run E2E tests (Playwright)
cd frontend
pnpm run test:e2e
```

### Performance Benchmarks

```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench command_execution
```

### Performance Targets (per spec-kit)

- Command execution latency: **< 100ms** (p95)
- WebSocket message latency: **< 20ms** (p95)
- Time to first input: **< 500ms**
- Memory per session: **< 50MB** (idle), **< 512MB** (active)
- Concurrent sessions: **10,000+** per server

## API Documentation

### REST API Endpoints

```
GET  /health                    # Health check
GET  /api/v1/sessions           # List sessions
POST /api/v1/sessions           # Create session
GET  /api/v1/sessions/:id       # Get session info
DELETE /api/v1/sessions/:id     # Destroy session
GET  /api/v1/metrics            # Prometheus metrics
```

### WebSocket Protocol

**Connect:**
```
WS /ws?session_id={uuid}
Authorization: Bearer {jwt_token}
```

**Messages:**
```json
// Client → Server (Command Input)
{
  "type": "input",
  "data": "ls -la\n"
}

// Server → Client (Output)
{
  "type": "output",
  "data": "total 48\ndrwxr-xr-x..."
}

// Server → Client (Error)
{
  "type": "error",
  "data": "Command not found: invalid"
}
```

Full API documentation: [docs/spec-kit/006-api-spec.md](docs/spec-kit/006-api-spec.md)

## Development

### Project Structure

```
web-terminal/
├── src/                    # Rust backend source
│   ├── main.rs            # Application entry point
│   ├── server.rs          # Actix-Web server setup
│   ├── session/           # Session management
│   ├── auth/              # Authentication (JWT/JWKS)
│   ├── pty/               # PTY spawning and management
│   └── websocket/         # WebSocket handlers
├── frontend/              # TypeScript frontend
│   ├── src/
│   │   ├── main.ts        # Entry point
│   │   ├── terminal.ts    # xterm.js integration
│   │   └── websocket.ts   # WebSocket client
│   ├── tests/             # Vitest unit tests
│   └── e2e/               # Playwright E2E tests
├── docs/
│   └── spec-kit/          # Architecture specifications
├── config/                # Configuration files
├── tests/                 # Rust integration tests
├── benches/               # Performance benchmarks
└── Cargo.toml             # Rust dependencies
```

### Building from Source

```bash
# Clone repository
git clone https://github.com/liamhelmer/web-terminal.git
cd web-terminal

# Build backend
cargo build --release

# Build frontend
cd frontend
pnpm install
pnpm run build
cd ..

# Run
./target/release/web-terminal
```

### Code Style

```bash
# Format Rust code
cargo fmt

# Lint Rust code
cargo clippy -- -D warnings

# Format TypeScript code
cd frontend
pnpm run format

# Lint TypeScript code
pnpm run lint
```

## Deployment

### Docker Compose (Recommended)

```yaml
# docker-compose.yml
version: '3.8'

services:
  web-terminal:
    image: web-terminal:latest
    ports:
      - "8080:8080"              # Single port for all traffic
    environment:
      - WEB_TERMINAL_PORT=8080
      - AUTH_ENABLED=true
      - AUTH_JWKS_PROVIDERS=[{"name":"backstage","url":"https://backstage.example.com/.well-known/jwks.json","issuer":"https://backstage.example.com"}]
      - AUTH_ALLOWED_GROUPS=["group:default/platform-team"]
    volumes:
      - ./config:/app/config:ro
      - workspaces:/app/data/workspaces
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3

volumes:
  workspaces:
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web-terminal
spec:
  replicas: 3
  selector:
    matchLabels:
      app: web-terminal
  template:
    metadata:
      labels:
        app: web-terminal
    spec:
      containers:
      - name: web-terminal
        image: web-terminal:latest
        ports:
        - containerPort: 8080
          name: http
        env:
        - name: AUTH_ENABLED
          value: "true"
        - name: AUTH_JWKS_PROVIDERS
          valueFrom:
            configMapKeyRef:
              name: web-terminal-config
              key: jwks-providers
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
```

### GitHub Actions CI/CD

All deployments **MUST** pass CI checks:

- Rust tests, clippy, fmt, security audit (cargo audit)
- Frontend tests, lint, typecheck
- Integration E2E tests (Playwright)
- Security scanning (OWASP ZAP, Trivy)

See [.github/workflows/](.github/workflows/) for complete CI/CD pipeline.

## Monitoring

### Health Check

```bash
curl http://localhost:8080/health
```

### Prometheus Metrics

```bash
curl http://localhost:8080/api/v1/metrics
```

**Key Metrics:**
- `web_terminal_active_sessions` - Current session count
- `web_terminal_command_duration_seconds` - Command execution latency
- `web_terminal_websocket_messages_total` - WebSocket message count
- `web_terminal_memory_usage_bytes` - Memory usage per session

### Logs

Structured JSON logging:

```json
{
  "timestamp": "2025-09-29T10:00:00Z",
  "level": "info",
  "target": "web_terminal::session",
  "message": "Session created",
  "session_id": "abc123",
  "user_id": "user:default/john.doe"
}
```

## Security

### Security Features

- **Authentication**: JWT/JWKS with public key verification
- **Authorization**: User and group-based access control
- **Sandboxing**: Process isolation with resource limits
- **Audit Logging**: All security events logged
- **TLS**: HTTPS/WSS encryption (production)
- **CSP Headers**: Content Security Policy
- **CORS**: Cross-origin request restrictions
- **Rate Limiting**: API and WebSocket rate limits

### Security Best Practices

1. **Always use HTTPS/WSS in production**
2. **Configure JWT/JWKS authentication** (disabled by default for development)
3. **Set resource limits** to prevent DoS
4. **Enable audit logging** for compliance
5. **Regularly update dependencies** (Dependabot enabled)
6. **Run security scans** (cargo audit, npm audit in CI)

### Reporting Security Issues

**DO NOT** open public issues for security vulnerabilities.

Email: security@example.com (replace with actual contact)

## Contributing

### Development Workflow

1. Fork the repository
2. Create feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Run tests: `cargo test && cd frontend && pnpm run test`
5. Format code: `cargo fmt && cd frontend && pnpm run format`
6. Commit changes: `git commit -m "feat: add my feature"`
7. Push branch: `git push origin feature/my-feature`
8. Open Pull Request

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: Add PTY spawning support
fix: Resolve WebSocket reconnection bug
docs: Update authentication guide
test: Add integration tests for session manager
refactor: Simplify command executor
perf: Optimize WebSocket message handling
```

### Code Review

All changes require:
- Passing CI checks (tests, lint, security)
- Code review approval (1+ reviewer)
- No merge conflicts
- Updated documentation

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Support

- **Documentation**: [docs/spec-kit/](docs/spec-kit/)
- **Issues**: [GitHub Issues](https://github.com/liamhelmer/web-terminal/issues)
- **Discussions**: [GitHub Discussions](https://github.com/liamhelmer/web-terminal/discussions)

## Acknowledgments

Built with:
- [Rust](https://rust-lang.org) - Systems programming language
- [Actix-Web](https://actix.rs) - Web framework
- [xterm.js](https://xtermjs.org) - Terminal emulator
- [Tokio](https://tokio.rs) - Async runtime

## Roadmap

### v1.0 (Current)
- ✅ Core terminal emulation
- ✅ WebSocket real-time communication
- ✅ JWT/JWKS authentication
- ✅ Session management
- ✅ Virtual file system
- ✅ Single-port architecture

### v1.1 (Planned)
- Multi-user session sharing
- Advanced ANSI support (truecolor)
- File upload/download UI
- Command history persistence

### v2.0 (Future)
- Horizontal scaling (Redis session storage)
- Plugin/extension system
- Advanced security features (2FA)
- Mobile app support

---

**Web-Terminal** - Browser-based terminal with enterprise security and native performance.