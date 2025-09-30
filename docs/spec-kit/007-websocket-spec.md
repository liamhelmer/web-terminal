# Web-Terminal: WebSocket Protocol Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

The WebSocket protocol enables real-time bidirectional communication between the browser client and Rust backend for terminal I/O streaming.

**WebSocket URL:** `/ws` (relative path)

**Protocol:** Automatically detected based on page protocol:
- `ws://` when page loaded via `http://`
- `wss://` when page loaded via `https://`

**Note:** WebSocket connections use the same host and port as the HTTP server. The frontend constructs the full URL dynamically from `window.location`.

---

## Connection Lifecycle

### 1. Connection Establishment

```
Client                                    Server
  |                                         |
  |--- HTTP GET /ws ------------------------>|
  |<-- 101 Switching Protocols ------------|
  |                                         |
  |<-- ServerMessage::Connected ------------|
  |--- ClientMessage::Authenticate -------->|
  |    { token: "eyJhbGc..." }              |
  |                                         |
  |      [JWT Validation]                   |
  |      - Verify signature via JWKS        |
  |      - Check expiration                 |
  |      - Verify issuer                    |
  |                                         |
  |<-- ServerMessage::Authenticated --------|
  |    { user_id: "user:default/john" }     |
  |                                         |
```

**Authentication Flow:**

1. Client establishes WebSocket connection (no token in URL for security)
2. Server sends `Connected` message
3. Client sends `Authenticate` message with JWT token
4. Server validates JWT using JWKS public keys
5. Server sends `Authenticated` message on success
6. All subsequent messages require authentication

#### Connection Request

```http
GET /ws HTTP/1.1
Host: <server-host>
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
```

**Note:** `Host` header is set automatically by the browser based on the current page's origin.

**Security Note:** JWT token is sent in the first WebSocket message, not in the URL, to prevent token leakage in logs.

#### Connection Response

```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
```

---

### 2. Active Communication

```
Client                                    Server
  |                                         |
  |--- ClientMessage::Command ------------>|
  |<-- ServerMessage::Output ---------------|
  |<-- ServerMessage::Output ---------------|
  |<-- ServerMessage::ProcessExited --------|
  |                                         |
  |--- ClientMessage::Resize -------------->|
  |<-- ServerMessage::Ack ------------------|
  |                                         |
```

---

### 3. Heartbeat Mechanism

```
Client                                    Server
  |                                         |
  |<-- Ping (every 5s) --------------------|
  |--- Pong ----------------------------->|
  |                                         |
```

**Timeout:** Connection closed if no Pong received within 30 seconds.

---

### 4. Connection Termination

```
Client                                    Server
  |                                         |
  |--- Close(1000, "Normal") ------------->|
  |<-- Close(1000, "Goodbye") --------------|
  |                                         |
  [Connection Closed]
```

---

## Message Protocol

### Message Format

All text messages are JSON-encoded with a `type` field:

```json
{
  "type": "command",
  "data": "ls -la"
}
```

Binary messages are used for file transfers (see File Transfer Protocol).

---

## Client Messages

### 1. Authenticate

**Type:** `authenticate`

**Description:** Authenticate WebSocket connection with JWT token

```json
{
  "type": "authenticate",
  "token": "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3QifQ..."
}
```

**Fields:**
- `type`: Always `"authenticate"`
- `token`: JWT token obtained from Identity Provider (IdP)

**Timing:**
- Must be sent immediately after connection establishment
- All other messages require authentication first
- If not sent within 30 seconds, connection is closed

**Response:**
- Success: `ServerMessage::Authenticated`
- Failure: `ServerMessage::Error` + connection closed (4000)

---

### 2. Command Execution

**Type:** `command`

**Description:** Execute a shell command

```json
{
  "type": "command",
  "data": "ls -la /workspace"
}
```

**Fields:**
- `type`: Always `"command"`
- `data`: Command string to execute

**Authentication:** Required ✅

---

### 2. Terminal Resize

**Type:** `resize`

**Description:** Notify server of terminal size change

