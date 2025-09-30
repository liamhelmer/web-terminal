# Web-Terminal: JWT/JWKS Authentication Specification

**Version:** 2.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29
**References:** [002-architecture.md](./002-architecture.md), [003-backend-spec.md](./003-backend-spec.md), [006-api-spec.md](./006-api-spec.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication Architecture](#authentication-architecture)
3. [JWKS Configuration](#jwks-configuration)
4. [JWT Token Structure](#jwt-token-structure)
5. [Authorization Model](#authorization-model)
6. [Authentication Flow](#authentication-flow)
7. [Security Considerations](#security-considerations)
8. [Error Handling](#error-handling)
9. [Rust Implementation Guide](#rust-implementation-guide)
10. [Configuration Schema](#configuration-schema)
11. [Backstage Integration](#backstage-integration)
12. [Testing Requirements](#testing-requirements)

---

## Overview

### Purpose

Web-Terminal implements JWT (JSON Web Token) based authentication with JWKS (JSON Web Key Set) verification. This architecture enables integration with multiple identity providers while maintaining security through cryptographic signature verification.

### Core Principles

1. **Zero-Trust Verification**: Every request is authenticated via JWT signature verification
2. **Multi-Provider Support**: Compatible with Backstage, Auth0, Keycloak, and custom providers
3. **Claim-Based Authorization**: Flexible user and group authorization using JWT claims
4. **Stateless Authentication**: No server-side session storage required
5. **JWKS-Based Key Management**: Automatic key rotation and caching support

### Key Features

- **JWKS endpoint discovery and caching**
- **Multiple identity provider support**
- **Backstage-compatible claim parsing**
- **User and group-based authorization**
- **Automatic key rotation handling**
- **WebSocket authentication support**
- **Configurable token validation rules**

### Authentication Flow

```
┌─────────────┐
│   Client    │
│  (Browser)  │
└──────┬──────┘
       │ 1. Request with JWT token in Authorization header
       │    (Bearer <token>)
       ▼
┌─────────────────────────────────────────────────────────┐
│              Web-Terminal Server                         │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │        Authentication Middleware               │    │
│  │  1. Extract JWT from Authorization header      │    │
│  │  2. Decode JWT header (kid, alg)               │    │
│  │  3. Fetch JWKS from provider                   │    │
│  │  4. Validate JWT signature with public key     │    │
│  │  5. Verify issuer, audience, expiration        │    │
│  │  6. Extract user/group claims                  │    │
│  │  7. Check authorization (allowed users/groups) │    │
│  └────────────────────────────────────────────────┘    │
│                       │                                  │
│                       │ 8. Authorized request            │
│                       ▼                                  │
│  ┌────────────────────────────────────────────────┐    │
│  │       Session Manager / API Handler            │    │
│  └────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## Authentication Architecture

### Components

#### 1. JWKS Client

**Responsibilities:**
- Fetch JWKS from configured providers
- Cache public keys with TTL
- Handle JWKS endpoint failures gracefully
- Support multiple JWKS providers

**Key Interfaces:**
```rust
pub trait JwksClient: Send + Sync {
    async fn fetch_keys(&self, provider: &str) -> Result<Vec<JsonWebKey>>;
    async fn get_key(&self, kid: &str, provider: &str) -> Result<Option<JsonWebKey>>;
    fn refresh_keys(&self, provider: &str) -> Result<()>;
}

pub struct JsonWebKey {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub use_: String,
    pub n: String,  // RSA modulus
    pub e: String,  // RSA exponent
}
```

#### 2. JWT Verifier

**Responsibilities:**
- Validate JWT signatures using JWKS keys
- Verify token claims (iss, aud, exp, nbf)
- Extract user identity from claims
- Support multiple signing algorithms (RS256, RS384, RS512)

**Key Interfaces:**
```rust
pub trait JwtVerifier: Send + Sync {
    async fn verify_token(&self, token: &str) -> Result<Claims>;
    fn extract_user_id(&self, claims: &Claims) -> Result<UserId>;
    fn extract_groups(&self, claims: &Claims) -> Vec<String>;
}

pub struct Claims {
    pub iss: String,        // Issuer
    pub sub: String,        // Subject (user ID)
    pub aud: Vec<String>,   // Audience
    pub exp: i64,           // Expiration time
    pub nbf: i64,           // Not before
    pub iat: i64,           // Issued at
    pub custom: HashMap<String, serde_json::Value>,
}
```

#### 3. Claims Service

**Responsibilities:**
- Extract Backstage user entities (e.g., "user:default/john.doe")
- Extract group memberships (e.g., "group:default/platform-team")
- Map JWT claims to internal user representation
- Handle claim format variations across providers

**Key Interfaces:**
```rust
pub trait ClaimsService: Send + Sync {
    fn extract_backstage_entity(&self, claims: &Claims) -> Result<String>;
    fn extract_groups(&self, claims: &Claims) -> Vec<String>;
    fn map_to_user(&self, claims: &Claims) -> Result<User>;
}

pub struct User {
    pub id: String,           // "user:default/john.doe"
    pub email: String,
    pub name: String,
    pub groups: Vec<String>,  // ["group:default/platform-team"]
    pub metadata: HashMap<String, String>,
}
```

#### 4. Authorization Service

**Responsibilities:**
- Check if user is authorized to access the terminal
- Enforce allowed users/groups configuration
- Support wildcard patterns
- Audit authorization decisions

**Key Interfaces:**
```rust
pub trait AuthorizationService: Send + Sync {
    fn is_authorized(&self, user: &User) -> bool;
    fn check_user_allowed(&self, user_id: &str) -> bool;
    fn check_group_allowed(&self, groups: &[String]) -> bool;
}

pub struct AuthorizationConfig {
    pub allowed_users: Vec<String>,    // ["user:default/admin", "*"]
    pub allowed_groups: Vec<String>,   // ["group:default/platform-team"]
    pub deny_users: Vec<String>,       // Explicit deny list
    pub deny_groups: Vec<String>,
}
```

---

## JWKS Configuration

### JWKS Provider Configuration

```yaml
auth:
  enabled: true
  jwks:
    providers:
      - name: backstage
        url: https://backstage.example.com/.well-known/jwks.json
        issuer: https://backstage.example.com
        audience: web-terminal
        cache_ttl_seconds: 3600
        refresh_interval_seconds: 900

      - name: auth0
        url: https://example.auth0.com/.well-known/jwks.json
        issuer: https://example.auth0.com/
        audience: https://web-terminal.example.com
        cache_ttl_seconds: 3600

      - name: custom
        url: https://auth.example.com/.well-known/jwks.json
        issuer: https://auth.example.com
        audience: web-terminal-api
```

### JWKS Key Structure

Standard JWKS response format:

```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "key-id-123",
      "alg": "RS256",
      "n": "0vx7agoebGcQ...",
      "e": "AQAB"
    }
  ]
}
```

### Cache and Refresh Strategy

**Cache Behavior:**
- JWKS keys are cached in-memory for `cache_ttl_seconds` (default: 3600s)
- Automatic refresh happens every `refresh_interval_seconds` (default: 900s)
- Failed refresh attempts do not invalidate cache
- Cache is shared across all threads

**Key Rotation Handling:**
- New keys are automatically discovered on next refresh
- Old keys remain valid until cache expiry
- Token verification attempts all keys with matching `kid`
- Graceful handling of key rotation overlap periods

**Error Recovery:**
- Network failures trigger exponential backoff (1s, 2s, 4s, 8s, max 60s)
- Stale cache is used during provider outages
- Health check reports JWKS provider connectivity status

---

## JWT Token Structure

### Standard Claims

All JWT tokens must include these standard claims:

```json
{
  "sub": "user:default/alice",           // Subject (user identifier)
  "iss": "https://backstage.example.com", // Issuer (identity provider)
  "aud": "web-terminal",                  // Audience (this application)
  "exp": 1633024800,                      // Expiration timestamp
  "iat": 1633021200,                      // Issued at timestamp
  "nbf": 1633021200                       // Not before timestamp (optional)
}
```

**Claim Validation Rules:**

| Claim | Validation Rule | Required |
|-------|----------------|----------|
| `sub` | Non-empty string | Yes |
| `iss` | Must match configured issuer | Yes |
| `aud` | Must match configured audience | Yes |
| `exp` | Must be in the future | Yes |
| `iat` | Must be in the past | Yes |
| `nbf` | If present, must be in the past | No |

### Backstage-Specific Claims

Backstage identity tokens include ownership claims:

```json
{
  "sub": "user:default/alice",
  "ent": [
    "user:default/alice",
    "group:default/platform-team",
    "group:default/developers",
    "group:default/admin"
  ],
  "iss": "https://backstage.example.com",
  "aud": "web-terminal",
  "exp": 1633024800,
  "iat": 1633021200
}
```

**Backstage Claim Structure:**
- `sub`: User entity reference (format: `user:<namespace>/<username>`)
- `ent`: Array of ownership entity references (users and groups)
- Entity references follow Backstage format: `<kind>:<namespace>/<name>`

### Custom Claims Support

Additional custom claims can be included:

```json
{
  "sub": "alice",
  "email": "alice@example.com",
  "name": "Alice Smith",
  "roles": ["admin", "developer"],
  "permissions": ["terminal:execute", "files:read", "files:write"],
  "metadata": {
    "department": "engineering",
    "employee_id": "E12345"
  }
}
```

**Custom Claims Configuration:**

```yaml
auth:
  custom_claims:
    user_name_claim: "name"              # Claim for user's display name
    email_claim: "email"                 # Claim for user's email
    roles_claim: "roles"                 # Claim for user roles
    permissions_claim: "permissions"     # Claim for permissions
```

---

## JWT/JWKS Implementation

### Supported Token Formats

#### Standard JWT Claims

```json
{
  "iss": "https://backstage.example.com",
  "sub": "user:default/john.doe",
  "aud": ["web-terminal"],
  "exp": 1735567200,
  "iat": 1735563600,
  "nbf": 1735563600
}
```

#### Backstage JWT Claims

```json
{
  "iss": "https://backstage.example.com",
  "sub": "user:default/john.doe",
  "aud": ["backstage"],
  "exp": 1735567200,
  "iat": 1735563600,
  "ent": ["user:default/john.doe"],
  "usc": {
    "ownershipEntityRefs": ["group:default/platform-team"],
    "displayName": "John Doe",
    "email": "john.doe@example.com"
  }
}
```

### JWKS Endpoint Format

Standard JWKS endpoint response:

```json
{
  "keys": [
    {
      "kid": "2024-key-1",
      "kty": "RSA",
      "alg": "RS256",
      "use": "sig",
      "n": "<base64-encoded-modulus>",
      "e": "AQAB"
    }
  ]
}
```

### Validation Process

1. **Extract Token**: Get JWT from `Authorization: Bearer <token>` header
2. **Decode Header**: Extract `kid` (key ID) and `alg` (algorithm)
3. **Fetch JWKS**: Retrieve JWKS from provider endpoint (cached)
4. **Find Key**: Match `kid` from token to JWKS keys
5. **Verify Signature**: Validate JWT signature using public key
6. **Verify Claims**:
   - Check `iss` (issuer) matches configured provider
   - Check `exp` (expiration) is in the future
   - Check `nbf` (not before) is in the past
   - Check `aud` (audience) if configured
7. **Extract Identity**: Get user ID and groups from claims
8. **Check Authorization**: Verify user/groups are allowed
9. **Create Session**: Associate validated user with WebSocket session

### Key Caching Strategy

```rust
pub struct JwksCache {
    // Cache JWKS keys by provider
    keys: Arc<DashMap<String, CachedKeys>>,
    ttl: Duration,
}

pub struct CachedKeys {
    keys: Vec<JsonWebKey>,
    fetched_at: Instant,
    expires_at: Instant,
}

impl JwksCache {
    // Refresh keys if expired or close to expiration
    async fn get_keys(&self, provider: &str) -> Result<Vec<JsonWebKey>> {
        if let Some(cached) = self.keys.get(provider) {
            if cached.expires_at > Instant::now() {
                return Ok(cached.keys.clone());
            }
        }

        // Fetch fresh keys
        let keys = self.fetch_from_provider(provider).await?;
        self.keys.insert(provider.to_string(), CachedKeys {
            keys: keys.clone(),
            fetched_at: Instant::now(),
            expires_at: Instant::now() + self.ttl,
        });

        Ok(keys)
    }
}
```

---

## Authentication Flow

### HTTP Request Authentication

**1. Client Request with JWT:**

```http
GET /api/v1/sessions HTTP/1.1
Host: web-terminal.example.com
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6ImtleS0xMjMifQ...
```

**2. Middleware Processing:**

```rust
async fn auth_middleware(
    req: ServiceRequest,
    config: &AuthConfig,
    jwks_cache: &JwksCache,
) -> Result<ServiceRequest, Error> {
    // Extract token from Authorization header
    let token = extract_bearer_token(&req)?;

    // Decode JWT header to get kid
    let header = decode_header(token)?;

    // Fetch JWKS for the issuer
    let jwks = jwks_cache.get_for_issuer(&header.iss).await?;

    // Find key with matching kid
    let key = jwks.find_key(&header.kid)?;

    // Verify signature and decode claims
    let claims = verify_token(token, key, &config.validation)?;

    // Validate claims
    validate_claims(&claims, &config)?;

    // Check authorization
    if !is_authorized(&claims, &config) {
        return Err(Error::Forbidden);
    }

    // Attach user context to request
    req.extensions_mut().insert(UserContext::from_claims(claims));

    Ok(req)
}
```

### WebSocket Authentication

**1. Connection Request with JWT:**

```javascript
// Option A: Query parameter
const ws = new WebSocket('wss://web-terminal.example.com/ws?token=eyJhbGci...');

// Option B: Subprotocol (preferred)
const ws = new WebSocket('wss://web-terminal.example.com/ws', ['jwt', token]);

// Option C: First message
const ws = new WebSocket('wss://web-terminal.example.com/ws');
ws.onopen = () => {
    ws.send(JSON.stringify({
        type: 'auth',
        token: 'eyJhbGci...'
    }));
};
```

**2. WebSocket Handshake Authentication:**

```rust
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    auth_service: web::Data<AuthService>,
) -> Result<HttpResponse, Error> {
    // Extract token from query parameter or upgrade header
    let token = extract_ws_token(&req)?;

    // Validate token (same as HTTP)
    let claims = auth_service.validate_token(token).await?;

    // Check authorization
    if !is_authorized(&claims, &auth_service.config) {
        return Err(Error::Forbidden);
    }

    // Create WebSocket session with user context
    let session = WebSocketSession::new(claims.sub.clone());

    // Complete WebSocket upgrade
    ws::start(session, &req, stream)
}
```

**3. Ongoing Validation:**

```rust
impl WebSocketSession {
    fn handle_message(&mut self, msg: Message, ctx: &mut ws::WebsocketContext<Self>) {
        // Check token expiration on each message
        if self.token_expired() {
            ctx.close(Some(CloseCode::Policy.into()));
            ctx.stop();
            return;
        }

        // Process message...
    }

    fn token_expired(&self) -> bool {
        self.token_exp < Utc::now().timestamp()
    }
}
```

---

## Error Handling

### Authentication Error Codes

| Error Code | HTTP Status | Description | User Action |
|-----------|-------------|-------------|-------------|
| `AUTH_TOKEN_MISSING` | 401 | No token provided | Provide token in Authorization header |
| `AUTH_TOKEN_INVALID` | 401 | Token format invalid | Check token format |
| `AUTH_TOKEN_EXPIRED` | 401 | Token has expired | Refresh token |
| `AUTH_TOKEN_NOT_YET_VALID` | 401 | Token nbf in future | Check system clock |
| `AUTH_SIGNATURE_INVALID` | 401 | Signature verification failed | Token tampered or wrong key |
| `AUTH_ISSUER_INVALID` | 401 | Issuer not recognized | Check identity provider |
| `AUTH_AUDIENCE_INVALID` | 401 | Audience mismatch | Token for different service |
| `AUTH_CLAIMS_INVALID` | 401 | Required claims missing | Check token generation |
| `AUTH_UNAUTHORIZED` | 403 | User not authorized | Contact administrator |
| `AUTH_JWKS_UNAVAILABLE` | 503 | JWKS endpoint unavailable | Try again later |
| `AUTH_INTERNAL_ERROR` | 500 | Internal auth error | Contact support |

### Error Response Format

```json
{
  "error": {
    "code": "AUTH_TOKEN_EXPIRED",
    "message": "JWT token has expired",
    "details": {
      "expired_at": "2025-09-29T10:00:00Z",
      "current_time": "2025-09-29T11:00:00Z"
    }
  }
}
```

### Error Handling Strategy

**Client-Side:**

```javascript
async function authenticatedRequest(url, options = {}) {
    const token = getAccessToken();

    const response = await fetch(url, {
        ...options,
        headers: {
            ...options.headers,
            'Authorization': `Bearer ${token}`
        }
    });

    if (response.status === 401) {
        const error = await response.json();

        if (error.code === 'AUTH_TOKEN_EXPIRED') {
            // Attempt token refresh
            const newToken = await refreshToken();
            if (newToken) {
                // Retry request with new token
                return authenticatedRequest(url, options);
            }
        }

        // Redirect to login
        window.location.href = '/login';
        throw new Error(error.message);
    }

    return response;
}
```

**Server-Side:**

```rust
fn handle_auth_error(e: AuthError) -> HttpResponse {
    let (status, code, message) = match e {
        AuthError::TokenMissing => (
            StatusCode::UNAUTHORIZED,
            "AUTH_TOKEN_MISSING",
            "No authentication token provided"
        ),
        AuthError::TokenExpired { exp } => (
            StatusCode::UNAUTHORIZED,
            "AUTH_TOKEN_EXPIRED",
            "JWT token has expired"
        ),
        AuthError::InvalidSignature => (
            StatusCode::UNAUTHORIZED,
            "AUTH_SIGNATURE_INVALID",
            "Token signature verification failed"
        ),
        AuthError::Unauthorized => (
            StatusCode::FORBIDDEN,
            "AUTH_UNAUTHORIZED",
            "User not authorized to access this resource"
        ),
        AuthError::JwksUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "AUTH_JWKS_UNAVAILABLE",
            "Authentication service temporarily unavailable"
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AUTH_INTERNAL_ERROR",
            "Internal authentication error"
        ),
    };

    HttpResponse::build(status).json(json!({
        "error": {
            "code": code,
            "message": message
        }
    }))
}
```

---

## Rust Implementation Guide

### Dependencies

```toml
[dependencies]
# JWT handling
jsonwebtoken = "9.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP client for JWKS
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Async runtime
tokio = { version = "1.40", features = ["full"] }

# Caching
moka = { version = "0.12", features = ["future"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# Web framework
actix-web = "4.9"
actix-web-actors = "4.3"
```

See [003-backend-spec.md](./003-backend-spec.md#authentication--authorization) for complete Rust implementation examples including:
- Core structures and types
- JWKS cache implementation
- Token validation logic
- Authorization service
- Actix-Web middleware integration

---

## Configuration Schema

### Complete Configuration Example

```yaml
# config/auth.yaml

auth:
  # Enable/disable authentication
  enabled: true

  # JWKS configuration
  jwks:
    providers:
      # Backstage identity provider
      - name: backstage
        url: https://backstage.example.com/.well-known/jwks.json
        issuer: https://backstage.example.com
        audience: web-terminal
        cache_ttl_seconds: 3600
        refresh_interval_seconds: 900

      # Auth0 provider (optional)
      - name: auth0
        url: https://example.auth0.com/.well-known/jwks.json
        issuer: https://example.auth0.com/
        audience: https://web-terminal.example.com
        cache_ttl_seconds: 3600
        refresh_interval_seconds: 900

      # Custom provider (optional)
      - name: custom
        url: https://auth.example.com/.well-known/jwks.json
        issuer: https://auth.example.com
        audience: web-terminal-api
        cache_ttl_seconds: 3600
        refresh_interval_seconds: 900

  # Authorization configuration
  authorization:
    # Authorization mode
    mode: claim-based  # Options: claim-based, role-based, permission-based

    # Allowed users (entity references)
    allowed_users:
      - user:default/alice
      - user:default/bob
      - user:prod/operator-charlie

    # Allowed groups (entity references)
    allowed_groups:
      - group:default/platform-team
      - group:default/developers
      - group:default/admin
      - group:prod/operators

    # Claim paths
    claims:
      ownership_path: ent        # Backstage ownership entities
      user_claim: sub            # User identifier claim
      groups_claim: groups       # Alternative groups claim
      email_claim: email         # Email claim
      name_claim: name           # Display name claim

    # Default policies
    deny_by_default: true
    allow_empty_groups: false

    # Wildcard support (optional)
    enable_wildcards: false

  # Token validation
  validation:
    # Required claims
    required_claims:
      - sub
      - iss
      - aud
      - exp
      - iat

    # Clock skew tolerance (seconds)
    clock_skew_seconds: 60

    # Allowed algorithms
    allowed_algorithms:
      - RS256
      - RS384
      - RS512
      - ES256
      - ES384
      - ES512

    # Minimum token lifetime (seconds)
    min_lifetime_seconds: 300

    # Maximum token lifetime (seconds)
    max_lifetime_seconds: 28800

  # Security settings
  security:
    # Rate limiting
    rate_limit:
      enabled: true
      max_failed_attempts: 5
      window_seconds: 300
      lockout_seconds: 900

    # Audit logging
    audit:
      enabled: true
      log_successful_auth: true
      log_failed_auth: true
      log_authorization_denials: true
      log_token_details: false  # Security: don't log full tokens

    # TLS requirements
    require_tls: true

    # Token sources (for WebSocket)
    allowed_token_sources:
      - header          # Authorization header
      - query           # Query parameter (?token=...)
      - subprotocol     # WebSocket subprotocol
      - message         # First message
```

### Environment Variables

```bash
# Override configuration via environment variables
export AUTH_ENABLED=true
export AUTH_JWKS_BACKSTAGE_URL="https://backstage.example.com/.well-known/jwks.json"
export AUTH_JWKS_BACKSTAGE_ISSUER="https://backstage.example.com"
export AUTH_JWKS_BACKSTAGE_AUDIENCE="web-terminal"
export AUTH_ALLOWED_USERS="user:default/alice,user:default/bob"
export AUTH_ALLOWED_GROUPS="group:default/platform-team,group:default/developers"
```

---

## Backstage Integration

### Backstage Token Format

Backstage issues JWTs with the following structure:

**Claims:**
- `sub`: Backstage entity reference (e.g., `user:default/john.doe`)
- `ent`: Array of entity references the user represents
- `usc`: User sign-in context containing:
  - `ownershipEntityRefs`: Groups the user belongs to
  - `displayName`: User's display name
  - `email`: User's email address

**Example Backstage Token Claims:**
```json
{
  "iss": "https://backstage.example.com",
  "sub": "user:default/john.doe",
  "aud": ["backstage"],
  "exp": 1735567200,
  "iat": 1735563600,
  "ent": ["user:default/john.doe"],
  "usc": {
    "ownershipEntityRefs": [
      "group:default/platform-team",
      "group:default/developers"
    ],
    "displayName": "John Doe",
    "email": "john.doe@example.com"
  }
}
```

### JWKS Endpoint

Backstage exposes JWKS at:
```
https://backstage.example.com/.well-known/jwks.json
```

### Configuration for Backstage

```bash
# Environment Variables
AUTH_ENABLED=true
AUTH_JWKS_PROVIDERS='[{"name":"backstage","url":"https://backstage.example.com/.well-known/jwks.json","issuer":"https://backstage.example.com"}]'
AUTH_ALLOWED_USERS='["user:default/admin"]'
AUTH_ALLOWED_GROUPS='["group:default/platform-team"]'
```

Or via config file:

```yaml
# config/auth.yaml
authentication:
  enabled: true
  jwks_providers:
    - name: backstage
      url: https://backstage.example.com/.well-known/jwks.json
      issuer: https://backstage.example.com
      cache_ttl: 3600

  authorization:
    # Allow specific users
    allowed_users:
      - "user:default/admin"
      - "user:default/platform-ops"

    # Allow groups (any member can access)
    allowed_groups:
      - "group:default/platform-team"
      - "group:default/sre-team"

    # Explicit deny list (takes precedence)
    deny_users: []
    deny_groups: []
```

### Integration with Backstage Frontend

**From Backstage Plugin:**

```typescript
// Get token from Backstage identity API
const { token } = await identityApi.getCredentials();

// Connect to web-terminal with token
const ws = new WebSocket('wss://web-terminal.example.com/api/v1/ws');

// Send token in first message
ws.send(JSON.stringify({
  type: 'authenticate',
  token: token
}));

// Or use Authorization header (preferred)
const headers = {
  'Authorization': `Bearer ${token}`
};
```

---

## Authorization Model

### User-Based Authorization

**Allow specific users:**
```yaml
allowed_users:
  - "user:default/admin"
  - "user:default/john.doe"
  - "user:default/jane.smith"
```

**Wildcard support:**
```yaml
allowed_users:
  - "user:default/*"  # All users in default namespace
  - "*"                # All authenticated users
```

### Group-Based Authorization

**Allow groups (any member can access):**
```yaml
allowed_groups:
  - "group:default/platform-team"
  - "group:default/sre-team"
  - "group:default/developers"
```

### Combined Authorization

Users are authorized if they match **either** condition:
- User ID is in `allowed_users` list, OR
- User belongs to any group in `allowed_groups` list

**Explicit deny takes precedence:**
- If user is in `deny_users`, access is denied
- If user belongs to any group in `deny_groups`, access is denied

### Authorization Examples

**Example 1: Admin Only**
```yaml
allowed_users: ["user:default/admin"]
allowed_groups: []
```

**Example 2: Platform Team Only**
```yaml
allowed_users: []
allowed_groups: ["group:default/platform-team"]
```

**Example 3: Multiple Teams**
```yaml
allowed_users: ["user:default/admin"]
allowed_groups:
  - "group:default/platform-team"
  - "group:default/sre-team"
```

**Example 4: All Authenticated Users Except Contractors**
```yaml
allowed_users: ["*"]
allowed_groups: []
deny_groups: ["group:default/contractors"]
```

---

## Configuration

### Environment Variables

```bash
# Enable/disable authentication
AUTH_ENABLED=true

# JWKS providers (JSON array)
AUTH_JWKS_PROVIDERS='[
  {
    "name": "backstage",
    "url": "https://backstage.example.com/.well-known/jwks.json",
    "issuer": "https://backstage.example.com"
  }
]'

# Allowed users (JSON array)
AUTH_ALLOWED_USERS='["user:default/admin","user:default/platform-ops"]'

# Allowed groups (JSON array)
AUTH_ALLOWED_GROUPS='["group:default/platform-team","group:default/sre-team"]'

# JWKS cache TTL (seconds)
AUTH_JWKS_CACHE_TTL=3600

# Audit logging
AUTH_AUDIT_LOG=true
AUTH_AUDIT_LOG_FILE=/app/data/logs/auth-audit.log
```

### Configuration File

```yaml
# config/auth.yaml

authentication:
  # Enable authentication (default: false)
  enabled: true

  # JWKS providers
  jwks_providers:
    - name: backstage
      url: https://backstage.example.com/.well-known/jwks.json
      issuer: https://backstage.example.com
      cache_ttl: 3600  # 1 hour

    - name: auth0
      url: https://tenant.auth0.com/.well-known/jwks.json
      issuer: https://tenant.auth0.com/
      cache_ttl: 3600

  # Authorization
  authorization:
    # Allowed users (Backstage entity format)
    allowed_users:
      - "user:default/admin"
      - "user:default/platform-ops"

    # Allowed groups (any member can access)
    allowed_groups:
      - "group:default/platform-team"
      - "group:default/sre-team"

    # Explicit deny list (takes precedence)
    deny_users: []
    deny_groups:
      - "group:default/contractors"

  # Audit logging
  audit:
    enabled: true
    log_file: /app/data/logs/auth-audit.log
    log_level: info
    include_claims: false  # Don't log full JWT claims for privacy
```

### Multiple Provider Configuration

```yaml
jwks_providers:
  - name: backstage-prod
    url: https://backstage.prod.example.com/.well-known/jwks.json
    issuer: https://backstage.prod.example.com

  - name: backstage-staging
    url: https://backstage.staging.example.com/.well-known/jwks.json
    issuer: https://backstage.staging.example.com

  - name: okta
    url: https://example.okta.com/.well-known/jwks.json
    issuer: https://example.okta.com
```

---

## Security Considerations

### Key Rotation

**Automatic Key Refresh:**
- JWKS keys cached with configurable TTL
- Background task refreshes keys before expiration
- Graceful fallback if JWKS endpoint unavailable

**Handling Key Rotation:**
1. New keys published to JWKS endpoint
2. Both old and new keys available during rotation period
3. Tokens signed with old keys still valid until expiration
4. Cache refreshes pick up new keys automatically

### Token Security

**Best Practices:**
- Always use HTTPS/WSS for token transmission
- Short token expiration (< 1 hour recommended)
- Validate all claims (iss, aud, exp, nbf)
- Use secure key algorithms (RS256, RS384, RS512)
- Never log full JWT tokens

**Mitigations:**
- Rate limiting on authentication attempts
- Audit logging of all authentication events
- Token replay protection (optional nonce/jti validation)
- Secure token storage in browser (memory only, no localStorage)

### JWKS Endpoint Security

**Requirements:**
- JWKS URL must be HTTPS
- JWKS endpoint must support CORS if accessed from browser
- Verify JWKS endpoint certificate
- Cache JWKS responses to reduce load on provider

**Failure Handling:**
- Cached keys used if JWKS endpoint unreachable
- Authentication fails if no cached keys available
- Alert/log JWKS fetch failures

### Audit Logging

**Logged Events:**
- Authentication attempts (success/failure)
- Authorization decisions (allow/deny)
- JWKS key refresh events
- Configuration changes
- Token validation errors

**Audit Log Format:**
```json
{
  "timestamp": "2025-09-29T10:00:00Z",
  "event_type": "authentication",
  "result": "success",
  "user_id": "user:default/john.doe",
  "groups": ["group:default/platform-team"],
  "provider": "backstage",
  "source_ip": "192.168.1.100",
  "session_id": "abc123"
}
```

---

## Testing Requirements

### Unit Tests

**JWT Verification:**
- Valid token parsing
- Expired token rejection
- Invalid signature rejection
- Missing claims handling
- Algorithm mismatch detection

**Authorization:**
- User allowlist matching
- Group membership checking
- Wildcard pattern matching
- Deny list precedence
- Empty configuration handling

**JWKS Client:**
- Key fetching from endpoint
- Cache hit/miss behavior
- TTL expiration handling
- Key ID matching
- Provider failure handling

### Integration Tests

**End-to-End Authentication:**
1. Start web-terminal with auth enabled
2. Fetch JWKS from mock provider
3. Generate valid JWT with test key
4. Send authenticated WebSocket connection
5. Verify session created with correct user
6. Send unauthenticated request
7. Verify connection rejected

**Backstage Integration:**
1. Mock Backstage JWKS endpoint
2. Generate Backstage-format JWT
3. Extract user entity and groups
4. Verify authorization based on group membership
5. Test token expiration handling
6. Test key rotation scenario

### Security Tests

**Attack Scenarios:**
- Token tampering (modified claims)
- Expired token usage
- Invalid signature
- Algorithm confusion (HS256 vs RS256)
- Missing required claims
- Audience mismatch
- Issuer mismatch

### Performance Tests

**JWKS Caching:**
- Measure key fetch latency
- Verify cache hit rate
- Test concurrent token validation
- Measure authentication overhead

**Load Testing:**
- 1000+ concurrent authenticated connections
- Token validation throughput
- JWKS cache performance under load

---

## Implementation Status

### Phase 1: Core JWT/JWKS Support ✅ COMPLETED
- [x] Implement JWKS client with caching (src/auth/jwks_client.rs)
- [x] Implement JWT verifier (RS256, RS384, RS512, ES256, ES384) (src/auth/jwt_validator.rs)
- [x] Add authentication middleware (src/server/middleware/auth.rs)
- [x] Support multiple JWKS providers (config/auth.yaml)
- [x] Add unit tests for JWT validation (tests/jwt_validation.rs)

### Phase 2: Authorization ✅ COMPLETED
- [x] Implement user allowlist checking (src/auth/authorization.rs)
- [x] Implement group membership checking (Backstage `ent` claims)
- [x] Add wildcard pattern support (config optional)
- [x] Add deny list support (config optional)
- [x] Add authorization tests (tests/authorization.rs)

### Phase 3: Backstage Integration ✅ COMPLETED
- [x] Parse Backstage JWT claims format (`sub`, `ent`, `usc`)
- [x] Extract user entity references (`user:default/username`)
- [x] Extract group memberships from `ent` array
- [x] Test with real Backstage instance (integration tests)
- [x] Document Backstage setup (docs/security/JWT_AUTHENTICATION.md)

### Phase 4: Configuration & Operations ✅ COMPLETED
- [x] Load configuration from file/env (config/auth.yaml, env vars)
- [x] Add audit logging (tracing with structured logs)
- [x] Add metrics (auth success/failure rates, Prometheus)
- [x] Add health checks (JWKS connectivity check)
- [x] Add key rotation monitoring (background refresh task)

### Phase 5: Security Hardening ✅ COMPLETED
- [x] Implement rate limiting (src/security/rate_limit.rs)
- [x] Add token replay protection (JWT `jti` claim validation)
- [x] Security audit and penetration testing (PASSED)
- [x] Document security best practices (docs/security/)
- [x] Add security alerts (Prometheus metrics with alerting)

## Implementation Summary

**All authentication phases COMPLETED** as of 2025-09-29.

### Key Implementation Files

- **JWKS Client**: `src/auth/jwks_client.rs` - Fetch and cache JWKS keys
- **JWT Validator**: `src/auth/jwt_validator.rs` - Validate JWT signatures
- **Authorization**: `src/auth/authorization.rs` - User/group authorization
- **Middleware**: `src/server/middleware/auth.rs` - HTTP/WebSocket auth
- **Configuration**: `config/auth.yaml` - Provider configuration
- **Tests**: `tests/{jwt_validation,authorization}.rs` - Security tests

### Configuration Files Created

- `config/auth.yaml` - Authentication provider configuration
- `config/permissions.yaml` - Authorization rules
- `.env.example` - Environment variable template
- `docs/security/JWT_AUTHENTICATION.md` - Implementation guide
- `docs/security/REMEDIATION_PLAN.md` - Security remediation tracking

### Supported Identity Providers

✅ **Backstage** - Full support with entity reference parsing
✅ **Auth0** - Standard OIDC JWT validation
✅ **Okta** - Standard OIDC JWT validation
✅ **Custom** - Any JWKS-compatible provider

### Security Features Implemented

- ✅ RS256/RS384/RS512 signature verification
- ✅ ES256/ES384 (ECDSA) support
- ✅ Multi-provider JWKS support
- ✅ Automatic key rotation (15-minute refresh)
- ✅ Key caching (1-hour TTL)
- ✅ Clock skew tolerance (60 seconds)
- ✅ User/group authorization
- ✅ Audit logging
- ✅ Rate limiting (per-IP, per-user)
- ✅ Token replay protection
- ✅ TLS enforcement (HTTPS-only in production)

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 2.0.0 | 2025-09-29 | System Architecture Designer | Comprehensive JWT/JWKS specification with JWKS configuration, detailed token structure, authentication flow, error handling, Rust implementation guide, and complete configuration schema |
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial authentication specification |

---

## References

- [RFC 7519: JSON Web Token (JWT)](https://tools.ietf.org/html/rfc7519)
- [RFC 7517: JSON Web Key (JWK)](https://tools.ietf.org/html/rfc7517)
- [Backstage Authentication](https://backstage.io/docs/auth/)
- [Auth0 JWKS Documentation](https://auth0.com/docs/secure/tokens/json-web-tokens/json-web-key-sets)
- [OWASP JWT Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)