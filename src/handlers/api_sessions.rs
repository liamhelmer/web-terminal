// REST API Session handlers
// Per docs/spec-kit/006-api-spec.md - Session Management API

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use validator::Validate;

use crate::error::{Error, Result};
use crate::handlers::api_types::*;
use crate::server::middleware::auth::UserContext;
use crate::session::manager::SessionManager;
use crate::session::state::SessionId;

/// POST /api/v1/sessions - Create new terminal session
///
/// Per docs/spec-kit/006-api-spec.md - Create Session
/// Requires JWT authentication (extracted from middleware)
pub async fn create_session(
    session_manager: web::Data<Arc<SessionManager>>,
    user_ctx: web::ReqData<UserContext>,
    req: web::Json<CreateSessionRequest>,
) -> Result<HttpResponse> {
    // Validate input
    req.validate()
        .map_err(|e| Error::validation(format!("Invalid request: {}", e)))?;

    tracing::info!(
        user = %user_ctx.user_id,
        initial_dir = ?req.initial_dir,
        "Creating new terminal session"
    );

    // Create session
    let session = session_manager
        .create_session(user_ctx.user_id.clone())
        .await?;

    let response = CreateSessionResponse {
        id: session.id.to_string(),
        user_id: session.user_id.to_string(),
        created_at: chrono::DateTime::<Utc>::from(
            std::time::SystemTime::UNIX_EPOCH + session.created_at.elapsed(),
        )
        .to_rfc3339(),
        state: SessionState {
            working_dir: session
                .get_working_dir()
                .await
                .to_string_lossy()
                .to_string(),
            environment: session.get_environment().await,
            processes: vec![],
        },
    };

    Ok(HttpResponse::Created().json(response))
}

