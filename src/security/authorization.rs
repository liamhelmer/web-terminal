//! Authorization service for role-based access control
//!
//! Per spec-kit/011-authentication-spec.md section 5: Authorization Model
//! Implements permission checking and resource ownership validation

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::session::state::{SessionId, UserId};

/// Authorization errors
#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid role: {0}")]
    InvalidRole(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for authorization operations
pub type Result<T> = std::result::Result<T, AuthorizationError>;

/// Permissions that can be granted to users
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Create a new session
    CreateSession,
    /// View a session (own or any based on ownership rules)
    ViewSession,
    /// Send input to a session
    SendInput,
    /// Kill a session
    KillSession,
    /// List all sessions (not just own)
    ListAllSessions,
    /// Kill any session (not just own)
    KillAnySession,
}

impl Permission {
    /// Convert permission to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateSession => "create_session",
            Self::ViewSession => "view_session",
            Self::SendInput => "send_input",
            Self::KillSession => "kill_session",
            Self::ListAllSessions => "list_all_sessions",
            Self::KillAnySession => "kill_any_session",
        }
    }
}

/// User role for authorization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
    ReadOnly,
}

impl Role {
    /// Parse role from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            "readonly" => Ok(Self::ReadOnly),
            _ => Err(AuthorizationError::InvalidRole(s.to_string())),
        }
    }

    /// Get default permissions for this role
    pub fn default_permissions(&self) -> Vec<Permission> {
        match self {
            Self::Admin => vec![
                Permission::CreateSession,
                Permission::ViewSession,
                Permission::SendInput,
                Permission::KillSession,
                Permission::ListAllSessions,
                Permission::KillAnySession,
            ],
            Self::User => vec![
                Permission::CreateSession,
                Permission::ViewSession,
                Permission::SendInput,
                Permission::KillSession,
            ],
            Self::ReadOnly => vec![Permission::ViewSession],
        }
    }
}

/// Ownership rules for resource access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipRules {
    /// Users can view their own sessions
    #[serde(default = "default_true")]
    pub own_sessions_view: bool,

    /// Users can kill their own sessions
    #[serde(default = "default_true")]
    pub own_sessions_kill: bool,

    /// Users can send input to their own sessions
    #[serde(default = "default_true")]
    pub own_sessions_input: bool,
}

fn default_true() -> bool {
    true
}

impl Default for OwnershipRules {
    fn default() -> Self {
        Self {
            own_sessions_view: true,
            own_sessions_kill: true,
            own_sessions_input: true,
        }
    }
}

/// Permission rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRules {
    /// Role-based permissions (role -> list of permissions)
    #[serde(default)]
    pub role_permissions: HashMap<String, Vec<Permission>>,

    /// Ownership rules for resource access
    #[serde(default)]
    pub ownership_rules: OwnershipRules,

    /// Default permissions for all authenticated users
    #[serde(default)]
    pub default_permissions: Vec<Permission>,
}

impl Default for PermissionRules {
    fn default() -> Self {
        let mut role_permissions = HashMap::new();
        role_permissions.insert("admin".to_string(), Role::Admin.default_permissions());
        role_permissions.insert("user".to_string(), Role::User.default_permissions());
        role_permissions.insert("readonly".to_string(), Role::ReadOnly.default_permissions());

        Self {
            role_permissions,
            ownership_rules: OwnershipRules::default(),
            default_permissions: vec![Permission::CreateSession, Permission::ViewSession],
        }
    }
}

impl PermissionRules {
    /// Load permission rules from YAML file
    pub fn from_yaml_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AuthorizationError::ConfigError(format!("Failed to read permissions file: {}", e))
        })?;

        serde_yaml::from_str(&content).map_err(|e| {
            AuthorizationError::ConfigError(format!("Failed to parse permissions YAML: {}", e))
        })
    }
}

/// Authorization service for checking permissions
pub struct AuthorizationService {
    rules: Arc<PermissionRules>,
}

impl AuthorizationService {
    /// Create a new authorization service with given rules
    pub fn new(rules: PermissionRules) -> Self {
        Self {
            rules: Arc::new(rules),
        }
    }

    /// Create authorization service from config file
    pub fn from_config_file(path: &Path) -> Result<Self> {
        let rules = PermissionRules::from_yaml_file(path)?;
        Ok(Self::new(rules))
    }

    /// Create authorization service with default rules
    pub fn with_defaults() -> Self {
        Self::new(PermissionRules::default())
    }

    /// Check if user has a specific permission
    ///
    /// # Arguments
    /// * `user_id` - User identifier
    /// * `role` - User's role
    /// * `permission` - Permission to check
    /// * `resource_owner` - Optional resource owner (for ownership checks)
    pub fn check_permission(
        &self,
        user_id: &UserId,
        role: &str,
        permission: Permission,
        resource_owner: Option<&UserId>,
    ) -> Result<()> {
        // Check role-based permissions
        if let Some(role_perms) = self.rules.role_permissions.get(role) {
            if role_perms.contains(&permission) {
                return Ok(());
            }
        }

        // Check default permissions
        if self.rules.default_permissions.contains(&permission) {
            return Ok(());
        }

        // Check ownership-based permissions
        if let Some(owner) = resource_owner {
            if user_id == owner {
                match permission {
                    Permission::ViewSession if self.rules.ownership_rules.own_sessions_view => {
                        return Ok(())
                    }
                    Permission::KillSession if self.rules.ownership_rules.own_sessions_kill => {
                        return Ok(())
                    }
                    Permission::SendInput if self.rules.ownership_rules.own_sessions_input => {
                        return Ok(())
                    }
                    _ => {}
                }
            }
        }

        Err(AuthorizationError::PermissionDenied(format!(
            "User {} with role {} does not have permission {}",
            user_id.as_str(),
            role,
            permission.as_str()
        )))
    }

