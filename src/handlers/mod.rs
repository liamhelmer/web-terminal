//! HTTP handlers for API endpoints
//!
//! Per spec-kit/006-api-spec.md

pub mod api_health;
pub mod api_sessions;
pub mod api_types;
pub mod sessions;

// Re-export REST API handlers
pub use api_health::health_check;
pub use api_sessions::{
    create_session, delete_session, get_session, get_session_history, list_sessions,
};
pub use api_types::*;

// Legacy handlers (will be removed)
pub use sessions::{
    create_session_handler, kill_session_handler, list_sessions_handler, send_input_handler,
};
