# Web-Terminal: Data Storage Architecture Decision

**Version:** 1.0.0
**Status:** Accepted
**Author:** Liam Helmer
**Date:** 2025-09-29
**Type:** Architecture Decision Record (ADR)

---

## Decision

**Web-terminal will use in-memory storage only. No persistent database will be used.**

---

## Context

The web-terminal application provides browser-based terminal emulation with WebSocket connections to a backend server. We needed to decide on a data persistence strategy for:

- Session state (working directory, environment variables, command history)
- User authentication tokens
- Active WebSocket connections
- Process state
- File system buffers

---

## Decision Rationale

### Why In-Memory Only?

1. **Ephemeral Nature of Terminal Sessions**
   - Terminal sessions are inherently temporary
   - Session state is only relevant while connected
   - Command history loss on disconnect is acceptable for this use case

2. **Simplicity**
   - No database schema to maintain
   - No migrations to manage
   - Easier deployment (single binary)
   - Reduced operational complexity

3. **Performance**
   - Zero database query latency
   - No network roundtrips for session lookups
   - Direct memory access for all operations

4. **Security**
   - No persistent storage of sensitive data
   - Session data automatically cleared on server restart
   - Reduced attack surface (no SQL injection, etc.)

5. **Stateless Design**
   - Enables horizontal scaling (with sticky sessions)
   - Crash recovery is simple: client reconnects
   - No database backup/restore procedures needed

---

## Implementation Details

### In-Memory Storage Strategy

**Session Management (DashMap):**
```rust
use dashmap::DashMap;

pub struct SessionManager {
    sessions: DashMap<SessionId, Arc<Session>>,
    user_sessions: DashMap<UserId, Vec<SessionId>>,
}
```

**Benefits of DashMap:**
- Lock-free concurrent HashMap
- Thread-safe without explicit locking
- High-performance for concurrent access
- Perfect for session registry

### Data Storage Locations

| Data Type | Storage | Lifecycle |
|-----------|---------|-----------|
| Session state | DashMap (in-memory) | While session active |
| Authentication tokens | JWT (stateless) | Token expiry |
| Command history | In-memory Vec | While session active |
| WebSocket connections | actix-web actors | While connected |
| Process state | OS process table | While process runs |
| File system buffers | Memory | While session active |

---

## What We're NOT Storing

- ❌ User accounts (authentication via JWT only)
- ❌ Command history across sessions
- ❌ File system state (workspaces are ephemeral)
- ❌ Session logs (use external logging if needed)
- ❌ Metrics history (use Prometheus for metrics)
- ❌ Configuration changes (config file only)

---

## Trade-offs Accepted

### Advantages ✅

1. **Simple deployment** - Single binary, no database required
2. **Fast performance** - Direct memory access
3. **Crash recovery** - Clean slate on restart
4. **Security** - No persistent sensitive data
5. **Horizontal scaling** - Stateless design (with sticky sessions)

### Disadvantages ❌

1. **Session loss on restart** - All sessions terminated
2. **No history** - Command history not persisted
3. **Memory limits** - All data in RAM
4. **No audit trail** - No persistent session logs
5. **Sticky sessions required** - For load balancing

---

## Mitigation Strategies

### For Session Loss on Restart:
- Clear documentation that sessions are ephemeral
- Graceful shutdown procedures (drain connections)
- Client auto-reconnect logic
- Session timeout warnings

### For Memory Limits:
- Configurable session limits per user
- Session timeout for inactive sessions
- Automatic cleanup of expired sessions
- Memory usage monitoring and alerts

### For No Audit Trail:
- Structured logging to external systems (ELK stack)
- Prometheus metrics for monitoring
- Optional integration with external audit systems

### For Load Balancing:
- Use sticky sessions (source IP hashing)
- Or implement session affinity at load balancer
- Document in deployment guide

---

## Alternative Considered: Redis

We considered using Redis for session storage:

**Pros:**
- Session persistence across server restarts
- Shared state for load balancing
- Built-in expiration (TTL)

**Cons:**
- Additional infrastructure dependency
- Network latency for every session lookup
- More complex deployment
- Overkill for ephemeral terminal sessions

**Decision:** Rejected. The added complexity doesn't justify the benefits for our use case.

---

## Alternative Considered: SQLite

We considered using SQLite for local storage:

**Pros:**
- No separate database server
- ACID transactions
- Queryable history

**Cons:**
- File I/O overhead
- Lock contention under load
- Still requires schema management
- Doesn't help with horizontal scaling

**Decision:** Rejected. In-memory is simpler and faster.

---

## Configuration Impact

**No database configuration needed:**
```toml
# config.toml

[session]
timeout_seconds = 1800        # 30 minutes
max_per_user = 10             # Memory limit
workspace_quota_bytes = 1GB   # Per-session limit

[limits]
max_sessions_total = 1000     # Server-wide memory limit
```

---

## Monitoring Requirements

Without a database, monitoring becomes critical:

**Required Metrics:**
- Active session count
- Memory usage per session
- Total memory usage
- Session creation/destruction rate
- Average session lifetime

**Alert Thresholds:**
- Memory usage > 80%
- Active sessions > 90% of limit
- High session churn rate

---

## Documentation Impact

**Updated Specifications:**
- ✅ Removed all database references from CLI spec (005-cli-spec.md)
- ✅ Updated architecture spec (002-architecture.md)
- ✅ Updated deployment spec (009-deployment-spec.md)
- ✅ Updated testing spec (008-testing-spec.md)

**Removed Components:**
- ❌ Database migration tools
- ❌ Database backup procedures
- ❌ Schema documentation
- ❌ Database health checks

---

## Deployment Impact

**Simplified Deployment:**
- No database server required
- No connection pooling configuration
- No database credentials to manage
- Single binary deployment
- Docker image is smaller (~50MB vs ~200MB+)

**Docker Compose Example:**
```yaml
services:
  web-terminal:
    image: web-terminal:latest
    ports:
      - "8080:8080"
    environment:
      - WEB_TERMINAL_JWT_SECRET=${JWT_SECRET}
    # No database service needed!
```

---

## Future Considerations

If persistence becomes necessary in the future, consider:

1. **Optional Redis Backend** - For deployments requiring persistence
2. **Session Snapshot API** - Allow clients to save/restore sessions
3. **External Logging** - For audit trail requirements
4. **Pluggable Storage** - Abstract storage interface

**But:** Start simple. Add complexity only when needed.

---

## Validation

This decision will be validated by:

1. ✅ Memory usage monitoring in production
2. ✅ User feedback on session loss experience
3. ✅ Performance benchmarks vs. database-backed alternatives
4. ⏳ 6-month review of decision

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial ADR documenting in-memory decision |

---

## Related Documents

- [002-architecture.md](./002-architecture.md) - System architecture
- [003-backend-spec.md](./003-backend-spec.md) - Backend implementation
- [008-testing-spec.md](./008-testing-spec.md) - Testing strategy
- [009-deployment-spec.md](./009-deployment-spec.md) - Deployment procedures