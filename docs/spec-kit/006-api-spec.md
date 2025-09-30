# Web-Terminal: REST API Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

The web-terminal REST API provides HTTP endpoints for session management, configuration, and monitoring. All endpoints require authentication unless otherwise noted.

**Base URL:** `/api/v1` (relative to server origin)

**Note:** All API endpoints use relative paths. The frontend automatically detects the server URL from `window.location.origin`.

---

## Authentication

### JWT Bearer Token Authentication

All API requests require a valid JWT token in the Authorization header:

```http
Authorization: Bearer <jwt_token>
```

The server validates JWT tokens using JWKS (JSON Web Key Set) discovery, supporting both self-issued tokens and external identity providers like Backstage.

### JWT Token Format

Tokens must be valid JWTs (RFC 7519) with the following structure:

**Header:**
```json
{
  "alg": "RS256",
  "typ": "JWT",
  "kid": "key-id-123"
}
```

**Required Claims:**
- `sub`: Subject (user identifier, e.g., `user:default/alice` for Backstage)
- `exp`: Expiration timestamp (Unix epoch)
- `iat`: Issued at timestamp (Unix epoch)
- `iss`: Token issuer URL (must match configured JWKS provider)

**Optional Claims (Backstage Integration):**
- `ent`: Entity ownership array (e.g., `["user:default/alice", "group:default/developers"]`)
- `aud`: Audience (token intended recipient)
- `nbf`: Not before timestamp

**Example Token Payload:**
```json
{
  "sub": "user:default/alice",
  "iss": "https://backstage.example.com",
  "aud": "web-terminal",
  "exp": 1727610000,
  "iat": 1727606400,
  "ent": [
    "user:default/alice",
    "group:default/developers",
    "group:default/platform-team"
  ]
}
```

### JWKS Discovery

The server fetches public keys from the configured JWKS endpoint to validate token signatures:

**Configuration:**
```toml
[security]
jwks_url = "https://backstage.example.com/.well-known/jwks.json"
jwks_refresh_interval = 3600  # seconds
issuer = "https://backstage.example.com"
```

**JWKS Endpoint Format:**
```http
GET https://backstage.example.com/.well-known/jwks.json

Response: 200 OK
{
  "keys": [
    {
      "kty": "RSA",
      "kid": "key-id-123",
      "use": "sig",
      "alg": "RS256",
      "n": "0vx7agoebGcQSuuPiLJ...",
      "e": "AQAB"
    }
  ]
}
```

### JWT Token Requirements

**Validation Rules:**

1. **Signature Verification:**
   - Token signature MUST be verified using JWKS public key
   - `kid` (key ID) in token header MUST match a key in JWKS
   - Algorithm MUST be RS256 (RSA with SHA-256)

2. **Required Claims Validation:**
   - `sub`: MUST be present and non-empty
   - `exp`: MUST be present and in the future (Unix timestamp)
   - `iat`: MUST be present and not in the future
   - `iss`: MUST match configured issuer URL

3. **Optional Claims Validation:**
   - `nbf` (not before): If present, current time MUST be >= nbf
   - `aud` (audience): If present, MUST contain expected audience value
   - `ent` (entities): If present, MUST be a valid array of strings

4. **Security Checks:**
   - Token MUST NOT be expired (`exp` > current time)
   - Token MUST NOT be used before valid (`nbf` <= current time, if present)
   - Issuer MUST match configured issuer exactly
   - Token age MUST be reasonable (reject if `iat` is too old, e.g., > 24 hours)

**Backstage Token Claims:**

When integrating with Backstage, tokens include:
- `sub`: User entity reference (format: `user:default/username` or `user:namespace/username`)
- `ent`: Array of owned entities including user and groups (format: `["user:default/alice", "group:default/developers"]`)
- `iss`: Backstage instance URL (e.g., `https://backstage.example.com`)

**Example Valid Token:**
```json
{
  "alg": "RS256",
  "kid": "backstage-key-2025",
  "typ": "JWT"
}
{
  "sub": "user:default/alice",
  "iss": "https://backstage.example.com",
  "aud": "web-terminal",
  "exp": 1727610000,
  "iat": 1727606400,
  "nbf": 1727606400,
  "ent": [
    "user:default/alice",
    "group:default/developers",
    "group:default/platform-team"
  ]
}
```

