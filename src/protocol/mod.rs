//! WebSocket protocol module
//!
//! Implements message types and protocol handling
//! Per spec-kit/007-websocket-spec.md

pub mod messages;

pub use messages::{
    error_codes, ClientMessage, ConnectionStatus, FlowControlAction, ServerMessage, Signal,
    MAX_MESSAGE_SIZE,
};
