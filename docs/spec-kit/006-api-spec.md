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

### Bearer Token Authentication

All API requests require a valid JWT token in the Authorization header:

```http
Authorization: Bearer <token>
```

### Obtain Token

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
  "expires_at": "2025-09-29T10:00:00Z",
  "user_id": "user123"
}
```

### Refresh Token

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "eyJhbGc..."
}

Response: 200 OK
{
  "access_token": "eyJhbGc...",
  "expires_at": "2025-09-29T10:00:00Z"
}
```

---

## Sessions API

### Create Session

```http
POST /api/v1/sessions
Authorization: Bearer <token>
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
  "user_id": "user123",
  "created_at": "2025-09-29T09:00:00Z",
  "state": {
    "working_dir": "/workspace",
    "environment": {},
    "processes": []
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
| `FORBIDDEN` | 403 | Permission denied |
| `NOT_FOUND` | 404 | Resource not found |
| `SESSION_NOT_FOUND` | 404 | Session doesn't exist |
| `USER_NOT_FOUND` | 404 | User doesn't exist |
| `VALIDATION_ERROR` | 422 | Input validation failed |
| `SESSION_LIMIT_EXCEEDED` | 429 | Too many sessions |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |

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
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial API specification |