### Authorization

Access control supports both user-based and group-based authorization:

**User Authorization:**
- Token's `sub` claim must match allowed user list
- Example: `sub: "user:default/alice"` matches allowed user `"user:default/alice"`

**Group Authorization (Backstage):**
- Any entity in `ent` array matches allowed groups
- Example: `ent: ["user:default/alice", "group:default/developers"]` grants access if `"group:default/developers"` is in allowed groups

**Authorization Flow:**
1. Extract `sub` claim from validated token
2. Check if `sub` is in `allowed_users` list → **ALLOW**
3. If `ent` claim exists, check if any entity is in `allowed_groups` list → **ALLOW**
4. If `require_group` is true and no group matches → **DENY (403)**
5. If no authorization rules match → **DENY (403)**

**Configuration:**
```toml
[security.authorization]
allowed_users = ["user:default/alice", "user:default/bob"]
allowed_groups = ["group:default/developers", "group:default/admins"]
require_group = false  # If true, at least one group membership required
```

**Authorization Examples:**

| Token `sub` | Token `ent` | Allowed Users | Allowed Groups | Result |
|-------------|-------------|---------------|----------------|--------|
| `user:default/alice` | `["user:default/alice", "group:default/devs"]` | `["user:default/alice"]` | `[]` | ✅ ALLOW (user match) |
| `user:default/bob` | `["user:default/bob", "group:default/devs"]` | `[]` | `["group:default/devs"]` | ✅ ALLOW (group match) |
| `user:default/charlie` | `["user:default/charlie"]` | `[]` | `["group:default/devs"]` | ❌ DENY 403 (no match) |
| `user:default/alice` | `["user:default/alice"]` | `[]` | `["group:default/devs"]` | ❌ DENY 403 if `require_group=true` |

### Authentication Endpoints

#### Get Token Info

Retrieve information about the current token:

```http
GET /api/v1/auth/info
Authorization: Bearer <token>

Response: 200 OK
{
  "valid": true,
  "sub": "user:default/alice",
  "iss": "https://backstage.example.com",
  "exp": 1727610000,
  "iat": 1727606400,
  "entities": [
    "user:default/alice",
    "group:default/developers"
  ],
  "expires_in": 3600
}
```

#### Validate Token

Explicitly validate a token:

```http
POST /api/v1/auth/validate
Content-Type: application/json

{
  "token": "eyJhbGc..."
}

Response: 200 OK
{
  "valid": true,
  "sub": "user:default/alice",
  "authorized": true,
  "expires_at": "2025-09-29T10:00:00Z"
}

Response: 401 Unauthorized (invalid token)
{
  "error": {
    "code": "JWT_INVALID",
    "message": "Token signature verification failed",
    "details": {
      "reason": "signature_mismatch"
    }
  }
}
```

#### JWKS Endpoint (Optional)

If server acts as identity provider, it can serve its own JWKS:

```http
GET /api/v1/.well-known/jwks.json

Response: 200 OK
{
  "keys": [
    {
      "kty": "RSA",
      "kid": "web-terminal-2025",
      "use": "sig",
      "alg": "RS256",
      "n": "xGOr-H7A...",
      "e": "AQAB"
    }
  ]
}
```

### Obtain Token (Local Auth)

For local authentication (when not using external IDP):

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "alice",
  "password": "secret"
}

Response: 200 OK
{
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc...",
  "token_type": "Bearer",
  "expires_at": "2025-09-29T10:00:00Z",
  "expires_in": 3600,
  "user_id": "user:default/alice"
}

Response: 401 Unauthorized
{
  "error": {
    "code": "INVALID_CREDENTIALS",
    "message": "Invalid username or password"
  }
}
```

### Refresh Token (Local Auth)

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJhbGc..."
}

Response: 200 OK
{
  "access_token": "eyJhbGc...",
  "token_type": "Bearer",
  "expires_at": "2025-09-29T10:00:00Z",
  "expires_in": 3600
}

Response: 401 Unauthorized
{
  "error": {
    "code": "JWT_EXPIRED",
    "message": "Refresh token has expired"
  }
}
```

### Logout

Invalidate current session:

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>

Response: 204 No Content