```json
{
  "type": "resize",
  "cols": 120,
  "rows": 40
}
```

**Fields:**
- `type`: Always `"resize"`
- `cols`: Terminal width in columns (1-500)
- `rows`: Terminal height in rows (1-200)

---

### 3. Send Signal

**Type:** `signal`

**Description:** Send signal to running process

```json
{
  "type": "signal",
  "signal": "SIGINT"
}
```

**Fields:**
- `type`: Always `"signal"`
- `signal`: Signal name (`"SIGINT"`, `"SIGTERM"`, `"SIGKILL"`)

---

### 4. File Upload Start

**Type:** `file_upload_start`

**Description:** Initiate file upload

```json
{
  "type": "file_upload_start",
  "path": "/workspace/file.txt",
  "size": 2048,
  "checksum": "sha256:abc123..."
}
```

**Fields:**
- `type`: Always `"file_upload_start"`
- `path`: Destination path (relative to workspace)
- `size`: File size in bytes
- `checksum`: SHA-256 checksum for verification

---

### 5. File Upload Chunk

**Type:** Binary message

**Description:** Send file data chunk

Binary format:
```
[chunk_id: u32][data: bytes]
```

---

### 6. File Upload Complete

**Type:** `file_upload_complete`

**Description:** Signal upload completion

```json
{
  "type": "file_upload_complete",
  "chunk_count": 10
}
```

---

### 7. File Download Request

**Type:** `file_download`

**Description:** Request file download

```json
{
  "type": "file_download",
  "path": "/workspace/file.txt"
}
```

**Fields:**
- `type`: Always `"file_download"`
- `path`: File path to download

---

### 8. Environment Variable Set

**Type:** `env_set`

**Description:** Set environment variable

```json
{
  "type": "env_set",
  "key": "MY_VAR",
  "value": "my_value"
}
```

**Fields:**
- `type`: Always `"env_set"`
- `key`: Environment variable name
- `value`: Environment variable value

---

### 9. Change Directory

**Type:** `chdir`

**Description:** Change working directory

```json
{
  "type": "chdir",
  "path": "/workspace/project"
}
```

**Fields:**
- `type`: Always `"chdir"`
- `path`: New working directory path

---

## Server Messages

### 1. Authenticated

**Type:** `authenticated`

**Description:** Authentication successful

```json
{
  "type": "authenticated",
  "user_id": "user:default/john.doe",
  "email": "john.doe@example.com",
  "groups": ["group:default/platform-team"]
}
```

**Fields:**
- `type`: Always `"authenticated"`
- `user_id`: User identity (Backstage entity reference or `sub` claim)
- `email`: User email address (optional)
- `groups`: User groups/roles (optional)

**Sent:** In response to successful `ClientMessage::Authenticate`

---

### 2. Output

**Type:** `output`

**Description:** Command output (stdout)

```json
{
  "type": "output",
  "stream": "stdout",
  "data": "file1.txt\nfile2.txt\n"
}
```

**Fields:**
- `type`: Always `"output"`
- `stream`: `"stdout"` or `"stderr"`
- `data`: Output text

---

### 2. Error

**Type:** `error`

**Description:** Error message

```json
{
  "type": "error",
  "code": "COMMAND_NOT_FOUND",
  "message": "Command 'invalid' not found",
  "details": {}
}
```

**Fields:**
- `type`: Always `"error"`
- `code`: Error code (see Error Codes section)
- `message`: Human-readable error message
- `details`: Additional error context (optional)

---

### 3. Process Started

**Type:** `process_started`

**Description:** Process execution started

```json
{
  "type": "process_started",
  "pid": 1234,
  "command": "sleep 60"
}
```

**Fields:**
- `type`: Always `"process_started"`
- `pid`: Process ID
- `command`: Command being executed

---

### 4. Process Exited

**Type:** `process_exited`

**Description:** Process execution completed

```json
{
  "type": "process_exited",
  "pid": 1234,
  "exit_code": 0,
  "signal": null
}
```

