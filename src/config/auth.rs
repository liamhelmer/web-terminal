// Per 011-authentication-spec.md section 10
// Authentication configuration structures

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Authentication configuration
/// Per 011-authentication-spec.md section 10.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable/disable authentication
    #[serde(default)]
    pub enabled: bool,

    /// JWKS providers configuration
    pub jwks: JwksConfig,

    /// Authorization configuration
    pub authorization: AuthorizationConfig,

    /// Token validation settings
    #[serde(default)]
    pub validation: ValidationConfig,

    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwks: JwksConfig::default(),
            authorization: AuthorizationConfig::default(),
            validation: ValidationConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// JWKS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksConfig {
    /// List of JWKS providers
    pub providers: Vec<JwksProvider>,
}

impl Default for JwksConfig {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
        }
    }
}

/// JWKS provider configuration
/// Per 011-authentication-spec.md section 3.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksProvider {
    /// Provider name (e.g., "backstage", "auth0")
    pub name: String,

    /// JWKS endpoint URL
    pub url: String,

    /// Expected token issuer
    pub issuer: String,

    /// Expected token audience
    pub audience: String,

    /// Allowed signing algorithms
    #[serde(default = "default_algorithms")]
    pub algorithms: Vec<String>,

    /// Cache TTL in seconds
    #[serde(
        default = "default_cache_ttl",
        with = "humantime_serde",
        rename = "cache_ttl"
    )]
    pub cache_ttl: Duration,

    /// Refresh interval in seconds
    #[serde(
        default = "default_refresh_interval",
        with = "humantime_serde",
        rename = "refresh_interval"
    )]
    pub refresh_interval: Duration,

    /// Request timeout in seconds
    #[serde(
        default = "default_timeout",
        with = "humantime_serde",
        rename = "timeout"
    )]
    pub timeout: Duration,
}

/// Authorization configuration
/// Per 011-authentication-spec.md section 5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization mode
    #[serde(default = "default_auth_mode")]
    pub mode: String,

    /// Allowed users (entity references)
    #[serde(default)]
    pub allowed_users: Vec<String>,

    /// Allowed groups (entity references)
    #[serde(default)]
    pub allowed_groups: Vec<String>,

    /// Denied users (takes precedence)
    #[serde(default)]
    pub deny_users: Vec<String>,

    /// Denied groups (takes precedence)
    #[serde(default)]
    pub deny_groups: Vec<String>,

    /// Claim mappings
    #[serde(default)]
    pub claims: ClaimMappings,

    /// Default deny policy
    #[serde(default = "default_true")]
    pub deny_by_default: bool,

    /// Allow empty groups
    #[serde(default)]
    pub allow_empty_groups: bool,

    /// Enable wildcard patterns
    #[serde(default)]
    pub enable_wildcards: bool,
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            mode: "claim-based".to_string(),
            allowed_users: Vec::new(),
            allowed_groups: Vec::new(),
            deny_users: Vec::new(),
            deny_groups: Vec::new(),
            claims: ClaimMappings::default(),
            deny_by_default: true,
            allow_empty_groups: false,
            enable_wildcards: false,
        }
    }
}

/// Claim mappings for extracting user information
/// Per 011-authentication-spec.md section 4.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimMappings {
    /// User ID claim path (default: "sub")
    #[serde(default = "default_user_claim")]
    pub user_id: String,

    /// Email claim path (default: "email")
    #[serde(default = "default_email_claim")]
    pub email: String,

    /// Groups claim path (default: "groups")
    #[serde(default = "default_groups_claim")]
    pub groups: String,

    /// Backstage entity reference path (default: "ent")
    #[serde(default = "default_entity_ref_claim")]
    pub entity_ref: String,

    /// Display name claim path (default: "name")
    #[serde(default = "default_name_claim")]
    pub name: String,
}

impl Default for ClaimMappings {
    fn default() -> Self {
        Self {
            user_id: "sub".to_string(),
            email: "email".to_string(),
            groups: "groups".to_string(),
            entity_ref: "ent".to_string(),
            name: "name".to_string(),
        }
    }
}

/// Token validation configuration
/// Per 011-authentication-spec.md section 4.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Required claims
    #[serde(default = "default_required_claims")]
    pub required_claims: Vec<String>,

    /// Clock skew tolerance
    #[serde(
        default = "default_clock_skew",
        with = "humantime_serde",
        rename = "clock_skew"
    )]
    pub clock_skew: Duration,

    /// Allowed algorithms
    #[serde(default = "default_algorithms")]
    pub allowed_algorithms: Vec<String>,

    /// Minimum token lifetime
    #[serde(
        default = "default_min_lifetime",
        with = "humantime_serde",
        rename = "min_lifetime"
    )]
    pub min_lifetime: Duration,

    /// Maximum token lifetime
    #[serde(
        default = "default_max_lifetime",
        with = "humantime_serde",
        rename = "max_lifetime"
    )]
    pub max_lifetime: Duration,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            required_claims: default_required_claims(),
            clock_skew: default_clock_skew(),
            allowed_algorithms: default_algorithms(),
            min_lifetime: default_min_lifetime(),
            max_lifetime: default_max_lifetime(),
        }
    }
}

impl ValidationConfig {
    /// Get cache TTL as Duration (from first provider or default 1 hour)
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(3600) // 1 hour default
    }

    /// Get timeout as Duration (default 10 seconds)
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(10)
    }

    /// Get clock skew in seconds
    pub fn clock_skew_seconds(&self) -> u64 {
        self.clock_skew.as_secs()
    }
}