Response: 401 Unauthorized
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or missing authentication token"
  }
}
```

---

## Sessions API

### Create Session

```http
POST /api/v1/sessions
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "initial_dir": "/workspace",
  "environment": {
    "VAR": "value"
  }
}

Response: 201 Created
{
  "id": "session123",
  "user_id": "user:default/alice",
  "created_at": "2025-09-29T09:00:00Z",
  "state": {
    "working_dir": "/workspace",
    "environment": {},
    "processes": []
  }
}

Response: 401 Unauthorized (JWT invalid/expired)
{
  "error": {
    "code": "JWT_EXPIRED",
    "message": "JWT token has expired",
    "details": {
      "expired_at": "2025-09-29T08:00:00Z"
    }
  },
  "www_authenticate": "Bearer realm=\"web-terminal\", error=\"invalid_token\", error_description=\"JWT token has expired\""
}

Response: 403 Forbidden (user not authorized)
{
  "error": {
    "code": "UNAUTHORIZED_USER",
    "message": "User is not authorized to create sessions",
    "details": {
      "user": "user:default/alice",
      "required_groups": ["group:default/developers"]
    }
  }
}
```

### Get Session

```http
GET /api/v1/sessions/{session_id}
Authorization: Bearer <token>

Response: 200 OK
{
  "id": "session123",
  "user_id": "user123",
  "created_at": "2025-09-29T09:00:00Z",
  "last_activity": "2025-09-29T09:30:00Z",
  "state": {
    "working_dir": "/workspace/project",
    "environment": {
      "PATH": "/usr/bin:/bin"
    },
    "processes": [
      {
        "pid": 1234,
        "command": "sleep 60",
        "status": "running"
      }
    ]
  }
}
```

### List Sessions

```http
GET /api/v1/sessions?user_id=user123&limit=10&offset=0
Authorization: Bearer <token>

Response: 200 OK
{
  "sessions": [
    {
      "id": "session123",
      "user_id": "user123",
      "created_at": "2025-09-29T09:00:00Z",
      "last_activity": "2025-09-29T09:30:00Z"
    }
  ],
  "total": 5,
  "limit": 10,
  "offset": 0
}
```

### Delete Session

```http
DELETE /api/v1/sessions/{session_id}
Authorization: Bearer <token>

Response: 204 No Content
```

### Get Session History

```http
GET /api/v1/sessions/{session_id}/history?limit=100
Authorization: Bearer <token>

Response: 200 OK
{
  "history": [
    {
      "timestamp": "2025-09-29T09:15:00Z",
      "command": "ls -la",
      "exit_code": 0
    },
    {
      "timestamp": "2025-09-29T09:16:00Z",
      "command": "cd /workspace",
      "exit_code": 0
    }
  ]
}
```

---

## File System API

### List Directory

```http
GET /api/v1/sessions/{session_id}/files?path=/workspace
Authorization: Bearer <token>

Response: 200 OK
{
  "path": "/workspace",
  "entries": [
    {
      "name": "file.txt",
      "type": "file",
      "size": 1024,
      "modified": "2025-09-29T09:00:00Z",
      "permissions": "rw-r--r--"
    },
    {
      "name": "directory",
      "type": "directory",
      "size": 4096,
      "modified": "2025-09-29T08:00:00Z",
      "permissions": "rwxr-xr-x"
    }
  ]
}
```

### Upload File

```http
POST /api/v1/sessions/{session_id}/files
Authorization: Bearer <token>
Content-Type: multipart/form-data

file: <binary data>
path: /workspace/uploaded.txt

Response: 201 Created
{
  "path": "/workspace/uploaded.txt",
  "size": 2048,
  "checksum": "sha256:abc123..."
}
```

### Download File

```http
GET /api/v1/sessions/{session_id}/files/download?path=/workspace/file.txt
Authorization: Bearer <token>

Response: 200 OK
Content-Type: application/octet-stream
Content-Disposition: attachment; filename="file.txt"

<binary data>
```

### Delete File

```http
DELETE /api/v1/sessions/{session_id}/files?path=/workspace/file.txt
Authorization: Bearer <token>

Response: 204 No Content
```

### Get Disk Usage

```http
GET /api/v1/sessions/{session_id}/disk-usage
Authorization: Bearer <token>