**Fields:**
- `type`: Always `"process_exited"`
- `pid`: Process ID
- `exit_code`: Exit code (0 = success, non-zero = error)
- `signal`: Signal that terminated process (if applicable)

---

### 5. Connection Status

**Type:** `connection_status`

**Description:** Connection state change

```json
{
  "type": "connection_status",
  "status": "connected",
  "session_id": "session123"
}
```

**Fields:**
- `type`: Always `"connection_status"`
- `status`: `"connected"`, `"reconnecting"`, `"disconnected"`
- `session_id`: Session identifier

---

### 6. Working Directory Changed

**Type:** `cwd_changed`

**Description:** Working directory updated

```json
{
  "type": "cwd_changed",
  "path": "/workspace/project"
}
```

**Fields:**
- `type`: Always `"cwd_changed"`
- `path`: New working directory path

---

### 7. Environment Variable Set

**Type:** `env_updated`

**Description:** Environment variable updated

```json
{
  "type": "env_updated",
  "key": "MY_VAR",
  "value": "new_value"
}
```

---

### 8. File Download Start

**Type:** `file_download_start`

**Description:** File download beginning

```json
{
  "type": "file_download_start",
  "path": "/workspace/file.txt",
  "size": 2048,
  "checksum": "sha256:abc123...",
  "chunk_size": 8192
}
```

**Fields:**
- `type`: Always `"file_download_start"`
- `path`: File path
- `size`: Total file size in bytes
- `checksum`: SHA-256 checksum
- `chunk_size`: Size of each chunk

---

### 9. File Download Chunk

**Type:** Binary message

**Description:** File data chunk

Binary format:
```
[chunk_id: u32][data: bytes]
```

---

### 10. File Download Complete

**Type:** `file_download_complete`

**Description:** File download finished

```json
{
  "type": "file_download_complete",
  "chunk_count": 10
}
```

---

### 11. Resource Usage

**Type:** `resource_usage`

**Description:** Session resource usage update

```json
{
  "type": "resource_usage",
  "cpu_percent": 25.5,
  "memory_bytes": 104857600,
  "disk_bytes": 524288000
}
```

**Fields:**
- `type`: Always `"resource_usage"`
- `cpu_percent`: CPU usage percentage (0-100)
- `memory_bytes`: Memory usage in bytes
- `disk_bytes`: Disk usage in bytes

---

### 12. Acknowledgment

**Type:** `ack`

**Description:** Acknowledge client message

```json
{
  "type": "ack",
  "message_id": "msg123"
}
```

---

## Error Codes

| Code | Description |
|------|-------------|
| `COMMAND_NOT_FOUND` | Command executable not found |
| `COMMAND_FAILED` | Command execution failed |
| `COMMAND_TIMEOUT` | Command exceeded timeout |
| `COMMAND_KILLED` | Command was killed by signal |
| `PERMISSION_DENIED` | Insufficient permissions |
| `PATH_NOT_FOUND` | File or directory not found |
| `PATH_INVALID` | Invalid or restricted path |
| `SESSION_EXPIRED` | Session has expired |
| `RESOURCE_LIMIT` | Resource limit exceeded |
| `QUOTA_EXCEEDED` | Storage quota exceeded |
| `INVALID_MESSAGE` | Malformed message |
| `INTERNAL_ERROR` | Server internal error |

---

## Close Codes

| Code | Name | Description |
|------|------|-------------|
| 1000 | Normal Closure | Normal connection close |
| 1001 | Going Away | Server shutting down |
| 1002 | Protocol Error | Protocol violation |
| 1003 | Unsupported Data | Invalid message type |
| 1008 | Policy Violation | Security policy violation |
| 1011 | Internal Error | Server error |
| 4000 | Authentication Failed | Invalid token |
| 4001 | Session Expired | Session timeout |
| 4002 | Rate Limit | Too many messages |

---

## Flow Control and Backpressure

### Client-Side Backpressure

If server cannot process messages fast enough:

```json
{
  "type": "flow_control",
  "action": "pause"
}
```

