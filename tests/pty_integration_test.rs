// Integration tests for PTY functionality
// Per spec-kit/008-testing-spec.md - Integration Tests

use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use web_terminal::pty::{PtyConfig, PtyManager, ShellConfig};
use web_terminal::protocol::Signal;

/// Test complete PTY lifecycle: spawn, write, read, kill
///
/// Per FR-1.2: Process Management
/// Per FR-3.3: Real-time streaming
#[tokio::test]
async fn test_complete_pty_lifecycle() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY
    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    assert!(manager.is_alive(&id).await);

    // Create writer and reader
    let writer = manager.create_writer(&id).expect("Failed to create writer");
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Start streaming output
    manager
        .stream_output(&id, tx)
        .await
        .expect("Failed to start streaming");

    // Write command to PTY
    writer
        .write_str("echo 'Hello PTY'\n")
        .await
        .expect("Failed to write to PTY");

    // Wait for output (with timeout)
    let mut received_output = false;
    let timeout = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(data) = rx.recv().await {
            let output = String::from_utf8_lossy(&data);
            if output.contains("Hello PTY") {
                received_output = true;
                break;
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "Should receive output within timeout");
    assert!(received_output, "Should receive 'Hello PTY' in output");

    // Kill PTY
    manager.kill(&id).await.expect("Failed to kill PTY");

    // Give it time to die
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(manager.count(), 0);
}

/// Test PTY with custom shell configuration
///
/// Per spec-kit/003-backend-spec.md: PTY configuration
#[tokio::test]
async fn test_pty_with_custom_shell() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY with sh and specific command
    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec!["-c".to_string(), "echo test123 && exit 0".to_string()],
            None,
        )
        .expect("Failed to spawn PTY with custom shell");

    let id = handle.id().to_string();

    // Create output channel
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Start streaming
    manager
        .stream_output(&id, tx)
        .await
        .expect("Failed to start streaming");

    // Wait for output
    let mut received = false;
    let timeout = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(data) = rx.recv().await {
            let output = String::from_utf8_lossy(&data);
            if output.contains("test123") {
                received = true;
                break;
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "Should receive output");
    assert!(received, "Should receive test123");

    // Wait for process to exit
    let exit_code = manager.wait(&id).await;
    assert!(exit_code.is_ok() || !manager.is_alive(&id).await);

    // Cleanup
    let _ = manager.kill(&id).await;
}

/// Test terminal resize during active session
///
/// Per FR-2.1.5: Support terminal dimensions
#[tokio::test]
async fn test_terminal_resize_during_session() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    // Create writer
    let writer = manager.create_writer(&id).expect("Failed to create writer");

    // Initial size check
    let config = handle.config().await;
    assert_eq!(config.cols, 80);
    assert_eq!(config.rows, 24);

    // Resize multiple times
    manager
        .resize(&id, 120, 40)
        .await
        .expect("Failed to resize to 120x40");

    let config = handle.config().await;
    assert_eq!(config.cols, 120);
    assert_eq!(config.rows, 40);

    manager
        .resize(&id, 160, 50)
        .await
        .expect("Failed to resize to 160x50");

    let config = handle.config().await;
    assert_eq!(config.cols, 160);
    assert_eq!(config.rows, 50);

    // PTY should still be alive and functional
    assert!(manager.is_alive(&id).await);

    writer
        .write_str("echo after resize\n")
        .await
        .expect("Failed to write after resize");

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test signal handling interrupts running process
///
/// Per FR-1.2.4: Support process termination (Ctrl+C / SIGINT)
#[tokio::test]
async fn test_signal_interrupts_running_process() {
    let manager = PtyManager::with_defaults();

    // Spawn a long-running process
    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec!["-c".to_string(), "sleep 30".to_string()],
            None,
        )
        .expect("Failed to spawn PTY");

    let id = handle.id().to_string();

    assert!(manager.is_alive(&id).await);

    // Give it a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send SIGTERM to interrupt
    manager
        .send_signal(&id, Signal::SIGTERM)
        .await
        .expect("Failed to send SIGTERM");

    // Give it time to die
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Process should be killed
    assert_eq!(manager.count(), 0);
}