Response: 200 OK
{
  "used_bytes": 104857600,
  "quota_bytes": 1073741824,
  "file_count": 42,
  "percent_used": 9.77
}
```

---

## Process Management API

### List Processes

```http
GET /api/v1/sessions/{session_id}/processes
Authorization: Bearer <token>

Response: 200 OK
{
  "processes": [
    {
      "pid": 1234,
      "command": "sleep 60",
      "status": "running",
      "started_at": "2025-09-29T09:00:00Z",
      "cpu_percent": 0.1,
      "memory_bytes": 2048000
    }
  ]
}
```

### Kill Process

```http
POST /api/v1/sessions/{session_id}/processes/{pid}/signal
Authorization: Bearer <token>
Content-Type: application/json

{
  "signal": "SIGTERM"
}

Response: 200 OK
{
  "pid": 1234,
  "signal": "SIGTERM",
  "sent_at": "2025-09-29T09:30:00Z"
}
```

---

## Configuration API

### Get Configuration

```http
GET /api/v1/config
Authorization: Bearer <token>
X-Admin-Required: true

Response: 200 OK
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "workers": 4
  },
  "session": {
    "timeout_seconds": 1800,
    "max_per_user": 10
  }
}
```

### Update Configuration

```http
PATCH /api/v1/config
Authorization: Bearer <token>
X-Admin-Required: true
Content-Type: application/json

{
  "session": {
    "timeout_seconds": 3600
  }
}

Response: 200 OK
{
  "updated": true,
  "restart_required": true
}
```

---

## Monitoring API

### Health Check

```http
GET /api/v1/health

Response: 200 OK
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "checks": {
    "database": "ok",
    "disk_space": "ok",
    "memory": "ok"
  }
}
```

### Metrics (Prometheus Format)

```http
GET /api/v1/metrics
Authorization: Bearer <token>

Response: 200 OK
Content-Type: text/plain

# HELP web_terminal_sessions_total Total number of sessions
# TYPE web_terminal_sessions_total counter
web_terminal_sessions_total 42

# HELP web_terminal_sessions_active Current active sessions
# TYPE web_terminal_sessions_active gauge
web_terminal_sessions_active 15

# HELP web_terminal_commands_total Total commands executed
# TYPE web_terminal_commands_total counter
web_terminal_commands_total{status="success"} 1234
web_terminal_commands_total{status="error"} 56
```

### Server Statistics

```http
GET /api/v1/stats
Authorization: Bearer <token>

Response: 200 OK
{
  "sessions": {
    "total": 42,
    "active": 15,
    "idle": 5
  },
  "commands": {
    "total": 1290,
    "success": 1234,
    "error": 56
  },
  "resources": {
    "cpu_percent": 25.5,
    "memory_used_bytes": 2147483648,
    "disk_used_bytes": 10737418240
  }
}
```

---

## Users API (Admin Only)

### List Users

```http
GET /api/v1/users?limit=20&offset=0
Authorization: Bearer <token>
X-Admin-Required: true

Response: 200 OK
{
  "users": [
    {
      "id": "user123",
      "username": "alice",
      "email": "alice@example.com",
      "role": "user",
      "created_at": "2025-09-01T00:00:00Z",
      "last_login": "2025-09-29T09:00:00Z"
    }
  ],
  "total": 5,
  "limit": 20,
  "offset": 0
}
```

### Create User

```http
POST /api/v1/users
Authorization: Bearer <token>
X-Admin-Required: true
Content-Type: application/json

{
  "username": "bob",
  "email": "bob@example.com",
  "password": "secure-password",
  "role": "user"
}

Response: 201 Created
{
  "id": "user456",
  "username": "bob",
  "email": "bob@example.com",
  "role": "user",
  "created_at": "2025-09-29T10:00:00Z"
}
```

### Update User

```http
PATCH /api/v1/users/{user_id}
Authorization: Bearer <token>
X-Admin-Required: true
Content-Type: application/json

{
  "role": "admin"
}

Response: 200 OK
{
  "id": "user456",
  "username": "bob",
  "role": "admin",
  "updated_at": "2025-09-29T10:05:00Z"
}
```

### Delete User

```http
DELETE /api/v1/users/{user_id}
Authorization: Bearer <token>
X-Admin-Required: true

