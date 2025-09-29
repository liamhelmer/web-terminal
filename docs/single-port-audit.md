# Single-Port Architecture Compliance Audit

**Project:** web-terminal
**Date:** 2025-09-29
**Audited By:** Code Review Agent
**Status:** Completed

---

## Executive Summary

This audit reviewed all 11 specification documents in `/docs/spec-kit/` to identify violations of the single-port design principle. The project generally adheres to the single-port architecture requirement, but several areas need clarification and minor corrections.

**Total Violations Found:** 8
- **Critical:** 0
- **Major:** 3
- **Minor:** 5

---

## Audit Findings

### MAJOR VIOLATIONS

#### 1. Frontend Development Server Configuration (004-frontend-spec.md)

**Location:** `004-frontend-spec.md:677-686`

**Violation Type:** Multi-port development configuration

**Severity:** MAJOR

**Current Code:**
```typescript
export default defineConfig({
  build: {
    target: 'es2020',
    outDir: 'dist',
    sourcemap: true,
    minify: 'terser',
  },
  server: {
    port: 3000,
    proxy: {
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
      },
    },
  },
});
```

**Issue:** Development configuration shows frontend dev server on port 3000 with proxy to backend on port 8080, creating a multi-port development workflow.

**Impact:**
- Developers may mistakenly deploy multi-port configurations
- Documentation suggests separate ports for frontend/backend
- Violates the principle that HTTP and WebSocket must share the same port

**Recommended Fix:**
```typescript
export default defineConfig({
  build: {
    target: 'es2020',
    outDir: 'dist',
    sourcemap: true,
    minify: 'terser',
  },
  server: {
    port: 3000, // Development only
    proxy: {
      // Proxy all requests to single backend port
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
      },
      '/api': {
        target: 'http://localhost:8080',
      },
    },
  },
  // Note: Production build serves from single port (backend serves static files)
});
```

**Add clarification comment:**
```markdown
### Development vs Production

**Development Mode:**
- Frontend dev server: Port 3000 (Vite with HMR)
- Backend server: Port 8080
- Proxy configuration routes requests to backend

**Production Mode:**
- Single port: 8080 (configurable)
- Backend serves static frontend assets
- HTTP and WebSocket on same port
```

---

#### 2. Missing Single-Port Architecture Emphasis (006-api-spec.md)

**Location:** `006-api-spec.md:14`

**Violation Type:** Hardcoded base URL without single-port emphasis

**Severity:** MAJOR

**Current Code:**
```markdown
**Base URL:** `http://localhost:8080/api/v1`
```

**Issue:**
- Hardcoded host and port in API documentation
- No mention of single-port requirement
- Could lead to assumptions about multiple service ports

**Impact:**
- API consumers may assume separate service ports
- Integration code may hardcode URLs incorrectly

**Recommended Fix:**
```markdown
**Base URL Pattern:** `{protocol}://{host}:{port}/api/v1`

**Single-Port Architecture:**
All services (HTTP API, WebSocket, static assets) operate on a single configurable port (default: 8080).

**Examples:**
- Development: `http://localhost:8080/api/v1`
- Production: `https://terminal.example.com/api/v1`

**Important:** Never hardcode absolute URLs. Always use relative paths in client code:
- Correct: `/api/v1/sessions`
- Incorrect: `http://localhost:8080/api/v1/sessions`
```

---

#### 3. WebSocket URL Construction (007-websocket-spec.md)

**Location:** `007-websocket-spec.md:14`

**Violation Type:** Hardcoded WebSocket URL

**Severity:** MAJOR

**Current Code:**
```markdown
**WebSocket URL:** `ws://localhost:8080/ws` or `wss://localhost:8080/ws` (TLS)
```

**Issue:**
- Hardcoded host and port
- No guidance on dynamic URL construction
- Violates relative path principle

**Impact:**
- Client code may hardcode WebSocket URLs
- Breaks portability across environments

**Recommended Fix:**
```markdown
**WebSocket Endpoint:** `/ws` (relative path)

