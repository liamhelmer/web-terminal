// REST API request/response types
// Per docs/spec-kit/006-api-spec.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

// ===== Session API Types =====

/// Request to create a new terminal session
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSessionRequest {
    /// Initial working directory (optional)
    #[validate(length(max = 4096))]
    pub initial_dir: Option<String>,

    /// Environment variables (optional)
    pub environment: Option<HashMap<String, String>>,
}

/// Response for session creation
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub id: String,
    pub user_id: String,
    pub created_at: String,
    pub state: SessionState,
}

/// Session state information
#[derive(Debug, Serialize)]
pub struct SessionState {
    pub working_dir: String,
    pub environment: HashMap<String, String>,
    pub processes: Vec<ProcessInfo>,
}

/// Process information
#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub status: String,
}

/// Response for getting session details
#[derive(Debug, Serialize)]
pub struct GetSessionResponse {
    pub id: String,
    pub user_id: String,
    pub created_at: String,
    pub last_activity: String,
    pub state: SessionState,
}

/// Query parameters for listing sessions
#[derive(Debug, Deserialize, Validate)]
pub struct ListSessionsQuery {
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,

    pub offset: Option<u32>,
}

/// Response for listing sessions
#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummary>,
    pub total: usize,
    pub limit: u32,
    pub offset: u32,
}

/// Session summary for list view
#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub user_id: String,
    pub created_at: String,
    pub last_activity: String,
}

/// Query parameters for session history
#[derive(Debug, Deserialize, Validate)]
pub struct SessionHistoryQuery {
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<u32>,
}

/// Response for session history
#[derive(Debug, Serialize)]
pub struct SessionHistoryResponse {
    pub history: Vec<HistoryEntry>,
}

/// Command history entry
#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub command: String,
    pub exit_code: Option<i32>,
}

// ===== Health API Types =====

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

/// Individual health check results
#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub sessions: String,
    pub memory: String,
    pub disk: String,
}

// ===== Error Response Types =====

/// Standard API error response
/// Per docs/spec-kit/006-api-spec.md - Error Responses
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub www_authenticate: Option<String>,
}

/// Error details
#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ErrorDetails {
                code: code.into(),
                message: message.into(),
                details: None,
            },
            www_authenticate: None,
        }
    }

    /// Create error with details
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: ErrorDetails {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            },
            www_authenticate: None,
        }
    }

    /// Add WWW-Authenticate header info
    pub fn with_www_authenticate(mut self, challenge: impl Into<String>) -> Self {
        self.www_authenticate = Some(challenge.into());
        self
    }

    /// Session not found error (404)
    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        Self::with_details(
            "SESSION_NOT_FOUND",
            format!("Session with ID '{}' not found", session_id),
            serde_json::json!({ "session_id": session_id }),
        )
    }

    /// JWT expired error (401)
    pub fn jwt_expired(expired_at: impl Into<String>) -> Self {
        Self::with_details(
            "JWT_EXPIRED",
            "JWT token has expired",
            serde_json::json!({ "expired_at": expired_at.into() }),
        )
        .with_www_authenticate(
            "Bearer realm=\"web-terminal\", error=\"invalid_token\", error_description=\"JWT token has expired\"",
        )
    }

    /// JWT invalid error (401)
    pub fn jwt_invalid() -> Self {
        Self::with_details(
            "JWT_INVALID",
            "JWT token is invalid",
            serde_json::json!({ "reason": "invalid_token" }),
        )
        .with_www_authenticate(
            "Bearer realm=\"web-terminal\", error=\"invalid_token\", error_description=\"JWT token is invalid\"",
        )
    }

    /// Session expired error (410)
    pub fn session_expired() -> Self {
        Self::new("SESSION_EXPIRED", "Session has expired")
    }

    /// Not found error (404)
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message)
    }

    /// Forbidden error (403)
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new("FORBIDDEN", message)
    }

    /// Unauthorized user error (403)
    pub fn unauthorized_user(user: impl Into<String>, required_groups: Vec<String>) -> Self {
        Self::with_details(
            "UNAUTHORIZED_USER",
            "User is not authorized to perform this action",
            serde_json::json!({
                "user": user.into(),
                "required_groups": required_groups
            }),
        )
    }

    /// Validation error (422)
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    /// Rate limit exceeded error (429)
    pub fn rate_limit_exceeded() -> Self {
        Self::new(
            "RATE_LIMIT_EXCEEDED",
            "Too many requests. Please try again later.",
        )
    }

    /// Internal server error (500)
    pub fn internal_error() -> Self {
        Self::new("INTERNAL_ERROR", "An internal server error occurred")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_session_not_found() {
        let err = ErrorResponse::session_not_found("session123");
        assert_eq!(err.error.code, "SESSION_NOT_FOUND");
        assert!(err.error.message.contains("session123"));
        assert!(err.error.details.is_some());
    }

    #[test]
    fn test_error_response_jwt_expired() {
        let err = ErrorResponse::jwt_expired("2025-09-29T08:00:00Z");
        assert_eq!(err.error.code, "JWT_EXPIRED");
        assert!(err.www_authenticate.is_some());
    }

    #[test]
    fn test_create_session_request_validation() {
        let req = CreateSessionRequest {
            initial_dir: Some("/workspace".to_string()),
            environment: Some(HashMap::new()),
        };
        assert!(req.validate().is_ok());
    }
}
