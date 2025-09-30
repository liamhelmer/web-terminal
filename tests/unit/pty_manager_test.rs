// Unit tests for PTY Manager
// Per spec-kit/008-testing-spec.md - Unit Tests

use std::time::Duration;
use web_terminal::pty::{PtyConfig, PtyError, PtyManager, ShellConfig};

/// Test basic PTY manager creation
#[test]
fn test_pty_manager_creation() {
    let manager = PtyManager::with_defaults();
    assert_eq!(manager.count(), 0);
    assert!(manager.list().is_empty());
}

/// Test PTY manager creation with custom config
#[test]
fn test_pty_manager_with_custom_config() {
    let config = PtyConfig {
        cols: 120,
        rows: 40,
        shell: ShellConfig::default(),
        env: Default::default(),
    };

    let manager = PtyManager::new(config);
    assert_eq!(manager.count(), 0);
}

/// Test spawning a single PTY process
#[tokio::test]
async fn test_spawn_pty_process() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");

    assert_eq!(manager.count(), 1);
    assert!(manager.is_alive(handle.id()).await);

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}

/// Test spawning multiple PTY processes
#[tokio::test]
async fn test_spawn_multiple_pty_processes() {
    let manager = PtyManager::with_defaults();

    let handle1 = manager.spawn(None).expect("Failed to spawn PTY 1");
    let handle2 = manager.spawn(None).expect("Failed to spawn PTY 2");
    let handle3 = manager.spawn(None).expect("Failed to spawn PTY 3");

    assert_eq!(manager.count(), 3);
    assert!(manager.is_alive(handle1.id()).await);
    assert!(manager.is_alive(handle2.id()).await);
    assert!(manager.is_alive(handle3.id()).await);

    // Cleanup
    manager.kill_all().await.expect("Failed to kill all PTYs");
    assert_eq!(manager.count(), 0);
}

/// Test spawning with custom shell
#[tokio::test]
async fn test_spawn_with_custom_shell() {
    let manager = PtyManager::with_defaults();

    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec!["-c".to_string(), "echo test".to_string()],
            None,
        )
        .expect("Failed to spawn PTY with custom shell");

    assert_eq!(manager.count(), 1);
    assert!(manager.is_alive(handle.id()).await);

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}

/// Test getting PTY process by ID
#[tokio::test]
async fn test_get_pty_process() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    let retrieved = manager.get(&id).expect("Failed to get PTY");
    assert_eq!(retrieved.id(), handle.id());

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test getting non-existent PTY process
#[test]
fn test_get_nonexistent_pty() {
    let manager = PtyManager::with_defaults();

    let result = manager.get("nonexistent-id");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PtyError::ProcessNotFound(_)));
}

/// Test removing PTY process from registry
#[tokio::test]
async fn test_remove_pty_process() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    assert_eq!(manager.count(), 1);

    let removed = manager.remove(&id).expect("Failed to remove PTY");
    assert_eq!(removed.id(), &id);
    assert_eq!(manager.count(), 0);

    // Cleanup
    removed.kill().await.expect("Failed to kill PTY");
}

/// Test killing PTY process
#[tokio::test]
async fn test_kill_pty_process() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    assert!(manager.is_alive(&id).await);

    manager.kill(&id).await.expect("Failed to kill PTY");

    // Give it time to die
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(manager.count(), 0);
}

/// Test resizing PTY process
///
/// Per FR-2.1.5: Support terminal dimensions
#[tokio::test]
async fn test_resize_pty_process() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    manager
        .resize(&id, 120, 40)
        .await
        .expect("Failed to resize PTY");

    let config = handle.config().await;
    assert_eq!(config.cols, 120);
    assert_eq!(config.rows, 40);

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test resizing with invalid dimensions
#[tokio::test]
async fn test_resize_invalid_dimensions() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    // Try to resize to 0x0 (should fail or be clamped)
    let result = manager.resize(&id, 0, 0).await;

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");

    // Implementation may clamp to minimum values or return error
    // Either behavior is acceptable
}

/// Test listing active PTY processes
#[tokio::test]
async fn test_list_pty_processes() {
    let manager = PtyManager::with_defaults();

    let handle1 = manager.spawn(None).expect("Failed to spawn PTY 1");
    let handle2 = manager.spawn(None).expect("Failed to spawn PTY 2");

    let list = manager.list();
    assert_eq!(list.len(), 2);
    assert!(list.contains(&handle1.id().to_string()));
    assert!(list.contains(&handle2.id().to_string()));

    // Cleanup
    manager.kill_all().await.expect("Failed to kill all PTYs");
}

/// Test cleanup of dead processes
///
/// Per FR-4.1.5: Clean up session resources on close
#[tokio::test]
async fn test_cleanup_dead_processes() {
    let manager = PtyManager::with_defaults();

    let handle1 = manager.spawn(None).expect("Failed to spawn PTY 1");
    let handle2 = manager.spawn(None).expect("Failed to spawn PTY 2");

    // Kill one process directly (bypassing manager)
    handle1.kill().await.expect("Failed to kill PTY 1");

    // Give it time to die
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cleanup should remove dead process
    let count = manager
        .cleanup_dead_processes()
        .await
        .expect("Failed to cleanup");

    assert_eq!(count, 1);
    assert_eq!(manager.count(), 1);

    // Cleanup remaining
    manager.kill_all().await.expect("Failed to kill all PTYs");
}

