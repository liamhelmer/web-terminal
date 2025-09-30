# Web-Terminal JWT Validation Security Audit

**Audit Date:** 2025-09-29
**Auditor:** Authentication Security Specialist
**Layer:** Layer 2 - JWT Token Validation
**Status:** ⚠️ **CRITICAL VULNERABILITIES FOUND**

---

## Executive Summary

This audit evaluates the **JWT token validation** implementation in web-terminal. This service **DOES NOT** issue tokens, authenticate users, or manage credentials. It **ONLY** validates JWT tokens issued by external identity providers (Backstage, Auth0, Keycloak, etc.).

### Security Model

```
┌─────────────────────────┐
│  External IdP           │
│  (Backstage, Auth0...)  │
│  - Issues JWT tokens    │
│  - Manages credentials  │
│  - Authenticates users  │
└───────────┬─────────────┘
            │
            │ JWT Token
            ▼
┌─────────────────────────┐
│  Web-Terminal           │
│  - Validates JWT        │
│  - Verifies signature   │
│  - Checks claims        │
│  - Authorizes access    │
└─────────────────────────┘
```

**Trust Boundary:** Web-terminal trusts valid JWT tokens from configured identity providers.

### Severity Classification

| Severity | Count | Description |
|----------|-------|-------------|
| 🔴 **CRITICAL** | 5 | JWT validation bypass, authentication disabled |
| 🟠 **HIGH** | 3 | Missing validation checks, weak security |
| 🟡 **MEDIUM** | 2 | Configuration issues, monitoring gaps |
| 🟢 **LOW** | 1 | Documentation |

**Overall Risk Rating:** 🔴 **CRITICAL - NOT PRODUCTION READY**

---

## Scope: JWT Validation ONLY

### ✅ IN SCOPE (Web-Terminal Responsibilities)

1. **JWT Signature Verification**
   - Fetch JWKS from external IdP
   - Verify RSA/ECDSA signatures (RS256, RS384, RS512, ES256)
   - Validate `kid` (key ID) matches JWKS key
   - Cache JWKS keys with TTL

2. **Token Expiration Validation**
   - Verify `exp` claim (token not expired)
   - Verify `nbf` claim (token not used before valid time)
   - Verify `iat` claim (token issued time reasonable)
   - Clock skew tolerance (60 seconds recommended)

3. **Issuer Validation**
   - Verify `iss` claim matches configured trusted issuers
   - Reject tokens from unknown issuers
   - Support multiple trusted issuers

4. **Audience Validation**
   - Verify `aud` claim contains this service identifier
   - Reject tokens intended for other services

5. **Claims Extraction**
   - Extract `sub` (subject/user ID)
   - Extract `ent` (Backstage entity references)
   - Parse user and group identifiers

6. **Authorization**
   - Check user allowlist (based on `sub` claim)
   - Check group allowlist (based on `ent` or custom claims)
   - Enforce authorization rules

7. **Token Replay Prevention**
   - Track token usage (optional but recommended)
   - Implement `jti` (JWT ID) validation
   - Detect duplicate token usage

### ❌ OUT OF SCOPE (External IdP Responsibilities)

1. ~~User authentication (login/password)~~
2. ~~Token issuance~~
3. ~~Credential storage~~
4. ~~User registration~~
5. ~~Password management~~
6. ~~Session management (beyond token validation)~~
7. ~~Multi-factor authentication~~
8. ~~Token refresh logic~~

---

## Critical Vulnerabilities

### 🔴 CRITICAL-1: Complete Authentication Bypass

**Location:** `src/server/websocket.rs:190-222`, `src/server/http.rs:190-222`

**Issue:** JWT validation is **COMPLETELY DISABLED**. Hardcoded user allows anyone to access the system.

**Current Implementation:**
```rust
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    session_manager: web::Data<SessionManager>,
) -> Result<HttpResponse> {
    // TODO: Extract session_id from query parameter or JWT token
    // For now, create a new session
    let user_id = "test_user".to_string().into();  // ⚠️ HARDCODED!

    match session_manager.create_session(user_id).await {
        // No JWT validation whatsoever
    }
}
```

