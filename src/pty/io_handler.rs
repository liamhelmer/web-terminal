// PTY I/O handling with async streaming
// Per spec-kit/003-backend-spec.md

use super::{PtyError, PtyProcessHandle, PtyResult};
use std::io::{Read, Write};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

/// PTY reader for async output streaming
///
/// Per FR-3.3: Real-time streaming (<20ms latency)
/// Per FR-1.2.3: Capture stdout and stderr streams
pub struct PtyReader {
    handle: PtyProcessHandle,
    buffer_size: usize,
}

impl PtyReader {
    /// Create a new PTY reader
    pub fn new(handle: PtyProcessHandle, buffer_size: usize) -> Self {
        Self {
            handle,
            buffer_size,
        }
    }

    /// Start streaming output to a channel
    ///
    /// Per NFR-1.1.2: WebSocket message latency < 20ms (p95)
    pub async fn stream_output(
        self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> PtyResult<()> {
        let handle_id = self.handle.id().to_string();
        let buffer_size = self.buffer_size;

        // Get reader outside of async context
        let reader = {
            let inner = self.handle.get_master().await;
            let mut inner = inner.write().await;
            inner.get_reader()?
        };

        // Spawn blocking task for reading (PTY I/O is blocking)
        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buffer = vec![0u8; buffer_size];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF - process exited
                        tracing::debug!("PTY {} reached EOF", handle_id);
                        break;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();

                        if tx.send(data).is_err() {
                            // Channel closed, stop reading
                            tracing::debug!("PTY {} output channel closed", handle_id);
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            // No data available, continue
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            continue;
                        }

                        tracing::error!("PTY {} read error: {}", handle_id, e);
                        break;
                    }
                }
            }

            tracing::info!("PTY {} reader stopped", handle_id);
        });

        Ok(())
    }

    /// Read output synchronously (blocking)
    pub fn read_blocking(&mut self, buf: &mut [u8]) -> PtyResult<usize> {
        let inner = self.handle.get_master_blocking();
        let mut inner = inner.blocking_write();

        if inner.is_closed() {
            return Err(PtyError::AlreadyClosed);
        }

        let mut reader = inner
            .get_reader()
            .map_err(|e| PtyError::IoError(e))?;

        reader
            .read(buf)
            .map_err(|e| PtyError::IoError(e))
    }
}

/// PTY writer for async input handling
///
/// Per FR-2.2: Input Handling
pub struct PtyWriter {
    handle: PtyProcessHandle,
}

impl PtyWriter {
    /// Create a new PTY writer
    pub fn new(handle: PtyProcessHandle) -> Self {
        Self { handle }
    }

    /// Write input to the PTY
    ///
    /// Per FR-2.2.1: Capture keyboard input in real-time
    pub async fn write(&self, data: &[u8]) -> PtyResult<usize> {
        // Get writer outside of async context
        let mut writer = {
            let inner = self.handle.get_master().await;
            let mut inner = inner.write().await;

            if inner.is_closed() {
                return Err(PtyError::AlreadyClosed);
            }

            inner.get_writer().map_err(|e| PtyError::IoError(e))?
        };

        // Spawn blocking task for writing
        let data = data.to_vec();
        let result = tokio::task::spawn_blocking(move || -> PtyResult<usize> {
            let n = writer
                .write(&data)
                .map_err(|e| PtyError::IoError(e))?;

            writer
                .flush()
                .map_err(|e| PtyError::IoError(e))?;

            Ok(n)
        })
        .await
        .map_err(|e| PtyError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Task join error: {}", e),
        )))??;

        Ok(result)
    }

    /// Write a string to the PTY
    pub async fn write_str(&self, s: &str) -> PtyResult<usize> {
        self.write(s.as_bytes()).await
    }

    /// Write data synchronously (blocking)
    pub fn write_blocking(&mut self, data: &[u8]) -> PtyResult<usize> {
        let inner = self.handle.get_master_blocking();
        let mut inner = inner.blocking_write();

        if inner.is_closed() {
            return Err(PtyError::AlreadyClosed);
        }

        let mut writer = inner
            .get_writer()
            .map_err(|e| PtyError::IoError(e))?;

        let n = writer
            .write(data)
            .map_err(|e| PtyError::IoError(e))?;

        writer
            .flush()
            .map_err(|e| PtyError::IoError(e))?;

        Ok(n)
    }
}