**URL Construction:**
WebSocket URLs MUST be constructed dynamically based on the current page location:

```typescript
function buildWebSocketUrl(token?: string): string {
  // Use current page protocol and host
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = location.host; // Includes port if non-standard

  let url = `${protocol}//${host}/ws`;
  if (token) {
    url += `?token=${encodeURIComponent(token)}`;
  }

  return url;
}
```

**Examples:**
- If page is at `http://localhost:8080`, WebSocket: `ws://localhost:8080/ws`
- If page is at `https://terminal.example.com`, WebSocket: `wss://terminal.example.com/ws`

**Anti-patterns (DO NOT USE):**
```typescript
// ❌ WRONG: Hardcoded URL
const ws = new WebSocket('ws://localhost:8080/ws');

// ✅ CORRECT: Dynamic construction
const ws = new WebSocket(buildWebSocketUrl(token));
```
```

---

### MINOR VIOLATIONS

#### 4. CLI Port Configuration (005-cli-spec.md)

**Location:** `005-cli-spec.md:46-51`

**Violation Type:** Port configuration without single-port context

**Severity:** MINOR

**Current Code:**
```bash
OPTIONS:
  --port <PORT>         Server port (default: 8080)
  --host <HOST>         Server host (default: 0.0.0.0)
  --workers <NUM>       Number of worker threads (default: auto)
```

**Issue:** Documentation doesn't emphasize that this is the ONLY port needed

**Recommended Fix:**
```bash
OPTIONS:
  --port <PORT>         Single server port for all services (default: 8080)
                        Serves HTTP API, WebSocket, and static assets
  --host <HOST>         Server host (default: 0.0.0.0)
  --workers <NUM>       Number of worker threads (default: auto)

EXAMPLES:
  # Start server on default port (8080)
  web-terminal start

  # Start on custom port
  web-terminal start --port 3000

  # All services available at:
  # - HTTP API: http://localhost:3000/api/v1
  # - WebSocket: ws://localhost:3000/ws
  # - Static assets: http://localhost:3000/
```

---

#### 5. Configuration File Port Settings (005-cli-spec.md)

**Location:** `005-cli-spec.md:332-336`

**Violation Type:** Missing single-port architecture documentation

**Severity:** MINOR

**Current Code:**
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4
max_connections = 10000
```

**Recommended Fix:**
```toml
[server]
# Single port for all services (HTTP, WebSocket, static assets)
host = "0.0.0.0"
port = 8080
workers = 4
max_connections = 10000

# Note: Changing this port affects:
# - REST API (http://host:port/api/v1/*)
# - WebSocket (ws://host:port/ws)
# - Static frontend (http://host:port/)
```

---

#### 6. Frontend WebSocket Client URL Construction (004-frontend-spec.md)

**Location:** `004-frontend-spec.md:422-431`

**Violation Type:** Non-relative URL construction

**Severity:** MINOR

**Current Code:**
```typescript
private buildUrl(): string {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  let url = `${protocol}//${this.config.url}/ws`;

  if (this.config.token) {
    url += `?token=${this.config.token}`;
  }

  return url;
}
```

**Issue:** Uses `this.config.url` which may contain hardcoded host/port

**Recommended Fix:**
```typescript
private buildUrl(): string {
  // Always use current page location for single-port architecture
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = location.host; // Includes port automatically

  let url = `${protocol}//${host}/ws`;

  if (this.config.token) {
    url += `?token=${encodeURIComponent(this.config.token)}`;
  }

  return url;
}
```

**Update interface:**
```typescript
export interface WebSocketConfig {
  // Remove url field - always derive from location
  token?: string;
  reconnectDelay?: number;
  maxReconnectAttempts?: number;
}
```

---

#### 7. Docker Compose Port Mapping (009-deployment-spec.md)

**Location:** `009-deployment-spec.md:192-194`

**Violation Type:** Missing single-port documentation

**Severity:** MINOR

**Current Code:**
```yaml
ports:
  - "8080:8080"