/// Test concurrent PTY sessions with streaming
///
/// Per NFR-3.3: Support multiple concurrent users
#[tokio::test]
async fn test_concurrent_pty_sessions() {
    use std::sync::Arc;

    let manager = Arc::new(PtyManager::with_defaults());
    let mut join_handles = vec![];

    // Spawn 10 concurrent sessions
    for i in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            // Spawn PTY
            let pty_handle = manager_clone.spawn(None).expect("Failed to spawn PTY");
            let id = pty_handle.id().to_string();

            // Stream output
            let (tx, mut rx) = mpsc::unbounded_channel();
            manager_clone
                .stream_output(&id, tx)
                .await
                .expect("Failed to start streaming");

            // Write to PTY
            let writer = manager_clone
                .create_writer(&id)
                .expect("Failed to create writer");

            writer
                .write_str(&format!("echo session {}\n", i))
                .await
                .expect("Failed to write");

            // Wait for output
            let mut received = false;
            let timeout = tokio::time::timeout(Duration::from_secs(2), async {
                while let Some(data) = rx.recv().await {
                    let output = String::from_utf8_lossy(&data);
                    if output.contains(&format!("session {}", i)) {
                        received = true;
                        break;
                    }
                }
            })
            .await;

            assert!(timeout.is_ok(), "Session {} timed out", i);
            assert!(received, "Session {} didn't receive output", i);

            // Kill PTY
            manager_clone
                .kill(&id)
                .await
                .expect("Failed to kill PTY");
        });

        join_handles.push(handle);
    }

    // Wait for all sessions to complete
    for handle in join_handles {
        handle.await.expect("Session task panicked");
    }

    assert_eq!(manager.count(), 0);
}

/// Test PTY environment variable handling
///
/// Per FR-2.3.1: Set session-specific variables
#[tokio::test]
async fn test_pty_environment_variables() {
    use std::collections::HashMap;

    let manager = PtyManager::with_defaults();

    let mut env = HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value".to_string());
    env.insert("TERM".to_string(), "xterm-256color".to_string());

    let config = PtyConfig {
        cols: 80,
        rows: 24,
        working_dir: std::path::PathBuf::from("/tmp"),
        env,
        shell: ShellConfig::sh(),
        max_buffer_size: 1024 * 1024,
        read_timeout_ms: 10,
    };

    let handle = manager.spawn(Some(config)).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    // Create output channel
    let (tx, mut rx) = mpsc::unbounded_channel();

    manager
        .stream_output(&id, tx)
        .await
        .expect("Failed to start streaming");

    // Write command to check environment variable
    let writer = manager.create_writer(&id).expect("Failed to create writer");
    writer
        .write_str("echo $TEST_VAR\n")
        .await
        .expect("Failed to write");

    // Wait for output
    let mut received = false;
    let timeout = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(data) = rx.recv().await {
            let output = String::from_utf8_lossy(&data);
            if output.contains("test_value") {
                received = true;
                break;
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "Should receive output");
    assert!(received, "Should receive test_value");

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test PTY output streaming performance
///
/// Per NFR-1.1.2: WebSocket message latency < 20ms (p95)
#[tokio::test]
async fn test_streaming_performance() {
    let manager = PtyManager::with_defaults();

    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec!["-c".to_string(), "echo test".to_string()],
            None,
        )
        .expect("Failed to spawn PTY");

    let id = handle.id().to_string();

    let (tx, mut rx) = mpsc::unbounded_channel();

    let start = std::time::Instant::now();

    manager
        .stream_output(&id, tx)
        .await
        .expect("Failed to start streaming");

    // Wait for first output
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;

    let duration = start.elapsed();

    assert!(result.is_ok(), "Should receive output quickly");
    assert!(
        duration < Duration::from_millis(50),
        "Output latency too high: {:?}",
        duration
    );

    // Cleanup
    let _ = manager.kill(&id).await;
}