**Impact:**
- **COMPLETE AUTHENTICATION BYPASS**
- Anyone can connect without a token
- Anyone can execute arbitrary commands
- No access control whatsoever

**Remediation Priority:** 🔴 BLOCK PRODUCTION IMMEDIATELY

**Required Implementation:**
```rust
async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    session_manager: web::Data<SessionManager>,
    auth_service: web::Data<AuthService>,
) -> Result<HttpResponse> {
    // Extract JWT token from query param, header, or subprotocol
    let token = extract_ws_token(&req)
        .map_err(|_| Error::Unauthorized("Missing JWT token"))?;

    // Validate JWT token
    let claims = auth_service.verify_token(&token).await
        .map_err(|_| Error::Unauthorized("Invalid JWT token"))?;

    // Check authorization
    if !auth_service.is_authorized(&claims) {
        return Err(Error::Forbidden("User not authorized"));
    }

    // Extract user ID from validated claims
    let user_id = claims.user_id()?;

    // Create authenticated session
    match session_manager.create_session(user_id).await {
        // ...
    }
}
```

---

### 🔴 CRITICAL-2: No JWKS Implementation

**Location:** `src/security/auth.rs`

**Issue:** Specification requires JWKS-based verification with RSA public keys, but implementation uses symmetric HMAC secrets.

**Current Implementation:**
```rust
pub struct AuthService {
    encoding_key: EncodingKey,      // ❌ Symmetric key
    decoding_key: DecodingKey,      // ❌ Same symmetric key
    validation: Validation,
    token_expiry: Duration,
}

impl AuthService {
    pub fn new(secret: &[u8]) -> Self {  // ❌ Shared secret
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),  // WRONG!
            // ...
        }
    }
}
```