```

**Recommended Fix:**
```yaml
ports:
  # Single port mapping for all services
  - "8080:8080"  # HTTP API + WebSocket + Static Assets
  # Note: All services accessible via this single port:
  # - Frontend: http://localhost:8080/
  # - REST API: http://localhost:8080/api/v1/*
  # - WebSocket: ws://localhost:8080/ws
```

---

#### 8. Kubernetes Service Configuration (009-deployment-spec.md)

**Location:** `009-deployment-spec.md:371-374`

**Violation Type:** Missing single-port documentation

**Severity:** MINOR

**Current Code:**
```yaml
ports:
- protocol: TCP
  port: 80
  targetPort: 8080
  name: http
```

**Recommended Fix:**
```yaml
ports:
- protocol: TCP
  port: 80           # External port (LoadBalancer)
  targetPort: 8080   # Container port (all services)
  name: http-and-ws  # Single port serves HTTP and WebSocket
# Note: WebSocket upgrade handled automatically by the same port
# All traffic (HTTP API, WebSocket, static assets) flows through this single port
```

---

## Positive Findings

### Correctly Implemented Single-Port Architecture

#### 1. Architecture Overview (000-overview.md)
**Lines 294-302**
```markdown
### Hard Constraints

1. **Single Port Deployment**
   - Application must serve on single configurable port
   - Default: port 8080
   - No additional ports for services
```
✅ Clear statement of single-port requirement

#### 2. Architecture Specification (002-architecture.md)
**Lines 802-817**
```markdown
### ADR-004: Single Port Architecture

**Status:** Accepted
**Date:** 2025-09-29
**Context:** Simplify deployment and firewall configuration
**Decision:** Serve HTTP and WebSocket on single configurable port
**Consequences:**
- ✅ Simplified firewall rules
- ✅ Easier deployment
- ✅ No port conflicts
- ❌ HTTP and WebSocket share same port (not an issue in practice)
```
✅ Proper architectural decision record

#### 3. Backend Server Implementation (003-backend-spec.md)
**Lines 100-114**
```rust
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
```
✅ Single server binding, routes for both WebSocket and static files

#### 4. Requirements Specification (001-requirements.md)
**Lines 607-611**
```markdown
**CR-1.1: Single Port Requirement**
- **Constraint:** Application must operate on a single configurable port
- **Rationale:** Simplifies firewall rules and deployment configuration
- **Impact:** HTTP and WebSocket must share same port
```
✅ Clear constraint definition

---

## Summary Statistics

### Files Audited
- ✅ 000-overview.md
- ✅ 001-requirements.md
- ✅ 002-architecture.md
- ✅ 003-backend-spec.md
- ⚠️ 004-frontend-spec.md (2 violations)
- ⚠️ 005-cli-spec.md (2 violations)
- ⚠️ 006-api-spec.md (1 violation)
- ⚠️ 007-websocket-spec.md (1 violation)
- ✅ 008-testing-spec.md
- ⚠️ 009-deployment-spec.md (2 violations)
- ✅ 010-documentation-spec.md

### Violation Breakdown by Category

| Category | Count |
|----------|-------|
| Multiple Port References | 0 |
| Absolute Path References | 3 |
| Multi-Port Architecture Patterns | 1 |
| Configuration Issues | 4 |
| **Total** | **8** |

### Violation Breakdown by Severity

| Severity | Count | Percentage |
|----------|-------|------------|
| Critical | 0 | 0% |
| Major | 3 | 37.5% |
| Minor | 5 | 62.5% |
| **Total** | **8** | **100%** |

---

## Recommended Actions

### Immediate Actions (Major Violations)

1. **Update 004-frontend-spec.md (Lines 677-686)**
   - Add clear distinction between development and production configurations
   - Emphasize that production uses single port
   - Add comments explaining dev proxy vs production serving

2. **Update 006-api-spec.md (Line 14)**
   - Replace hardcoded base URL with pattern
   - Add single-port architecture explanation
   - Add guidance on relative path usage

3. **Update 007-websocket-spec.md (Line 14)**
   - Replace hardcoded URL with dynamic construction guidance
   - Add code examples showing correct URL building
   - Add anti-patterns section

### Follow-up Actions (Minor Violations)

4. **Update 005-cli-spec.md**
   - Add single-port context to port configuration
   - Update config.toml example with explanatory comments

5. **Update 004-frontend-spec.md (WebSocket client)**
   - Remove URL field from WebSocketConfig
   - Always derive from location.host

6. **Update 009-deployment-spec.md**
   - Add explanatory comments to port mappings
   - Clarify single-port architecture in examples

### Documentation Improvements

7. **Create Single-Port Architecture Guide**
   - Create `/docs/architecture/single-port-design.md`
   - Explain rationale and benefits
   - Provide implementation patterns
   - Include common pitfalls and solutions

8. **Add Development Workflow Documentation**
   - Create `/docs/developer-guide/development-workflow.md`
   - Explain dev server proxy setup
   - Clarify difference between dev and production
   - Provide best practices

---

## Validation Checklist

Use this checklist when reviewing future changes:

- [ ] No hardcoded host:port combinations in code
- [ ] All API calls use relative paths
- [ ] WebSocket URLs constructed from location.host
- [ ] Configuration docs emphasize single-port design
- [ ] Development proxy setup clearly documented as dev-only
- [ ] Deployment configs show single port mapping
- [ ] All examples use relative or dynamic URLs
- [ ] No separate service ports mentioned without context

---

## Conclusion

The web-terminal project demonstrates good adherence to the single-port architecture principle at the core design level. The violations found are primarily in documentation and example code rather than fundamental architecture flaws.

**Key Strengths:**
- Clear architectural decision record (ADR-004)
- Single port constraint properly defined in requirements
- Backend implementation correctly uses single port
- Core design decisions support single-port architecture

**Areas for Improvement:**
- Development configuration needs clearer distinction from production
- Documentation needs consistent emphasis on relative paths
- Example code should never show hardcoded URLs
- Configuration examples need explanatory comments

**Overall Assessment:** The project is architecturally sound for single-port deployment. The recommended fixes are primarily documentation improvements to prevent misunderstanding and ensure consistent implementation.

---

## Audit Metadata

**Audit Date:** 2025-09-29
**Documents Audited:** 11
**Lines Reviewed:** ~7,500
**Violations Found:** 8
**Time to Fix (Estimated):** 2-3 hours
**Risk Level:** LOW

**Next Review:** After implementing recommended fixes

---

## Appendix: Single-Port Architecture Best Practices

### ✅ DO: Use Relative Paths

```typescript
// Frontend code
fetch('/api/v1/sessions');  // ✅ Relative path

