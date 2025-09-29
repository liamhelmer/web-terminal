// Server configuration
// Per spec-kit/003-backend-spec.md section 6

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Server configuration
/// Per spec-kit/003-backend-spec.md: Single-port deployment (default 8080)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port (default 8080)
    /// Per spec-kit: Single port for all services
    #[serde(default = "default_port")]
    pub port: u16,

    /// TLS certificate path (optional)
    pub tls_cert: Option<PathBuf>,

    /// TLS key path (optional)
    pub tls_key: Option<PathBuf>,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Number of worker threads
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            tls_cert: None,
            tls_key: None,
            max_connections: default_max_connections(),
            worker_threads: default_worker_threads(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080 // Single port for HTTP, WebSocket, and static files
}

fn default_max_connections() -> usize {
    10000
}

fn default_worker_threads() -> usize {
    num_cpus::get()
}

/// Security configuration
/// Per spec-kit/003-backend-spec.md: JWT authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT secret key
    pub jwt_secret: String,

    /// JWT token expiry in seconds (default 8 hours)
    #[serde(default = "default_token_expiry")]
    pub token_expiry_secs: u64,

    /// Enable CORS
    #[serde(default = "default_cors_enabled")]
    pub cors_enabled: bool,

    /// Allowed origins for CORS
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change_me_in_production".to_string(),
            token_expiry_secs: default_token_expiry(),
            cors_enabled: default_cors_enabled(),
            cors_origins: vec!["*".to_string()],
        }
    }
}

fn default_token_expiry() -> u64 {
    8 * 3600 // 8 hours
}

fn default_cors_enabled() -> bool {
    true
}

/// Logging configuration
/// Per spec-kit/003-backend-spec.md: tracing + tracing-subscriber
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable JSON logging
    #[serde(default)]
    pub json: bool,

    /// Log file path (optional)
    pub file: Option<PathBuf>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            json: false,
            file: None,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 10000);
    }

    #[test]
    fn test_security_config_defaults() {
        let config = SecurityConfig::default();
        assert_eq!(config.token_expiry_secs, 8 * 3600);
        assert!(config.cors_enabled);
    }

    #[test]
    fn test_logging_config_defaults() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.json);
    }
}