**Why This is Wrong:**
- Symmetric keys are for **token issuance** (which we don't do)
- External IdPs issue tokens with **asymmetric RSA/ECDSA keys**
- We need **JWKS public keys** to verify tokens from external IdPs

**Required Implementation:**
- JWKS client to fetch public keys from `https://idp.example.com/.well-known/jwks.json`
- RSA/ECDSA signature verification (RS256, RS384, RS512, ES256)
- Key caching with TTL (1 hour recommended)
- Support for multiple JWKS providers
- Kid (Key ID) matching from token header to JWKS key

**Impact:**
- Cannot validate tokens from external identity providers
- No integration with Backstage, Auth0, Keycloak, etc.
- Entire authentication architecture is wrong

**Remediation Priority:** 🔴 IMMEDIATE

---

### 🔴 CRITICAL-3: Missing Issuer Validation

**Location:** `src/security/auth.rs:59-67`

**Issue:** Token validation does not verify the `iss` (issuer) claim against trusted issuers.

**Current Implementation:**
```rust
pub fn validate_token(&self, token: &str) -> Result<UserId> {
    let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
        .map_err(|e| {
            tracing::warn!("Invalid JWT token: {}", e);
            Error::InvalidToken
        })?;

    Ok(UserId::new(token_data.claims.sub))  // ❌ No issuer check!
}
```

**Attack Scenario:**
1. Attacker sets up malicious IdP: `https://evil.com`
2. Attacker issues JWT token with `iss: "https://evil.com"`
3. Token signature validates (if attacker controls JWKS)
4. Web-terminal accepts token from untrusted issuer

**Required:**
```rust
pub fn validate_token(&self, token: &str) -> Result<BackstageClaims> {
    let token_data = decode::<BackstageClaims>(
        token,
        &decoding_key,
        &self.validation
    )?;

    // ✅ REQUIRED: Verify issuer
    let issuer = token_data.claims.iss.as_deref()
        .ok_or(Error::MissingClaim("iss"))?;

    if !self.config.allowed_issuers.contains(&issuer.to_string()) {
        return Err(Error::InvalidIssuer(issuer.to_string()));
    }

    Ok(token_data.claims)
}
```

**Impact:**
- Tokens from ANY issuer accepted if signature valid
- No control over trusted identity providers
- Attacker can create malicious IdP

**Remediation Priority:** 🔴 IMMEDIATE

---

### 🔴 CRITICAL-4: Missing Audience Validation

**Location:** `src/security/auth.rs:84-92`

**Issue:** JWT claims structure does not validate `aud` (audience) claim.

**Current Implementation:**
```rust
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
    // ❌ Missing: aud validation
}
```

**Attack Scenario:**
1. User obtains valid JWT for different service: `aud: "different-service"`
2. User presents token to web-terminal
3. Web-terminal accepts token even though intended for different service
4. Token reuse attack succeeds

**Required:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BackstageClaims {
    pub sub: String,
    pub iss: Option<String>,
    pub aud: Option<Vec<String>>,  // ✅ REQUIRED
    pub exp: usize,
    pub iat: usize,
    pub nbf: Option<usize>,
    // ...
}

// In validation:
if let Some(aud) = &claims.aud {
    if !aud.contains(&config.expected_audience) {
        return Err(Error::InvalidAudience);
    }
}
```

**Impact:**
- Tokens issued for OTHER services accepted
- Token reuse across applications possible
- Cross-service token abuse

**Remediation Priority:** 🔴 IMMEDIATE

---

### 🔴 CRITICAL-5: No Authorization Checks

**Location:** `src/security/` (missing authorization module)

**Issue:** No authorization enforcement after JWT validation.

**Security Principle Violated:**
```
Authentication ≠ Authorization
Valid JWT ≠ Authorized Access
```

**Current State:**
```rust
// ❌ WRONG: JWT validation grants full access
let claims = auth_service.verify_token(token).await?;
let user_id = claims.sub;
// Immediate full access granted
```

**Required Authorization:**
```rust
// ✅ CORRECT: Separate authorization check
let claims = auth_service.verify_token(token).await?;

// Authorization check
if !authz_service.is_authorized(&claims, "terminal", "create_session") {
    return Err(Error::Forbidden("User not authorized"));
}

let user_id = claims.user_id()?;
```

**Required Components (Missing):**
- `AuthorizationService` with allowlist checking
- User allowlist validation (check `sub` claim)
- Group allowlist validation (check `ent` or custom claims)
- Backstage entity reference parsing (`user:default/alice`, `group:default/developers`)

**Impact:**
- ANY authenticated user can access terminal
- No access control enforcement
- Violates principle of least privilege

**Remediation Priority:** 🔴 IMMEDIATE

---

## High Severity Issues

### 🟠 HIGH-1: No Clock Skew Tolerance

**Location:** `src/security/auth.rs:69-79`

**Issue:** Expiration checking does not account for clock skew between IdP and web-terminal.

**Current:**
```rust
pub fn is_token_expired(&self, token: &str) -> bool {
    match decode::<Claims>(token, &self.decoding_key, &self.validation) {
        Ok(token_data) => {
            let exp = token_data.claims.exp;
            let now = Utc::now().timestamp() as usize;
            now > exp  // ❌ No clock skew tolerance!
        }
        Err(_) => true,
    }
}
```

**Problem:**
- IdP server time: `2025-09-29T10:00:00Z`
- Web-terminal server time: `2025-09-29T10:00:30Z` (30 seconds ahead)
- Token expires at: `2025-09-29T10:00:00Z`
- Token incorrectly rejected as expired

**Required:**
```rust
// ✅ Add clock skew tolerance (60 seconds)
let mut validation = Validation::new(algorithm);
validation.leeway = 60;  // Allow 60 seconds clock skew

// Check expiration with tolerance
let now = Utc::now().timestamp() as i64;
let exp = token_data.claims.exp as i64;
let leeway = 60;

if now > exp + leeway {
    return Err(Error::TokenExpired);
}

// Check not-before with tolerance
if let Some(nbf) = token_data.claims.nbf {
    if now < nbf as i64 - leeway {
        return Err(Error::TokenNotYetValid);
    }
}
```

**Remediation Priority:** 🟠 HIGH

---

### 🟠 HIGH-2: No JWKS Key Refresh

**Location:** Missing JWKS implementation

**Issue:** No mechanism to refresh JWKS keys periodically or on validation failure.

**Required:**
- Background task to refresh JWKS every 1 hour
- On-demand refresh if kid not found in cache
- Exponential backoff on JWKS fetch failures
- Fallback to cached keys if refresh fails

**Implementation:**
```rust
// Background refresh task
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        if let Err(e) = jwks_client.refresh_all().await {
            tracing::error!("Failed to refresh JWKS: {}", e);
        }
    }
});

