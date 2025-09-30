# JWT Authentication Architecture

**Per 011-authentication-spec.md - External JWT Authentication Only**

## Overview

Web-Terminal implements external JWT (JSON Web Token) authentication with JWKS (JSON Web Key Set) verification. This architecture enables secure, stateless authentication with multiple identity providers.

### Core Principle: External JWT Only

**This system does NOT:**
- Issue JWTs
- Manage user sessions with cookies
- Store passwords
- Authenticate users directly

**This system DOES:**
- Verify JWT tokens from external providers (Backstage, Auth0, etc.)
- Validate JWT signatures using JWKS public keys
- Extract user identity and groups from JWT claims
- Enforce authorization based on user/group membership

## Architecture Components

### 1. JWKS Foundation (src/security/jwks.rs)

The JWKS module provides data structures and key conversion for JWT verification:

```rust
// JSON Web Key Set structure
pub struct JsonWebKeySet {
    pub keys: Vec<JsonWebKey>,
}

// JSON Web Key structure
pub struct JsonWebKey {
    pub kid: String,      // Key ID
    pub kty: String,      // Key type (RSA, EC)
    pub alg: String,      // Algorithm (RS256, ES256, etc.)
    pub use_: String,     // Public key use ("sig" for signature)
    pub n: Option<String>,  // RSA modulus (base64url)
    pub e: Option<String>,  // RSA exponent (base64url)
    // ... EC fields (x, y, crv)
}
```

**Supported Algorithms:**
- RSA: RS256, RS384, RS512
- EC: ES256, ES384, ES512

**Key Features:**
- JWK to DecodingKey conversion
- Base64url decoding for RSA components
- Algorithm validation
- Key validation

### 2. Configuration (src/config/auth.rs)

Authentication configuration supports multiple JWKS providers:

```rust
pub struct AuthConfig {
    pub enabled: bool,
    pub jwks: JwksConfig,
    pub authorization: AuthorizationConfig,
    pub validation: ValidationConfig,
    pub security: SecurityConfig,
}

pub struct JwksProvider {
    pub name: String,           // Provider identifier
    pub url: String,            // JWKS endpoint URL
    pub issuer: String,         // Expected token issuer
    pub audience: String,       // Expected token audience
    pub algorithms: Vec<String>, // Allowed algorithms
    pub cache_ttl: Duration,    // Key cache duration
    pub refresh_interval: Duration, // Key refresh frequency
    pub timeout: Duration,      // HTTP request timeout
}
```

**Default Values:**
- Cache TTL: 1 hour
- Refresh interval: 15 minutes
- Request timeout: 30 seconds
- Clock skew: 60 seconds

### 3. Token Validation Rules

**Standard Claim Validation:**

| Claim | Validation | Required |
|-------|-----------|----------|
| `sub` | Non-empty string | Yes |
| `iss` | Must match configured issuer | Yes |
| `aud` | Must match configured audience | Yes |
| `exp` | Must be in the future | Yes |
| `iat` | Must be in the past | Yes |
| `nbf` | If present, must be in the past | No |

**Security Validation:**
- JWT signature verification using JWKS public key
- Algorithm verification (must match allowed algorithms)
- Token expiration with clock skew tolerance
- Issuer and audience validation

### 4. Claims Extraction

**Standard Claims:**
```json
{
  "sub": "user:default/alice",
  "iss": "https://backstage.example.com",
  "aud": "web-terminal",
  "exp": 1735567200,
  "iat": 1735563600
}
```

**Backstage Claims:**
```json
{
  "sub": "user:default/alice",
  "ent": [
    "user:default/alice",
    "group:default/platform-team"
  ],
  "usc": {
    "ownershipEntityRefs": ["group:default/platform-team"],
    "displayName": "Alice Smith",
    "email": "alice@example.com"
  }
}
```

**Claim Mappings:**
- `user_id`: Extracted from `sub` claim
- `email`: Extracted from `email` or `usc.email`
- `groups`: Extracted from `groups`, `ent`, or `usc.ownershipEntityRefs`
- `entity_ref`: Backstage entity references from `ent`

### 5. Authorization

**User-Based Authorization:**
```yaml
allowed_users:
  - user:default/alice
  - user:default/bob
```

**Group-Based Authorization:**
```yaml
allowed_groups:
  - group:default/platform-team
  - group:default/developers
```

**Authorization Logic:**
- User is authorized if:
  - User ID is in `allowed_users` list, OR
  - User belongs to any group in `allowed_groups` list
- Explicit deny takes precedence:
  - If user is in `deny_users`, access is denied
  - If user belongs to any group in `deny_groups`, access is denied

## Configuration Guide

### Example: Backstage Integration

**config/auth.yaml:**
```yaml
auth:
  enabled: true
  jwks:
    providers:
      - name: backstage
        url: https://backstage.example.com/.well-known/jwks.json
        issuer: https://backstage.example.com
        audience: web-terminal
        cache_ttl: 1h
        refresh_interval: 15m

  authorization:
    allowed_groups:
      - group:default/platform-team
```

**Environment Variables:**
```bash
AUTH_ENABLED=true
AUTH_JWKS_BACKSTAGE_URL="https://backstage.example.com/.well-known/jwks.json"
AUTH_JWKS_BACKSTAGE_ISSUER="https://backstage.example.com"
AUTH_ALLOWED_GROUPS="group:default/platform-team"
```