/// Security settings
/// Per 011-authentication-spec.md section 7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Rate limiting settings
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// Audit logging settings
    #[serde(default)]
    pub audit: AuditConfig,

    /// Require TLS for authentication
    #[serde(default = "default_true")]
    pub require_tls: bool,

    /// Allowed token sources for WebSocket
    #[serde(default = "default_token_sources")]
    pub allowed_token_sources: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit: RateLimitConfig::default(),
            audit: AuditConfig::default(),
            require_tls: true,
            allowed_token_sources: default_token_sources(),
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum failed attempts
    #[serde(default = "default_max_attempts")]
    pub max_failed_attempts: u32,

    /// Rate limit window
    #[serde(
        default = "default_rate_limit_window",
        with = "humantime_serde",
        rename = "window"
    )]
    pub window: Duration,

    /// Lockout duration
    #[serde(
        default = "default_lockout_duration",
        with = "humantime_serde",
        rename = "lockout"
    )]
    pub lockout: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_failed_attempts: 5,
            window: Duration::from_secs(300),
            lockout: Duration::from_secs(900),
        }
    }
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Log successful authentication
    #[serde(default = "default_true")]
    pub log_successful_auth: bool,

    /// Log failed authentication
    #[serde(default = "default_true")]
    pub log_failed_auth: bool,

    /// Log authorization denials
    #[serde(default = "default_true")]
    pub log_authorization_denials: bool,

    /// Log token details (security risk)
    #[serde(default)]
    pub log_token_details: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_successful_auth: true,
            log_failed_auth: true,
            log_authorization_denials: true,
            log_token_details: false,
        }
    }
}

// Default value functions

fn default_algorithms() -> Vec<String> {
    vec![
        "RS256".to_string(),
        "RS384".to_string(),
        "RS512".to_string(),
        "ES256".to_string(),
        "ES384".to_string(),
    ]
}

fn default_cache_ttl() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_refresh_interval() -> Duration {
    Duration::from_secs(900) // 15 minutes
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_auth_mode() -> String {
    "claim-based".to_string()
}

fn default_user_claim() -> String {
    "sub".to_string()
}

fn default_email_claim() -> String {
    "email".to_string()
}

fn default_groups_claim() -> String {
    "groups".to_string()
}

fn default_entity_ref_claim() -> String {
    "ent".to_string()
}

fn default_name_claim() -> String {
    "name".to_string()
}

fn default_required_claims() -> Vec<String> {
    vec![
        "sub".to_string(),
        "iss".to_string(),
        "aud".to_string(),
        "exp".to_string(),
        "iat".to_string(),
    ]
}

fn default_clock_skew() -> Duration {
    Duration::from_secs(60) // 1 minute
}

fn default_min_lifetime() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_max_lifetime() -> Duration {
    Duration::from_secs(28800) // 8 hours
}

fn default_true() -> bool {
    true
}

fn default_max_attempts() -> u32 {
    5
}

fn default_rate_limit_window() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_lockout_duration() -> Duration {
    Duration::from_secs(900) // 15 minutes
}

fn default_token_sources() -> Vec<String> {
    vec![
        "header".to_string(),
        "query".to_string(),
        "subprotocol".to_string(),
        "message".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_auth_config() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.validation.clock_skew == Duration::from_secs(60));
        assert!(config.security.require_tls);
    }

    #[test]
    fn test_jwks_provider_defaults() {
        let provider = JwksProvider {
            name: "test".to_string(),
            url: "https://example.com/.well-known/jwks.json".to_string(),
            issuer: "https://example.com".to_string(),
            audience: "test-audience".to_string(),
            algorithms: default_algorithms(),
            cache_ttl: default_cache_ttl(),
            refresh_interval: default_refresh_interval(),
            timeout: default_timeout(),
        };

        assert_eq!(provider.cache_ttl, Duration::from_secs(3600));
        assert_eq!(provider.refresh_interval, Duration::from_secs(900));
        assert_eq!(provider.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_claim_mappings_defaults() {
        let mappings = ClaimMappings::default();
        assert_eq!(mappings.user_id, "sub");
        assert_eq!(mappings.email, "email");
        assert_eq!(mappings.groups, "groups");
        assert_eq!(mappings.entity_ref, "ent");
    }

    #[test]
    fn test_authorization_config() {
        let config = AuthorizationConfig {
            mode: "claim-based".to_string(),
            allowed_users: vec!["user:default/alice".to_string()],
            allowed_groups: vec!["group:default/platform-team".to_string()],
            deny_users: vec![],
            deny_groups: vec![],
            claims: ClaimMappings::default(),
            deny_by_default: true,
            allow_empty_groups: false,
            enable_wildcards: false,
        };

        assert_eq!(config.allowed_users.len(), 1);
        assert_eq!(config.allowed_groups.len(), 1);
        assert!(config.deny_by_default);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_failed_attempts, 5);
        assert_eq!(config.window, Duration::from_secs(300));
        assert_eq!(config.lockout, Duration::from_secs(900));
    }

    #[test]
    fn test_audit_config() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert!(config.log_successful_auth);
        assert!(config.log_failed_auth);
        assert!(config.log_authorization_denials);
        assert!(!config.log_token_details); // Security: should be false by default
    }
}