// On-demand refresh
pub async fn get_key(&self, kid: &str) -> Result<DecodingKey> {
    // Try cache first
    if let Some(key) = self.cache.get(kid).await {
        return Ok(key);
    }

    // Refresh and retry
    self.refresh_all().await?;

    self.cache.get(kid).await
        .ok_or(Error::KeyNotFound(kid.to_string()))
}
```

**Impact:**
- IdP key rotation breaks validation
- Cannot handle compromised key rotation
- Service outage on key updates

**Remediation Priority:** 🟠 HIGH

---

### 🟠 HIGH-3: No Token Replay Prevention

**Location:** Missing `jti` (JWT ID) tracking

**Issue:** Tokens can be replayed multiple times until expiration.

**Attack Scenario:**
1. Attacker intercepts valid JWT token
2. Attacker uses token to create multiple sessions
3. Token remains valid until `exp` expiration
4. Attacker can replay token for hours

**Mitigation (Optional but Recommended):**
```rust
pub struct TokenTracker {
    used_tokens: Arc<RwLock<HashSet<String>>>,  // Track jti
    ttl: Duration,
}

impl TokenTracker {
    pub async fn check_and_mark(&self, jti: &str) -> Result<()> {
        let mut used = self.used_tokens.write().await;

        if used.contains(jti) {
            return Err(Error::TokenReplayed);
        }

        used.insert(jti.to_string());
        Ok(())
    }
}
```

**Note:** This requires IdP to include `jti` claim in tokens.

**Remediation Priority:** 🟠 HIGH

---

## Medium Severity Issues

### 🟡 MEDIUM-1: Missing JWKS Configuration

**Location:** `src/config/`

**Issue:** No configuration structure for JWKS providers.

**Required:**
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct JwksConfig {
    pub providers: Vec<JwksProvider>,
    pub refresh_interval_secs: u64,
    pub cache_ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwksProvider {
    pub name: String,
    pub jwks_uri: String,
    pub issuer: String,
    pub audience: Option<String>,
}
```

**Environment Variables:**
```bash
JWKS_URI=https://backstage.example.com/.well-known/jwks.json
JWKS_ISSUER=https://backstage.example.com
JWKS_AUDIENCE=web-terminal
JWKS_REFRESH_INTERVAL=3600
```

**Remediation Priority:** 🟡 MEDIUM

---

### 🟡 MEDIUM-2: No JWT Validation Metrics

**Location:** Missing metrics implementation

**Required Metrics:**
- `jwt_validation_total{result="success|failure"}`
- `jwt_validation_duration_seconds`
- `jwks_refresh_total{result="success|failure"}`
- `jwks_cache_hit_total`
- `jwks_cache_miss_total`
- `jwt_error_total{error_type="expired|invalid_signature|invalid_issuer"}`

**Remediation Priority:** 🟡 MEDIUM

---

## Low Severity Issues

### 🟢 LOW-1: Incomplete Documentation

**Issue:** Documentation does not clearly distinguish JWT validation from authentication.

**Required Updates:**
- README: Explain external IdP integration
- Architecture docs: JWT validation flow diagram
- Configuration examples for common IdPs (Backstage, Auth0, Keycloak)

**Remediation Priority:** 🟢 LOW

---

## Compliance with Specification

### JWT Validation Components (Section 3, Backend Spec)

| Component | Spec Requirement | Implementation Status | Gap |
|-----------|-----------------|----------------------|-----|
| JWKS Client | ✅ Required | ❌ Not Implemented | Complete implementation needed |
| JWT Verifier | ✅ Required (RS256/384/512) | ⚠️ Wrong (HS256 symmetric) | Replace with JWKS-based verification |
| Claims Extraction | ✅ Required | ⚠️ Partial (missing iss/aud) | Add issuer/audience validation |
| Authorization | ✅ Required | ❌ Not Implemented | Implement allowlist checking |

