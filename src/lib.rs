// Web-Terminal library
// Per spec-kit/003-backend-spec.md

pub mod cli;
pub mod config;
pub mod error;
pub mod execution;
pub mod filesystem;
pub mod handlers;
pub mod monitoring;
pub mod protocol;
pub mod pty;
pub mod security;
pub mod server;
pub mod session;

// Re-export commonly used types
pub use error::{Error, Result};