### Example: Multiple Providers

```yaml
auth:
  enabled: true
  jwks:
    providers:
      - name: backstage
        url: https://backstage.example.com/.well-known/jwks.json
        issuer: https://backstage.example.com
        audience: web-terminal

      - name: auth0
        url: https://example.auth0.com/.well-known/jwks.json
        issuer: https://example.auth0.com/
        audience: https://web-terminal.example.com
```

## Authentication Flow

```
1. Client requests with JWT token in Authorization header
   └─> Authorization: Bearer <token>

2. Middleware extracts JWT token
   └─> Parse Authorization header

3. Decode JWT header
   └─> Extract kid (key ID) and alg (algorithm)

4. Fetch JWKS from provider (cached)
   └─> GET https://provider/.well-known/jwks.json
   └─> Find key with matching kid

5. Verify JWT signature
   └─> Use public key from JWKS
   └─> Validate algorithm matches

6. Verify JWT claims
   └─> Check iss (issuer)
   └─> Check aud (audience)
   └─> Check exp (expiration)
   └─> Check nbf (not before)

7. Extract user identity
   └─> Get user_id from sub claim
   └─> Get groups from ent/groups claims

8. Check authorization
   └─> Verify user is in allowed_users or allowed_groups
   └─> Check deny_users and deny_groups

9. Attach user context to request
   └─> Create session with user identity

10. Process request
   └─> Continue to API handler
```

## Supported Algorithms

**RSA Algorithms:**
- RS256: RSASSA-PKCS1-v1_5 with SHA-256
- RS384: RSASSA-PKCS1-v1_5 with SHA-384
- RS512: RSASSA-PKCS1-v1_5 with SHA-512

**EC Algorithms:**
- ES256: ECDSA with P-256 and SHA-256
- ES384: ECDSA with P-384 and SHA-384
- ES512: ECDSA with P-521 and SHA-512

## Security Considerations

### Token Security

**Best Practices:**
- Always use HTTPS/WSS for token transmission
- Short token expiration (< 1 hour recommended)
- Validate all claims (iss, aud, exp, nbf)
- Use secure key algorithms (RS256, RS384, RS512)
- Never log full JWT tokens

**Rate Limiting:**
```yaml
security:
  rate_limit:
    enabled: true
    max_failed_attempts: 5
    window: 5m
    lockout: 15m
```

### JWKS Key Management

**Automatic Key Refresh:**
- JWKS keys cached with configurable TTL (default: 1 hour)
- Background task refreshes keys before expiration (default: 15 minutes)
- Graceful fallback if JWKS endpoint unavailable

**Key Rotation Handling:**
1. New keys published to JWKS endpoint
2. Both old and new keys available during rotation period
3. Tokens signed with old keys remain valid until expiration
4. Cache refreshes pick up new keys automatically

### Audit Logging

**Logged Events:**
- Authentication attempts (success/failure)
- Authorization decisions (allow/deny)
- JWKS key refresh events
- Token validation errors

**Configuration:**
```yaml
security:
  audit:
    enabled: true
    log_successful_auth: true
    log_failed_auth: true
    log_authorization_denials: true
    log_token_details: false  # Never log full tokens
```

## Testing Strategy

### Unit Tests

**JWKS Module:**
- JWK parsing and validation
- JWK to DecodingKey conversion
- Base64url decoding
- Algorithm validation
- Error handling

**Configuration:**
- Configuration loading from file
- Environment variable overrides
- Default value validation
- Duration serialization

### Integration Tests

**End-to-End:**
1. Start web-terminal with auth enabled
2. Fetch JWKS from mock provider
3. Generate valid JWT with test key
4. Send authenticated request
5. Verify session created with correct user
6. Send unauthenticated request
7. Verify connection rejected

**Security Tests:**
- Token tampering (modified claims)
- Expired token usage
- Invalid signature
- Algorithm confusion (HS256 vs RS256)
- Missing required claims
- Audience mismatch
- Issuer mismatch

## Implementation Checklist

### Phase 1: JWKS Foundation (Day 1) ✓
- [x] Implement JWKS data structures
- [x] Implement JWK to DecodingKey conversion
- [x] Add configuration structures
- [x] Create example configuration
- [x] Add unit tests

### Phase 2: JWKS Client (Day 2)
- [ ] Implement JWKS HTTP client
- [ ] Add JWKS caching
- [ ] Add key refresh mechanism
- [ ] Error handling and retry logic
- [ ] Unit tests

### Phase 3: JWT Validator (Day 3)
- [ ] Implement JWT signature verification
- [ ] Add claim validation
- [ ] Extract user identity
- [ ] Unit tests

### Phase 4: Authorization (Day 4)
- [ ] Implement user allowlist checking
- [ ] Implement group membership checking
- [ ] Add wildcard pattern support
- [ ] Unit tests

### Phase 5: Middleware Integration (Day 5)
- [ ] Actix-Web middleware
- [ ] WebSocket authentication
- [ ] Error responses
- [ ] Integration tests

## References

- [RFC 7519: JSON Web Token (JWT)](https://tools.ietf.org/html/rfc7519)
- [RFC 7517: JSON Web Key (JWK)](https://tools.ietf.org/html/rfc7517)
- [Backstage Authentication](https://backstage.io/docs/auth/)
- [OWASP JWT Security](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
- [011-authentication-spec.md](../spec-kit/011-authentication-spec.md)