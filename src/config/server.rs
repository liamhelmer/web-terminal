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

    /// TLS configuration (optional)
    pub tls: Option<TlsConfig>,

    /// CORS configuration
    #[serde(default)]
    pub cors: CorsConfig,

    /// Security headers configuration
    #[serde(default)]
    pub security_headers: SecurityHeadersConfig,

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
            tls: None,
            cors: CorsConfig::default(),
            security_headers: SecurityHeadersConfig::default(),
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

/// TLS configuration
/// Per spec-kit/009-deployment-spec.md: TLS 1.2+ enforcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to TLS certificate file (PEM format)
    pub cert_path: String,

    /// Path to TLS private key file (PEM format)
    pub key_path: String,

    /// Enforce HTTPS (redirect HTTP to HTTPS)
    #[serde(default = "default_enforce_https")]
    pub enforce_https: bool,
}

fn default_enforce_https() -> bool {
    true
}

/// CORS configuration
/// Per spec-kit/002-architecture.md Layer 1: Network Security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins (e.g., ["https://example.com"])
    /// Use ["*"] to allow all origins (NOT recommended for production)
    #[serde(default = "default_cors_origins")]
    pub allowed_origins: Vec<String>,

    /// Allowed HTTP methods
    #[serde(default = "default_cors_methods")]
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    #[serde(default = "default_cors_headers")]
    pub allowed_headers: Vec<String>,

    /// Max age for preflight cache (seconds)
    #[serde(default = "default_cors_max_age")]
    pub max_age: usize,

    /// Allow credentials (cookies, authorization headers)
    #[serde(default = "default_cors_credentials")]
    pub supports_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_cors_origins(),
            allowed_methods: default_cors_methods(),
            allowed_headers: default_cors_headers(),
            max_age: default_cors_max_age(),
            supports_credentials: default_cors_credentials(),
        }
    }
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "OPTIONS".to_string(),
    ]
}

fn default_cors_headers() -> Vec<String> {
    vec![
        "Authorization".to_string(),
        "Content-Type".to_string(),
    ]
}

fn default_cors_max_age() -> usize {
    3600
}

fn default_cors_credentials() -> bool {
    true
}

/// Security headers configuration
/// Per spec-kit/002-architecture.md Layer 1: Network Security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS header
    #[serde(default = "default_enable_hsts")]
    pub enable_hsts: bool,

    /// HSTS max-age in seconds (default: 1 year)
    #[serde(default = "default_hsts_max_age")]
    pub hsts_max_age: u32,

    /// Enable CSP header
    #[serde(default = "default_enable_csp")]
    pub enable_csp: bool,

    /// CSP policy (default: default-src 'self')
    #[serde(default = "default_csp_policy")]
    pub csp_policy: String,

    /// Enable X-Frame-Options
    #[serde(default = "default_enable_frame_options")]
    pub enable_frame_options: bool,

    /// X-Frame-Options value (default: DENY)
    #[serde(default = "default_frame_options")]
    pub frame_options: String,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: default_enable_hsts(),
            hsts_max_age: default_hsts_max_age(),
            enable_csp: default_enable_csp(),
            csp_policy: default_csp_policy(),
            enable_frame_options: default_enable_frame_options(),
            frame_options: default_frame_options(),
        }
    }
}

fn default_enable_hsts() -> bool {
    true
}

fn default_hsts_max_age() -> u32 {
    31536000 // 1 year
}

fn default_enable_csp() -> bool {
    true
}

fn default_csp_policy() -> String {
    "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".to_string()
}

fn default_enable_frame_options() -> bool {
    true
}

fn default_frame_options() -> String {
    "DENY".to_string()
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
        assert!(config.tls.is_none());
    }

    #[test]
    fn test_cors_config_defaults() {
        let config = CorsConfig::default();
        assert_eq!(config.allowed_origins, vec!["*"]);
        assert_eq!(config.max_age, 3600);
        assert!(config.supports_credentials);
    }

    #[test]
    fn test_security_headers_config_defaults() {
        let config = SecurityHeadersConfig::default();
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31536000);
        assert!(config.enable_csp);
        assert!(config.enable_frame_options);
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

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::default();
        assert_eq!(config.ip_requests_per_minute, 100);
        assert_eq!(config.user_requests_per_hour, 1000);
        assert_eq!(config.ws_messages_per_second, 100);
        assert_eq!(config.lockout_threshold, 5);
        assert_eq!(config.lockout_duration_minutes, 15);
    }
}

/// Rate limiting configuration
/// Per spec-kit/002-architecture.md Layer 1 Network Security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// IP-based rate limit: requests per minute
    #[serde(default = "default_ip_requests_per_minute")]
    pub ip_requests_per_minute: u32,

    /// User-based rate limit: requests per hour
    #[serde(default = "default_user_requests_per_hour")]
    pub user_requests_per_hour: u32,

    /// WebSocket messages per second
    #[serde(default = "default_ws_messages_per_second")]
    pub ws_messages_per_second: u32,

    /// Number of violations before temporary lockout
    #[serde(default = "default_lockout_threshold")]
    pub lockout_threshold: u32,

    /// Lockout duration in minutes
    #[serde(default = "default_lockout_duration_minutes")]
    pub lockout_duration_minutes: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            ip_requests_per_minute: default_ip_requests_per_minute(),
            user_requests_per_hour: default_user_requests_per_hour(),
            ws_messages_per_second: default_ws_messages_per_second(),
            lockout_threshold: default_lockout_threshold(),
            lockout_duration_minutes: default_lockout_duration_minutes(),
        }
    }
}

fn default_ip_requests_per_minute() -> u32 {
    100
}

fn default_user_requests_per_hour() -> u32 {
    1000
}

fn default_ws_messages_per_second() -> u32 {
    100
}

fn default_lockout_threshold() -> u32 {
    5
}

fn default_lockout_duration_minutes() -> u64 {
    15
}