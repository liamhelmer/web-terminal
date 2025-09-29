// Configuration module
// Per spec-kit/003-backend-spec.md section 6

pub mod server;

pub use server::{LoggingConfig, SecurityConfig, ServerConfig};

use crate::error::Result;
use crate::session::SessionConfig;
use serde::{Deserialize, Serialize};

/// Main application configuration
/// Per spec-kit/003-backend-spec.md section 6
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from environment and default values
    /// Per spec-kit/003-backend-spec.md: Configuration from environment
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server: ServerConfig::default(),
            session: SessionConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            session: SessionConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}