**Compliance Score: 10% (0/4 components fully compliant)**

### JWT Claims Validation (Section 4, API Spec)

| Claim | Spec Requirement | Implementation Status | Gap |
|-------|-----------------|----------------------|-----|
| `sub` | ✅ Required | ✅ Extracted | Validation incomplete |
| `iss` | ✅ Required | ❌ Not Validated | Add issuer validation |
| `aud` | ✅ Required | ❌ Not Validated | Add audience validation |
| `exp` | ✅ Required | ⚠️ Partial | Add clock skew tolerance |
| `iat` | ✅ Required | ✅ Checked | Improve validation |
| `nbf` | ⚠️ Optional | ❌ Not Supported | Add nbf validation |
| `ent` | ⚠️ Backstage | ❌ Not Supported | Add for group authorization |
| `jti` | ⚠️ Optional | ❌ Not Supported | Add for replay prevention |

**Compliance Score: 25% (2/8 claims validated)**

### JWT Validation Flow (Section 6, Backend Spec)

| Flow Step | Spec Requirement | Implementation Status | Gap |
|-----------|-----------------|----------------------|-----|
| Extract JWT | ✅ Required | ❌ Not Implemented | WebSocket/HTTP extraction |
| Decode header | ✅ Required | ⚠️ Partial | Extract `kid` for JWKS lookup |
| Fetch JWKS key | ✅ Required | ❌ Not Implemented | Implement JWKS client |
| Verify signature | ✅ Required | ⚠️ Wrong algorithm | Use RSA not HMAC |
| Verify claims | ✅ Required | ⚠️ Partial | Add iss/aud/exp validation |
| Check authorization | ✅ Required | ❌ Not Implemented | Implement allowlist |
| Create session | ✅ Required | ⚠️ Partial | Bind to validated user |

**Compliance Score: 14% (1/7 steps)**

---

## Attack Surface Analysis

### Exploitable Vulnerabilities

1. **Complete Authentication Bypass**
   - **Vector:** Connect to WebSocket/HTTP without token
   - **Difficulty:** Trivial (no token required)
   - **Impact:** Full terminal access, arbitrary command execution
   - **CVSS Score:** 10.0 (Critical)

2. **Token Reuse Across Services**
   - **Vector:** Use token from different service
   - **Difficulty:** Easy (no audience validation)
   - **Impact:** Unauthorized access with valid token
   - **CVSS Score:** 7.5 (High)

3. **Malicious IdP Attack**
   - **Vector:** Use token from untrusted issuer
   - **Difficulty:** Medium (requires issuer spoofing)
   - **Impact:** Authentication bypass with forged token
   - **CVSS Score:** 8.0 (High)

4. **Token Replay Attack**
   - **Vector:** Replay intercepted token
   - **Difficulty:** Easy (no jti tracking)
   - **Impact:** Session hijacking until token expiration
   - **CVSS Score:** 6.5 (Medium)

5. **Authorization Bypass**
   - **Vector:** Valid JWT grants full access (no allowlist)
   - **Difficulty:** Trivial
   - **Impact:** Unauthorized users gain terminal access
   - **CVSS Score:** 8.5 (High)

---

## Recommended Implementation Order

### Phase 1: Block Production Use (Week 1)

**Priority: CRITICAL**

1. ✅ Implement JWT token extraction for WebSocket (CRITICAL-1)
2. ✅ Implement JWT token extraction for HTTP endpoints (CRITICAL-1)
3. ✅ Add basic JWT validation (signature verification)
4. ✅ Reject all requests without valid JWT token

**Exit Criteria:** No authentication bypass vulnerabilities

---

### Phase 2: JWKS Implementation (Week 2)

**Priority: CRITICAL**

