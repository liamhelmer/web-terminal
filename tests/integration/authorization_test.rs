//! Integration tests for authorization service
//!
//! Tests role-based access control and resource ownership checks
//! Per spec-kit/011-authentication-spec.md section 5: Authorization Model

use std::sync::Arc;
use std::time::Duration;

use web_terminal::error::Result;
use web_terminal::security::{AuthorizationService, Permission, PermissionRules};
use web_terminal::session::manager::{SessionConfig, SessionManager};
use web_terminal::session::state::UserId;

/// Helper to create test users
fn alice() -> UserId {
    UserId::new("user:default/alice".to_string())
}

fn bob() -> UserId {
    UserId::new("user:default/bob".to_string())
}

fn admin() -> UserId {
    UserId::new("user:default/admin".to_string())
}

fn readonly_user() -> UserId {
    UserId::new("user:default/readonly".to_string())
}

/// Test: User can create session
#[tokio::test]
async fn test_user_can_create_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let alice = alice();

    // User should be able to create session
    let result = authz.check_permission(&alice, "user", Permission::CreateSession, None);
    assert!(result.is_ok(), "User should be able to create session");

    Ok(())
}

/// Test: User can view own session
#[tokio::test]
async fn test_user_can_view_own_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();

    // Create session
    let session = session_manager.create_session(alice.clone()).await?;

    // Check authorization to view own session
    let result = authz.authorize_session_action(&alice, "user", Permission::ViewSession, &alice);
    assert!(result.is_ok(), "User should be able to view own session");

    Ok(())
}

/// Test: User cannot view other's session
#[tokio::test]
async fn test_user_cannot_view_others_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();
    let bob = bob();

    // Alice creates a session
    let _session = session_manager.create_session(alice.clone()).await?;

    // Bob tries to view Alice's session
    let result = authz.authorize_session_action(&bob, "user", Permission::ViewSession, &alice);
    assert!(
        result.is_err(),
        "User should NOT be able to view other's session"
    );

    Ok(())
}

/// Test: Admin can view any session
#[tokio::test]
async fn test_admin_can_view_any_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();
    let admin_user = admin();

    // Alice creates a session
    let _session = session_manager.create_session(alice.clone()).await?;

    // Admin views Alice's session
    let result =
        authz.authorize_session_action(&admin_user, "admin", Permission::ViewSession, &alice);
    assert!(result.is_ok(), "Admin should be able to view any session");

    Ok(())
}

/// Test: User can kill own session
#[tokio::test]
async fn test_user_can_kill_own_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();

    // Create session
    let session = session_manager.create_session(alice.clone()).await?;

    // Check authorization to kill own session
    let result = authz.authorize_session_action(&alice, "user", Permission::KillSession, &alice);
    assert!(result.is_ok(), "User should be able to kill own session");

    // Actually kill the session
    session_manager.destroy_session(&session.id).await?;

    Ok(())
}

/// Test: User cannot kill other's session
#[tokio::test]
async fn test_user_cannot_kill_others_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();
    let bob = bob();

    // Alice creates a session
    let _session = session_manager.create_session(alice.clone()).await?;

    // Bob tries to kill Alice's session
    let result = authz.authorize_session_action(&bob, "user", Permission::KillSession, &alice);
    assert!(
        result.is_err(),
        "User should NOT be able to kill other's session"
    );

    Ok(())
}

/// Test: Admin can kill any session
#[tokio::test]
async fn test_admin_can_kill_any_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();
    let admin_user = admin();

    // Alice creates a session
    let session = session_manager.create_session(alice.clone()).await?;

    // Admin kills Alice's session
    let result =
        authz.authorize_session_action(&admin_user, "admin", Permission::KillSession, &alice);
    assert!(result.is_ok(), "Admin should be able to kill any session");

    // Actually kill the session
    session_manager.destroy_session(&session.id).await?;

    Ok(())
}

/// Test: Role-based permissions work correctly
#[tokio::test]
async fn test_role_based_permissions() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let alice = alice();

    // Admin role has all permissions
    assert!(authz
        .check_permission(&alice, "admin", Permission::CreateSession, None)
        .is_ok());
    assert!(authz
        .check_permission(&alice, "admin", Permission::KillAnySession, None)
        .is_ok());
    assert!(authz
        .check_permission(&alice, "admin", Permission::ListAllSessions, None)
        .is_ok());

    // User role has limited permissions
    assert!(authz
        .check_permission(&alice, "user", Permission::CreateSession, None)
        .is_ok());
    assert!(authz
        .check_permission(&alice, "user", Permission::KillAnySession, None)
        .is_err());
    assert!(authz
        .check_permission(&alice, "user", Permission::ListAllSessions, None)
        .is_err());

    // Readonly role has very limited permissions
    assert!(authz
        .check_permission(&alice, "readonly", Permission::CreateSession, None)
        .is_err());
    assert!(authz
        .check_permission(&alice, "readonly", Permission::ViewSession, None)
        .is_ok());

    Ok(())
}

