//! Process execution module
//!
//! Implements command execution and process management
//! Per spec-kit/003-backend-spec.md section 3

pub mod process;

pub use process::{ProcessManager, ProcessStatus};
