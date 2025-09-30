// Unit tests for Error Handling
// Per spec-kit/008-testing-spec.md - Unit Tests
// Per spec-kit/003-backend-spec.md - Error handling

use web_terminal::error::Error;
use std::io;

/// Test session error categorization
#[test]
fn test_session_error_categorization() {
    // Arrange
    let errors = vec![
        Error::SessionNotFound("test".to_string()),
        Error::SessionLimitExceeded("test".to_string()),
        Error::SessionExpired("test".to_string()),
    ];

    // Act & Assert
    for error in errors {
        assert!(error.is_session_error());
        assert!(!error.is_security_error());
    }
}

/// Test security error categorization
#[test]
fn test_security_error_categorization() {
    // Arrange
    let errors = vec![
        Error::AuthenticationFailed,
        Error::InvalidToken,
        Error::Unauthorized,
    ];

    // Act & Assert
    for error in errors {
        assert!(error.is_security_error());
        assert!(!error.is_session_error());
    }
}

/// Test error status codes
#[test]
fn test_error_status_codes() {
    // Arrange & Act & Assert
    assert_eq!(Error::SessionNotFound("test".to_string()).status_code(), 404);
    assert_eq!(Error::SessionLimitExceeded("test".to_string()).status_code(), 429);
    assert_eq!(Error::SessionExpired("test".to_string()).status_code(), 410);
    assert_eq!(Error::InvalidCommand("test".to_string()).status_code(), 400);
    assert_eq!(Error::CommandNotAllowed("test".to_string()).status_code(), 400);
    assert_eq!(Error::EmptyCommand.status_code(), 400);
    assert_eq!(Error::ResourceLimitExceeded("test".to_string()).status_code(), 429);
    assert_eq!(Error::InvalidPath("test".to_string()).status_code(), 400);
    assert_eq!(Error::AuthenticationFailed.status_code(), 401);
    assert_eq!(Error::InvalidToken.status_code(), 401);
    assert_eq!(Error::Unauthorized.status_code(), 403);
    assert_eq!(Error::ProcessNotFound(123).status_code(), 404);
}

/// Test error display messages
#[test]
fn test_error_display_messages() {
    // Arrange
    let error1 = Error::SessionNotFound("session123".to_string());
    let error2 = Error::InvalidCommand("rm -rf /".to_string());
    let error3 = Error::AuthenticationFailed;

    // Act
    let msg1 = format!("{}", error1);
    let msg2 = format!("{}", error2);
    let msg3 = format!("{}", error3);

    // Assert
    assert!(msg1.contains("Session not found"));
    assert!(msg1.contains("session123"));
    assert!(msg2.contains("Invalid command"));
    assert!(msg2.contains("rm -rf /"));
    assert!(msg3.contains("Authentication failed"));
}

/// Test Error::from for io::Error
#[test]
fn test_error_from_io_error() {
    // Arrange
    let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");

    // Act
    let error: Error = io_error.into();

    // Assert
    assert!(matches!(error, Error::Io(_)));
    assert_eq!(error.status_code(), 500);
}

/// Test Error::from for serde_json::Error
#[test]
fn test_error_from_serde_error() {
    // Arrange
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json")
        .expect_err("Should fail");

    // Act
    let error: Error = json_error.into();

    // Assert
    assert!(matches!(error, Error::Serialization(_)));
    assert_eq!(error.status_code(), 400);
}

/// Test session errors
#[test]
fn test_session_not_found_error() {
    let error = Error::SessionNotFound("abc123".to_string());
    assert_eq!(error.status_code(), 404);
    assert!(error.is_session_error());
    assert!(format!("{}", error).contains("abc123"));
}

#[test]
fn test_session_limit_exceeded_error() {
    let error = Error::SessionLimitExceeded("User has 10 sessions".to_string());
    assert_eq!(error.status_code(), 429);
    assert!(error.is_session_error());
}

#[test]
fn test_session_expired_error() {
    let error = Error::SessionExpired("session456".to_string());
    assert_eq!(error.status_code(), 410);
    assert!(error.is_session_error());
}

/// Test command execution errors
#[test]
fn test_invalid_command_error() {
    let error = Error::InvalidCommand("!!!".to_string());
    assert_eq!(error.status_code(), 400);
    assert!(!error.is_session_error());
}

#[test]
fn test_command_not_allowed_error() {
    let error = Error::CommandNotAllowed("rm -rf /".to_string());
    assert_eq!(error.status_code(), 400);
}

#[test]
fn test_empty_command_error() {
    let error = Error::EmptyCommand;
    assert_eq!(error.status_code(), 400);
}

#[test]
fn test_execution_failed_error() {
    let error = Error::ExecutionFailed("Process crashed".to_string());
    assert_eq!(error.status_code(), 500);
}

