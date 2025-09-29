// Protocol message types for WebSocket communication
// Per spec-kit/003-backend-spec.md section "Data Models"
// Per spec-kit/007-websocket-spec.md

use serde::{Deserialize, Serialize};

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Execute a command in the terminal
    /// Per FR-1.1: Command execution
    Command { data: String },

    /// Resize the terminal
    /// Per FR-2.1.5: Support terminal dimensions
    Resize { cols: u16, rows: u16 },

    /// Send a signal to the process
    /// Per FR-1.2.4: Support process termination
    Signal { signal: Signal },

    /// Ping message for heartbeat
    Ping,
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Terminal output data
    /// Per FR-3.3: Real-time streaming
    Output { data: String },

    /// Error message
    Error { message: String },

    /// Process exited with code
    /// Per FR-1.2.2: Monitor running processes
    ProcessExited { exit_code: i32 },

    /// Connection status update
    ConnectionStatus { status: ConnectionStatus },

    /// Pong response to ping
    Pong,
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