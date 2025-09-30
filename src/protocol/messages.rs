// Protocol message types for WebSocket communication
// Per spec-kit/003-backend-spec.md section "Data Models"
// Per spec-kit/007-websocket-spec.md

use serde::{Deserialize, Serialize};

/// Maximum message size: 1 MB
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Authenticate with JWT token
    /// Per spec-kit/007-websocket-spec.md: WebSocket authentication
    /// Per spec-kit/011-authentication-spec.md: Authentication flow
    Authenticate { token: String },

    /// Execute a command in the terminal
    /// Per FR-1.1: Command execution
    Command { data: String },

    /// Resize the terminal
    /// Per FR-2.1.5: Support terminal dimensions
    Resize { cols: u16, rows: u16 },

    /// Send a signal to the process
    /// Per FR-1.2.4: Support process termination
    Signal { signal: Signal },

    /// Set environment variable
    /// Per spec-kit/007-websocket-spec.md
    EnvSet { key: String, value: String },

    /// Change working directory
    /// Per spec-kit/007-websocket-spec.md
    Chdir { path: String },

    /// File upload start
    /// Per spec-kit/007-websocket-spec.md: File transfer protocol
    FileUploadStart {
        path: String,
        size: u64,
        checksum: String,
    },

    /// File upload complete
    /// Per spec-kit/007-websocket-spec.md: File transfer protocol
    FileUploadComplete { chunk_count: u32 },

    /// File download request
    /// Per spec-kit/007-websocket-spec.md: File transfer protocol
    FileDownload { path: String },

    /// Ping message for heartbeat
    /// Per spec-kit/007-websocket-spec.md: Heartbeat mechanism
    Ping,

    /// Echo test message
    /// Per spec-kit/007-websocket-spec.md: Testing protocol
    Echo { data: String },
}