/// Test resource errors
#[test]
fn test_resource_limit_exceeded_error() {
    let error = Error::ResourceLimitExceeded("CPU limit".to_string());
    assert_eq!(error.status_code(), 429);
}

#[test]
fn test_invalid_path_error() {
    let error = Error::InvalidPath("../../../etc/passwd".to_string());
    assert_eq!(error.status_code(), 400);
}

/// Test authentication errors
#[test]
fn test_authentication_failed_error() {
    let error = Error::AuthenticationFailed;
    assert_eq!(error.status_code(), 401);
    assert!(error.is_security_error());
}

#[test]
fn test_invalid_token_error() {
    let error = Error::InvalidToken;
    assert_eq!(error.status_code(), 401);
    assert!(error.is_security_error());
}

#[test]
fn test_unauthorized_error() {
    let error = Error::Unauthorized;
    assert_eq!(error.status_code(), 403);
    assert!(error.is_security_error());
}

/// Test PTY errors
#[test]
fn test_pty_error() {
    let error = Error::PtyError("Failed to create PTY".to_string());
    assert_eq!(error.status_code(), 500);
}

#[test]
fn test_process_spawn_failed_error() {
    let error = Error::ProcessSpawnFailed("fork failed".to_string());
    assert_eq!(error.status_code(), 500);
}

#[test]
fn test_process_not_found_error() {
    let error = Error::ProcessNotFound(12345);
    assert_eq!(error.status_code(), 404);
    assert!(format!("{}", error).contains("12345"));
}

/// Test WebSocket errors
#[test]
fn test_websocket_error() {
    let error = Error::WebSocketError("Connection closed".to_string());
    assert_eq!(error.status_code(), 500);
}

/// Test internal errors
#[test]
fn test_internal_error() {
    let error = Error::Internal("Unexpected condition".to_string());
    assert_eq!(error.status_code(), 500);
}

/// Test error Result type
#[test]
fn test_result_type_ok() {
    let result: web_terminal::error::Result<i32> = Ok(42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_result_type_err() {
    let result: web_terminal::error::Result<i32> = Err(Error::EmptyCommand);
    assert!(result.is_err());
}

/// Test error conversion and propagation
#[test]
fn test_error_propagation() {
    fn inner_function() -> web_terminal::error::Result<()> {
        Err(Error::InvalidCommand("test".to_string()))
    }

    fn outer_function() -> web_terminal::error::Result<()> {
        inner_function()?;
        Ok(())
    }

    let result = outer_function();
    assert!(result.is_err());
}

/// Test error debug formatting
#[test]
fn test_error_debug_format() {
    let error = Error::SessionNotFound("test123".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("SessionNotFound"));
    assert!(debug_str.contains("test123"));
}

/// Test multiple error types in match
#[test]
fn test_error_matching() {
    let errors = vec![
        Error::SessionNotFound("s1".to_string()),
        Error::AuthenticationFailed,
        Error::InvalidCommand("cmd".to_string()),
    ];

    for error in errors {
        match error {
            Error::SessionNotFound(_) => assert!(error.is_session_error()),
            Error::AuthenticationFailed => assert!(error.is_security_error()),
            Error::InvalidCommand(_) => assert_eq!(error.status_code(), 400),
            _ => panic!("Unexpected error type"),
        }
    }
}

/// Test error equality (via string representation)
#[test]
fn test_error_string_equality() {
    let error1 = Error::SessionNotFound("abc".to_string());
    let error2 = Error::SessionNotFound("abc".to_string());
    assert_eq!(format!("{}", error1), format!("{}", error2));
}

/// Test all HTTP status codes are valid
#[test]
fn test_all_status_codes_valid() {
    let errors = vec![
        Error::SessionNotFound("test".to_string()),
        Error::SessionLimitExceeded("test".to_string()),
        Error::SessionExpired("test".to_string()),
        Error::InvalidCommand("test".to_string()),
        Error::CommandNotAllowed("test".to_string()),
        Error::EmptyCommand,
        Error::ExecutionFailed("test".to_string()),
        Error::ResourceLimitExceeded("test".to_string()),
        Error::InvalidPath("test".to_string()),
        Error::AuthenticationFailed,
        Error::InvalidToken,
        Error::Unauthorized,
        Error::PtyError("test".to_string()),
        Error::ProcessSpawnFailed("test".to_string()),
        Error::ProcessNotFound(1),
        Error::WebSocketError("test".to_string()),
        Error::Internal("test".to_string()),
    ];

    for error in errors {
        let code = error.status_code();
        assert!(code >= 400 && code < 600, "Invalid HTTP status code: {}", code);
    }
}