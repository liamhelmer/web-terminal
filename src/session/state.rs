//! Session state management
//!
//! Implements session state structure as per spec-kit/003-backend-spec.md section 2.2

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Result;

/// Unique session identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new random session ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create a SessionId from a string
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Get the session ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// User identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    /// Create a new user ID
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Get the user ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for UserId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl FromStr for UserId {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Self(s.to_string()))
    }
}

use std::str::FromStr;

/// Process identifier
pub type ProcessId = u32;

/// Process handle for tracking running processes
#[derive(Debug, Clone)]
pub struct ProcessHandle {
    pub pid: ProcessId,
    pub command: String,
    pub started_at: Instant,
}

/// Session state containing all session-specific data
/// Per spec-kit/003-backend-spec.md section 2.2
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Current working directory
    pub working_dir: PathBuf,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Command history
    pub command_history: Vec<String>,
    /// Running processes
    pub processes: HashMap<ProcessId, ProcessHandle>,
}

impl SessionState {
    /// Create default session state
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            working_dir: workspace_root,
            environment: Self::default_environment(),
            command_history: Vec::new(),
            processes: HashMap::new(),
        }
    }

    /// Get default environment variables
    fn default_environment() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env.insert("HOME".to_string(), "/workspace".to_string());
        env.insert("SHELL".to_string(), "/bin/bash".to_string());
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env
    }
}

/// Session struct containing session metadata and state
/// Per spec-kit/003-backend-spec.md section 2.2
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// User who owns this session
    pub user_id: UserId,
    /// When the session was created
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Session state (protected by RwLock for concurrent access)
    state: Arc<RwLock<SessionState>>,
}

impl Session {
    /// Create a new session
    pub fn new(user_id: UserId, workspace_root: PathBuf) -> Self {
        let id = SessionId::generate();
        let now = Instant::now();

        Self {
            id,
            user_id,
            created_at: now,
            last_activity: now,
            state: Arc::new(RwLock::new(SessionState::new(workspace_root))),
        }
    }

    /// Update the last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if session has expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        Instant::now().duration_since(self.last_activity) > timeout
    }

    /// Update working directory
    /// Per spec-kit/003-backend-spec.md section 2.2
    pub async fn update_working_dir(&self, path: PathBuf) -> Result<()> {
        let mut state = self.state.write().await;

        // Validate path is within workspace (security requirement)
        if !path.starts_with(&state.working_dir) {
            return Err(crate::error::Error::InvalidPath(
                "Path must be within workspace".to_string(),
            ));
        }

        state.working_dir = path;
        Ok(())
    }

    /// Add command to history
    /// Per spec-kit/003-backend-spec.md section 2.2
    pub async fn add_to_history(&self, command: String) {
        let mut state = self.state.write().await;
        state.command_history.push(command);

        // Limit history size to 1000 commands
        if state.command_history.len() > 1000 {
            state.command_history.remove(0);
        }
    }

    /// Get command history
    pub async fn get_history(&self) -> Vec<String> {
        let state = self.state.read().await;
        state.command_history.clone()
    }

    /// Get current working directory
    pub async fn get_working_dir(&self) -> PathBuf {
        let state = self.state.read().await;
        state.working_dir.clone()
    }

    /// Get environment variables
    pub async fn get_environment(&self) -> HashMap<String, String> {
        let state = self.state.read().await;
        state.environment.clone()
    }

    /// Set environment variable
    pub async fn set_env(&self, key: String, value: String) {
        let mut state = self.state.write().await;
        state.environment.insert(key, value);
    }

    /// Add running process
    pub async fn add_process(&self, handle: ProcessHandle) {
        let mut state = self.state.write().await;
        state.processes.insert(handle.pid, handle);
    }

    /// Remove process
    pub async fn remove_process(&self, pid: ProcessId) -> Option<ProcessHandle> {
        let mut state = self.state.write().await;
        state.processes.remove(&pid)
    }

    /// Get all running processes
    pub async fn get_processes(&self) -> Vec<ProcessHandle> {
        let state = self.state.read().await;
        state.processes.values().cloned().collect()
    }

    /// Set PTY process ID for this session
    pub async fn set_pty(&self, pty_id: String) {
        let mut state = self.state.write().await;
        // Store PTY ID in environment for reference
        state.environment.insert("PTY_ID".to_string(), pty_id);
    }

    /// Get PTY process ID
    pub async fn get_pty(&self) -> Option<String> {
        let state = self.state.read().await;
        state.environment.get("PTY_ID").cloned()
    }

    /// Kill all processes in this session
    pub async fn kill_all_processes(&self) -> Result<()> {
        let state = self.state.read().await;

        // TODO: Implement actual process killing
        // This will be implemented by the process management module
        tracing::info!("Killing {} processes for session {}", state.processes.len(), self.id);

        Ok(())
    }

    /// Clean up session filesystem
    pub async fn cleanup_filesystem(&self) -> Result<()> {
        let state = self.state.read().await;

        // TODO: Implement filesystem cleanup
        // This will be implemented by the filesystem module
        tracing::info!("Cleaning up filesystem for session {} at {:?}", self.id, state.working_dir);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::generate();
        let id2 = SessionId::generate();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_session_creation() {
        let user_id = UserId::new("test_user".to_string());
        let workspace = PathBuf::from("/workspace/test");
        let session = Session::new(user_id.clone(), workspace.clone());

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.get_working_dir().await, workspace);
    }

    #[tokio::test]
    async fn test_command_history() {
        let user_id = UserId::new("test_user".to_string());
        let workspace = PathBuf::from("/workspace/test");
        let session = Session::new(user_id, workspace);

        session.add_to_history("ls -la".to_string()).await;
        session.add_to_history("cd /tmp".to_string()).await;

        let history = session.get_history().await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], "ls -la");
        assert_eq!(history[1], "cd /tmp");
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let user_id = UserId::new("test_user".to_string());
        let workspace = PathBuf::from("/workspace/test");
        let session = Session::new(user_id, workspace);

        let timeout = Duration::from_secs(1);
        assert!(!session.is_expired(timeout));

        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(session.is_expired(timeout));
    }

    #[tokio::test]
    async fn test_environment_variables() {
        let user_id = UserId::new("test_user".to_string());
        let workspace = PathBuf::from("/workspace/test");
        let session = Session::new(user_id, workspace);

        session.set_env("MY_VAR".to_string(), "my_value".to_string()).await;

        let env = session.get_environment().await;
        assert_eq!(env.get("MY_VAR"), Some(&"my_value".to_string()));
    }
}