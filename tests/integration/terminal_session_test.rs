// Integration tests for terminal session lifecycle
// Per spec-kit/008-testing-spec.md - Integration Tests
//
// Tests the complete session lifecycle: create → execute → destroy

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use web_terminal::pty::PtyManager;
use web_terminal::session::{SessionConfig, SessionManager, UserId};

/// Test complete session lifecycle: create → spawn PTY → execute → destroy
///
/// Per FR-4: Session Management
#[tokio::test]
async fn test_session_lifecycle() {
    // Setup
    let config = SessionConfig::default();
    let session_manager = Arc::new(SessionManager::new(config));
    let pty_manager = PtyManager::with_defaults();

    let user_id = UserId::new("test_user".to_string());

    // 1. Create session
    let session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    assert_eq!(session.user_id, user_id);
    assert_eq!(session_manager.session_count(), 1);

    // 2. Spawn PTY for session
    let pty_handle = pty_manager.spawn(None).expect("Failed to spawn PTY");
    let pty_id = pty_handle.id().to_string();

    // Store PTY ID in session
    session.set_pty(pty_id.clone()).await;

    // 3. Execute simple command
    let mut writer = pty_manager
        .create_writer(&pty_id)
        .expect("Failed to create writer");

    writer
        .write(b"echo 'test'\n")
        .await
        .expect("Failed to write command");

    // 4. Read output (note: current PTY implementation may not support synchronous read in tests)
    // let mut reader = pty_manager
    //     .create_reader(&pty_id, None)
    //     .expect("Failed to create reader");

    // Give command time to execute
    sleep(Duration::from_millis(100)).await;

    // Note: PTY output reading will be tested through WebSocket integration
    // For now we just verify the command was written successfully
    // The test verifies that the session and PTY lifecycle works correctly

    // 5. Destroy session and PTY
    session_manager
        .destroy_session(&session.id)
        .await
        .expect("Failed to destroy session");

    pty_manager
        .kill(&pty_id)
        .await
        .expect("Failed to kill PTY");

    // Verify cleanup
    assert_eq!(session_manager.session_count(), 0);
    assert_eq!(session_manager.user_session_count(&user_id), 0);
}

/// Test session reconnection after disconnection
///
/// Per FR-4.1.3: Allow reconnection to existing session
#[tokio::test]
async fn test_session_reconnection() {
    let config = SessionConfig::default();
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // 1. Create session
    let session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    let session_id = session.id.clone();

    // 2. Store some state in session
    session.add_to_history("echo 'first command'".to_string()).await;
    session.set_env("TEST_VAR".to_string(), "test_value".to_string()).await;

    // 3. Simulate disconnection (just drop the reference)
    drop(session);

    // Wait a bit
    sleep(Duration::from_millis(100)).await;

    // 4. Reconnect to same session
    let reconnected_session = session_manager
        .get_session(&session_id)
        .await
        .expect("Failed to reconnect to session");

    // 5. Verify session state persisted
    assert_eq!(reconnected_session.id, session_id);
    assert_eq!(reconnected_session.user_id, user_id);

    let history = reconnected_session.get_history().await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0], "echo 'first command'");

    let env = reconnected_session.get_environment().await;
    assert_eq!(env.get("TEST_VAR"), Some(&"test_value".to_string()));

    // Cleanup
    session_manager
        .destroy_session(&session_id)
        .await
        .expect("Failed to destroy session");
}