    /// Check if user owns a session
    pub fn check_session_ownership(
        &self,
        user_id: &UserId,
        session_owner: &UserId,
    ) -> Result<()> {
        if user_id == session_owner {
            Ok(())
        } else {
            Err(AuthorizationError::PermissionDenied(format!(
                "User {} does not own this session (owned by {})",
                user_id.as_str(),
                session_owner.as_str()
            )))
        }
    }

    /// Get all permissions for a role
    pub fn get_role_permissions(&self, role: &str) -> Vec<Permission> {
        self.rules
            .role_permissions
            .get(role)
            .cloned()
            .unwrap_or_else(|| self.rules.default_permissions.clone())
    }

    /// Check if user can perform action on session
    ///
    /// Convenience method that combines permission and ownership checks
    pub fn authorize_session_action(
        &self,
        user_id: &UserId,
        role: &str,
        permission: Permission,
        session_owner: &UserId,
    ) -> Result<()> {
        self.check_permission(user_id, role, permission, Some(session_owner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user() -> UserId {
        UserId::new("user:default/alice".to_string())
    }

    fn test_admin() -> UserId {
        UserId::new("user:default/admin".to_string())
    }

    fn test_other_user() -> UserId {
        UserId::new("user:default/bob".to_string())
    }

    #[test]
    fn test_role_default_permissions() {
        let admin_perms = Role::Admin.default_permissions();
        assert!(admin_perms.contains(&Permission::CreateSession));
        assert!(admin_perms.contains(&Permission::KillAnySession));
        assert!(admin_perms.contains(&Permission::ListAllSessions));

        let user_perms = Role::User.default_permissions();
        assert!(user_perms.contains(&Permission::CreateSession));
        assert!(!user_perms.contains(&Permission::KillAnySession));

        let readonly_perms = Role::ReadOnly.default_permissions();
        assert!(!readonly_perms.contains(&Permission::CreateSession));
        assert!(readonly_perms.contains(&Permission::ViewSession));
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let service = AuthorizationService::with_defaults();
        let admin = test_admin();
        let user = test_user();

        // Admin can do everything
        assert!(service
            .check_permission(&admin, "admin", Permission::CreateSession, None)
            .is_ok());
        assert!(service
            .check_permission(&admin, "admin", Permission::KillAnySession, None)
            .is_ok());
        assert!(service
            .check_permission(&admin, "admin", Permission::ListAllSessions, None)
            .is_ok());

        // Admin can access other users' resources
        assert!(service
            .authorize_session_action(&admin, "admin", Permission::ViewSession, &user)
            .is_ok());
        assert!(service
            .authorize_session_action(&admin, "admin", Permission::KillSession, &user)
            .is_ok());
    }

    #[test]
    fn test_user_can_access_own_resources() {
        let service = AuthorizationService::with_defaults();
        let user = test_user();

        // User can view, kill, and send input to their own sessions
        assert!(service
            .authorize_session_action(&user, "user", Permission::ViewSession, &user)
            .is_ok());
        assert!(service
            .authorize_session_action(&user, "user", Permission::KillSession, &user)
            .is_ok());
        assert!(service
            .authorize_session_action(&user, "user", Permission::SendInput, &user)
            .is_ok());
    }

    #[test]
    fn test_user_cannot_access_others_resources() {
        let service = AuthorizationService::with_defaults();
        let user = test_user();
        let other = test_other_user();

        // User cannot view other users' sessions without ListAllSessions permission
        assert!(service
            .authorize_session_action(&user, "user", Permission::ViewSession, &other)
            .is_err());
        assert!(service
            .authorize_session_action(&user, "user", Permission::KillSession, &other)
            .is_err());
    }

    #[test]
    fn test_readonly_user_cannot_modify() {
        let service = AuthorizationService::with_defaults();
        let user = test_user();

        // Readonly cannot create or kill sessions
        assert!(service
            .check_permission(&user, "readonly", Permission::CreateSession, None)
            .is_err());
        assert!(service
            .authorize_session_action(&user, "readonly", Permission::KillSession, &user)
            .is_err());

        // Readonly can view own sessions
        assert!(service
            .authorize_session_action(&user, "readonly", Permission::ViewSession, &user)
            .is_ok());
    }

    #[test]
    fn test_ownership_check() {
        let service = AuthorizationService::with_defaults();
        let user = test_user();
        let other = test_other_user();

        assert!(service.check_session_ownership(&user, &user).is_ok());
        assert!(service.check_session_ownership(&user, &other).is_err());
    }

    #[test]
    fn test_get_role_permissions() {
        let service = AuthorizationService::with_defaults();

        let admin_perms = service.get_role_permissions("admin");
        assert!(admin_perms.contains(&Permission::KillAnySession));

        let user_perms = service.get_role_permissions("user");
        assert!(!user_perms.contains(&Permission::KillAnySession));
        assert!(user_perms.contains(&Permission::CreateSession));

        let unknown_perms = service.get_role_permissions("unknown_role");
        assert_eq!(unknown_perms.len(), 2); // Returns default permissions
    }
}