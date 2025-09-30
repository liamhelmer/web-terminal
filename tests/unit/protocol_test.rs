// Unit tests for Protocol Messages
// Per spec-kit/008-testing-spec.md - Unit Tests
// Per spec-kit/007-websocket-spec.md - WebSocket Protocol

use serde_json;
use web_terminal::protocol::{ClientMessage, ConnectionStatus, ServerMessage, Signal};

/// Test ClientMessage::Command serialization
///
/// Per FR-1.1: Command execution
#[test]
fn test_client_command_serialization() {
    // Arrange
    let msg = ClientMessage::Command {
        data: "ls -la".to_string(),
    };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"command""#));
    assert!(json.contains(r#""data":"ls -la""#));
    match parsed {
        ClientMessage::Command { data } => assert_eq!(data, "ls -la"),
        _ => panic!("Wrong message type"),
    }
}

/// Test ClientMessage::Resize serialization
///
/// Per FR-2.1.5: Support terminal dimensions
#[test]
fn test_client_resize_serialization() {
    // Arrange
    let msg = ClientMessage::Resize {
        cols: 120,
        rows: 40,
    };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"resize""#));
    assert!(json.contains(r#""cols":120"#));
    assert!(json.contains(r#""rows":40"#));
    match parsed {
        ClientMessage::Resize { cols, rows } => {
            assert_eq!(cols, 120);
            assert_eq!(rows, 40);
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test ClientMessage::Signal serialization
///
/// Per FR-1.2.4: Support process termination
#[test]
fn test_client_signal_serialization() {
    // Arrange
    let msg = ClientMessage::Signal {
        signal: Signal::SIGINT,
    };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"signal""#));
    match parsed {
        ClientMessage::Signal { signal } => {
            assert_eq!(signal as i32, Signal::SIGINT as i32);
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test ClientMessage::Ping serialization
#[test]
fn test_client_ping_serialization() {
    // Arrange
    let msg = ClientMessage::Ping;

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"ping""#));
    match parsed {
        ClientMessage::Ping => (),
        _ => panic!("Wrong message type"),
    }
}

/// Test ServerMessage::Output serialization
///
/// Per FR-3.3: Real-time streaming
#[test]
fn test_server_output_serialization() {
    // Arrange
    let msg = ServerMessage::Output {
        data: "Hello World\n".to_string(),
    };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"output""#));
    assert!(json.contains(r#""data":"Hello World\n""#));
    match parsed {
        ServerMessage::Output { data } => assert_eq!(data, "Hello World\n"),
        _ => panic!("Wrong message type"),
    }
}

/// Test ServerMessage::Error serialization
#[test]
fn test_server_error_serialization() {
    // Arrange
    let msg = ServerMessage::Error {
        message: "Command not found".to_string(),
    };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"error""#));
    assert!(json.contains(r#""message":"Command not found""#));
    match parsed {
        ServerMessage::Error { message } => assert_eq!(message, "Command not found"),
        _ => panic!("Wrong message type"),
    }
}

/// Test ServerMessage::ProcessExited serialization
///
/// Per FR-1.2.2: Monitor running processes
#[test]
fn test_server_process_exited_serialization() {
    // Arrange
    let msg = ServerMessage::ProcessExited { exit_code: 0 };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"process_exited""#));
    assert!(json.contains(r#""exit_code":0"#));
    match parsed {
        ServerMessage::ProcessExited { exit_code } => assert_eq!(exit_code, 0),
        _ => panic!("Wrong message type"),
    }
}

/// Test ServerMessage::ConnectionStatus serialization
#[test]
fn test_server_connection_status_serialization() {
    // Arrange
    let msg = ServerMessage::ConnectionStatus {
        status: ConnectionStatus::Connected,
    };

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"connection_status""#));
    assert!(json.contains(r#""status":"connected""#));
    match parsed {
        ServerMessage::ConnectionStatus { status } => {
            assert!(matches!(status, ConnectionStatus::Connected));
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test ServerMessage::Pong serialization
#[test]
fn test_server_pong_serialization() {
    // Arrange
    let msg = ServerMessage::Pong;

    // Act
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert!(json.contains(r#""type":"pong""#));
    match parsed {
        ServerMessage::Pong => (),
        _ => panic!("Wrong message type"),
    }
}

/// Test Signal enum values
#[test]
fn test_signal_values() {
    assert_eq!(Signal::SIGINT as i32, 2);
    assert_eq!(Signal::SIGTERM as i32, 15);
    assert_eq!(Signal::SIGKILL as i32, 9);
}

/// Test ConnectionStatus enum serialization
#[test]
fn test_connection_status_all_variants() {
    let statuses = vec![
        ConnectionStatus::Connected,
        ConnectionStatus::Disconnected,
        ConnectionStatus::Reconnecting,
    ];

    for status in statuses {
        let json = serde_json::to_value(&status).expect("Failed to serialize");
        let parsed: ConnectionStatus = serde_json::from_value(json).expect("Failed to deserialize");
        // Verify round-trip serialization works
        match (status, parsed) {
            (ConnectionStatus::Connected, ConnectionStatus::Connected) => (),
            (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected) => (),
            (ConnectionStatus::Reconnecting, ConnectionStatus::Reconnecting) => (),
            _ => panic!("Round-trip serialization failed"),
        }
    }
}

/// Test malformed JSON handling
#[test]
fn test_invalid_json_deserialization() {
    let invalid_json = r#"{"type":"invalid"}"#;
    let result: Result<ClientMessage, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}

/// Test missing required field
#[test]
fn test_missing_field_deserialization() {
    let invalid_json = r#"{"type":"command"}"#;
    let result: Result<ClientMessage, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}

/// Test empty command data
#[test]
fn test_empty_command_data() {
    let msg = ClientMessage::Command {
        data: String::new(),
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        ClientMessage::Command { data } => assert_eq!(data, ""),
        _ => panic!("Wrong message type"),
    }
}

/// Test special characters in messages
#[test]
fn test_special_characters_in_output() {
    let msg = ServerMessage::Output {
        data: "Hello\nWorld\r\n\tTab\x1b[31mRed\x1b[0m".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        ServerMessage::Output { data } => {
            assert!(data.contains("\n"));
            assert!(data.contains("\t"));
            assert!(data.contains("\x1b[31m"));
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test Unicode characters in messages
#[test]
fn test_unicode_in_messages() {
    let msg = ClientMessage::Command {
        data: "echo '你好世界 🌍'".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        ClientMessage::Command { data } => {
            assert!(data.contains("你好世界"));
            assert!(data.contains("🌍"));
        }
        _ => panic!("Wrong message type"),
    }
}

/// Test all signal types
#[test]
fn test_all_signal_types() {
    let signals = vec![
        (Signal::SIGINT, "SIGINT"),
        (Signal::SIGTERM, "SIGTERM"),
        (Signal::SIGKILL, "SIGKILL"),
    ];

    for (signal, name) in signals {
        let msg = ClientMessage::Signal { signal };
        let json = serde_json::to_string(&msg).expect(&format!("Failed to serialize {}", name));
        let parsed: ClientMessage =
            serde_json::from_str(&json).expect(&format!("Failed to deserialize {}", name));

        match parsed {
            ClientMessage::Signal { signal: s } => assert_eq!(s as i32, signal as i32),
            _ => panic!("Wrong message type for {}", name),
        }
    }
}

/// Test large output message
#[test]
fn test_large_output_message() {
    let large_data = "A".repeat(1024 * 1024); // 1MB of data
    let msg = ServerMessage::Output {
        data: large_data.clone(),
    };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        ServerMessage::Output { data } => assert_eq!(data.len(), large_data.len()),
        _ => panic!("Wrong message type"),
    }
}

/// Test negative exit code
#[test]
fn test_negative_exit_code() {
    let msg = ServerMessage::ProcessExited { exit_code: -1 };
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let parsed: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

    match parsed {
        ServerMessage::ProcessExited { exit_code } => assert_eq!(exit_code, -1),
        _ => panic!("Wrong message type"),
    }
}
