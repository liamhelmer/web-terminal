// PTY process management
// Per spec-kit/003-backend-spec.md

use super::{PtyConfig, PtyError, PtyResult};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Handle to a running PTY process
#[derive(Clone)]
pub struct PtyProcessHandle {
    id: String,
    inner: Arc<RwLock<PtyProcessInner>>,
}

pub(crate) struct PtyProcessInner {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    config: PtyConfig,
    closed: bool,
}

impl PtyProcessHandle {
    /// Get the process ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Resize the PTY
    ///
    /// Per FR-2.1.5: Support terminal dimensions (rows, columns)
    pub async fn resize(&self, cols: u16, rows: u16) -> PtyResult<()> {
        let mut inner = self.inner.write().await;

        if inner.closed {
            return Err(PtyError::AlreadyClosed);
        }

        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        inner
            .master
            .resize(size)
            .map_err(|e| PtyError::ResizeFailed(e.to_string()))?;

        // Update stored config
        inner.config.cols = cols;
        inner.config.rows = rows;

        tracing::debug!("Resized PTY {} to {}x{}", self.id, cols, rows);

        Ok(())
    }

    /// Check if the PTY process is still running
    pub async fn is_alive(&self) -> bool {
        let mut inner = self.inner.write().await;
        if inner.closed {
            return false;
        }

        // Try to get exit status without blocking
        matches!(inner.child.try_wait(), Ok(None))
    }

    /// Wait for the process to exit and get exit status
    pub async fn wait(&self) -> PtyResult<Option<i32>> {
        let mut inner = self.inner.write().await;

        if inner.closed {
            return Err(PtyError::AlreadyClosed);
        }

        let status = inner.child.wait().map_err(|e| PtyError::IoError(e))?;

        let exit_code = Some(status.exit_code() as i32);

        tracing::info!("PTY process {} exited with code {:?}", self.id, exit_code);

        Ok(exit_code)
    }

    /// Kill the PTY process
    ///
    /// Per FR-1.2.4: Support process termination (Ctrl+C / SIGINT)
    pub async fn kill(&self) -> PtyResult<()> {
        let mut inner = self.inner.write().await;

        if inner.closed {
            return Err(PtyError::AlreadyClosed);
        }

        inner
            .child
            .kill()
            .map_err(|e| PtyError::SignalFailed(e.to_string()))?;

        inner.closed = true;

        tracing::info!("Killed PTY process {}", self.id);

        Ok(())
    }

    /// Get the master PTY for I/O operations (async)
    pub(crate) async fn get_master(&self) -> Arc<RwLock<PtyProcessInner>> {
        self.inner.clone()
    }

    /// Get the master PTY for I/O operations (blocking)
    pub(crate) fn get_master_blocking(&self) -> Arc<RwLock<PtyProcessInner>> {
        self.inner.clone()
    }

    /// Get current PTY configuration
    pub async fn config(&self) -> PtyConfig {
        let inner = self.inner.read().await;
        inner.config.clone()
    }
}

/// PTY process builder and spawner
pub struct PtyProcess;

impl PtyProcess {
    /// Spawn a new PTY process
    ///
    /// Per FR-1.2.1: Start processes for executed commands
    /// Per NFR-1.1: Session creation time < 200ms
    pub fn spawn(config: PtyConfig) -> PtyResult<PtyProcessHandle> {
        let pty_system = native_pty_system();

        // Create PTY with specified size
        let size = PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        // Build command with shell
        let mut cmd = CommandBuilder::new(&config.shell.shell_path);
        cmd.args(&config.shell.args);
        cmd.cwd(&config.working_dir);

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the child process
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        // Generate unique ID
        let id = uuid::Uuid::new_v4().to_string();

        let inner = PtyProcessInner {
            master: pair.master,
            child,
            config: config.clone(),
            closed: false,
        };

        tracing::info!(
            "Spawned PTY process {} with shell {} in {}",
            id,
            config.shell.shell_path.display(),
            config.working_dir.display()
        );

        Ok(PtyProcessHandle {
            id,
            inner: Arc::new(RwLock::new(inner)),
        })
    }
}

// Internal accessor for I/O operations
impl PtyProcessInner {
    pub(crate) fn get_reader(&mut self) -> std::io::Result<Box<dyn std::io::Read + Send>> {
        self.master
            .try_clone_reader()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub(crate) fn get_writer(&mut self) -> std::io::Result<Box<dyn std::io::Write + Send>> {
        self.master
            .take_writer()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }
}
