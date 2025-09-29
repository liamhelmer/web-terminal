//! WebSocket protocol module
//!
//! Implements message types and protocol handling
//! Per spec-kit/007-websocket-spec.md

pub mod messages;

pub use messages::{ClientMessage, ConnectionStatus, ServerMessage, Signal};