/// Test: Default permissions for all authenticated users
#[tokio::test]
async fn test_default_permissions() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let alice = alice();

    // Default permissions should allow CreateSession and ViewSession
    // even for unknown roles
    assert!(authz
        .check_permission(&alice, "unknown_role", Permission::CreateSession, None)
        .is_ok());
    assert!(authz
        .check_permission(&alice, "unknown_role", Permission::ViewSession, None)
        .is_ok());

    // But not other permissions
    assert!(authz
        .check_permission(&alice, "unknown_role", Permission::KillAnySession, None)
        .is_err());

    Ok(())
}

/// Test: Ownership checks work correctly
#[tokio::test]
async fn test_session_ownership_checks() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();
    let bob = bob();

    // Alice creates a session
    let session = session_manager.create_session(alice.clone()).await?;

    // Get session owner
    let owner = session_manager.get_session_owner(&session.id)?;
    assert_eq!(owner, alice);

    // Check ownership
    assert!(authz.check_session_ownership(&alice, &owner).is_ok());
    assert!(authz.check_session_ownership(&bob, &owner).is_err());

    Ok(())
}

/// Test: ReadOnly user cannot create or kill sessions
#[tokio::test]
async fn test_readonly_user_restrictions() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let readonly = readonly_user();

    // Cannot create sessions
    assert!(authz
        .check_permission(&readonly, "readonly", Permission::CreateSession, None)
        .is_err());

    // Cannot kill sessions (even own)
    assert!(authz
        .authorize_session_action(&readonly, "readonly", Permission::KillSession, &readonly)
        .is_err());

    // Cannot send input
    assert!(authz
        .authorize_session_action(&readonly, "readonly", Permission::SendInput, &readonly)
        .is_err());

    // Can view sessions
    assert!(authz
        .authorize_session_action(&readonly, "readonly", Permission::ViewSession, &readonly)
        .is_ok());

    Ok(())
}

/// Test: User can send input to own session
#[tokio::test]
async fn test_user_can_send_input_to_own_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let alice = alice();

    let result = authz.authorize_session_action(&alice, "user", Permission::SendInput, &alice);
    assert!(
        result.is_ok(),
        "User should be able to send input to own session"
    );

    Ok(())
}

/// Test: User cannot send input to other's session
#[tokio::test]
async fn test_user_cannot_send_input_to_others_session() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let alice = alice();
    let bob = bob();

    let result = authz.authorize_session_action(&bob, "user", Permission::SendInput, &alice);
    assert!(
        result.is_err(),
        "User should NOT be able to send input to other's session"
    );

    Ok(())
}

/// Test: Admin can list all sessions
#[tokio::test]
async fn test_admin_can_list_all_sessions() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let admin_user = admin();

    let result = authz.check_permission(&admin_user, "admin", Permission::ListAllSessions, None);
    assert!(result.is_ok(), "Admin should be able to list all sessions");

    Ok(())
}

/// Test: Regular user cannot list all sessions
#[tokio::test]
async fn test_user_cannot_list_all_sessions() -> Result<()> {
    let authz = AuthorizationService::with_defaults();
    let alice = alice();

    let result = authz.check_permission(&alice, "user", Permission::ListAllSessions, None);
    assert!(
        result.is_err(),
        "Regular user should NOT be able to list all sessions"
    );

    Ok(())
}

/// Test: Multiple concurrent users with different permissions
#[tokio::test]
async fn test_concurrent_users_different_permissions() -> Result<()> {
    let authz = Arc::new(AuthorizationService::with_defaults());
    let session_manager = Arc::new(SessionManager::new(SessionConfig::default()));

    let alice = alice();
    let bob = bob();
    let admin_user = admin();

    // Create sessions for different users
    let alice_session = session_manager.create_session(alice.clone()).await?;
    let bob_session = session_manager.create_session(bob.clone()).await?;

    // Alice can only access her own session
    assert!(authz
        .authorize_session_action(&alice, "user", Permission::ViewSession, &alice)
        .is_ok());
    assert!(authz
        .authorize_session_action(&alice, "user", Permission::ViewSession, &bob)
        .is_err());

    // Bob can only access his own session
    assert!(authz
        .authorize_session_action(&bob, "user", Permission::ViewSession, &bob)
        .is_ok());
    assert!(authz
        .authorize_session_action(&bob, "user", Permission::ViewSession, &alice)
        .is_err());

    // Admin can access all sessions
    assert!(authz
        .authorize_session_action(&admin_user, "admin", Permission::ViewSession, &alice)
        .is_ok());
    assert!(authz
        .authorize_session_action(&admin_user, "admin", Permission::ViewSession, &bob)
        .is_ok());

    // Cleanup
    session_manager.destroy_session(&alice_session.id).await?;
    session_manager.destroy_session(&bob_session.id).await?;

    Ok(())
}
