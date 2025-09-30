// Unit tests for Server Configuration
// Per spec-kit/008-testing-spec.md - Unit Tests
// Per spec-kit/003-backend-spec.md section 6 - Configuration

use web_terminal::config::{ServerConfig, SecurityConfig, LoggingConfig};
use std::path::PathBuf;

/// Test ServerConfig default values
///
/// Per spec-kit/003-backend-spec.md: Single-port deployment (default 8080)
#[test]
fn test_server_config_defaults() {
    // Arrange & Act
    let config = ServerConfig::default();

    // Assert
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 8080); // Single port for all services
    assert_eq!(config.max_connections, 10000);
    assert_eq!(config.worker_threads, num_cpus::get());
    assert!(config.tls_cert.is_none());
    assert!(config.tls_key.is_none());
}

/// Test ServerConfig with custom values
#[test]
fn test_server_config_custom() {
    // Arrange
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        max_connections: 5000,
        worker_threads: 4,
        tls_cert: Some(PathBuf::from("/path/to/cert.pem")),
        tls_key: Some(PathBuf::from("/path/to/key.pem")),
    };

    // Act & Assert
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 3000);
    assert_eq!(config.max_connections, 5000);
    assert_eq!(config.worker_threads, 4);
    assert!(config.tls_cert.is_some());
    assert!(config.tls_key.is_some());
}

/// Test ServerConfig serialization
#[test]
fn test_server_config_serialization() {
    // Arrange
    let config = ServerConfig::default();

    // Act
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: ServerConfig = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert_eq!(deserialized.host, config.host);
    assert_eq!(deserialized.port, config.port);
    assert_eq!(deserialized.max_connections, config.max_connections);
}

/// Test SecurityConfig default values
///
/// Per spec-kit/003-backend-spec.md: JWT authentication
#[test]
fn test_security_config_defaults() {
    // Arrange & Act
    let config = SecurityConfig::default();

    // Assert
    assert_eq!(config.jwt_secret, "change_me_in_production");
    assert_eq!(config.token_expiry_secs, 8 * 3600); // 8 hours
    assert!(config.cors_enabled);
    assert_eq!(config.cors_origins, vec!["*"]);
}

/// Test SecurityConfig with custom values
#[test]
fn test_security_config_custom() {
    // Arrange
    let config = SecurityConfig {
        jwt_secret: "my_secure_secret_key".to_string(),
        token_expiry_secs: 3600, // 1 hour
        cors_enabled: false,
        cors_origins: vec!["https://example.com".to_string()],
    };

    // Act & Assert
    assert_eq!(config.jwt_secret, "my_secure_secret_key");
    assert_eq!(config.token_expiry_secs, 3600);
    assert!(!config.cors_enabled);
    assert_eq!(config.cors_origins.len(), 1);
}

/// Test SecurityConfig multiple CORS origins
#[test]
fn test_security_config_multiple_cors_origins() {
    // Arrange
    let config = SecurityConfig {
        jwt_secret: "secret".to_string(),
        token_expiry_secs: 3600,
        cors_enabled: true,
        cors_origins: vec![
            "https://example.com".to_string(),
            "https://app.example.com".to_string(),
            "http://localhost:3000".to_string(),
        ],
    };

    // Act & Assert
    assert_eq!(config.cors_origins.len(), 3);
    assert!(config.cors_origins.contains(&"https://example.com".to_string()));
    assert!(config.cors_origins.contains(&"https://app.example.com".to_string()));
    assert!(config.cors_origins.contains(&"http://localhost:3000".to_string()));
}

/// Test LoggingConfig default values
///
/// Per spec-kit/003-backend-spec.md: tracing + tracing-subscriber
#[test]
fn test_logging_config_defaults() {
    // Arrange & Act
    let config = LoggingConfig::default();

    // Assert
    assert_eq!(config.level, "info");
    assert!(!config.json);
    assert!(config.file.is_none());
}

/// Test LoggingConfig with custom values
#[test]
fn test_logging_config_custom() {
    // Arrange
    let config = LoggingConfig {
        level: "debug".to_string(),
        json: true,
        file: Some(PathBuf::from("/var/log/web-terminal.log")),
    };

    // Act & Assert
    assert_eq!(config.level, "debug");
    assert!(config.json);
    assert!(config.file.is_some());
    assert_eq!(config.file.unwrap(), PathBuf::from("/var/log/web-terminal.log"));
}

/// Test LoggingConfig all log levels
#[test]
fn test_logging_config_all_levels() {
    let levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in levels {
        let config = LoggingConfig {
            level: level.to_string(),
            json: false,
            file: None,
        };
        assert_eq!(config.level, level);
    }
}