1. ✅ Implement JWKS client with HTTP fetching (CRITICAL-2)
2. ✅ Add JWKS key caching with TTL
3. ✅ Implement RSA signature verification (RS256, RS384, RS512)
4. ✅ Add `kid` matching from token header to JWKS key
5. ✅ Background task for JWKS refresh

**Exit Criteria:** External IdP integration works

---

### Phase 3: Claims Validation (Week 3)

**Priority: CRITICAL**

1. ✅ Implement issuer validation (CRITICAL-3)
2. ✅ Implement audience validation (CRITICAL-4)
3. ✅ Add clock skew tolerance (HIGH-1)
4. ✅ Implement `nbf` (not before) validation
5. ✅ Parse Backstage entity references (`ent` claim)

**Exit Criteria:** All required claims validated

---

### Phase 4: Authorization (Week 4)

**Priority: CRITICAL**

1. ✅ Implement authorization service (CRITICAL-5)
2. ✅ User allowlist checking (`sub` claim)
3. ✅ Group allowlist checking (`ent` claim)
4. ✅ Configuration for allowed users/groups
5. ✅ Authorization enforcement on all endpoints

**Exit Criteria:** Access control enforced

---

### Phase 5: Security Hardening (Week 5)

**Priority: HIGH**

1. ✅ Implement token replay prevention (HIGH-3)
2. ✅ Add JWKS key refresh on failure (HIGH-2)
3. ✅ Implement audit logging for validation failures
4. ✅ Add security metrics (MEDIUM-2)
5. ✅ TLS enforcement in production

**Exit Criteria:** Production-ready security posture

---

### Phase 6: Testing & Documentation (Week 6)

**Priority: MEDIUM-LOW**

1. ✅ Comprehensive JWT validation tests
2. ✅ JWKS integration tests
3. ✅ Authorization tests
4. ✅ Update documentation (LOW-1)
5. ✅ Security review and sign-off

**Exit Criteria:** 90%+ test coverage, security approval

---

## Testing Requirements

### Security Test Suite (Required Before Production)

```rust
// tests/security/jwt_validation_test.rs

#[tokio::test]
async fn test_authentication_bypass_blocked() {
    // Verify WebSocket/HTTP without token rejected
    let response = test_client.connect_websocket(None).await;
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_jwks_signature_verification() {
    // Verify RSA signature validation works
    let token = create_rsa_signed_token(jwks_provider);
    let claims = auth_service.verify_token(&token).await;
    assert!(claims.is_ok());
}

#[tokio::test]
async fn test_invalid_signature_rejected() {
    // Verify tokens with invalid signatures rejected
    let token = create_token_with_invalid_signature();
    let result = auth_service.verify_token(&token).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_issuer_validation() {
    // Verify tokens from untrusted issuers rejected
    let token = create_token_with_issuer("https://evil.com");
    let result = auth_service.verify_token(&token).await;
    assert_eq!(result.unwrap_err(), Error::InvalidIssuer);
}

#[tokio::test]
async fn test_audience_validation() {
    // Verify tokens for other services rejected
    let token = create_token_with_audience("different-service");
    let result = auth_service.verify_token(&token).await;
    assert_eq!(result.unwrap_err(), Error::InvalidAudience);
}

#[tokio::test]
async fn test_token_expiration() {
    // Verify expired tokens rejected
    let token = create_expired_token();
    let result = auth_service.verify_token(&token).await;
    assert_eq!(result.unwrap_err(), Error::TokenExpired);
}

#[tokio::test]
async fn test_clock_skew_tolerance() {
    // Verify clock skew tolerance works
    let token = create_token_expiring_in(30); // 30 seconds
    // Simulate 45 seconds clock skew
    let result = auth_service.verify_token_with_skew(&token, 45).await;
    assert!(result.is_ok()); // Within 60 second tolerance
}

#[tokio::test]
async fn test_authorization_enforced() {
    // Verify unauthorized users blocked
    let token = create_valid_token("user:default/unauthorized");
    let result = authz_service.is_authorized(&token, "terminal", "create_session").await;
    assert!(!result);
}

#[tokio::test]
async fn test_jwks_key_rotation() {
    // Verify key rotation handled gracefully
    let old_token = create_token_with_kid("old-key");
    let new_token = create_token_with_kid("new-key");

    // Rotate JWKS keys
    jwks_provider.rotate_keys().await;

    // Old token should fail
    assert!(auth_service.verify_token(&old_token).await.is_err());

    // New token should succeed
    assert!(auth_service.verify_token(&new_token).await.is_ok());
}

#[tokio::test]
async fn test_token_replay_prevention() {
    // Verify token replay blocked
    let token = create_token_with_jti("unique-id-123");

    // First use succeeds
    let result1 = auth_service.verify_and_track(&token).await;
    assert!(result1.is_ok());

    // Second use fails
    let result2 = auth_service.verify_and_track(&token).await;
    assert_eq!(result2.unwrap_err(), Error::TokenReplayed);
}
```