Client should pause sending until:

```json
{
  "type": "flow_control",
  "action": "resume"
}
```

### Message Buffering

- Client buffers messages when disconnected
- Maximum buffer size: 1000 messages
- Messages older than 5 minutes are discarded
- Buffered messages replayed on reconnection

---

## File Transfer Protocol

### Upload Flow

```
Client                                    Server
  |                                         |
  |--- file_upload_start ----------------->|
  |<-- ack --------------------------------|
  |--- binary chunk 0 -------------------->|
  |--- binary chunk 1 -------------------->|
  |--- binary chunk N -------------------->|
  |--- file_upload_complete -------------->|
  |<-- ack --------------------------------|
  |                                         |
```

### Download Flow

```
Client                                    Server
  |                                         |
  |--- file_download ---------------------->|
  |<-- file_download_start -----------------|
  |<-- binary chunk 0 ----------------------|
  |<-- binary chunk 1 ----------------------|
  |<-- binary chunk N ----------------------|
  |<-- file_download_complete --------------|
  |--- ack -------------------------------->|
  |                                         |
```

### Chunk Size

- Default: 8192 bytes (8 KB)
- Maximum: 65536 bytes (64 KB)
- Configurable per transfer

---

## Security Considerations

### 1. Authentication

**JWT-Based Authentication:**

- JWT token sent in `Authenticate` message (not URL)
- Token validated using JWKS public keys from IdP
- Signature verification (RS256/RS384/RS512/ES256/ES384)
- Standard claims validated (exp, nbf, iss, aud)
- Clock skew tolerance: 60 seconds
- Token expiration strictly enforced

**Authentication Flow:**

1. WebSocket connection established
2. Server sends `Connected` message
3. Client sends `Authenticate` message with JWT
4. Server validates JWT:
   - Fetches JWKS keys from provider (cached)
   - Verifies signature using public key
   - Checks expiration, issuer, audience
5. Server extracts user identity from claims
6. Server attaches UserContext to connection
7. All subsequent messages require authentication

**Failure Handling:**

- Invalid token: `Error` message + close code 4000
- Expired token: `Error` message + close code 4001
- Missing token: Connection closed after 30 seconds
- Wrong issuer: `Error` message + close code 4000

### 2. Authorization

**Resource Access Control:**

- Every message requires authenticated user
- Session operations check resource ownership
- Admin users can access all sessions
- Regular users can only access own sessions
- Group-based permissions supported

**Permission Checks:**

- `command`: User must own session
- `resize`: User must own session
- `signal`: User must own session
- `file_upload`: User must own session
- `file_download`: User must own session

### 3. Message Validation

- All messages validated against schema
- Unknown message types rejected
- Size limits enforced (max 1 MB per message)
- Path traversal attempts blocked
- Command injection prevention

### 4. Rate Limiting

- Maximum 100 messages/second per connection
- Excess messages trigger throttling
- Burst allowance: 20 messages
- Persistent violations result in disconnect (close code 4002)
- Rate limit status in `flow_control` messages

### 4. Command Execution

- Command whitelist/blacklist applied
- Path traversal attempts blocked
- Resource limits enforced

---

## Performance Characteristics

### Latency

- Message latency: <20ms (p95)
- Command execution start: <50ms (p95)
- First output byte: <100ms (p95)

### Throughput

- Messages: 1000+ per second per connection
- Output streaming: 10 MB/s per session
- File transfer: 50 MB/s per transfer

### Concurrency

- 10,000+ concurrent WebSocket connections per server
- 100,000+ messages/second server-wide

---

## Testing Protocol

### Echo Test

```json
// Client sends
{
  "type": "echo",
  "data": "test message"
}

// Server responds
{
  "type": "echo",
  "data": "test message"
}
```

### Latency Test

```json
// Client sends
{
  "type": "ping",
  "timestamp": 1633024800000
}

// Server responds
{
  "type": "pong",
  "timestamp": 1633024800000,
  "latency_ms": 15
}
```

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial WebSocket protocol specification |