/// Test ServerConfig clone
#[test]
fn test_server_config_clone() {
    // Arrange
    let config1 = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        max_connections: 10000,
        worker_threads: 4,
        tls_cert: None,
        tls_key: None,
    };

    // Act
    let config2 = config1.clone();

    // Assert
    assert_eq!(config1.host, config2.host);
    assert_eq!(config1.port, config2.port);
    assert_eq!(config1.max_connections, config2.max_connections);
}

/// Test SecurityConfig clone
#[test]
fn test_security_config_clone() {
    // Arrange
    let config1 = SecurityConfig::default();

    // Act
    let config2 = config1.clone();

    // Assert
    assert_eq!(config1.jwt_secret, config2.jwt_secret);
    assert_eq!(config1.token_expiry_secs, config2.token_expiry_secs);
    assert_eq!(config1.cors_enabled, config2.cors_enabled);
}

/// Test LoggingConfig clone
#[test]
fn test_logging_config_clone() {
    // Arrange
    let config1 = LoggingConfig::default();

    // Act
    let config2 = config1.clone();

    // Assert
    assert_eq!(config1.level, config2.level);
    assert_eq!(config1.json, config2.json);
}

/// Test ServerConfig with TLS enabled
#[test]
fn test_server_config_with_tls() {
    // Arrange
    let config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8443,
        tls_cert: Some(PathBuf::from("/etc/ssl/certs/server.crt")),
        tls_key: Some(PathBuf::from("/etc/ssl/private/server.key")),
        max_connections: 10000,
        worker_threads: num_cpus::get(),
    };

    // Act & Assert
    assert!(config.tls_cert.is_some());
    assert!(config.tls_key.is_some());
    assert_eq!(config.tls_cert.unwrap(), PathBuf::from("/etc/ssl/certs/server.crt"));
}

/// Test worker threads defaults to CPU count
#[test]
fn test_worker_threads_defaults_to_cpu_count() {
    // Arrange & Act
    let config = ServerConfig::default();

    // Assert
    assert_eq!(config.worker_threads, num_cpus::get());
    assert!(config.worker_threads > 0);
}

/// Test ServerConfig port validation (conceptual)
#[test]
fn test_server_config_port_range() {
    // Arrange - test various port numbers
    let configs = vec![
        ServerConfig { port: 1, ..Default::default() },
        ServerConfig { port: 80, ..Default::default() },
        ServerConfig { port: 8080, ..Default::default() },
        ServerConfig { port: 65535, ..Default::default() },
    ];

    // Act & Assert - all should be valid u16 values
    for config in configs {
        assert!(config.port > 0);
        assert!(config.port <= 65535);
    }
}

/// Test SecurityConfig empty CORS origins
#[test]
fn test_security_config_empty_cors_origins() {
    // Arrange
    let config = SecurityConfig {
        jwt_secret: "secret".to_string(),
        token_expiry_secs: 3600,
        cors_enabled: true,
        cors_origins: vec![],
    };

    // Act & Assert
    assert!(config.cors_origins.is_empty());
}

/// Test LoggingConfig JSON format
#[test]
fn test_logging_config_json_format() {
    // Arrange
    let config = LoggingConfig {
        level: "info".to_string(),
        json: true,
        file: None,
    };

    // Act & Assert
    assert!(config.json);
}

/// Test configuration deserialization from JSON
#[test]
fn test_config_deserialization_from_json() {
    // Arrange
    let json = r#"{
        "host": "localhost",
        "port": 9000,
        "max_connections": 5000,
        "worker_threads": 8
    }"#;

    // Act
    let config: ServerConfig = serde_json::from_str(json).expect("Failed to deserialize");

    // Assert
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 9000);
    assert_eq!(config.max_connections, 5000);
    assert_eq!(config.worker_threads, 8);
}

/// Test SecurityConfig serialization and deserialization
#[test]
fn test_security_config_round_trip() {
    // Arrange
    let original = SecurityConfig {
        jwt_secret: "test_secret".to_string(),
        token_expiry_secs: 7200,
        cors_enabled: true,
        cors_origins: vec!["https://test.com".to_string()],
    };

    // Act
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let deserialized: SecurityConfig = serde_json::from_str(&json).expect("Failed to deserialize");

    // Assert
    assert_eq!(original.jwt_secret, deserialized.jwt_secret);
    assert_eq!(original.token_expiry_secs, deserialized.token_expiry_secs);
    assert_eq!(original.cors_enabled, deserialized.cors_enabled);
    assert_eq!(original.cors_origins, deserialized.cors_origins);
}

/// Test LoggingConfig with file path
#[test]
fn test_logging_config_with_file_path() {
    // Arrange
    let log_file = PathBuf::from("/var/log/terminal.log");
    let config = LoggingConfig {
        level: "debug".to_string(),
        json: false,
        file: Some(log_file.clone()),
    };

    // Act & Assert
    assert!(config.file.is_some());
    assert_eq!(config.file.unwrap(), log_file);
}