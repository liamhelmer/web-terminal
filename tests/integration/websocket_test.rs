// Integration tests for WebSocket communication
// Per spec-kit/008-testing-spec.md - Integration Tests
//
// Tests WebSocket message protocol and real-time communication

use std::time::Duration;
use web_terminal::protocol::{ClientMessage, ConnectionStatus, ServerMessage, Signal};

/// Test message serialization and deserialization
///
/// Per FR-3: Real-time Communication via WebSocket
/// Per spec-kit/007-websocket-spec.md
#[tokio::test]
async fn test_message_serialization() {
    // Test ClientMessage serialization
    let command_msg = ClientMessage::Command {
        data: "echo 'test'".to_string(),
    };

    let json = serde_json::to_string(&command_msg).expect("Failed to serialize command message");
    assert!(json.contains(r#""type":"command""#));
    assert!(json.contains(r#""data":"echo 'test'""#));

    // Test deserialization
    let parsed: ClientMessage =
        serde_json::from_str(&json).expect("Failed to deserialize command message");
    match parsed {
        ClientMessage::Command { data } => assert_eq!(data, "echo 'test'"),
        _ => panic!("Wrong message type"),
    }

    // Test ServerMessage serialization
    let output_msg = ServerMessage::Output {
        data: "Hello World\n".to_string(),
    };

    let json = serde_json::to_string(&output_msg).expect("Failed to serialize output message");
    assert!(json.contains(r#""type":"output""#));
    assert!(json.contains(r#""data":"Hello World\n""#));

    // Test deserialization
    let parsed: ServerMessage =
        serde_json::from_str(&json).expect("Failed to deserialize output message");
    match parsed {
        ServerMessage::Output { data } => assert_eq!(data, "Hello World\n"),
        _ => panic!("Wrong message type"),
    }
}

/// Test resize message format
///
/// Per FR-2.1.5: Support terminal dimensions
#[tokio::test]
async fn test_resize_message() {
    let resize_msg = ClientMessage::Resize { cols: 80, rows: 24 };

    let json = serde_json::to_string(&resize_msg).expect("Failed to serialize resize message");
    assert!(json.contains(r#""type":"resize""#));
    assert!(json.contains(r#""cols":80"#));
    assert!(json.contains(r#""rows":24"#));

    // Test deserialization
    let parsed: ClientMessage =
        serde_json::from_str(&json).expect("Failed to deserialize resize message");
    match parsed {
        ClientMessage::Resize { cols, rows } => {
            assert_eq!(cols, 80);
            assert_eq!(rows, 24);
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test signal message format
///
/// Per FR-1.2.4: Support process termination
#[tokio::test]
async fn test_signal_message() {
    let signal_msg = ClientMessage::Signal {
        signal: Signal::SIGINT,
    };

    let json = serde_json::to_string(&signal_msg).expect("Failed to serialize signal message");
    assert!(json.contains(r#""type":"signal""#));

    // Test all signal types
    let signals = vec![Signal::SIGINT, Signal::SIGTERM, Signal::SIGKILL];

    for signal in signals {
        let msg = ClientMessage::Signal { signal };
        let json = serde_json::to_string(&msg).expect("Failed to serialize signal");
        let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match parsed {
            ClientMessage::Signal { signal: s } => {
                assert_eq!(s as i32, signal as i32);
            }
            _ => panic!("Wrong message type"),
        }
    }
}

/// Test error message format
///
/// Per error handling requirements
#[tokio::test]
async fn test_error_message() {
    let error_msg = ServerMessage::Error {
        message: "Command not found".to_string(),
    };

    let json = serde_json::to_string(&error_msg).expect("Failed to serialize error message");
    assert!(json.contains(r#""type":"error""#));
    assert!(json.contains(r#""message":"Command not found""#));

    let parsed: ServerMessage =
        serde_json::from_str(&json).expect("Failed to deserialize error message");
    match parsed {
        ServerMessage::Error { message } => assert_eq!(message, "Command not found"),
        _ => panic!("Wrong message type"),
    }
}

/// Test process exited message format
///
/// Per FR-1.2.2: Monitor running processes
#[tokio::test]
async fn test_process_exited_message() {
    let exited_msg = ServerMessage::ProcessExited { exit_code: 0 };

    let json = serde_json::to_string(&exited_msg).expect("Failed to serialize exited message");
    assert!(json.contains(r#""type":"process_exited""#));
    assert!(json.contains(r#""exit_code":0"#));

    // Test with error exit code
    let exited_msg = ServerMessage::ProcessExited { exit_code: 127 };

    let json = serde_json::to_string(&exited_msg).expect("Failed to serialize exited message");
    let parsed: ServerMessage =
        serde_json::from_str(&json).expect("Failed to deserialize exited message");

    match parsed {
        ServerMessage::ProcessExited { exit_code } => assert_eq!(exit_code, 127),
        _ => panic!("Wrong message type"),
    }
}

/// Test ping/pong message format
///
/// Per FR-3.4: Maintain connection with heartbeat
#[tokio::test]
async fn test_ping_pong_messages() {
    // Test Ping message
    let ping_msg = ClientMessage::Ping;
    let json = serde_json::to_string(&ping_msg).expect("Failed to serialize ping message");
    assert!(json.contains(r#""type":"ping""#));

    let parsed: ClientMessage =
        serde_json::from_str(&json).expect("Failed to deserialize ping message");
    matches!(parsed, ClientMessage::Ping);

    // Test Pong message
    let pong_msg = ServerMessage::Pong;
    let json = serde_json::to_string(&pong_msg).expect("Failed to serialize pong message");
    assert!(json.contains(r#""type":"pong""#));

    let parsed: ServerMessage =
        serde_json::from_str(&json).expect("Failed to deserialize pong message");
    matches!(parsed, ServerMessage::Pong);
}

/// Test connection status message format
///
/// Per FR-3: Real-time Communication
#[tokio::test]
async fn test_connection_status_message() {
    let status_msg = ServerMessage::ConnectionStatus {
        status: ConnectionStatus::Connected,
    };

    let json =
        serde_json::to_string(&status_msg).expect("Failed to serialize connection status message");
    assert!(json.contains(r#""type":"connection_status""#));
    assert!(json.contains(r#""status":"connected""#));

    // Test all status types
    let statuses = vec![
        ConnectionStatus::Connected,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Reconnecting,
    ];

    for status in statuses {
        let msg = ServerMessage::ConnectionStatus { status };
        let json = serde_json::to_string(&msg).expect("Failed to serialize status");
        let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match parsed {
            ServerMessage::ConnectionStatus { status: s } => {
                // Verify serialization roundtrip
                assert_eq!(
                    format!("{:?}", s).to_lowercase(),
                    format!("{:?}", status).to_lowercase()
                );
            }
            _ => panic!("Wrong message type"),
        }
    }
}

/// Test invalid message handling
///
/// Per error handling requirements
#[tokio::test]
async fn test_invalid_message_format() {
    // Test invalid JSON
    let invalid_json = "{invalid json}";
    let result = serde_json::from_str::<ClientMessage>(invalid_json);
    assert!(result.is_err(), "Invalid JSON should fail to parse");

    // Test missing required fields
    let missing_fields = r#"{"type":"command"}"#;
    let result = serde_json::from_str::<ClientMessage>(missing_fields);
    assert!(
        result.is_err(),
        "Message missing required fields should fail"
    );

    // Test unknown message type
    let unknown_type = r#"{"type":"unknown_type","data":"test"}"#;
    let result = serde_json::from_str::<ClientMessage>(unknown_type);
    assert!(result.is_err(), "Unknown message type should fail");
}

/// Test message protocol completeness
///
/// Verify all message types can be serialized and deserialized
#[tokio::test]
async fn test_message_protocol_completeness() {
    // Test all ClientMessage variants
    let client_messages = vec![
        ClientMessage::Command {
            data: "test".to_string(),
        },
        ClientMessage::Resize { cols: 80, rows: 24 },
        ClientMessage::Signal {
            signal: Signal::SIGINT,
        },
        ClientMessage::Ping,
    ];

    for msg in client_messages {
        let json = serde_json::to_string(&msg).expect("Failed to serialize client message");
        let _parsed: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize client message");
    }

    // Test all ServerMessage variants
    let server_messages = vec![
        ServerMessage::Output {
            data: "output".to_string(),
        },
        ServerMessage::Error {
            message: "error".to_string(),
        },
        ServerMessage::ProcessExited { exit_code: 0 },
        ServerMessage::ConnectionStatus {
            status: ConnectionStatus::Connected,
        },
        ServerMessage::Pong,
    ];

    for msg in server_messages {
        let json = serde_json::to_string(&msg).expect("Failed to serialize server message");
        let _parsed: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize server message");
    }
}

/// Test message size limits
///
/// Per NFR-1.1.4: Support streaming output up to 10MB
#[tokio::test]
async fn test_large_message_handling() {
    // Test large output message (1MB)
    let large_data = "x".repeat(1024 * 1024);
    let large_msg = ServerMessage::Output {
        data: large_data.clone(),
    };

    let json = serde_json::to_string(&large_msg).expect("Failed to serialize large message");
    let parsed: ServerMessage =
        serde_json::from_str(&json).expect("Failed to deserialize large message");

    match parsed {
        ServerMessage::Output { data } => {
            assert_eq!(data.len(), large_data.len());
            assert_eq!(data, large_data);
        }
        _ => panic!("Wrong message type"),
    }
}
