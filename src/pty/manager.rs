// PTY manager for lifecycle management
// Per spec-kit/003-backend-spec.md

use super::{PtyConfig, PtyError, PtyProcess, PtyProcessHandle, PtyReader, PtyResult, PtyWriter};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// PTY manager for managing multiple PTY processes per session
///
/// Per FR-4: Session Management
/// Per spec-kit/002-architecture.md: Use DashMap for in-memory storage
pub struct PtyManager {
    /// Active PTY processes by ID
    processes: Arc<DashMap<String, PtyProcessHandle>>,

    /// Default configuration for new PTY processes
    default_config: PtyConfig,
}

impl PtyManager {
    /// Create a new PTY manager
    pub fn new(default_config: PtyConfig) -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            default_config,
        }
    }

    /// Create a new PTY manager with default configuration
    pub fn with_defaults() -> Self {
        Self::new(PtyConfig::default())
    }

    /// Spawn a new PTY process
    ///
    /// Per FR-1.2.1: Start processes for executed commands
    /// Per NFR-1.1.5: Session creation time < 200ms
    pub fn spawn(&self, config: Option<PtyConfig>) -> PtyResult<PtyProcessHandle> {
        let config = config.unwrap_or_else(|| self.default_config.clone());

        let handle = PtyProcess::spawn(config)?;

        // Store in registry
        self.processes
            .insert(handle.id().to_string(), handle.clone());

        tracing::info!(
            "PTY manager: spawned process {} (total: {})",
            handle.id(),
            self.processes.len()
        );

        Ok(handle)
    }

    /// Spawn a PTY process with custom shell
    pub fn spawn_with_shell(
        &self,
        shell_path: &str,
        args: Vec<String>,
        config: Option<PtyConfig>,
    ) -> PtyResult<PtyProcessHandle> {
        let mut config = config.unwrap_or_else(|| self.default_config.clone());

        config.shell.shell_path = std::path::PathBuf::from(shell_path);
        config.shell.args = args;

        self.spawn(Some(config))
    }

    /// Get a PTY process by ID
    pub fn get(&self, id: &str) -> PtyResult<PtyProcessHandle> {
        self.processes
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| PtyError::ProcessNotFound(id.to_string()))
    }

    /// Remove a PTY process from the registry
    ///
    /// Per FR-4.1.5: Clean up session resources on close
    pub fn remove(&self, id: &str) -> PtyResult<PtyProcessHandle> {
        self.processes
            .remove(id)
            .map(|(_, handle)| handle)
            .ok_or_else(|| PtyError::ProcessNotFound(id.to_string()))
    }

    /// Kill and remove a PTY process
    pub async fn kill(&self, id: &str) -> PtyResult<()> {
        let handle = self.remove(id)?;
        handle.kill().await?;

        tracing::info!(
            "PTY manager: killed process {} (total: {})",
            id,
            self.processes.len()
        );

        Ok(())
    }

    /// Resize a PTY process
    ///
    /// Per FR-2.1.5: Support terminal dimensions
    pub async fn resize(&self, id: &str, cols: u16, rows: u16) -> PtyResult<()> {
        let handle = self.get(id)?;
        handle.resize(cols, rows).await
    }

    /// Create a reader for PTY output streaming
    pub fn create_reader(&self, id: &str, buffer_size: Option<usize>) -> PtyResult<PtyReader> {
        let handle = self.get(id)?;
        let buffer_size = buffer_size.unwrap_or(4096);
        Ok(PtyReader::new(handle, buffer_size))
    }

    /// Create a writer for PTY input
    pub fn create_writer(&self, id: &str) -> PtyResult<PtyWriter> {
        let handle = self.get(id)?;
        Ok(PtyWriter::new(handle))
    }

    /// Start streaming output from a PTY to a channel
    ///
    /// Per FR-3.3: Real-time streaming
    pub async fn stream_output(
        &self,
        id: &str,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> PtyResult<()> {
        let reader = self.create_reader(id, None)?;
        reader.stream_output(tx).await
    }

    /// List all active PTY processes
    pub fn list(&self) -> Vec<String> {
        self.processes
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Count active PTY processes
    pub fn count(&self) -> usize {
        self.processes.len()
    }

    /// Cleanup dead processes
    ///
    /// Per FR-4.1.5: Clean up session resources on close
    pub async fn cleanup_dead_processes(&self) -> PtyResult<usize> {
        let mut dead = Vec::new();

        for entry in self.processes.iter() {
            let handle = entry.value();
            if !handle.is_alive().await {
                dead.push(entry.key().clone());
            }
        }

        let count = dead.len();
        for id in dead {
            self.processes.remove(&id);
            tracing::debug!("Cleaned up dead PTY process {}", id);
        }

        if count > 0 {
            tracing::info!("PTY manager: cleaned up {} dead processes", count);
        }

        Ok(count)
    }

    /// Kill all PTY processes
    ///
    /// Per FR-4.1.5: Clean up session resources on close
    pub async fn kill_all(&self) -> PtyResult<usize> {
        let ids: Vec<String> = self.list();
        let count = ids.len();

        for id in ids {
            if let Err(e) = self.kill(&id).await {
                tracing::error!("Failed to kill PTY process {}: {}", id, e);
            }
        }

        tracing::info!("PTY manager: killed all {} processes", count);

        Ok(count)
    }

    /// Wait for a PTY process to exit
    pub async fn wait(&self, id: &str) -> PtyResult<Option<i32>> {
        let handle = self.get(id)?;
        handle.wait().await
    }

    /// Check if a PTY process is alive
    pub async fn is_alive(&self, id: &str) -> bool {
        if let Ok(handle) = self.get(id) {
            handle.is_alive().await
        } else {
            false
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_manager_spawn() {
        let manager = PtyManager::with_defaults();
        let handle = manager.spawn(None).expect("Failed to spawn PTY");

        assert!(manager.is_alive(handle.id()).await);
        assert_eq!(manager.count(), 1);

        manager.kill(handle.id()).await.expect("Failed to kill PTY");
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_pty_manager_resize() {
        let manager = PtyManager::with_defaults();
        let handle = manager.spawn(None).expect("Failed to spawn PTY");

        manager
            .resize(handle.id(), 120, 40)
            .await
            .expect("Failed to resize PTY");

        let config = handle.config().await;
        assert_eq!(config.cols, 120);
        assert_eq!(config.rows, 40);

        manager.kill(handle.id()).await.expect("Failed to kill PTY");
    }

    #[tokio::test]
    async fn test_pty_manager_cleanup() {
        let manager = PtyManager::with_defaults();
        let handle = manager.spawn(None).expect("Failed to spawn PTY");

        // Kill the process
        handle.kill().await.expect("Failed to kill PTY");

        // Give it time to exit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Cleanup should remove the dead process
        let count = manager
            .cleanup_dead_processes()
            .await
            .expect("Failed to cleanup");

        assert_eq!(count, 1);
        assert_eq!(manager.count(), 0);
    }
}