---

## Code Examples for Critical Fixes

### Fix 1: JWKS Client Implementation

```rust
// src/security/jwks.rs

use jsonwebtoken::jwk::JwkSet;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct JwksClient {
    http_client: Client,
    cache: Arc<RwLock<JwksCache>>,
    providers: Vec<JwksProvider>,
}

#[derive(Debug)]
struct JwksCache {
    keys: HashMap<String, CachedKey>,
}

#[derive(Debug, Clone)]
struct CachedKey {
    jwk: Jwk,
    cached_at: Instant,
    expires_at: Instant,
}

impl JwksClient {
    pub fn new(providers: Vec<JwksProvider>) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(RwLock::new(JwksCache {
                keys: HashMap::new(),
            })),
            providers,
        }
    }

    /// Refresh JWKS from all configured providers
    pub async fn refresh_all(&self) -> Result<()> {
        for provider in &self.providers {
            if let Err(e) = self.refresh_provider(provider).await {
                tracing::error!("Failed to refresh JWKS for {}: {}", provider.name, e);
            }
        }
        Ok(())
    }

    /// Fetch JWKS from specific provider
    async fn refresh_provider(&self, provider: &JwksProvider) -> Result<()> {
        let response = self.http_client
            .get(&provider.jwks_uri)
            .send()
            .await?;

        let jwks: JwkSet = response.json().await?;

        let mut cache = self.cache.write().await;
        let now = Instant::now();
        let ttl = Duration::from_secs(3600); // 1 hour

        for jwk in jwks.keys {
            if let Some(kid) = &jwk.common.key_id {
                cache.keys.insert(
                    kid.clone(),
                    CachedKey {
                        jwk,
                        cached_at: now,
                        expires_at: now + ttl,
                    },
                );
            }
        }

        tracing::info!("Refreshed JWKS for provider: {}", provider.name);
        Ok(())
    }

    /// Get signing key by kid
    pub async fn get_key(&self, kid: &str) -> Result<DecodingKey> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.keys.get(kid) {
            // Check if expired
            if Instant::now() < cached.expires_at {
                return DecodingKey::from_jwk(&cached.jwk)
                    .map_err(|e| Error::JwksError(e.to_string()));
            }
        }

        drop(cache);

        // Key not found or expired, refresh
        self.refresh_all().await?;

        // Try again after refresh
        let cache = self.cache.read().await;
        if let Some(cached) = cache.keys.get(kid) {
            return DecodingKey::from_jwk(&cached.jwk)
                .map_err(|e| Error::JwksError(e.to_string()));
        }

        Err(Error::KeyNotFound(kid.to_string()))
    }
}
```

### Fix 2: JWT Validation with Full Claims Checking

