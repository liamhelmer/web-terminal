// TLS configuration module
// Per spec-kit/009-deployment-spec.md: TLS 1.2+ enforcement

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};

#[cfg(feature = "tls")]
use rustls::{ServerConfig as RustlsServerConfig, pki_types::CertificateDer};
#[cfg(feature = "tls")]
use rustls_pemfile::{certs, private_key};

/// TLS configuration
/// Per spec-kit/002-architecture.md Layer 1: Network Security
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to TLS certificate file (PEM format)
    pub cert_path: String,

    /// Path to TLS private key file (PEM format)
    pub key_path: String,

    /// Enforce HTTPS (redirect HTTP to HTTPS)
    pub enforce_https: bool,
}

impl TlsConfig {
    pub fn new(cert_path: String, key_path: String, enforce_https: bool) -> Self {
        Self {
            cert_path,
            key_path,
            enforce_https,
        }
    }
}

/// Load TLS configuration from PEM files
/// Per spec-kit/009-deployment-spec.md: TLS 1.2+ only, secure cipher suites
#[cfg(feature = "tls")]
pub fn load_tls_config(config: &TlsConfig) -> Result<RustlsServerConfig> {
    // Load certificate chain
    let cert_file = File::open(&config.cert_path)
        .with_context(|| format!("Failed to open certificate file: {}", config.cert_path))?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain: Vec<CertificateDer> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse certificate chain")?;

    if cert_chain.is_empty() {
        anyhow::bail!("No certificates found in {}", config.cert_path);
    }

    // Load private key
    let key_file = File::open(&config.key_path)
        .with_context(|| format!("Failed to open private key file: {}", config.key_path))?;
    let mut key_reader = BufReader::new(key_file);
    let private_key = private_key(&mut key_reader)
        .context("Failed to parse private key")?
        .ok_or_else(|| anyhow::anyhow!("No private key found in {}", config.key_path))?;

    // Build TLS configuration with secure defaults
    // Per spec-kit/009-deployment-spec.md: TLS 1.2+ only
    let tls_config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .context("Failed to build TLS configuration")?;

    tracing::info!("TLS configuration loaded successfully");
    tracing::info!("Certificate: {}", config.cert_path);
    tracing::info!("Private key: {}", config.key_path);
    tracing::info!("HTTPS enforcement: {}", config.enforce_https);

    Ok(tls_config)
}

#[cfg(not(feature = "tls"))]
pub fn load_tls_config(_config: &TlsConfig) -> Result<()> {
    anyhow::bail!("TLS support not enabled. Compile with --features tls")
}

/// Validate TLS configuration files exist and are readable
pub fn validate_tls_files(cert_path: &str, key_path: &str) -> Result<()> {
    if !Path::new(cert_path).exists() {
        anyhow::bail!("Certificate file not found: {}", cert_path);
    }

    if !Path::new(key_path).exists() {
        anyhow::bail!("Private key file not found: {}", key_path);
    }

    // Try to open files to check permissions
    File::open(cert_path)
        .with_context(|| format!("Cannot read certificate file: {}", cert_path))?;

    File::open(key_path)
        .with_context(|| format!("Cannot read private key file: {}", key_path))?;

    tracing::info!("TLS files validated successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_creation() {
        let config = TlsConfig::new(
            "cert.pem".to_string(),
            "key.pem".to_string(),
            true,
        );
        assert_eq!(config.cert_path, "cert.pem");
        assert_eq!(config.key_path, "key.pem");
        assert!(config.enforce_https);
    }

    #[test]
    fn test_validate_tls_files_missing() {
        let result = validate_tls_files("nonexistent_cert.pem", "nonexistent_key.pem");
        assert!(result.is_err());
    }
}