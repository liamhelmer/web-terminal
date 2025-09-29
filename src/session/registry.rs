//! Session registry utilities
//!
//! Provides additional session management utilities

use std::collections::HashMap;
use super::state::{SessionId, UserId};

/// Session metadata for registry
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub created_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}

/// Session registry for tracking session metadata
pub struct SessionRegistry {
    metadata: HashMap<SessionId, SessionMetadata>,
}

impl SessionRegistry {
    /// Create a new session registry
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    /// Register a session
    pub fn register(&mut self, metadata: SessionMetadata) {
        self.metadata.insert(metadata.session_id.clone(), metadata);
    }

    /// Unregister a session
    pub fn unregister(&mut self, session_id: &SessionId) -> Option<SessionMetadata> {
        self.metadata.remove(session_id)
    }

    /// Get session metadata
    pub fn get(&self, session_id: &SessionId) -> Option<&SessionMetadata> {
        self.metadata.get(session_id)
    }

    /// List all session IDs
    pub fn list_sessions(&self) -> Vec<SessionId> {
        self.metadata.keys().cloned().collect()
    }

    /// List sessions for a user
    pub fn list_user_sessions(&self, user_id: &UserId) -> Vec<SessionId> {
        self.metadata
            .values()
            .filter(|meta| &meta.user_id == user_id)
            .map(|meta| meta.session_id.clone())
            .collect()
    }

    /// Get total session count
    pub fn count(&self) -> usize {
        self.metadata.len()
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_register_unregister() {
        let mut registry = SessionRegistry::new();
        let session_id = SessionId::generate();
        let user_id = UserId::new("test_user".to_string());

        let metadata = SessionMetadata {
            session_id: session_id.clone(),
            user_id,
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
        };

        registry.register(metadata);
        assert_eq!(registry.count(), 1);

        let removed = registry.unregister(&session_id);
        assert!(removed.is_some());
        assert_eq!(registry.count(), 0);
    }
}