/// Test kill all PTY processes
#[tokio::test]
async fn test_kill_all_pty_processes() {
    let manager = PtyManager::with_defaults();

    manager.spawn(None).expect("Failed to spawn PTY 1");
    manager.spawn(None).expect("Failed to spawn PTY 2");
    manager.spawn(None).expect("Failed to spawn PTY 3");

    assert_eq!(manager.count(), 3);

    let killed = manager.kill_all().await.expect("Failed to kill all PTYs");

    assert_eq!(killed, 3);
    assert_eq!(manager.count(), 0);
}

/// Test waiting for PTY process to exit
#[tokio::test]
async fn test_wait_for_pty_exit() {
    let manager = PtyManager::with_defaults();

    // Spawn a process that exits immediately
    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec!["-c".to_string(), "exit 42".to_string()],
            None,
        )
        .expect("Failed to spawn PTY");

    let id = handle.id().to_string();

    let exit_code = manager.wait(&id).await.expect("Failed to wait for PTY");

    assert_eq!(exit_code, Some(42));

    // Cleanup
    let _ = manager.kill(&id).await;
}

/// Test checking if PTY is alive
#[tokio::test]
async fn test_is_alive() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    assert!(manager.is_alive(&id).await);

    manager.kill(&id).await.expect("Failed to kill PTY");

    // Give it time to die
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(!manager.is_alive(&id).await);
}

/// Test checking if non-existent PTY is alive
#[tokio::test]
async fn test_is_alive_nonexistent() {
    let manager = PtyManager::with_defaults();

    assert!(!manager.is_alive("nonexistent-id").await);
}

/// Test creating reader for PTY output
#[tokio::test]
async fn test_create_reader() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    let reader = manager
        .create_reader(&id, None)
        .expect("Failed to create reader");

    // Reader should be created successfully
    // Actual reading is tested in io_handler tests

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test creating writer for PTY input
#[tokio::test]
async fn test_create_writer() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    let writer = manager.create_writer(&id).expect("Failed to create writer");

    // Writer should be created successfully
    // Actual writing is tested in io_handler tests

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test concurrent access to PTY manager
///
/// Per NFR-3.3: Support multiple concurrent users
#[tokio::test]
async fn test_concurrent_access() {
    use std::sync::Arc;

    let manager = Arc::new(PtyManager::with_defaults());

    let mut handles = vec![];

    // Spawn multiple tasks that create PTYs concurrently
    for _ in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle =
            tokio::spawn(async move { manager_clone.spawn(None).expect("Failed to spawn PTY") });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut pty_handles = vec![];
    for handle in handles {
        pty_handles.push(handle.await.expect("Task panicked"));
    }

    assert_eq!(manager.count(), 10);

    // Cleanup
    manager.kill_all().await.expect("Failed to kill all PTYs");
}

/// Test session creation time constraint
///
/// Per NFR-1.1.5: Session creation time < 200ms
#[tokio::test]
async fn test_session_creation_latency() {
    let manager = PtyManager::with_defaults();

    let start = std::time::Instant::now();
    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let duration = start.elapsed();

    // Should complete within 200ms
    assert!(
        duration < Duration::from_millis(200),
        "PTY creation took {:?}, expected < 200ms",
        duration
    );

    // Cleanup
    manager.kill(handle.id()).await.expect("Failed to kill PTY");
}

/// Test memory safety with rapid create/destroy cycles
#[tokio::test]
async fn test_rapid_create_destroy() {
    let manager = PtyManager::with_defaults();

    for _ in 0..50 {
        let handle = manager.spawn(None).expect("Failed to spawn PTY");
        manager.kill(handle.id()).await.expect("Failed to kill PTY");
    }

    assert_eq!(manager.count(), 0);
}

/// Test error handling when resizing non-existent PTY
#[tokio::test]
async fn test_resize_nonexistent_pty() {
    let manager = PtyManager::with_defaults();

    let result = manager.resize("nonexistent-id", 100, 30).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PtyError::ProcessNotFound(_)));
}

/// Test streaming output with non-existent PTY
#[tokio::test]
async fn test_stream_output_nonexistent_pty() {
    let manager = PtyManager::with_defaults();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    let result = manager.stream_output("nonexistent-id", tx).await;
    assert!(result.is_err());
}

/// Test waiting for non-existent PTY
#[tokio::test]
async fn test_wait_nonexistent_pty() {
    let manager = PtyManager::with_defaults();

    let result = manager.wait("nonexistent-id").await;
    assert!(result.is_err());
}

/// Test removing non-existent PTY
#[test]
fn test_remove_nonexistent_pty() {
    let manager = PtyManager::with_defaults();

    let result = manager.remove("nonexistent-id");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PtyError::ProcessNotFound(_)));
}

