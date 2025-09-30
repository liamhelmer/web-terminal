# Web-Terminal: Security Implementation Guide

**Version:** 1.0.0
**Status:** Complete
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

This document provides a comprehensive guide to the security implementation in web-terminal, including JWT/JWKS authentication, authorization, rate limiting, TLS configuration, and security monitoring.

**Security Architecture:** External JWT-based authentication with JWKS public key validation.

---

## Table of Contents

1. [Authentication Architecture](#authentication-architecture)
2. [JWKS Client Implementation](#jwks-client-implementation)
3. [JWT Validation](#jwt-validation)
4. [Authorization Service](#authorization-service)
5. [Rate Limiting](#rate-limiting)
6. [TLS/HTTPS Configuration](#tlshttps-configuration)
7. [Security Headers](#security-headers)
8. [Audit Logging](#audit-logging)
9. [Security Monitoring](#security-monitoring)
10. [Testing Security](#testing-security)
11. [Production Deployment](#production-deployment)
12. [Incident Response](#incident-response)

---

## Authentication Architecture

### Overview

Web-terminal uses **external JWT authentication** with JWKS public key validation. There are **no user registration or login endpoints** - all authentication is handled by external Identity Providers (IdPs).

### Authentication Flow

```
┌─────────┐                  ┌──────────┐                 ┌──────────────┐
│  User   │                  │   IdP    │                 │ Web-Terminal │
└────┬────┘                  └────┬─────┘                 └──────┬───────┘
     │                            │                               │
     │  1. Login Request          │                               │
     ├───────────────────────────>│                               │
     │                            │                               │
     │  2. JWT Token              │                               │
     │<───────────────────────────┤                               │
     │                            │                               │
     │  3. API Request            │                               │
     │    Authorization: Bearer <token>                           │
     ├───────────────────────────────────────────────────────────>│
     │                            │                               │
     │                            │  4. Fetch JWKS (if not cached)│
     │                            │<──────────────────────────────┤
     │                            │                               │
     │                            │  5. JWKS Keys                 │
     │                            ├──────────────────────────────>│
     │                            │                               │
     │                            │      6. Validate JWT          │
     │                            │      - Verify signature       │
     │                            │      - Check expiration       │
     │                            │      - Verify issuer          │
     │                            │                               │
     │  7. Authorized Response    │                               │
     │<───────────────────────────────────────────────────────────┤
     │                            │                               │
```

### Key Components

1. **JWKS Client** (`src/auth/jwks_client.rs`)
   - Fetches public keys from IdP JWKS endpoints
   - Caches keys with configurable TTL (default: 1 hour)
   - Background refresh task (default: 15 minutes)
   - Supports multiple providers simultaneously

2. **JWT Validator** (`src/auth/jwt_validator.rs`)
   - Validates JWT signatures using JWKS public keys
   - Verifies standard claims (exp, nbf, iss, aud)
   - Extracts user identity and claims
   - Supports multiple algorithms (RS256, RS384, RS512, ES256, ES384)

3. **Authentication Middleware** (`src/server/middleware/auth.rs`)
   - HTTP: Extracts `Authorization: Bearer <token>` header
   - WebSocket: Handles `Authenticate` message type
   - Attaches `UserContext` to requests
   - Returns 401 Unauthorized on failures

4. **Authorization Service** (`src/auth/authorization.rs`)
   - Enforces user/group allowlists
   - Implements resource ownership checks
   - Role-based permissions
   - Returns 403 Forbidden on authorization failures

---

## JWKS Client Implementation

### Configuration

**File:** `config/auth.yaml`

```yaml
authentication:
  enabled: true

  jwks_providers:
    - name: backstage
      url: https://backstage.example.com/.well-known/jwks.json
      issuer: https://backstage.example.com
      cache_ttl: 3600  # 1 hour
      algorithms:
        - RS256
        - RS384
        - RS512

    - name: auth0
      url: https://tenant.auth0.com/.well-known/jwks.json
      issuer: https://tenant.auth0.com/
      cache_ttl: 3600
      algorithms:
        - RS256

  cache_ttl_seconds: 3600        # 1 hour
  refresh_interval_seconds: 900   # 15 minutes
  timeout_seconds: 10             # HTTP timeout
  clock_skew_seconds: 60          # Token validation tolerance
```

### Environment Variables

```bash
# Enable authentication (default: false)
export AUTH_ENABLED=true

# JWKS providers (JSON array)
export AUTH_JWKS_PROVIDERS='[
  {
    "name": "backstage",
    "url": "https://backstage.example.com/.well-known/jwks.json",
    "issuer": "https://backstage.example.com"
  }
]'

# Cache TTL (default: 3600 seconds)
export AUTH_JWKS_CACHE_TTL=3600

# Refresh interval (default: 900 seconds)
export AUTH_JWKS_REFRESH_INTERVAL=900
```

### Key Features

1. **Automatic Caching**
   - Keys cached in memory (DashMap)
   - Configurable TTL per provider
   - Cache miss triggers fetch from IdP
   - Cache hit reduces latency to microseconds

2. **Background Refresh**
   - Tokio background task runs every 15 minutes (configurable)
   - Proactively refreshes cached keys before expiry
   - Prevents cache misses during token validation
   - Continues on failure (uses cached keys)

3. **Multi-Provider Support**
   - Multiple IdPs configured simultaneously
   - Each provider has independent cache
   - Provider selected by JWT `iss` claim
   - Fallback to next provider on failure

4. **Error Handling**
   - HTTP timeouts (default: 10 seconds)
   - Retry logic with exponential backoff
   - Graceful degradation (uses cached keys)
   - Detailed error logging with tracing

### Implementation Details

**File:** `src/auth/jwks_client.rs`

```rust
pub struct JwksClient {
    client: Client,
    cache: Arc<DashMap<String, CachedJwks>>,
    config: Arc<AuthConfig>,
}

impl JwksClient {
    pub async fn fetch_keys(&self, provider_name: &str) -> Result<JsonWebKeySet> {
        // Check cache first
        if let Some(cached) = self.cache.get(provider_name) {
            if Instant::now() < cached.expires_at {
                return Ok(cached.keys.clone());
            }
        }

        // Cache miss/expired - fetch from provider
        self.fetch_keys_from_provider(provider_name).await
    }

    pub async fn get_key(&self, provider_name: &str, kid: &str) -> Result<JsonWebKey> {
        let jwks = self.fetch_keys(provider_name).await?;
        jwks.keys.iter()
            .find(|key| key.kid == kid)
            .cloned()
            .ok_or_else(|| JwksError::KeyNotFound(kid.to_string()))
    }
}
```

---

## JWT Validation

### Validation Process

1. **Extract Header**: Decode JWT header to get `kid` (Key ID) and `alg` (Algorithm)
2. **Identify Provider**: Decode claims to get `iss` (Issuer), match to configured provider
3. **Verify Algorithm**: Ensure algorithm is in provider's allowed list
4. **Fetch Public Key**: Get JWK from cache/provider using `kid`
5. **Verify Signature**: Validate JWT signature using public key
6. **Verify Claims**:
   - `exp` (Expiration): Must be in the future (with clock skew tolerance)
   - `nbf` (Not Before): Must be in the past (if present)
   - `iss` (Issuer): Must match provider configuration
   - `aud` (Audience): Must match provider audience (if configured)

### Supported Algorithms

| Algorithm | Type | Curve | Key Size | Recommended |
|-----------|------|-------|----------|-------------|
| **RS256** | RSA | N/A | 2048-4096 bits | ✅ Yes (default) |
| **RS384** | RSA | N/A | 2048-4096 bits | ✅ Yes |
| **RS512** | RSA | N/A | 2048-4096 bits | ✅ Yes |
| **ES256** | ECDSA | P-256 | 256 bits | ✅ Yes |
| **ES384** | ECDSA | P-384 | 384 bits | ✅ Yes |
| **PS256** | RSA-PSS | N/A | 2048-4096 bits | ⚠️ Optional |
| **PS384** | RSA-PSS | N/A | 2048-4096 bits | ⚠️ Optional |
| **PS512** | RSA-PSS | N/A | 2048-4096 bits | ⚠️ Optional |
| **HS256** | HMAC | N/A | 256 bits | ❌ No (symmetric) |

**Note**: HMAC algorithms (HS256/HS384/HS512) are **not supported** as they require shared secrets, which violates the external authentication design.

### Claims Extraction

#### Standard Claims

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Issuer (IdP URL)
    pub iss: String,

    /// Audience (application identifier)
    pub aud: Option<serde_json::Value>,

    /// Expiration time (seconds since epoch)
    pub exp: u64,

    /// Not before time (optional)
    pub nbf: Option<u64>,

    /// Issued at time (optional)
    pub iat: Option<u64>,

    /// Email
    pub email: Option<String>,

    /// Groups/roles
    pub groups: Option<Vec<String>>,

    // Additional custom claims
    #[serde(flatten)]
    pub additional: serde_json::Map<String, serde_json::Value>,
}
```

#### Backstage-Specific Claims

```rust
// Backstage entity references (array)
pub ent: Option<Vec<String>>,  // ["user:default/john.doe", "group:default/platform-team"]

// Backstage user/service claims (object)
pub usc: Option<serde_json::Value>,
```

**Example Backstage JWT**:

```json
{
  "sub": "user:default/john.doe",
  "iss": "https://backstage.example.com",
  "aud": "web-terminal",
  "exp": 1735574400,
  "iat": 1735566400,
  "email": "john.doe@example.com",
  "ent": [
    "user:default/john.doe",
    "group:default/platform-team",
    "group:default/sre-team"
  ],
  "usc": {
    "ownershipEntityRefs": ["group:default/platform-team"]
  }
}
```

### Clock Skew Tolerance

JWT validation allows configurable clock skew to account for time differences between servers:

- **Default**: 60 seconds
- **Configurable**: `auth.clock_skew_seconds` in config
- **Applies to**: `exp` and `nbf` claims

Example:
- Token expires at: `2025-09-29T12:00:00Z`
- Server time: `2025-09-29T12:00:30Z` (30 seconds after expiry)
- Result: **Token still valid** (within 60-second tolerance)

---

## Authorization Service

### Permission Model

Web-terminal implements a **role-based access control (RBAC)** model with resource ownership checks.

### Configuration

**File:** `config/permissions.yaml`

```yaml
role_permissions:
  # Admin role - full access
  admin:
    - CreateSession
    - ViewSession
    - SendInput
    - KillSession
    - ListAllSessions
    - KillAnySession

  # Standard user role
  user:
    - CreateSession
    - ViewSession
    - SendInput
    - KillSession

  # Read-only access
  readonly:
    - ViewSession

# Resource ownership rules
ownership_rules:
  own_sessions_view: true   # Users can view their own sessions
  own_sessions_kill: true   # Users can kill their own sessions

# Default permissions for all authenticated users
default_permissions:
  - CreateSession
```

### Environment Variables

```bash
# Allowed users (Backstage entity format)
export AUTH_ALLOWED_USERS='["user:default/admin","user:default/john.doe"]'

# Allowed groups (any member can access)
export AUTH_ALLOWED_GROUPS='["group:default/platform-team","group:default/sre-team"]'

# Deny users (takes precedence)
export AUTH_DENY_USERS='["user:default/contractor"]'

# Deny groups (takes precedence)
export AUTH_DENY_GROUPS='["group:default/external"]'
```

### Authorization Flow

```
┌─────────────────────────────────────────────────────┐
│ 1. Extract user identity from validated JWT        │
│    - User ID (sub claim)                            │
│    - Groups (ent claim for Backstage)               │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│ 2. Check deny lists (if configured)                 │
│    - Deny user list: AUTH_DENY_USERS                │
│    - Deny group list: AUTH_DENY_GROUPS              │
│    - If match: DENY (403 Forbidden)                 │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│ 3. Check allow lists (if configured)                │
│    - Allow user list: AUTH_ALLOWED_USERS            │
│    - Allow group list: AUTH_ALLOWED_GROUPS          │
│    - If no match: DENY (403 Forbidden)              │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│ 4. Check permission for requested action            │
│    - Role-based permissions (from config)           │
│    - Resource ownership (for session operations)    │
│    - Default permissions (if no role match)         │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│ 5. Authorization Decision                           │
│    - ALLOW: Continue to handler                     │
│    - DENY: Return 403 Forbidden                     │
└─────────────────────────────────────────────────────┘
```

### Permission Types

| Permission | Description | Resource Ownership |
|-----------|-------------|-------------------|
| `CreateSession` | Create new terminal session | N/A |
| `ViewSession` | View session output | ✅ Check ownership |
| `SendInput` | Send input to session | ✅ Check ownership |
| `KillSession` | Terminate session | ✅ Check ownership |
| `ListAllSessions` | View all users' sessions | ❌ Admin only |
| `KillAnySession` | Kill any user's session | ❌ Admin only |

### Example Authorization Checks

```rust
// Example 1: User can create session (default permission)
authz.check_permission(&user, Permission::CreateSession, None)?;

// Example 2: User can view own session
authz.check_permission(&user, Permission::ViewSession, Some(&session.owner_id))?;

// Example 3: Admin can list all sessions
authz.check_permission(&admin, Permission::ListAllSessions, None)?;

// Example 4: User cannot view other user's session (ownership check fails)
authz.check_permission(&user, Permission::ViewSession, Some(&other_user_id))?;
// Returns: AuthorizationError::ResourceOwnershipRequired
```

---

## Rate Limiting

### Implementation

Rate limiting is implemented at three levels:

1. **IP-Based Limiting**: Protects against DoS attacks
2. **User-Based Limiting**: Prevents individual user abuse
3. **WebSocket Message Limiting**: Prevents message flooding

### Configuration

**File:** `config/server.toml`

```toml
[security.rate_limit]
# IP-based rate limiting (per IP address)
ip_requests_per_minute = 100
ip_burst_size = 20

# User-based rate limiting (per authenticated user)
user_requests_per_hour = 1000
user_burst_size = 50

# WebSocket message rate limiting
ws_messages_per_second = 100
ws_burst_size = 20

# Lockout settings
lockout_threshold = 5        # Violations before lockout
lockout_duration_seconds = 300  # 5 minutes
```

### Rate Limit Headers

All HTTP responses include rate limit headers:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 87
X-RateLimit-Reset: 1735566400
Retry-After: 43  (only on 429 responses)
```

### Rate Limit Responses

**HTTP 429 Too Many Requests:**

```json
{
  "error": "Rate limit exceeded",
  "message": "Too many requests from this IP. Try again in 43 seconds.",
  "retry_after": 43
}
```

**WebSocket Close (4002):**

```json
{
  "type": "error",
  "code": "RATE_LIMIT",
  "message": "Too many messages per second. Connection closed."
}
```

### Exponential Backoff

After violations:

1. **First violation**: Warning logged
2. **Second violation**: 30-second cooldown
3. **Third violation**: 1-minute cooldown
4. **Fourth violation**: 5-minute cooldown
5. **Fifth violation**: 15-minute lockout

---

## TLS/HTTPS Configuration

### Requirements

**Production deployments MUST use TLS:**

- ✅ TLS 1.2 minimum (TLS 1.3 recommended)
- ✅ Valid TLS certificate (Let's Encrypt, DigiCert, etc.)
- ✅ Secure cipher suites only
- ✅ HSTS header enabled
- ❌ Self-signed certificates (production)
- ❌ TLS 1.0/1.1 (deprecated)

### Configuration

**File:** `config/tls.toml`

```toml
[tls]
enabled = true
cert_file = "/path/to/cert.pem"
key_file = "/path/to/key.pem"
min_tls_version = "1.2"  # or "1.3"

# Secure cipher suites (TLS 1.2)
cipher_suites = [
  "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
  "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
  "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
  "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
]
```

### Let's Encrypt Setup

```bash
# Install certbot
sudo apt-get install certbot

# Obtain certificate (HTTP-01 challenge)
sudo certbot certonly --standalone \
  --preferred-challenges http \
  -d web-terminal.example.com

# Certificates saved to:
# /etc/letsencrypt/live/web-terminal.example.com/fullchain.pem
# /etc/letsencrypt/live/web-terminal.example.com/privkey.pem

# Configure web-terminal
export TLS_CERT_FILE=/etc/letsencrypt/live/web-terminal.example.com/fullchain.pem
export TLS_KEY_FILE=/etc/letsencrypt/live/web-terminal.example.com/privkey.pem
```

### Certificate Renewal

```bash
# Renew certificates (dry run)
sudo certbot renew --dry-run

# Renew certificates (production)
sudo certbot renew

# Reload web-terminal after renewal
sudo systemctl reload web-terminal
```

**Automated renewal** (cron):

```cron
# Renew Let's Encrypt certificates daily at 2 AM
0 2 * * * certbot renew --quiet && systemctl reload web-terminal
```

---

## Security Headers

### HTTP Security Headers

All HTTP responses include security headers:

```
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), microphone=(), camera=()
```

### CORS Configuration

**File:** `config/cors.toml`

```toml
[cors]
enabled = true
allowed_origins = ["https://app.example.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["Authorization", "Content-Type"]
allow_credentials = true
max_age = 3600
```

### Content Security Policy (CSP)

**Default CSP:**

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' data:;
font-src 'self';
connect-src 'self' wss://web-terminal.example.com;
frame-ancestors 'none';
base-uri 'self';
form-action 'self';
```

**Development CSP** (more permissive):

```
default-src 'self';
script-src 'self' 'unsafe-eval';
style-src 'self' 'unsafe-inline';
connect-src 'self' ws://localhost:8080 wss://localhost:8080;
```

---

## Audit Logging

### Log Structure

All security events are logged with structured JSON:

```json
{
  "timestamp": "2025-09-29T12:00:00.000Z",
  "level": "info",
  "event": "authentication.success",
  "user_id": "user:default/john.doe",
  "provider": "backstage",
  "ip_address": "203.0.113.42",
  "user_agent": "Mozilla/5.0...",
  "session_id": "sess_abc123",
  "duration_ms": 45
}
```

### Security Events

| Event | Level | Description |
|-------|-------|-------------|
| `authentication.success` | INFO | JWT validation successful |
| `authentication.failed` | WARN | JWT validation failed |
| `authentication.expired` | WARN | Token expired |
| `authentication.invalid_signature` | WARN | Signature verification failed |
| `authorization.granted` | INFO | Permission granted |
| `authorization.denied` | WARN | Permission denied |
| `rate_limit.exceeded` | WARN | Rate limit violation |
| `rate_limit.lockout` | ERROR | User locked out |
| `jwks.fetch.success` | INFO | JWKS fetched successfully |
| `jwks.fetch.failed` | ERROR | JWKS fetch failed |
| `jwks.cache.hit` | DEBUG | JWKS cache hit |
| `jwks.cache.miss` | DEBUG | JWKS cache miss |
| `session.created` | INFO | Terminal session created |
| `session.terminated` | INFO | Terminal session terminated |
| `tls.handshake.failed` | ERROR | TLS handshake failed |

### Configuration

**File:** `config/logging.toml`

```toml
[logging]
level = "info"
format = "json"
output = "stdout"

[logging.audit]
enabled = true
log_file = "/var/log/web-terminal/audit.log"
include_claims = false  # Don't log full JWT claims (privacy)
include_ip = true
include_user_agent = true
retention_days = 90
```

### Log Aggregation

**Filebeat Configuration** (ELK Stack):

```yaml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/web-terminal/audit.log
    json.keys_under_root: true
    json.add_error_key: true

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "web-terminal-audit-%{+yyyy.MM.dd}"

setup.kibana:
  host: "kibana:5601"
```

---

## Security Monitoring

### Prometheus Metrics

**Authentication Metrics:**

```
# Total authentication attempts
web_terminal_auth_attempts_total{result="success|failure",provider="backstage"}

# Authentication duration (seconds)
web_terminal_auth_duration_seconds{provider="backstage"}

# JWKS fetch attempts
web_terminal_jwks_fetch_total{result="success|failure",provider="backstage"}

# JWKS cache hits/misses
web_terminal_jwks_cache_total{result="hit|miss",provider="backstage"}

# Active sessions
web_terminal_sessions_active{user="user:default/john.doe"}

# Rate limit violations
web_terminal_rate_limit_violations_total{type="ip|user|websocket"}
```

**Authorization Metrics:**

```
# Authorization checks
web_terminal_authz_checks_total{result="granted|denied",permission="CreateSession"}

# Resource ownership denials
web_terminal_authz_ownership_denied_total{resource="session"}
```

### Grafana Dashboards

**Security Dashboard** includes:

- Authentication success/failure rate (over time)
- Top 10 failed authentication attempts (by IP)
- JWKS cache hit rate
- Authorization denial rate (by permission)
- Rate limit violations (by type)
- Active sessions (by user)
- TLS handshake failures

**Sample PromQL Queries:**

```promql
# Authentication success rate (last 5 minutes)
rate(web_terminal_auth_attempts_total{result="success"}[5m])
  /
rate(web_terminal_auth_attempts_total[5m])

# Top 10 IPs with failed auth attempts
topk(10, sum by(ip_address) (
  increase(web_terminal_auth_attempts_total{result="failure"}[1h])
))

# JWKS cache hit rate
rate(web_terminal_jwks_cache_total{result="hit"}[5m])
  /
rate(web_terminal_jwks_cache_total[5m])
```

### Alerting Rules

**File:** `monitoring/prometheus/alerts.yml`

```yaml
groups:
  - name: security
    interval: 1m
    rules:
      # High authentication failure rate
      - alert: HighAuthFailureRate
        expr: |
          rate(web_terminal_auth_attempts_total{result="failure"}[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High authentication failure rate"
          description: "More than 10 failed auth attempts per second for 5 minutes"

      # JWKS fetch failures
      - alert: JwksFetchFailure
        expr: |
          rate(web_terminal_jwks_fetch_total{result="failure"}[5m]) > 0.1
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "JWKS fetch failures detected"
          description: "Unable to fetch JWKS keys from provider {{ $labels.provider }}"

      # Rate limit violations spike
      - alert: RateLimitSpike
        expr: |
          rate(web_terminal_rate_limit_violations_total[5m]) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Rate limit violation spike"
          description: "More than 5 rate limit violations per second"

      # TLS handshake failures
      - alert: TlsHandshakeFailures
        expr: |
          rate(web_terminal_tls_handshake_failures_total[5m]) > 1
        for: 5m
        labels:
          severity: error
        annotations:
          summary: "TLS handshake failures detected"
          description: "Clients unable to establish TLS connections"
```

---

## Testing Security

### Unit Tests

**JWT Validation Tests** (`tests/jwt_validation.rs`):

```bash
# Run all JWT validation tests
cargo test jwt_validation

# Test specific scenarios
cargo test test_jwt_validation_expired
cargo test test_jwt_validation_wrong_issuer
cargo test test_jwt_validation_missing_kid
cargo test test_jwt_validation_algorithm_mismatch
```

**Authorization Tests** (`tests/authorization.rs`):

```bash
# Run all authorization tests
cargo test authorization

# Test specific scenarios
cargo test test_user_can_access_own_session
cargo test test_user_cannot_access_other_session
cargo test test_admin_can_access_any_session
cargo test test_deny_list_takes_precedence
```

### Integration Tests

**End-to-End Authentication** (`tests/e2e_auth.rs`):

```bash
# Full authentication flow with mock JWKS server
cargo test --test e2e_auth

# Test with real Backstage instance (requires environment)
BACKSTAGE_URL=https://backstage.example.com \
BACKSTAGE_TOKEN=<valid-token> \
cargo test --test e2e_auth_backstage
```

### Security Scanning

**Cargo Audit** (dependency vulnerabilities):

```bash
# Install cargo-audit
cargo install cargo-audit

# Check for known vulnerabilities
cargo audit

# Auto-fix where possible
cargo audit fix
```

**Cargo Deny** (supply chain security):

```bash
# Install cargo-deny
cargo install cargo-deny

# Check licenses, bans, advisories
cargo deny check
```

**OWASP ZAP** (dynamic security testing):

```bash
# Start web-terminal
cargo run

# Run baseline scan
docker run -t owasp/zap2docker-stable zap-baseline.py \
  -t https://localhost:8080 \
  -r zap-report.html
```

### Penetration Testing

**Manual Testing Checklist:**

- [ ] JWT bypass attempts (forged tokens, algorithm confusion)
- [ ] JWKS manipulation (malicious JWKS endpoint)
- [ ] Authorization bypass (access other users' resources)
- [ ] Rate limit bypass (IP spoofing, distributed requests)
- [ ] Token replay attacks
- [ ] TLS downgrade attacks
- [ ] CORS misconfigurations
- [ ] CSP bypass attempts
- [ ] WebSocket hijacking
- [ ] Session fixation

**Automated Tools:**

- Burp Suite Professional
- OWASP ZAP with active scanning
- SQLMap (API injection testing)
- Nikto (web server scanning)

---

## Production Deployment

### Pre-Deployment Checklist

**Security Configuration:**

- [ ] TLS enabled with valid certificate
- [ ] JWKS provider URLs configured and accessible
- [ ] Allowed users/groups configured
- [ ] Rate limiting enabled
- [ ] Security headers configured
- [ ] CORS configured (if needed)
- [ ] Audit logging enabled

**Authentication Testing:**

- [ ] JWKS endpoint accessible from server
- [ ] JWT validation working with production IdP
- [ ] Token expiration enforced
- [ ] Clock skew tolerance appropriate
- [ ] Multi-provider support tested (if applicable)

**Authorization Testing:**

- [ ] User allowlist enforced
- [ ] Group membership checked correctly
- [ ] Deny list takes precedence
- [ ] Resource ownership validated
- [ ] Role-based permissions working

**Monitoring:**

- [ ] Prometheus metrics endpoint secured
- [ ] Grafana dashboards configured
- [ ] Alert rules configured
- [ ] Audit logs forwarded to SIEM
- [ ] Log retention policy configured

### Deployment Verification

After deployment, verify:

```bash
# 1. Health check
curl -f https://web-terminal.example.com/health

# 2. TLS certificate valid
openssl s_client -connect web-terminal.example.com:443 \
  -showcerts | grep -A 2 "Verify return code"

# 3. Security headers present
curl -I https://web-terminal.example.com/ | grep -E "(Strict-Transport|X-Content-Type|X-Frame)"

# 4. Authentication required
curl -i https://web-terminal.example.com/api/v1/sessions
# Expected: 401 Unauthorized

# 5. Valid token works
curl -H "Authorization: Bearer <valid-token>" \
     https://web-terminal.example.com/api/v1/sessions
# Expected: 200 OK

# 6. Rate limiting works
for i in {1..150}; do
  curl -s -o /dev/null -w "%{http_code}\n" \
    https://web-terminal.example.com/health
done
# Expected: Some 429 responses

# 7. Metrics endpoint secured
curl -i https://web-terminal.example.com/metrics
# Expected: 401 Unauthorized (unless configured otherwise)
```

### Rollback Procedure

If security issues detected:

```bash
# 1. Identify the issue
kubectl logs deployment/web-terminal -n production | grep -i error

# 2. Rollback deployment
kubectl rollout undo deployment/web-terminal -n production

# 3. Verify rollback
kubectl rollout status deployment/web-terminal -n production

# 4. Check health
curl https://web-terminal.example.com/health

# 5. Review security logs
kubectl logs deployment/web-terminal -n production | grep -E "(authentication|authorization)"
```

---

## Incident Response

### Security Incident Types

1. **Unauthorized Access Attempt**
   - Multiple failed authentication attempts
   - JWT forgery attempts
   - Authorization bypass attempts

2. **JWKS Provider Compromise**
   - JWKS endpoint unreachable
   - Malicious keys in JWKS
   - Provider certificate issues

3. **Rate Limit Abuse**
   - Distributed DoS attack
   - Single IP flooding requests
   - User account abuse

4. **Token Leakage**
   - JWT token exposed in logs
   - Token stolen via XSS/CSRF
   - Insider threat

### Response Procedures

#### Unauthorized Access Attempt

1. **Detect**: Alert fires for high authentication failure rate
2. **Investigate**:
   ```bash
   # Check audit logs for failed attempts
   grep "authentication.failed" /var/log/web-terminal/audit.log | tail -100

   # Identify attacker IPs
   jq -r 'select(.event == "authentication.failed") | .ip_address' \
     /var/log/web-terminal/audit.log | sort | uniq -c | sort -nr
   ```

3. **Respond**:
   ```bash
   # Block attacker IPs at firewall
   iptables -A INPUT -s 203.0.113.42 -j DROP

   # Or block in Nginx
   echo "deny 203.0.113.42;" >> /etc/nginx/conf.d/blocked-ips.conf
   nginx -s reload
   ```

4. **Recover**:
   - Monitor for continued attempts
   - Update rate limiting rules if needed
   - Document incident in security log

#### JWKS Provider Compromise

1. **Detect**: JWKS fetch failures or certificate warnings
2. **Investigate**:
   ```bash
   # Check JWKS provider connectivity
   curl -v https://backstage.example.com/.well-known/jwks.json

   # Verify TLS certificate
   openssl s_client -connect backstage.example.com:443 \
     -showcerts | grep -A 10 "Certificate"
   ```

3. **Respond**:
   - Contact IdP provider security team
   - Disable compromised provider temporarily
   - Force re-authentication of all users
   ```bash
   # Clear JWKS cache
   curl -X DELETE https://web-terminal.example.com/admin/jwks-cache
   ```

4. **Recover**:
   - Wait for IdP to resolve issue
   - Re-enable provider after verification
   - Monitor for authentication errors

#### Token Leakage

1. **Detect**: Token found in logs or reported by user
2. **Investigate**:
   ```bash
   # Search logs for exposed tokens
   grep -r "eyJhbGciOi" /var/log/web-terminal/ | wc -l

   # Identify affected users
   jq -r 'select(.message | contains("Bearer")) | .user_id' \
     /var/log/web-terminal/audit.log | sort -u
   ```

3. **Respond**:
   - Immediately revoke leaked tokens at IdP
   - Force user re-authentication
   - Scrub tokens from logs
   ```bash
   # Redact tokens from logs (replace with [REDACTED])
   sed -i 's/eyJhbGciOi[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*/[REDACTED]/g' \
     /var/log/web-terminal/audit.log
   ```

4. **Recover**:
   - Implement token redaction in logging
   - Review access patterns for suspicious activity
   - Update incident response procedures

### Post-Incident Actions

1. **Document**:
   - Incident timeline
   - Root cause analysis
   - Actions taken
   - Lessons learned

2. **Improve**:
   - Update security controls
   - Enhance monitoring
   - Train team on new procedures
   - Update runbooks

3. **Report**:
   - Notify stakeholders
   - Update security metrics
   - Share lessons with team
   - Conduct blameless postmortem

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial security implementation guide |

---

**Document Status:** ✅ **COMPLETE** - All security features implemented and documented