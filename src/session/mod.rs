//! Session management module
//!
//! Provides session lifecycle management, state tracking, and registry
//! as specified in spec-kit/003-backend-spec.md section 2

pub mod manager;
pub mod registry;
pub mod state;

pub use manager::{SessionConfig, SessionManager};
pub use registry::SessionRegistry;
pub use state::{ProcessHandle, ProcessId, Session, SessionId, SessionState, UserId};
