// Error types for web-terminal
// Per spec-kit/003-backend-spec.md

use thiserror::Error;

/// Main error type for web-terminal
#[derive(Debug, Error)]
pub enum Error {
    // Session errors
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session limit exceeded: {0}")]
    SessionLimitExceeded(String),

    #[error("Session expired: {0}")]
    SessionExpired(String),

    // Command execution errors
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Command not allowed: {0}")]
    CommandNotAllowed(String),

    #[error("Empty command")]
    EmptyCommand,

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    // Resource errors
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    // Authentication errors
    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Unauthorized")]
    Unauthorized,

    // PTY errors
    #[error("PTY error: {0}")]
    PtyError(String),

    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),

    #[error("Process not found: {0}")]
    ProcessNotFound(u32),

    // WebSocket errors
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    // I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // REST API errors
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    // Other errors
    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type using our Error
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Check if error is session-related
    pub fn is_session_error(&self) -> bool {
        matches!(
            self,
            Error::SessionNotFound(_) | Error::SessionLimitExceeded(_) | Error::SessionExpired(_)
        )
    }

    /// Check if error is security-related
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            Error::AuthenticationFailed | Error::InvalidToken | Error::Unauthorized
        )
    }

    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            Error::SessionNotFound(_) | Error::NotFound(_) | Error::ProcessNotFound(_) => 404,
            Error::SessionLimitExceeded(_) | Error::ResourceLimitExceeded(_) => 429,
            Error::SessionExpired(_) => 410,
            Error::InvalidCommand(_) | Error::CommandNotAllowed(_) | Error::EmptyCommand => 400,
            Error::InvalidPath(_) | Error::Serialization(_) => 400,
            Error::AuthenticationFailed | Error::InvalidToken => 401,
            Error::Unauthorized | Error::Forbidden(_) => 403,
            Error::ValidationError(_) => 422,
            Error::Io(_) | Error::Internal(_) | Error::PtyError(_) => 500,
            Error::ProcessSpawnFailed(_) | Error::WebSocketError(_) | Error::ExecutionFailed(_) | Error::Other(_) => 500,
        }
    }

    /// Session not found error
    pub fn session_not_found(session_id: impl std::fmt::Display) -> Self {
        Self::SessionNotFound(format!("{}", session_id))
    }

    /// Generic not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    /// Forbidden/insufficient permissions error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    /// Validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }
}

impl From<crate::pty::PtyError> for Error {
    fn from(e: crate::pty::PtyError) -> Self {
        Error::PtyError(e.to_string())
    }
}

/// Implement ResponseError for Actix-Web integration
/// Per spec-kit/006-api-spec.md: Structured error responses
impl actix_web::ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse {
        use crate::handlers::api_types::ErrorResponse;

        let error_response = match self {
            Error::SessionNotFound(id) => ErrorResponse::session_not_found(id),
            Error::SessionLimitExceeded(_) => ErrorResponse::rate_limit_exceeded(),
            Error::SessionExpired(_) => ErrorResponse::session_expired(),
            Error::InvalidCommand(_) | Error::CommandNotAllowed(_) | Error::EmptyCommand => {
                ErrorResponse::validation_error(format!("Command error: {}", self))
            }
            Error::AuthenticationFailed | Error::InvalidToken => {
                ErrorResponse::jwt_invalid()
            }
            Error::Unauthorized => ErrorResponse::unauthorized_user("unknown", vec![]),
            Error::NotFound(msg) => ErrorResponse::not_found(msg),
            Error::Forbidden(msg) => ErrorResponse::forbidden(msg),
            Error::ValidationError(msg) => ErrorResponse::validation_error(msg),
            _ => ErrorResponse::internal_error(),
        };

        actix_web::HttpResponse::build(actix_web::http::StatusCode::from_u16(self.status_code()).unwrap())
            .json(error_response)
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::from_u16(Error::status_code(self)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        assert!(Error::SessionNotFound("test".to_string()).is_session_error());
        assert!(Error::AuthenticationFailed.is_security_error());
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(Error::SessionNotFound("test".to_string()).status_code(), 404);
        assert_eq!(Error::AuthenticationFailed.status_code(), 401);
        assert_eq!(Error::InvalidCommand("test".to_string()).status_code(), 400);
    }
}