const ws = new WebSocket(`${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`);  // ✅ Dynamic
```

### ❌ DON'T: Hardcode URLs

```typescript
// Frontend code
fetch('http://localhost:8080/api/v1/sessions');  // ❌ Hardcoded

const ws = new WebSocket('ws://localhost:8080/ws');  // ❌ Hardcoded
```

### ✅ DO: Dynamic Configuration

```typescript
export class WebSocketClient {
  private buildUrl(token?: string): string {
    // Always derive from current page location
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = location.host;
    const url = `${protocol}//${host}/ws`;
    return token ? `${url}?token=${token}` : url;
  }
}
```

### ✅ DO: Single Port Configuration

```rust
// Backend configuration
HttpServer::new(|| {
    App::new()
        .service(api_routes)      // /api/v1/*
        .service(websocket_route) // /ws
        .service(Files::new("/", "./static"))  // Static assets
})
.bind("0.0.0.0:8080")?  // Single port for everything
.run()
```

### ✅ DO: Clear Documentation

```markdown
## Single-Port Architecture

All services run on a single configurable port:

- **HTTP API:** `http://{host}:{port}/api/v1/*`
- **WebSocket:** `ws://{host}:{port}/ws`
- **Static Assets:** `http://{host}:{port}/`

**Default Port:** 8080

This design simplifies:
- Firewall configuration (one port to open)
- Deployment (one service to manage)
- Client configuration (no port coordination needed)
```

---

**End of Audit Report**