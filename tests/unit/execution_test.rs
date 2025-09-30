// Unit tests for Process Execution
// Per spec-kit/008-testing-spec.md - Unit Tests
// Per spec-kit/003-backend-spec.md section 3 - Process management

use std::collections::HashMap;
use std::path::PathBuf;
use web_terminal::execution::ProcessManager;

/// Test process manager creation
#[test]
fn test_process_manager_creation() {
    // Arrange & Act
    let manager = ProcessManager::new();

    // Assert - manager should be created successfully
    // Process list is internal, we just verify creation works
}

/// Test process manager default
#[test]
fn test_process_manager_default() {
    // Arrange & Act
    let manager = ProcessManager::default();

    // Assert - default should work same as new
}

/// Test list processes returns empty initially
#[tokio::test]
async fn test_list_processes_empty_initially() {
    // Arrange
    let manager = ProcessManager::new();

    // Act
    let processes = manager.list_processes().await;

    // Assert
    assert_eq!(processes.len(), 0);
}

/// Test spawn process with valid command
///
/// Per spec-kit/003-backend-spec.md section 3
#[tokio::test]
async fn test_spawn_process_valid_command() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act
    let result = manager
        .spawn("echo", &["test".to_string()], &env, &working_dir)
        .await;

    // Assert
    assert!(result.is_ok());
    let handle = result.unwrap();
    assert!(handle.pid > 0);
    assert_eq!(handle.command, "echo");
}

/// Test spawn process increments process ID
#[tokio::test]
async fn test_spawn_process_increments_pid() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act
    let handle1 = manager
        .spawn("echo", &["test1".to_string()], &env, &working_dir)
        .await
        .unwrap();
    let handle2 = manager
        .spawn("echo", &["test2".to_string()], &env, &working_dir)
        .await
        .unwrap();
    let handle3 = manager
        .spawn("echo", &["test3".to_string()], &env, &working_dir)
        .await
        .unwrap();

    // Assert
    assert_eq!(handle1.pid, 1);
    assert_eq!(handle2.pid, 2);
    assert_eq!(handle3.pid, 3);
}

/// Test spawn process with environment variables
#[tokio::test]
async fn test_spawn_process_with_env() {
    // Arrange
    let manager = ProcessManager::new();
    let mut env = HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value".to_string());
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
    let working_dir = PathBuf::from("/tmp");

    // Act
    let result = manager.spawn("env", &[], &env, &working_dir).await;

    // Assert
    assert!(result.is_ok());
}

/// Test spawn process with custom working directory
#[tokio::test]
async fn test_spawn_process_custom_working_dir() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/var/tmp");

    // Act
    let result = manager.spawn("pwd", &[], &env, &working_dir).await;

    // Assert
    assert!(result.is_ok());
}

/// Test spawn process with arguments
#[tokio::test]
async fn test_spawn_process_with_arguments() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");
    let args = vec!["-la".to_string(), "/tmp".to_string()];

    // Act
    let result = manager.spawn("ls", &args, &env, &working_dir).await;

    // Assert
    assert!(result.is_ok());
}

/// Test get process status for running process
#[tokio::test]
async fn test_get_status_running_process() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");
    let handle = manager
        .spawn("sleep", &["10".to_string()], &env, &working_dir)
        .await
        .unwrap();

    // Act
    let result = manager.get_status(handle.pid).await;

    // Assert
    assert!(result.is_ok());
    // Note: Process might have already exited in test environment
}

/// Test get process status for non-existent process
#[tokio::test]
async fn test_get_status_nonexistent_process() {
    // Arrange
    let manager = ProcessManager::new();

    // Act
    let result = manager.get_status(99999).await;

    // Assert
    assert!(result.is_err());
}

/// Test send signal to process
///
/// Per spec-kit/003-backend-spec.md section 3
#[tokio::test]
async fn test_send_signal_to_process() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");
    let handle = manager
        .spawn("sleep", &["10".to_string()], &env, &working_dir)
        .await
        .unwrap();

    // Act
    let result = manager.send_signal(handle.pid, 15).await; // SIGTERM

    // Assert
    assert!(result.is_ok());
}

