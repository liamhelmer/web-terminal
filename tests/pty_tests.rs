// PTY integration tests
// Per spec-kit/008-testing-spec.md

use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use web_terminal::pty::{PtyConfig, PtyManager, ShellConfig};

#[tokio::test]
async fn test_pty_spawn_and_execute() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY
    let handle = manager.spawn(None).expect("Failed to spawn PTY");

    assert!(manager.is_alive(handle.id()).await);

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}

#[tokio::test]
async fn test_pty_write_and_read() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY
    let handle = manager.spawn(None).expect("Failed to spawn PTY");

    // Create writer and reader
    let writer = manager.create_writer(handle.id()).expect("Failed to create writer");
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Start streaming output
    manager
        .stream_output(handle.id(), tx)
        .await
        .expect("Failed to start streaming");

    // Write a command
    writer
        .write_str("echo 'test'\n")
        .await
        .expect("Failed to write");

    // Read output with timeout
    let result = timeout(Duration::from_secs(2), async {
        let mut output = Vec::new();

        while let Some(data) = rx.recv().await {
            output.extend_from_slice(&data);

            // Convert to string to check if we got the expected output
            let output_str = String::from_utf8_lossy(&output);
            if output_str.contains("test") {
                return output;
            }

            // Break if we've read enough data
            if output.len() > 1024 {
                break;
            }
        }

        output
    })
    .await;

    assert!(result.is_ok(), "Timeout waiting for output");

    let output = result.unwrap();
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains("test"),
        "Output should contain 'test', got: {}",
        output_str
    );

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}

#[tokio::test]
async fn test_pty_resize() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY with initial size
    let mut config = PtyConfig::default();
    config.cols = 80;
    config.rows = 24;

    let handle = manager.spawn(Some(config)).expect("Failed to spawn PTY");

    // Verify initial size
    let config = handle.config().await;
    assert_eq!(config.cols, 80);
    assert_eq!(config.rows, 24);

    // Resize
    manager
        .resize(handle.id(), 120, 40)
        .await
        .expect("Failed to resize");

    // Verify new size
    let config = handle.config().await;
    assert_eq!(config.cols, 120);
    assert_eq!(config.rows, 40);

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}

#[tokio::test]
async fn test_pty_multiple_processes() {
    let manager = PtyManager::with_defaults();

    // Spawn multiple PTYs
    let handle1 = manager.spawn(None).expect("Failed to spawn PTY 1");
    let handle2 = manager.spawn(None).expect("Failed to spawn PTY 2");
    let handle3 = manager.spawn(None).expect("Failed to spawn PTY 3");

    assert_eq!(manager.count(), 3);

    // Verify all are alive
    assert!(manager.is_alive(handle1.id()).await);
    assert!(manager.is_alive(handle2.id()).await);
    assert!(manager.is_alive(handle3.id()).await);

    // Kill one
    manager.kill(handle2.id()).await.expect("Failed to kill PTY 2");
    assert_eq!(manager.count(), 2);

    // Kill all remaining
    let count = manager.kill_all().await.expect("Failed to kill all");
    assert_eq!(count, 2);
    assert_eq!(manager.count(), 0);
}

#[tokio::test]
async fn test_pty_cleanup_dead_processes() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY
    let handle = manager.spawn(None).expect("Failed to spawn PTY");

    // Kill process directly (not through manager)
    handle.kill().await.expect("Failed to kill PTY");

    // Give it time to exit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Process should still be in registry
    assert_eq!(manager.count(), 1);

    // Cleanup should remove it
    let count = manager
        .cleanup_dead_processes()
        .await
        .expect("Failed to cleanup");

    assert_eq!(count, 1);
    assert_eq!(manager.count(), 0);
}

#[tokio::test]
async fn test_pty_shell_variants() {
    let manager = PtyManager::with_defaults();

    // Test bash
    let mut config = PtyConfig::default();
    config.shell = ShellConfig::bash();
    let handle = manager.spawn(Some(config)).expect("Failed to spawn bash");
    assert!(manager.is_alive(handle.id()).await);
    manager.kill(handle.id()).await.expect("Failed to kill bash");

    // Test sh
    let mut config = PtyConfig::default();
    config.shell = ShellConfig::sh();
    let handle = manager.spawn(Some(config)).expect("Failed to spawn sh");
    assert!(manager.is_alive(handle.id()).await);
    manager.kill(handle.id()).await.expect("Failed to kill sh");
}

#[tokio::test]
async fn test_pty_environment_variables() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY with custom env
    let mut config = PtyConfig::default();
    config.env.insert("TEST_VAR".to_string(), "test_value".to_string());

    let handle = manager.spawn(Some(config)).expect("Failed to spawn PTY");

    // Create writer and reader
    let writer = manager.create_writer(handle.id()).expect("Failed to create writer");
    let (tx, mut rx) = mpsc::unbounded_channel();

    manager
        .stream_output(handle.id(), tx)
        .await
        .expect("Failed to start streaming");

    // Write command to check env var
    writer
        .write_str("echo $TEST_VAR\n")
        .await
        .expect("Failed to write");

    // Read output
    let result = timeout(Duration::from_secs(2), async {
        let mut output = Vec::new();

        while let Some(data) = rx.recv().await {
            output.extend_from_slice(&data);

            let output_str = String::from_utf8_lossy(&output);
            if output_str.contains("test_value") {
                return output;
            }

            if output.len() > 1024 {
                break;
            }
        }

        output
    })
    .await;

    assert!(result.is_ok(), "Timeout waiting for output");

    let output = result.unwrap();
    let output_str = String::from_utf8_lossy(&output);

    assert!(
        output_str.contains("test_value"),
        "Output should contain 'test_value', got: {}",
        output_str
    );

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}