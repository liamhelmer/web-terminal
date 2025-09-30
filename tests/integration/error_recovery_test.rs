// Integration tests for error handling and recovery
// Per spec-kit/008-testing-spec.md - Integration Tests
//
// Tests error handling, recovery, and resilience

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use web_terminal::pty::PtyManager;
use web_terminal::session::{SessionConfig, SessionManager, UserId};

/// Test session recovery after timeout
///
/// Per FR-4.1.3: Session timeout handling
#[tokio::test]
async fn test_session_recovery_after_timeout() {
    let config = SessionConfig {
        timeout: Duration::from_millis(200),
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create session
    let session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    let session_id = session.id.clone();

    // Wait for session to approach timeout
    sleep(Duration::from_millis(100)).await;

    // Touch session to refresh timeout
    session_manager
        .touch_session(&session_id)
        .await
        .expect("Failed to touch session");

    // Wait less than timeout
    sleep(Duration::from_millis(100)).await;

    // Session should still be valid
    let result = session_manager.get_session(&session_id).await;
    assert!(result.is_ok(), "Session should still be valid after touch");

    // Now wait for full timeout without touching
    sleep(Duration::from_millis(250)).await;

    // Run cleanup
    session_manager
        .cleanup_expired_sessions()
        .await
        .expect("Failed to cleanup");

    // Session should be expired now
    let result = session_manager.get_session(&session_id).await;
    assert!(result.is_err(), "Session should be expired and cleaned up");
}

/// Test PTY recovery after process exit
///
/// Per FR-1.2.2: Monitor running processes
#[tokio::test]
async fn test_pty_recovery_after_process_exit() {
    let pty_manager = PtyManager::with_defaults();

    // Spawn PTY
    let pty_handle = pty_manager.spawn(None).expect("Failed to spawn PTY");
    let pty_id = pty_handle.id().to_string();

    // Execute command that exits immediately
    let mut writer = pty_manager
        .create_writer(&pty_id)
        .expect("Failed to create writer");

    writer
        .write(b"exit\n")
        .await
        .expect("Failed to write exit command");

    // Wait for process to exit
    sleep(Duration::from_millis(200)).await;

    // PTY should still exist but process is terminated
    // Subsequent writes may fail
    let result = writer.write(b"echo 'after exit'\n").await;

    // The write may succeed or fail depending on timing and PTY state
    // This test verifies we handle the scenario gracefully
    match result {
        Ok(_) => {
            // Write succeeded, PTY may still be in shutdown state
        }
        Err(e) => {
            // Expected: PTY process has exited
            assert!(
                e.to_string().contains("PTY") || e.to_string().contains("I/O"),
                "Error should be related to PTY or I/O"
            );
        }
    }

    // Cleanup
    let _ = pty_manager.kill(&pty_id).await; // May already be dead
}

/// Test session limit enforcement
///
/// Per FR-4.1.4: Enforce resource limits per session
#[tokio::test]
async fn test_session_limit_enforcement() {
    let config = SessionConfig {
        max_sessions_per_user: 3,
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create max sessions
    let _s1 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 1");
    let _s2 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 2");
    let _s3 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 3");

    // Try to exceed limit
    let result = session_manager.create_session(user_id.clone()).await;
    assert!(
        result.is_err(),
        "Should not be able to exceed session limit"
    );

    // Verify error type
    match result {
        Err(e) => {
            assert!(e.to_string().contains("limit"));
        }
        Ok(_) => panic!("Expected error when exceeding limit"),
    }
}

/// Test handling of invalid PTY operations
///
/// Per error handling requirements
#[tokio::test]
async fn test_invalid_pty_operations() {
    let pty_manager = PtyManager::with_defaults();

    // Try to operate on non-existent PTY
    let invalid_id = "invalid-pty-id";

    let result = pty_manager.create_writer(invalid_id);
    assert!(result.is_err(), "Should fail with invalid PTY ID");

    let result = pty_manager.create_reader(invalid_id, None);
    assert!(result.is_err(), "Should fail with invalid PTY ID");

    let result = pty_manager.kill(invalid_id).await;
    assert!(result.is_err(), "Should fail with invalid PTY ID");
}

/// Test session not found error
///
/// Per error handling requirements
#[tokio::test]
async fn test_session_not_found_error() {
    let config = SessionConfig::default();
    let session_manager = Arc::new(SessionManager::new(config));

    // Try to get non-existent session
    let invalid_id = web_terminal::session::SessionId::from("invalid-session-id".to_string());

    let result = session_manager.get_session(&invalid_id).await;
    assert!(result.is_err(), "Should fail with non-existent session");

    match result {
        Err(e) => {
            assert!(e.to_string().contains("not found"));
        }
        Ok(_) => panic!("Expected error"),
    }

    // Try to destroy non-existent session
    let result = session_manager.destroy_session(&invalid_id).await;
    assert!(
        result.is_err(),
        "Should fail to destroy non-existent session"
    );
}

/// Test concurrent session operations error handling
///
/// Verify thread safety and proper error handling under concurrent load
#[tokio::test]
async fn test_concurrent_error_handling() {
    let config = SessionConfig {
        max_sessions_per_user: 5,
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Spawn multiple concurrent session creation attempts
    let mut handles = vec![];

    for i in 0..10 {
        let manager = session_manager.clone();
        let uid = user_id.clone();

        let handle = tokio::spawn(async move {
            let result = manager.create_session(uid).await;
            (i, result)
        });

        handles.push(handle);
    }

    // Wait for all operations
    let mut successful = 0;
    let mut failed = 0;

    for handle in handles {
        match handle.await {
            Ok((_, Ok(_))) => successful += 1,
            Ok((_, Err(_))) => failed += 1,
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    // Should have max_sessions_per_user successes and rest failures
    assert_eq!(successful, 5, "Should have exactly 5 successful sessions");
    assert_eq!(failed, 5, "Should have 5 failed attempts");

    // Verify session count
    assert_eq!(
        session_manager.user_session_count(&user_id),
        5,
        "Should have exactly 5 active sessions"
    );
}

/// Test PTY cleanup on error
///
/// Verify PTYs are properly cleaned up even when operations fail
#[tokio::test]
async fn test_pty_cleanup_on_error() {
    let pty_manager = PtyManager::with_defaults();

    // Spawn PTY
    let pty_handle = pty_manager.spawn(None).expect("Failed to spawn PTY");
    let pty_id = pty_handle.id().to_string();

    // Kill PTY
    pty_manager.kill(&pty_id).await.expect("Failed to kill PTY");

    // Subsequent operations should fail gracefully
    let result = pty_manager.create_writer(&pty_id);
    assert!(result.is_err(), "Operations on killed PTY should fail");

    // Multiple kill attempts should be handled gracefully
    let result = pty_manager.kill(&pty_id).await;
    assert!(
        result.is_err(),
        "Killing already-dead PTY should return error"
    );
}

/// Test session state consistency after errors
///
/// Verify session state remains consistent even after failed operations
#[tokio::test]
async fn test_session_state_consistency() {
    let config = SessionConfig::default();
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create session
    let session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    // Add some state
    session.add_to_history("command1".to_string()).await;
    session
        .set_env("VAR1".to_string(), "value1".to_string())
        .await;

    // Try to do an invalid operation (path traversal)
    let invalid_path = std::path::PathBuf::from("/etc");
    let result = session.update_working_dir(invalid_path).await;
    assert!(result.is_err(), "Invalid operation should fail");

    // Verify state is still consistent
    let history = session.get_history().await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], "command1");

    let env = session.get_environment().await;
    assert_eq!(env.get("VAR1"), Some(&"value1".to_string()));

    // Cleanup
    session_manager
        .destroy_session(&session.id)
        .await
        .expect("Failed to destroy session");
}
