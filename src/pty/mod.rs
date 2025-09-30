// PTY (Pseudo-Terminal) spawning and management
// Per spec-kit/003-backend-spec.md and spec-kit/001-requirements.md
//
// Requirements:
// - FR-1.2: Process Management (start, monitor, capture I/O, signals, limits)
// - FR-3.3: Real-time streaming (<20ms latency)
// - NFR-1.1: Command execution latency < 100ms (p95)

mod config;
mod io_handler;
mod manager;
mod process;

pub use config::{PtyConfig, ShellConfig};
pub use io_handler::{PtyReader, PtyWriter};
pub use manager::PtyManager;
pub use process::{PtyProcess, PtyProcessHandle};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("Failed to spawn PTY: {0}")]
    SpawnFailed(String),

    #[error("PTY process not found: {0}")]
    ProcessNotFound(String),

    #[error("PTY I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("PTY already closed")]
    AlreadyClosed,

    #[error("Invalid PTY configuration: {0}")]
    InvalidConfig(String),

    #[error("PTY resize failed: {0}")]
    ResizeFailed(String),

    #[error("Signal send failed: {0}")]
    SignalFailed(String),
}

pub type PtyResult<T> = Result<T, PtyError>;