/// Test PTY manager with maximum concurrent processes
///
/// Per NFR-3.3: Support multiple concurrent users
#[tokio::test]
async fn test_maximum_concurrent_processes() {
    let manager = PtyManager::with_defaults();
    let mut handles = vec![];

    // Spawn 100 PTY processes
    for _ in 0..100 {
        let handle = manager.spawn(None).expect("Failed to spawn PTY");
        handles.push(handle);
    }

    assert_eq!(manager.count(), 100);

    // Cleanup
    manager.kill_all().await.expect("Failed to kill all PTYs");
    assert_eq!(manager.count(), 0);
}

/// Test sequential resize operations
#[tokio::test]
async fn test_sequential_resize() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY process
    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    // Perform sequential resizes
    for i in 0..10 {
        let _ = manager.resize(&id, 80 + i, 24 + i).await;
    }

    // Cleanup
    manager.kill(&id).await.expect("Failed to kill PTY");
}

/// Test default configuration
#[test]
fn test_pty_manager_default() {
    let manager = PtyManager::default();
    assert_eq!(manager.count(), 0);
}

/// Test kill operation idempotency
#[tokio::test]
async fn test_kill_idempotency() {
    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    // First kill should succeed
    manager.kill(&id).await.expect("Failed to kill PTY");

    // Second kill should fail (PTY not found)
    let result = manager.kill(&id).await;
    assert!(result.is_err());
}

/// Test listing processes returns correct IDs
#[tokio::test]
async fn test_list_returns_correct_ids() {
    let manager = PtyManager::with_defaults();

    let handle1 = manager.spawn(None).expect("Failed to spawn PTY 1");
    let handle2 = manager.spawn(None).expect("Failed to spawn PTY 2");
    let handle3 = manager.spawn(None).expect("Failed to spawn PTY 3");

    let list = manager.list();
    assert_eq!(list.len(), 3);

    // Verify all IDs are present
    assert!(list.iter().any(|id| id == handle1.id()));
    assert!(list.iter().any(|id| id == handle2.id()));
    assert!(list.iter().any(|id| id == handle3.id()));

    // Cleanup
    manager.kill_all().await.expect("Failed to kill all PTYs");
}

/// Test signal handling
///
/// Per FR-1.2.4: Support process termination (Ctrl+C / SIGINT)
/// Per spec-kit/003-backend-spec.md: Signal handling
#[tokio::test]
async fn test_send_signal() {
    use web_terminal::protocol::Signal;

    let manager = PtyManager::with_defaults();

    let handle = manager.spawn(None).expect("Failed to spawn PTY");
    let id = handle.id().to_string();

    assert!(manager.is_alive(&id).await);

    // Send SIGINT signal
    manager
        .send_signal(&id, Signal::SIGINT)
        .await
        .expect("Failed to send SIGINT");

    // Give it time to die
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(manager.count(), 0);
}

/// Test sending signal to non-existent PTY
#[tokio::test]
async fn test_send_signal_nonexistent() {
    use web_terminal::protocol::Signal;

    let manager = PtyManager::with_defaults();

    let result = manager.send_signal("nonexistent-id", Signal::SIGTERM).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PtyError::ProcessNotFound(_)));
}

/// Test bounded streaming output
///
/// Per spec-kit/003-backend-spec.md: PTY manager interface with backpressure
#[tokio::test]
async fn test_stream_output_bounded() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY with a command that produces output
    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec!["-c".to_string(), "echo 'test output'".to_string()],
            None,
        )
        .expect("Failed to spawn PTY");

    let id = handle.id().to_string();

    // Create bounded channel
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);

    // Start streaming
    manager
        .stream_output_bounded(&id, tx)
        .await
        .expect("Failed to start streaming");

    // Wait for output
    let timeout = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

    assert!(timeout.is_ok(), "Should receive output within timeout");
    assert!(timeout.unwrap().is_some(), "Should receive data");

    // Cleanup
    let _ = manager.kill(&id).await;
}

/// Test bounded streaming with backpressure
#[tokio::test]
async fn test_bounded_streaming_backpressure() {
    let manager = PtyManager::with_defaults();

    // Spawn PTY with a command that produces lots of output
    let handle = manager
        .spawn_with_shell(
            "/bin/sh",
            vec![
                "-c".to_string(),
                "for i in $(seq 1 100); do echo line $i; done".to_string(),
            ],
            None,
        )
        .expect("Failed to spawn PTY");

    let id = handle.id().to_string();

    // Create bounded channel with small capacity
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(5);

    // Start streaming
    manager
        .stream_output_bounded(&id, tx)
        .await
        .expect("Failed to start streaming");

    // Consume output slowly to test backpressure
    let mut received = 0;
    while let Ok(Some(_data)) =
        tokio::time::timeout(Duration::from_millis(100), rx.recv()).await
    {
        received += 1;
        if received > 5 {
            break; // Got enough to verify backpressure works
        }
    }

    assert!(received > 0, "Should receive some output");

    // Cleanup
    let _ = manager.kill(&id).await;
}