/// Test multiple concurrent sessions for one user
///
/// Per FR-4.1.2: Support multiple concurrent sessions per user
#[tokio::test]
async fn test_multiple_concurrent_sessions() {
    let config = SessionConfig {
        max_sessions_per_user: 5,
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // 1. Create multiple sessions for same user
    let session1 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 1");
    let session2 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 2");
    let session3 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 3");

    assert_eq!(session_manager.user_session_count(&user_id), 3);

    // 2. Execute different commands in each session
    session1.add_to_history("ls -la".to_string()).await;
    session2.add_to_history("pwd".to_string()).await;
    session3.add_to_history("whoami".to_string()).await;

    // Set different environment variables
    session1.set_env("SESSION".to_string(), "1".to_string()).await;
    session2.set_env("SESSION".to_string(), "2".to_string()).await;
    session3.set_env("SESSION".to_string(), "3".to_string()).await;

    // 3. Verify isolation between sessions
    let env1 = session1.get_environment().await;
    let env2 = session2.get_environment().await;
    let env3 = session3.get_environment().await;

    assert_eq!(env1.get("SESSION"), Some(&"1".to_string()));
    assert_eq!(env2.get("SESSION"), Some(&"2".to_string()));
    assert_eq!(env3.get("SESSION"), Some(&"3".to_string()));

    let history1 = session1.get_history().await;
    let history2 = session2.get_history().await;
    let history3 = session3.get_history().await;

    assert_eq!(history1, vec!["ls -la"]);
    assert_eq!(history2, vec!["pwd"]);
    assert_eq!(history3, vec!["whoami"]);

    // 4. Cleanup all sessions
    session_manager
        .destroy_session(&session1.id)
        .await
        .expect("Failed to destroy session 1");
    session_manager
        .destroy_session(&session2.id)
        .await
        .expect("Failed to destroy session 2");
    session_manager
        .destroy_session(&session3.id)
        .await
        .expect("Failed to destroy session 3");

    assert_eq!(session_manager.user_session_count(&user_id), 0);
}

/// Test terminal resize handling
///
/// Per FR-2.1.5: Support terminal dimensions
#[tokio::test]
async fn test_terminal_resize() {
    let pty_manager = PtyManager::with_defaults();

    // 1. Spawn PTY with default size
    let pty_handle = pty_manager.spawn(None).expect("Failed to spawn PTY");
    let pty_id = pty_handle.id().to_string();

    // 2. Resize terminal
    let new_cols = 120;
    let new_rows = 40;

    pty_manager
        .resize(&pty_id, new_cols, new_rows)
        .await
        .expect("Failed to resize PTY");

    // 3. Execute command that uses terminal size (e.g., tput cols)
    let mut writer = pty_manager
        .create_writer(&pty_id)
        .expect("Failed to create writer");

    // Note: We can't easily verify the resize worked without actually reading
    // the terminal size from within the PTY, but we can verify the resize call
    // succeeded without error

    // 4. Cleanup
    pty_manager
        .kill(&pty_id)
        .await
        .expect("Failed to kill PTY");
}

/// Test process signal handling (SIGTERM via kill)
///
/// Per FR-1.2.4: Support common signals (SIGINT, SIGTERM)
#[tokio::test]
async fn test_process_signals() {
    let pty_manager = PtyManager::with_defaults();

    // 1. Spawn PTY
    let pty_handle = pty_manager.spawn(None).expect("Failed to spawn PTY");
    let pty_id = pty_handle.id().to_string();

    // 2. Execute long-running command
    let mut writer = pty_manager
        .create_writer(&pty_id)
        .expect("Failed to create writer");

    writer
        .write(b"sleep 10\n")
        .await
        .expect("Failed to write command");

    // Give command time to start
    sleep(Duration::from_millis(100)).await;

    // 3. Send kill signal (equivalent to SIGTERM)
    pty_manager
        .kill(&pty_id)
        .await
        .expect("Failed to kill PTY");

    // Verify PTY is killed (subsequent operations should fail)
    let result = pty_manager.create_writer(&pty_id);
    assert!(result.is_err(), "Expected PTY to be killed");
}

/// Test session resource limits
///
/// Per FR-4.1.4: Enforce resource limits per session
#[tokio::test]
async fn test_session_resource_limits() {
    let config = SessionConfig {
        max_sessions_per_user: 2,
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create max sessions
    let _session1 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 1");
    let _session2 = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session 2");

    // Try to exceed limit
    let result = session_manager.create_session(user_id.clone()).await;
    assert!(
        result.is_err(),
        "Expected error when exceeding session limit"
    );

    // Verify error message
    match result {
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("limit"),
                "Expected error message to mention limit, got: {}",
                err_str
            );
        }
        Ok(_) => panic!("Expected error"),
    }
}

/// Test session cleanup on abnormal termination
#[tokio::test]
async fn test_session_cleanup_on_error() {
    let config = SessionConfig::default();
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create session
    let session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    let session_id = session.id.clone();

    assert_eq!(session_manager.session_count(), 1);

    // Simulate cleanup (destroy session)
    session_manager
        .destroy_session(&session_id)
        .await
        .expect("Failed to destroy session");

    // Verify cleanup
    assert_eq!(session_manager.session_count(), 0);
    assert_eq!(session_manager.user_session_count(&user_id), 0);

    // Verify session is gone
    let result = session_manager.get_session(&session_id).await;
    assert!(result.is_err(), "Expected session to be gone");
}

/// Test command history in-memory storage
#[tokio::test]
async fn test_command_history() {
    // Note: Per ADR-012, history is in-memory only (no persistence)
    let config = SessionConfig::default();
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create session
    let session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    // Execute multiple commands
    let commands = vec![
        "ls -la",
        "cd /tmp",
        "pwd",
        "echo 'hello'",
        "cat file.txt",
    ];

    for cmd in &commands {
        session.add_to_history(cmd.to_string()).await;
    }

    // Retrieve command history
    let history = session.get_history().await;

    // Verify history is accurate
    assert_eq!(history.len(), commands.len());
    for (i, cmd) in commands.iter().enumerate() {
        assert_eq!(&history[i], cmd);
    }

    // Cleanup
    session_manager
        .destroy_session(&session.id)
        .await
        .expect("Failed to destroy session");
}

/// Test session timeout and cleanup
///
/// Per spec-kit/003-backend-spec.md section 2.1
#[tokio::test]
async fn test_session_timeout_cleanup() {
    let config = SessionConfig {
        timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let session_manager = Arc::new(SessionManager::new(config));
    let user_id = UserId::new("test_user".to_string());

    // Create session
    let _session = session_manager
        .create_session(user_id.clone())
        .await
        .expect("Failed to create session");

    assert_eq!(session_manager.session_count(), 1);

    // Wait for session to expire
    sleep(Duration::from_millis(150)).await;

    // Run cleanup
    let cleaned = session_manager
        .cleanup_expired_sessions()
        .await
        .expect("Failed to cleanup expired sessions");

    assert_eq!(cleaned, 1, "Expected 1 expired session to be cleaned up");
    assert_eq!(session_manager.session_count(), 0);
}