/// Test send signal to non-existent process
#[tokio::test]
async fn test_send_signal_nonexistent_process() {
    // Arrange
    let manager = ProcessManager::new();

    // Act
    let result = manager.send_signal(99999, 15).await;

    // Assert
    assert!(result.is_err());
}

/// Test kill process
#[tokio::test]
async fn test_kill_process() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");
    let handle = manager
        .spawn("sleep", &["10".to_string()], &env, &working_dir)
        .await
        .unwrap();

    // Act
    let result = manager.kill(handle.pid).await;

    // Assert
    assert!(result.is_ok());
}

/// Test kill non-existent process
#[tokio::test]
async fn test_kill_nonexistent_process() {
    // Arrange
    let manager = ProcessManager::new();

    // Act
    let result = manager.kill(99999).await;

    // Assert
    assert!(result.is_err());
}

/// Test remove process from registry
#[tokio::test]
async fn test_remove_process() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");
    let handle = manager
        .spawn("echo", &["test".to_string()], &env, &working_dir)
        .await
        .unwrap();

    // Act
    let result = manager.remove_process(handle.pid).await;

    // Assert
    assert!(result.is_ok());
}

/// Test remove non-existent process
#[tokio::test]
async fn test_remove_nonexistent_process() {
    // Arrange
    let manager = ProcessManager::new();

    // Act
    let result = manager.remove_process(99999).await;

    // Assert - should succeed (idempotent operation)
    assert!(result.is_ok());
}

/// Test list processes after spawning
#[tokio::test]
async fn test_list_processes_after_spawn() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act - spawn multiple processes
    let handle1 = manager
        .spawn("sleep", &["10".to_string()], &env, &working_dir)
        .await
        .unwrap();
    let handle2 = manager
        .spawn("sleep", &["10".to_string()], &env, &working_dir)
        .await
        .unwrap();
    let processes = manager.list_processes().await;

    // Assert
    assert!(processes.len() >= 2);
    assert!(processes.iter().any(|p| p.pid == handle1.pid));
    assert!(processes.iter().any(|p| p.pid == handle2.pid));
}

/// Test sequential process spawning
///
/// Per NFR-3.3: Support multiple concurrent users
#[tokio::test]
async fn test_sequential_process_spawning() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act - spawn processes sequentially
    let mut results = vec![];
    for _ in 0..10 {
        let result = manager
            .spawn("echo", &["test".to_string()], &env, &working_dir)
            .await;
        results.push(result);
    }

    // Assert - all spawns should complete
    for result in results {
        assert!(result.is_ok());
    }
}

/// Test process handle contains correct information
#[tokio::test]
async fn test_process_handle_information() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act
    let handle = manager
        .spawn("ls", &["-la".to_string()], &env, &working_dir)
        .await
        .unwrap();

    // Assert
    assert!(handle.pid > 0);
    assert_eq!(handle.command, "ls");
    assert!(handle.started_at.elapsed().as_secs() < 1);
}

/// Test spawn process with empty command
#[tokio::test]
async fn test_spawn_process_empty_command() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act
    let result = manager.spawn("", &[], &env, &working_dir).await;

    // Assert - should fail with empty command
    assert!(result.is_err());
}

/// Test spawn multiple different commands
#[tokio::test]
async fn test_spawn_multiple_different_commands() {
    // Arrange
    let manager = ProcessManager::new();
    let env = HashMap::new();
    let working_dir = PathBuf::from("/tmp");

    // Act
    let handle1 = manager
        .spawn("echo", &["test1".to_string()], &env, &working_dir)
        .await
        .unwrap();
    let handle2 = manager
        .spawn("ls", &["-la".to_string()], &env, &working_dir)
        .await
        .unwrap();
    let handle3 = manager.spawn("pwd", &[], &env, &working_dir).await.unwrap();

    // Assert
    assert_ne!(handle1.pid, handle2.pid);
    assert_ne!(handle2.pid, handle3.pid);
    assert_eq!(handle1.command, "echo");
    assert_eq!(handle2.command, "ls");
    assert_eq!(handle3.command, "pwd");
}
