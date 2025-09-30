//! HTTP handlers for API endpoints
//!
//! Per spec-kit/006-api-spec.md

pub mod sessions;

pub use sessions::{
    create_session_handler, kill_session_handler, list_sessions_handler, send_input_handler,
};