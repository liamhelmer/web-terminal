//! Process management for PTY
//!
//! Implements PTY process spawning and management
//! Per spec-kit/003-backend-spec.md section 3

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use portable_pty::{CommandBuilder, PtySize, PtySystem, native_pty_system};
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::session::state::{ProcessHandle, ProcessId};

/// Process manager for PTY processes
pub struct ProcessManager {
    pty_system: Box<dyn PtySystem + Send>,
    processes: Arc<RwLock<HashMap<ProcessId, ProcessInfo>>>,
    next_pid: Arc<RwLock<ProcessId>>,
}

/// Process information
#[derive(Debug, Clone)]
struct ProcessInfo {
    pub pid: ProcessId,
    pub command: String,
    pub started_at: Instant,
    pub status: ProcessStatus,
}

/// Process status
#[derive(Debug, Clone, Copy)]
pub enum ProcessStatus {
    Running,
    Exited(i32),
    Signaled(i32),
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        Self {
            pty_system: native_pty_system(),
            processes: Arc::new(RwLock::new(HashMap::new())),
            next_pid: Arc::new(RwLock::new(1)),
        }
    }

    /// Spawn a new PTY process
    /// Per spec-kit/003-backend-spec.md section 3
    pub async fn spawn(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        working_dir: &std::path::Path,
    ) -> Result<ProcessHandle> {
        // Allocate process ID
        let mut next_pid = self.next_pid.write().await;
        let pid = *next_pid;
        *next_pid += 1;
        drop(next_pid);

        // Create PTY pair
        let pair = self.pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::ProcessSpawnFailed(e.to_string()))?;

        // Create command
        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.cwd(working_dir);

        // Spawn child process
        let child = pair.slave.spawn_command(cmd)
            .map_err(|e| Error::ProcessSpawnFailed(e.to_string()))?;

        // Store process info
        let process_info = ProcessInfo {
            pid,
            command: format!("{} {}", command, args.join(" ")),
            started_at: Instant::now(),
            status: ProcessStatus::Running,
        };

        let mut processes = self.processes.write().await;
        processes.insert(pid, process_info);
        drop(processes);

        tracing::info!("Spawned process {} ({})", pid, command);

        Ok(ProcessHandle {
            pid,
            command: command.to_string(),
            started_at: Instant::now(),
        })
    }

    /// Send signal to process
    /// Per spec-kit/003-backend-spec.md section 3
    pub async fn send_signal(&self, pid: ProcessId, signal: i32) -> Result<()> {
        let processes = self.processes.read().await;
        if let Some(info) = processes.get(&pid) {
            tracing::info!("Sending signal {} to process {}", signal, pid);
            // TODO: Implement actual signal sending via PTY
            Ok(())
        } else {
            Err(Error::ProcessNotFound(pid))
        }
    }

    /// Kill a process
    pub async fn kill(&self, pid: ProcessId) -> Result<()> {
        self.send_signal(pid, 9).await // SIGKILL
    }

    /// Get process status
    pub async fn get_status(&self, pid: ProcessId) -> Result<ProcessStatus> {
        let processes = self.processes.read().await;
        processes.get(&pid)
            .map(|info| info.status)
            .ok_or(Error::ProcessNotFound(pid))
    }

    /// List all running processes
    pub async fn list_processes(&self) -> Vec<ProcessHandle> {
        let processes = self.processes.read().await;
        processes.values()
            .filter(|info| matches!(info.status, ProcessStatus::Running))
            .map(|info| ProcessHandle {
                pid: info.pid,
                command: info.command.clone(),
                started_at: info.started_at,
            })
            .collect()
    }

    /// Remove process from registry
    pub async fn remove_process(&self, pid: ProcessId) -> Result<()> {
        let mut processes = self.processes.write().await;
        processes.remove(&pid);
        tracing::info!("Removed process {}", pid);
        Ok(())
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager_creation() {
        let manager = ProcessManager::new();
        let processes = manager.list_processes().await;
        assert_eq!(processes.len(), 0);
    }

    // Note: Additional tests require actual process spawning which may not work in all test environments
}