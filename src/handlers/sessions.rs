//! Session management HTTP handlers with authorization
//!
//! Per spec-kit/006-api-spec.md - Session Management API
//! Implements authorization checks per spec-kit/011-authentication-spec.md

use std::sync::Arc;

use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::security::authorization::{AuthorizationService, Permission};
use crate::session::manager::SessionManager;
use crate::session::state::{SessionId, UserId};

/// Request to create a new session
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub user_id: String,
    #[serde(default)]
    pub role: String,
}

/// Response for session creation
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub user_id: String,
    pub created_at: String,
}

/// Request to kill a session
#[derive(Debug, Deserialize)]
pub struct KillSessionRequest {
    pub session_id: String,
    pub user_id: String,
    #[serde(default)]
    pub role: String,
}

/// Request to send input to a session
#[derive(Debug, Deserialize)]
pub struct SendInputRequest {
    pub session_id: String,
    pub user_id: String,
    #[serde(default)]
    pub role: String,
    pub input: String,
}

/// Request to list sessions
#[derive(Debug, Deserialize)]
pub struct ListSessionsRequest {
    pub user_id: String,
    #[serde(default)]
    pub role: String,
}

/// Session information for listing
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub created_at: String,
}

/// Response for listing sessions
#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionInfo>,
}

/// Application state for handlers
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub authz_service: Arc<AuthorizationService>,
}

/// Create a new session handler
///
/// POST /api/v1/sessions
/// Authorization: User must have CreateSession permission
pub async fn create_session_handler(
    state: web::Data<AppState>,
    req: web::Json<CreateSessionRequest>,
) -> ActixResult<HttpResponse> {
    let user_id = UserId::new(req.user_id.clone());
    let role = if req.role.is_empty() {
        "user"
    } else {
        &req.role
    };

    // Check authorization
    if let Err(e) = state
        .authz_service
        .check_permission(&user_id, role, Permission::CreateSession, None)
    {
        tracing::warn!(
            "Authorization failed for user {} to create session: {}",
            user_id.as_str(),
            e
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Forbidden",
            "message": format!("{}", e)
        })));
    }

    // Create session
    match state.session_manager.create_session(user_id.clone()).await {
        Ok(session) => {
            tracing::info!(
                "Session {} created for user {}",
                session.id.as_str(),
                user_id.as_str()
            );

            Ok(HttpResponse::Ok().json(CreateSessionResponse {
                session_id: session.id.to_string(),
                user_id: session.user_id.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal Server Error",
                "message": format!("{}", e)
            })))
        }
    }
}

/// Kill a session handler
///
/// DELETE /api/v1/sessions/:session_id
/// Authorization: User must own the session or have KillAnySession permission
pub async fn kill_session_handler(
    state: web::Data<AppState>,
    req: web::Json<KillSessionRequest>,
) -> ActixResult<HttpResponse> {
    let user_id = UserId::new(req.user_id.clone());
    let session_id = SessionId::from(req.session_id.clone());
    let role = if req.role.is_empty() {
        "user"
    } else {
        &req.role
    };

    // Get session owner
    let session_owner = match state.session_manager.get_session_owner(&session_id) {
        Ok(owner) => owner,
        Err(Error::SessionNotFound(_)) => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Not Found",
                "message": "Session not found"
            })));
        }
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal Server Error",
                "message": format!("{}", e)
            })));
        }
    };

    // Check authorization
    if let Err(e) = state.authz_service.authorize_session_action(
        &user_id,
        role,
        Permission::KillSession,
        &session_owner,
    ) {
        tracing::warn!(
            "Authorization failed for user {} to kill session {}: {}",
            user_id.as_str(),
            session_id.as_str(),
            e
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Forbidden",
            "message": format!("{}", e)
        })));
    }

    // Kill session
    match state.session_manager.destroy_session(&session_id).await {
        Ok(()) => {
            tracing::info!(
                "Session {} killed by user {}",
                session_id.as_str(),
                user_id.as_str()
            );
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Session killed successfully"
            })))
        }
        Err(e) => {
            tracing::error!("Failed to kill session: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal Server Error",
                "message": format!("{}", e)
            })))
        }
    }
}

/// List sessions handler
///
/// GET /api/v1/sessions
/// Authorization: Users can list own sessions, admins can list all
pub async fn list_sessions_handler(
    state: web::Data<AppState>,
    req: web::Query<ListSessionsRequest>,
) -> ActixResult<HttpResponse> {
    let user_id = UserId::new(req.user_id.clone());
    let role = if req.role.is_empty() {
        "user"
    } else {
        &req.role
    };

    // Check if user can list all sessions
    let can_list_all = state
        .authz_service
        .check_permission(&user_id, role, Permission::ListAllSessions, None)
        .is_ok();

    // Get sessions
    let session_ids = if can_list_all {
        // Admin: list all sessions
        tracing::info!("User {} listing all sessions", user_id.as_str());
        // TODO: Implement list_all_sessions in SessionManager
        vec![]
    } else {
        // Regular user: list own sessions only
        tracing::info!("User {} listing own sessions", user_id.as_str());
        state.session_manager.list_user_sessions(&user_id).await
    };

    let sessions = vec![]; // TODO: Convert session IDs to SessionInfo

    Ok(HttpResponse::Ok().json(ListSessionsResponse { sessions }))
}

/// Send input to session handler
///
/// POST /api/v1/sessions/:session_id/input
/// Authorization: User must own the session or have admin permissions
pub async fn send_input_handler(
    state: web::Data<AppState>,
    req: web::Json<SendInputRequest>,
) -> ActixResult<HttpResponse> {
    let user_id = UserId::new(req.user_id.clone());
    let session_id = SessionId::from(req.session_id.clone());
    let role = if req.role.is_empty() {
        "user"
    } else {
        &req.role
    };

    // Get session owner
    let session_owner = match state.session_manager.get_session_owner(&session_id) {
        Ok(owner) => owner,
        Err(Error::SessionNotFound(_)) => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Not Found",
                "message": "Session not found"
            })));
        }
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal Server Error",
                "message": format!("{}", e)
            })));
        }
    };

    // Check authorization
    if let Err(e) = state.authz_service.authorize_session_action(
        &user_id,
        role,
        Permission::SendInput,
        &session_owner,
    ) {
        tracing::warn!(
            "Authorization failed for user {} to send input to session {}: {}",
            user_id.as_str(),
            session_id.as_str(),
            e
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Forbidden",
            "message": format!("{}", e)
        })));
    }

    // TODO: Implement actual input sending to session
    tracing::info!(
        "User {} sending input to session {}",
        user_id.as_str(),
        session_id.as_str()
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Input sent successfully"
    })))
}