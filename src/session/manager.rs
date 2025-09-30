//! Session manager implementation
//!
//! Implements SessionManager with DashMap for in-memory storage
//! as specified in spec-kit/003-backend-spec.md section 2.1

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use super::state::{Session, SessionId, UserId};
use crate::error::{Error, Result};

/// Session configuration
/// Per spec-kit/003-backend-spec.md section 2.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout duration
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Maximum sessions per user
    pub max_sessions_per_user: usize,
    /// Workspace quota in bytes
    pub workspace_quota: u64,
    /// Maximum processes per session
    pub max_processes: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30 * 60), // 30 minutes
            max_sessions_per_user: 10,
            workspace_quota: 1024 * 1024 * 1024, // 1GB
            max_processes: 10,
        }
    }
}

/// Session manager for tracking and managing sessions
/// Uses DashMap for concurrent in-memory storage (per ADR-000)
/// Per spec-kit/003-backend-spec.md section 2.1
pub struct SessionManager {
    /// Session registry (session_id -> Session)
    sessions: DashMap<SessionId, Arc<Session>>,
    /// User sessions tracking (user_id -> Vec<SessionId>)
    user_sessions: DashMap<UserId, Vec<SessionId>>,
    /// Configuration
    config: SessionConfig,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionConfig) -> Self {
        tracing::info!("Initializing SessionManager with config: {:?}", config);
        Self {
            sessions: DashMap::new(),
            user_sessions: DashMap::new(),
            config,
        }
    }

    /// Create a new session for a user
    /// Per spec-kit/003-backend-spec.md section 2.1
    pub async fn create_session(&self, user_id: UserId) -> Result<Arc<Session>> {
        // Check user session limit
        if let Some(sessions) = self.user_sessions.get(&user_id) {
            if sessions.len() >= self.config.max_sessions_per_user {
                return Err(Error::SessionLimitExceeded(format!(
                    "User {} has reached maximum session limit of {}",
                    user_id, self.config.max_sessions_per_user
                )));
            }
        }

        // Create workspace directory
        let workspace_root = PathBuf::from(format!("/workspace/{}", user_id));

        // Create session
        let session = Session::new(user_id.clone(), workspace_root);
        let session_id = session.id.clone();
        let session_arc = Arc::new(session);

        // Store session
        self.sessions
            .insert(session_id.clone(), session_arc.clone());

        // Track user sessions
        self.user_sessions
            .entry(user_id.clone())
            .or_insert_with(Vec::new)
            .push(session_id.clone());

        tracing::info!("Created session {} for user {}", session_id, user_id);
        Ok(session_arc)
    }

    /// Get a session by ID
    /// Per spec-kit/003-backend-spec.md section 2.1
    pub async fn get_session(&self, session_id: &SessionId) -> Result<Arc<Session>> {
        self.sessions
            .get(session_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))
    }

    /// Get the owner of a session
    /// Returns the user ID that owns the session
    pub fn get_session_owner(&self, session_id: &SessionId) -> Result<UserId> {
        self.sessions
            .get(session_id)
            .map(|entry| entry.value().user_id.clone())
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))
    }

    /// Update session activity timestamp
    pub async fn touch_session(&self, session_id: &SessionId) -> Result<()> {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            let session = Arc::make_mut(entry.value_mut());
            session.touch();
            Ok(())
        } else {
            Err(Error::SessionNotFound(session_id.to_string()))
        }
    }

    /// Destroy a session
    /// Per spec-kit/003-backend-spec.md section 2.1
    pub async fn destroy_session(&self, session_id: &SessionId) -> Result<()> {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            // Kill all processes
            session.kill_all_processes().await?;

            // Clean up file system
            session.cleanup_filesystem().await?;

            // Remove from user sessions
            if let Some(mut user_sessions) = self.user_sessions.get_mut(&session.user_id) {
                user_sessions.retain(|id| id != session_id);
            }

            tracing::info!("Destroyed session {}", session_id);
            Ok(())
        } else {
            Err(Error::SessionNotFound(session_id.to_string()))
        }
    }

    /// List all sessions for a user
    pub async fn list_user_sessions(&self, user_id: &UserId) -> Vec<SessionId> {
        self.user_sessions
            .get(user_id)
            .map(|sessions| sessions.clone())
            .unwrap_or_default()
    }

    /// List all sessions (for admin/API use)
    /// Returns basic session info without full state
    pub async fn list_sessions(&self) -> Vec<Arc<Session>> {
        self.sessions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get total session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get session count for a specific user
    pub fn user_session_count(&self, user_id: &UserId) -> usize {
        self.user_sessions
            .get(user_id)
            .map(|sessions| sessions.len())
            .unwrap_or(0)
    }

    /// Clean up expired sessions
    /// Per spec-kit/003-backend-spec.md section 2.1
    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let now = Instant::now();
        let mut expired = Vec::new();

        // Find expired sessions
        for entry in self.sessions.iter() {
            let session = entry.value();
            if now.duration_since(session.last_activity) > self.config.timeout {
                expired.push(entry.key().clone());
            }
        }

        let count = expired.len();

        // Destroy expired sessions
        for session_id in expired {
            if let Err(e) = self.destroy_session(&session_id).await {
                tracing::error!("Failed to destroy expired session {}: {}", session_id, e);
            }
        }

        if count > 0 {
            tracing::info!("Cleaned up {} expired sessions", count);
        }

        Ok(count)
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Run every minute
            loop {
                interval.tick().await;
                if let Err(e) = self.cleanup_expired_sessions().await {
                    tracing::error!("Failed to cleanup expired sessions: {}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let manager = SessionManager::new(SessionConfig::default());
        let user_id = UserId::new("test_user".to_string());

        let session = manager.create_session(user_id.clone()).await.unwrap();
        assert_eq!(session.user_id, user_id);
        assert_eq!(manager.session_count(), 1);
    }

    #[tokio::test]
    async fn test_get_session() {
        let manager = SessionManager::new(SessionConfig::default());
        let user_id = UserId::new("test_user".to_string());

        let session = manager.create_session(user_id.clone()).await.unwrap();
        let session_id = session.id.clone();

        let retrieved = manager.get_session(&session_id).await.unwrap();
        assert_eq!(retrieved.id, session_id);
    }

    #[tokio::test]
    async fn test_session_limit() {
        let config = SessionConfig {
            max_sessions_per_user: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(config);
        let user_id = UserId::new("test_user".to_string());

        // Create max sessions
        manager.create_session(user_id.clone()).await.unwrap();
        manager.create_session(user_id.clone()).await.unwrap();

        // Try to exceed limit
        let result = manager.create_session(user_id.clone()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_destroy_session() {
        let manager = SessionManager::new(SessionConfig::default());
        let user_id = UserId::new("test_user".to_string());

        let session = manager.create_session(user_id.clone()).await.unwrap();
        let session_id = session.id.clone();

        assert_eq!(manager.session_count(), 1);

        manager.destroy_session(&session_id).await.unwrap();
        assert_eq!(manager.session_count(), 0);

        let result = manager.get_session(&session_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let config = SessionConfig {
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let manager = SessionManager::new(config);
        let user_id = UserId::new("test_user".to_string());

        manager.create_session(user_id.clone()).await.unwrap();
        assert_eq!(manager.session_count(), 1);

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(150)).await;

        let cleaned = manager.cleanup_expired_sessions().await.unwrap();
        assert_eq!(cleaned, 1);
        assert_eq!(manager.session_count(), 0);
    }

    #[tokio::test]
    async fn test_list_user_sessions() {
        let manager = SessionManager::new(SessionConfig::default());
        let user_id = UserId::new("test_user".to_string());

        let session1 = manager.create_session(user_id.clone()).await.unwrap();
        let session2 = manager.create_session(user_id.clone()).await.unwrap();

        let sessions = manager.list_user_sessions(&user_id).await;
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&session1.id));
        assert!(sessions.contains(&session2.id));
    }
}