/// GET /api/v1/sessions/{id} - Get session details
///
/// Per docs/spec-kit/006-api-spec.md - Get Session
/// Requires JWT authentication
pub async fn get_session(
    session_manager: web::Data<Arc<SessionManager>>,
    user_ctx: web::ReqData<UserContext>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id = SessionId::new(path.into_inner());

    tracing::debug!(
        user = %user_ctx.user_id,
        session_id = %session_id,
        "Fetching session details"
    );

    // Get session
    let session = session_manager.get_session(&session_id).await?;

    // Authorization check: user can only access their own sessions
    if session.user_id != user_ctx.user_id {
        return Err(Error::forbidden(
            "You are not authorized to access this session",
        ));
    }

    let response = GetSessionResponse {
        id: session.id.to_string(),
        user_id: session.user_id.to_string(),
        created_at: chrono::DateTime::<Utc>::from(
            std::time::SystemTime::UNIX_EPOCH + session.created_at.elapsed(),
        )
        .to_rfc3339(),
        last_activity: chrono::DateTime::<Utc>::from(
            std::time::SystemTime::UNIX_EPOCH + session.last_activity.elapsed(),
        )
        .to_rfc3339(),
        state: SessionState {
            working_dir: session
                .get_working_dir()
                .await
                .to_string_lossy()
                .to_string(),
            environment: session.get_environment().await,
            processes: vec![],
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/sessions - List sessions
///
/// Per docs/spec-kit/006-api-spec.md - List Sessions
/// Requires JWT authentication
/// Lists only the authenticated user's sessions
pub async fn list_sessions(
    session_manager: web::Data<Arc<SessionManager>>,
    user_ctx: web::ReqData<UserContext>,
    query: web::Query<ListSessionsQuery>,
) -> Result<HttpResponse> {
    // Validate query parameters
    query
        .validate()
        .map_err(|e| Error::validation(format!("Invalid query parameters: {}", e)))?;

    let limit = query.limit.unwrap_or(10).min(100);
    let offset = query.offset.unwrap_or(0);

    tracing::debug!(
        user = %user_ctx.user_id,
        limit = limit,
        offset = offset,
        "Listing user sessions"
    );

    // Get all sessions for the user
    let all_sessions = session_manager.list_sessions().await;

    // Filter to user's sessions only
    let user_sessions: Vec<_> = all_sessions
        .into_iter()
        .filter(|s| s.user_id == user_ctx.user_id)
        .collect();

    let total = user_sessions.len();

    // Apply pagination
    let sessions: Vec<SessionSummary> = user_sessions
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|s| SessionSummary {
            id: s.id.to_string(),
            user_id: s.user_id.to_string(),
            created_at: chrono::DateTime::<Utc>::from(
                std::time::SystemTime::UNIX_EPOCH + s.created_at.elapsed(),
            )
            .to_rfc3339(),
            last_activity: chrono::DateTime::<Utc>::from(
                std::time::SystemTime::UNIX_EPOCH + s.last_activity.elapsed(),
            )
            .to_rfc3339(),
        })
        .collect();

    let response = ListSessionsResponse {
        sessions,
        total,
        limit,
        offset,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// DELETE /api/v1/sessions/{id} - Delete session
///
/// Per docs/spec-kit/006-api-spec.md - Delete Session
/// Requires JWT authentication
pub async fn delete_session(
    session_manager: web::Data<Arc<SessionManager>>,
    user_ctx: web::ReqData<UserContext>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let session_id = SessionId::new(path.into_inner());

    tracing::info!(
        user = %user_ctx.user_id,
        session_id = %session_id,
        "Deleting terminal session"
    );

    // Get session first to check authorization
    let session = session_manager.get_session(&session_id).await?;

    // Authorization check: user can only delete their own sessions
    if session.user_id != user_ctx.user_id {
        return Err(Error::forbidden(
            "You are not authorized to delete this session",
        ));
    }

    // Delete the session
    session_manager.destroy_session(&session_id).await?;

    Ok(HttpResponse::NoContent().finish())
}

/// GET /api/v1/sessions/{id}/history - Get command history
///
/// Per docs/spec-kit/006-api-spec.md - Get Session History
/// Requires JWT authentication
pub async fn get_session_history(
    session_manager: web::Data<Arc<SessionManager>>,
    user_ctx: web::ReqData<UserContext>,
    path: web::Path<String>,
    query: web::Query<SessionHistoryQuery>,
) -> Result<HttpResponse> {
    let session_id = SessionId::new(path.into_inner());

    // Validate query parameters
    query
        .validate()
        .map_err(|e| Error::validation(format!("Invalid query parameters: {}", e)))?;

    let limit = query.limit.unwrap_or(100).min(1000);

    tracing::debug!(
        user = %user_ctx.user_id,
        session_id = %session_id,
        limit = limit,
        "Fetching session command history"
    );

    // Get session first to check authorization
    let session = session_manager.get_session(&session_id).await?;

    // Authorization check
    if session.user_id != user_ctx.user_id {
        return Err(Error::forbidden(
            "You are not authorized to access this session history",
        ));
    }

    // Get command history
    let history = session.get_history().await;
    let history_entries: Vec<HistoryEntry> = history
        .iter()
        .rev() // Most recent first
        .take(limit as usize)
        .enumerate()
        .map(|(i, cmd)| HistoryEntry {
            timestamp: chrono::DateTime::<Utc>::from(
                std::time::SystemTime::UNIX_EPOCH
                    + session.created_at.elapsed()
                    + std::time::Duration::from_secs(i as u64),
            )
            .to_rfc3339(),
            command: cmd.clone(),
            exit_code: None,
        })
        .collect();

    let response = SessionHistoryResponse {
        history: history_entries,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::manager::SessionConfig;
    use crate::session::state::UserId;

    #[actix_web::test]
    async fn test_create_session_request_validation() {
        let req = CreateSessionRequest {
            initial_dir: Some("/workspace".to_string()),
            environment: None,
        };
        assert!(req.validate().is_ok());

        // Test path traversal attempt
        let bad_req = CreateSessionRequest {
            initial_dir: Some("../../../etc/passwd".to_string()),
            environment: None,
        };
        // Validation passes (path traversal is blocked at execution layer)
        assert!(bad_req.validate().is_ok());
    }

    #[actix_web::test]
    async fn test_list_sessions_pagination_validation() {
        let query = ListSessionsQuery {
            limit: Some(50),
            offset: Some(0),
        };
        assert!(query.validate().is_ok());

        let bad_query = ListSessionsQuery {
            limit: Some(200), // Over max of 100
            offset: Some(0),
        };
        assert!(bad_query.validate().is_err());
    }
}