Response: 204 No Content
```

---

## Error Responses

### Standard Error Format

```json
{
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "Session with ID 'session123' not found",
    "details": {
      "session_id": "session123"
    }
  }
}
```

### HTTP Status Codes

| Status | Meaning | Use Case |
|--------|---------|----------|
| 200 | OK | Successful GET/PATCH/POST |
| 201 | Created | Resource created |
| 204 | No Content | Successful DELETE |
| 400 | Bad Request | Invalid input |
| 401 | Unauthorized | Missing/invalid auth token |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Resource conflict (e.g., duplicate) |
| 422 | Unprocessable Entity | Validation error |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | Server overloaded |

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_REQUEST` | 400 | Malformed request |
| `UNAUTHORIZED` | 401 | Authentication required |
| `JWT_INVALID` | 401 | JWT token is invalid or malformed |
| `JWT_EXPIRED` | 401 | JWT token has expired |
| `JWT_SIGNATURE_INVALID` | 401 | JWT signature verification failed |
| `INVALID_CREDENTIALS` | 401 | Invalid username or password |
| `FORBIDDEN` | 403 | Permission denied |
| `UNAUTHORIZED_USER` | 403 | User not in allowed users list |
| `UNAUTHORIZED_GROUP` | 403 | User not in any allowed groups |
| `NOT_FOUND` | 404 | Resource not found |
| `SESSION_NOT_FOUND` | 404 | Session doesn't exist |
| `USER_NOT_FOUND` | 404 | User doesn't exist |
| `VALIDATION_ERROR` | 422 | Input validation failed |
| `SESSION_LIMIT_EXCEEDED` | 429 | Too many sessions |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |
| `JWKS_UNAVAILABLE` | 503 | JWKS endpoint unreachable |

---

## Security Headers

### WWW-Authenticate Challenge

When authentication fails (401 Unauthorized), the server includes a `WWW-Authenticate` header:

```http
WWW-Authenticate: Bearer realm="web-terminal", error="invalid_token", error_description="JWT token has expired"
```

**Error Types:**
- `invalid_token`: Token is malformed, expired, or signature invalid
- `invalid_request`: Missing or malformed Authorization header
- `insufficient_scope`: Token lacks required permissions

### Token Expiration Warnings

Responses include expiration information in custom headers when token is near expiry:

```http
X-Token-Expires-In: 300
X-Token-Expires-At: 2025-09-29T10:00:00Z
X-Token-Refresh-Recommended: true
```

**Headers:**
- `X-Token-Expires-In`: Seconds until token expiry
- `X-Token-Expires-At`: ISO 8601 timestamp of expiry
- `X-Token-Refresh-Recommended`: Present when < 5 minutes remain

### CORS Headers

For cross-origin requests, the server includes appropriate CORS headers:

```http
Access-Control-Allow-Origin: https://backstage.example.com
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, PATCH
Access-Control-Allow-Headers: Authorization, Content-Type
Access-Control-Max-Age: 3600
Access-Control-Allow-Credentials: true
```

### Security Best Practices

**Token Handling:**
- Always use HTTPS in production
- Never log or expose tokens in error messages
- Rotate JWKS keys regularly (recommended: 90 days)
- Implement token revocation for compromised tokens
- Set reasonable token expiration (recommended: 1 hour access, 7 days refresh)

**Authorization:**
- Validate token signature before processing claims
- Check `exp`, `nbf`, and `iat` claims
- Verify `iss` matches expected issuer
- Validate `aud` if used
- Cache JWKS keys but refresh periodically

---

## Rate Limiting

### Headers

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1633024800
```

### Limits

| Endpoint Pattern | Limit | Window |
|-----------------|-------|--------|
| `/api/v1/auth/login` | 5 requests | 1 minute |
| `/api/v1/sessions` (POST) | 10 requests | 1 minute |
| `/api/v1/files` (POST) | 20 requests | 1 minute |
| `/api/v1/*` (all others) | 100 requests | 1 minute |

---

## Versioning

API versions are specified in the URL path:

```
/api/v1/sessions
/api/v2/sessions  (future)
```

---

## OpenAPI Specification

Full OpenAPI 3.0 specification available at:

```
GET /api/v1/openapi.json
```

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.1.0 | 2025-09-29 | System Architecture Designer | Added JWT authentication with JWKS support, Backstage integration, authorization flows, security headers, and JWT-specific error codes |
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial API specification |