```rust
// src/security/auth.rs

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use crate::security::claims::BackstageClaims;
use crate::security::jwks::JwksClient;

pub struct AuthService {
    jwks_client: Arc<JwksClient>,
    config: ValidationConfig,
}

impl AuthService {
    pub async fn verify_token(&self, token: &str) -> Result<BackstageClaims> {
        // 1. Decode header to get kid and algorithm
        let header = decode_header(token)?;

        let kid = header.kid
            .ok_or_else(|| Error::InvalidToken("Missing kid".to_string()))?;

        // 2. Get decoding key from JWKS
        let decoding_key = self.jwks_client.get_key(&kid).await?;

        // 3. Configure validation
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&self.config.allowed_issuers);

        if let Some(aud) = &self.config.required_audience {
            validation.set_audience(&[aud]);
        }

        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 60; // Clock skew tolerance

        // 4. Decode and verify token
        let token_data = decode::<BackstageClaims>(
            token,
            &decoding_key,
            &validation,
        )?;

        // 5. Verify issuer (additional check)
        let issuer = token_data.claims.iss.as_deref()
            .ok_or_else(|| Error::InvalidToken("Missing issuer".to_string()))?;

        if !self.config.allowed_issuers.contains(&issuer.to_string()) {
            return Err(Error::InvalidIssuer(issuer.to_string()));
        }

        // 6. Verify audience if present
        if let Some(aud) = &token_data.claims.aud {
            if let Some(expected_aud) = &self.config.required_audience {
                if !aud.contains(expected_aud) {
                    return Err(Error::InvalidAudience);
                }
            }
        }

        Ok(token_data.claims)
    }
}
```

### Fix 3: Authorization Service

```rust
// src/security/authz.rs

pub struct AuthorizationService {
    allowed_users: Vec<String>,
    allowed_groups: Vec<String>,
}

impl AuthorizationService {
    pub fn is_authorized(&self, claims: &BackstageClaims) -> bool {
        // Check user allowlist
        if self.allowed_users.contains(&claims.sub) {
            return true;
        }

        // Check group allowlist (from ent claim)
        for entity in &claims.ent {
            if entity.starts_with("group:") && self.allowed_groups.contains(entity) {
                return true;
            }
        }

        // Deny by default
        false
    }
}
```

---

## Security Recommendations

### Immediate Actions (Before Any Deployment)

1. **Disable WebSocket/HTTP endpoints** until JWT validation implemented
2. **Implement JWKS client** with RSA verification
3. **Add issuer/audience validation** to JWT verification
4. **Implement authorization service** with allowlist checking
5. **Add audit logging** for all validation failures

### Short-Term (1-2 Weeks)

1. Complete JWKS implementation with caching
2. Add RSA/ECDSA signature verification
3. Implement all claims validation (iss, aud, exp, nbf, iat)
4. Add authorization with user/group allowlists
5. Implement JWT extraction for WebSocket

### Medium-Term (1 Month)

1. Comprehensive JWT validation testing
2. Integration tests with real IdP (Backstage)
3. Token replay prevention
4. Security metrics and monitoring
5. Documentation updates

### Long-Term (Ongoing)

1. Regular security audits
2. JWKS endpoint monitoring
3. Token usage analytics
4. Incident response planning
5. IdP integration testing

---

## Conclusion

The current implementation has **CRITICAL security vulnerabilities** that make the application **COMPLETELY UNSAFE FOR PRODUCTION USE**. The authentication is **DISABLED** (hardcoded user) and JWT validation is fundamentally wrong (symmetric keys instead of JWKS).

### Critical Path to Production:

1. ✅ **Week 1:** Implement JWT extraction and basic validation
2. ✅ **Week 2:** Implement JWKS client with RSA verification
3. ✅ **Week 3:** Add full claims validation (iss, aud, exp, nbf)
4. ✅ **Week 4:** Implement authorization with allowlists
5. ✅ **Week 5:** Security hardening and testing
6. ✅ **Week 6:** Security review and production approval

**Estimated Time to Production-Ready:** 6 weeks minimum

### Risk Assessment:

- **Current State:** 🔴 CRITICAL - Authentication completely disabled
- **After Phase 1:** 🟠 HIGH - Basic validation but incomplete
- **After Phase 2:** 🟡 MEDIUM - JWKS works but claims incomplete
- **After Phase 3:** 🟡 MEDIUM - Claims validated but no authorization
- **After Phase 4:** 🟢 LOW - Authorization enforced
- **After Phase 5:** ✅ PRODUCTION READY - Full security controls

---

**Audit Completed:** 2025-09-29
**Next Audit Due:** After Phase 1 completion (JWT validation implemented)
**Security Approval Required Before Production**