impl ClientMessage {
    /// Validate message contents
    /// Per spec-kit/007-websocket-spec.md: Message validation
    pub fn validate(&self) -> Result<(), String> {
        match self {
            ClientMessage::Authenticate { token } => {
                if token.is_empty() || token.len() > 10000 {
                    return Err("Token length must be between 1 and 10000 characters".to_string());
                }
            }
            ClientMessage::Command { data } => {
                if data.is_empty() || data.len() > 65536 {
                    return Err("Command length must be between 1 and 65536 characters".to_string());
                }
            }
            ClientMessage::Resize { cols, rows } => {
                if *cols < 1 || *cols > 500 {
                    return Err("Columns must be between 1 and 500".to_string());
                }
                if *rows < 1 || *rows > 200 {
                    return Err("Rows must be between 1 and 200".to_string());
                }
            }
            ClientMessage::EnvSet { key, value } => {
                if key.is_empty() || key.len() > 256 {
                    return Err("Key length must be between 1 and 256 characters".to_string());
                }
                if value.len() > 4096 {
                    return Err("Value length must be less than 4096 characters".to_string());
                }
            }
            ClientMessage::Chdir { path } | ClientMessage::FileDownload { path } => {
                if path.is_empty() || path.len() > 4096 {
                    return Err("Path length must be between 1 and 4096 characters".to_string());
                }
            }
            ClientMessage::FileUploadStart { path, .. } => {
                if path.is_empty() || path.len() > 4096 {
                    return Err("Path length must be between 1 and 4096 characters".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Authentication successful
    /// Per spec-kit/007-websocket-spec.md: WebSocket authentication
    /// Per spec-kit/011-authentication-spec.md: Authentication flow
    Authenticated {
        user_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        groups: Option<Vec<String>>,
    },

    /// Terminal output data
    /// Per FR-3.3: Real-time streaming
    Output {
        #[serde(skip_serializing_if = "Option::is_none")]
        stream: Option<String>, // "stdout" or "stderr"
        data: String,
    },

    /// Error message with error code
    /// Per spec-kit/007-websocket-spec.md: Error codes
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },

    /// Process started
    /// Per spec-kit/007-websocket-spec.md
    ProcessStarted { pid: u32, command: String },

    /// Process exited with code
    /// Per FR-1.2.2: Monitor running processes
    ProcessExited {
        pid: u32,
        exit_code: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        signal: Option<String>,
    },

    /// Connection status update
    /// Per spec-kit/007-websocket-spec.md
    ConnectionStatus {
        status: ConnectionStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },

    /// Working directory changed
    /// Per spec-kit/007-websocket-spec.md
    CwdChanged { path: String },

    /// Environment variable updated
    /// Per spec-kit/007-websocket-spec.md
    EnvUpdated { key: String, value: String },

    /// File download start
    /// Per spec-kit/007-websocket-spec.md: File transfer protocol
    FileDownloadStart {
        path: String,
        size: u64,
        checksum: String,
        chunk_size: u32,
    },

    /// File download complete
    /// Per spec-kit/007-websocket-spec.md: File transfer protocol
    FileDownloadComplete { chunk_count: u32 },

    /// Resource usage update
    /// Per spec-kit/007-websocket-spec.md
    ResourceUsage {
        cpu_percent: f32,
        memory_bytes: u64,
        disk_bytes: u64,
    },

    /// Acknowledgment of client message
    /// Per spec-kit/007-websocket-spec.md
    Ack {
        #[serde(skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
    },

    /// Flow control message
    /// Per spec-kit/007-websocket-spec.md: Backpressure
    FlowControl { action: FlowControlAction },

    /// Pong response to ping
    /// Per spec-kit/007-websocket-spec.md: Heartbeat
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        latency_ms: Option<u64>,
    },

    /// Echo response
    /// Per spec-kit/007-websocket-spec.md: Testing protocol
    Echo { data: String },
}

/// Signal types for process control
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(i32)]
pub enum Signal {
    /// Interrupt signal (Ctrl+C)
    SIGINT = 2,
    /// Termination signal
    SIGTERM = 15,
    /// Kill signal (non-catchable)
    SIGKILL = 9,
}

/// WebSocket connection status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    /// Successfully connected
    Connected,
    /// Disconnected from server
    Disconnected,
    /// Attempting to reconnect
    Reconnecting,
}

/// Flow control actions for backpressure
/// Per spec-kit/007-websocket-spec.md: Flow control and backpressure
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowControlAction {
    /// Pause sending messages
    Pause,
    /// Resume sending messages
    Resume,
}

/// Error codes for WebSocket errors
/// Per spec-kit/007-websocket-spec.md: Error codes
pub mod error_codes {
    pub const COMMAND_NOT_FOUND: &str = "COMMAND_NOT_FOUND";
    pub const COMMAND_FAILED: &str = "COMMAND_FAILED";
    pub const COMMAND_TIMEOUT: &str = "COMMAND_TIMEOUT";
    pub const COMMAND_KILLED: &str = "COMMAND_KILLED";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const PATH_NOT_FOUND: &str = "PATH_NOT_FOUND";
    pub const PATH_INVALID: &str = "PATH_INVALID";
    pub const SESSION_EXPIRED: &str = "SESSION_EXPIRED";
    pub const RESOURCE_LIMIT: &str = "RESOURCE_LIMIT";
    pub const QUOTA_EXCEEDED: &str = "QUOTA_EXCEEDED";
    pub const INVALID_MESSAGE: &str = "INVALID_MESSAGE";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const AUTHENTICATION_REQUIRED: &str = "AUTHENTICATION_REQUIRED";
    pub const AUTHENTICATION_FAILED: &str = "AUTHENTICATION_FAILED";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Command {
            data: "ls -la".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"command""#));
        assert!(json.contains(r#""data":"ls -la""#));
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Output {
            stream: Some("stdout".to_string()),
            data: "Hello World\n".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"output""#));
        assert!(json.contains(r#""data":"Hello World\n""#));
    }

    #[test]
    fn test_resize_message() {
        let msg = ClientMessage::Resize { cols: 80, rows: 24 };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::Resize { cols, rows } => {
                assert_eq!(cols, 80);
                assert_eq!(rows, 24);
            }
            _ => panic!("Wrong message type"),
        }
    }
}