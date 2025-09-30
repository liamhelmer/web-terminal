// Per 011-authentication-spec.md section 2.1: JWKS Client Implementation
// Responsibilities:
// - Fetch JWKS from configured providers
// - Cache public keys with TTL
// - Handle JWKS endpoint failures gracefully
// - Support multiple JWKS providers

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::sleep;

/// JWKS client error types
#[derive(Error, Debug)]
pub enum JwksError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Key not found: kid={0}")]
    KeyNotFound(String),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Invalid JWKS response: {0}")]
    InvalidResponse(String),

    #[error("Cache expired and refresh failed")]
    CacheExpired,
}

/// JSON Web Key as defined in RFC 7517
/// Per 011-authentication-spec.md section 2.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKey {
    pub kid: String,       // Key ID
    pub kty: String,       // Key Type (RSA, EC, etc.)
    pub alg: String,       // Algorithm (RS256, RS384, etc.)
    #[serde(rename = "use")]
    pub use_: String,      // Public key use (sig, enc)
    pub n: String,         // RSA modulus (base64url encoded)
    pub e: String,         // RSA exponent (base64url encoded)
}

/// JWKS response from provider
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JsonWebKey>,
}

/// Cached JWKS with expiration metadata
/// Per 011-authentication-spec.md section 3.3: Cache and Refresh Strategy
#[derive(Debug, Clone)]
struct CachedJwks {
    keys: Vec<JsonWebKey>,
    fetched_at: Instant,
    expires_at: Instant,
}

/// Provider configuration
#[derive(Debug, Clone)]
pub struct JwksProvider {
    pub name: String,
    pub jwks_url: String,
    pub issuer: String,
    pub cache_ttl: Duration,
}

/// JWKS client with caching and background refresh
/// Per 011-authentication-spec.md section 2.1: JWKS Client
pub struct JwksClient {
    http_client: reqwest::Client,
    cache: Arc<DashMap<String, CachedJwks>>,
    providers: Arc<Vec<JwksProvider>>,
    default_ttl: Duration,
}

impl JwksClient {
    /// Create a new JWKS client from auth configuration
    /// Per 011-authentication-spec.md section 10: Configuration
    pub fn new(auth_config: crate::config::AuthConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(auth_config.validation.timeout())
            .build()
            .expect("Failed to create HTTP client");

        // Convert config providers to internal providers
        let providers: Vec<JwksProvider> = auth_config
            .jwks
            .providers
            .into_iter()
            .map(|p| JwksProvider {
                name: p.name,
                jwks_url: p.url,  // Config field is 'url'
                issuer: p.issuer,
                cache_ttl: auth_config.validation.cache_ttl(),
            })
            .collect();

        Self {
            http_client,
            cache: Arc::new(DashMap::new()),
            providers: Arc::new(providers),
            default_ttl: auth_config.validation.cache_ttl(),
        }
    }

    /// Fetch JWKS keys for a provider (cache-first strategy)
    /// Per 011-authentication-spec.md section 3.3: Cache and Refresh Strategy
    pub async fn fetch_keys(&self, provider_name: &str) -> Result<Vec<JsonWebKey>, JwksError> {
        // Check cache first
        if let Some(cached) = self.cache.get(provider_name) {
            if cached.expires_at > Instant::now() {
                tracing::debug!(
                    provider = provider_name,
                    keys_count = cached.keys.len(),
                    "JWKS cache hit"
                );
                return Ok(cached.keys.clone());
            }
            tracing::debug!(provider = provider_name, "JWKS cache expired");
        }

        // Cache miss or expired - fetch from provider
        self.fetch_keys_from_provider(provider_name).await
    }

    /// Fetch JWKS from provider's HTTP endpoint
    async fn fetch_keys_from_provider(&self, provider_name: &str) -> Result<Vec<JsonWebKey>, JwksError> {
        let provider = self.providers
            .iter()
            .find(|p| p.name == provider_name)
            .ok_or_else(|| JwksError::ProviderNotConfigured(provider_name.to_string()))?;

        tracing::info!(
            provider = provider_name,
            url = %provider.jwks_url,
            "Fetching JWKS from provider"
        );

        let response = self.http_client
            .get(&provider.jwks_url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(JwksError::InvalidResponse(format!(
                "HTTP {} from {}",
                response.status(),
                provider.jwks_url
            )));
        }

        let jwks: JwksResponse = response.json().await?;

        if jwks.keys.is_empty() {
            return Err(JwksError::InvalidResponse("Empty keys array".to_string()));
        }

        tracing::info!(
            provider = provider_name,
            keys_count = jwks.keys.len(),
            "Successfully fetched JWKS"
        );

        // Cache the keys
        let cached = CachedJwks {
            keys: jwks.keys.clone(),
            fetched_at: Instant::now(),
            expires_at: Instant::now() + provider.cache_ttl,
        };

        self.cache.insert(provider_name.to_string(), cached);

        Ok(jwks.keys)
    }

    /// Get a specific key by kid
    /// Per 011-authentication-spec.md section 2.1: JWKS Client
    pub async fn get_key(&self, kid: &str, provider_name: &str) -> Result<Option<JsonWebKey>, JwksError> {
        let keys = self.fetch_keys(provider_name).await?;

        Ok(keys.into_iter().find(|key| key.kid == kid))
    }

    /// Start background refresh task
    /// Per 011-authentication-spec.md section 3.3: Key Rotation Handling
    pub fn start_refresh_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let refresh_interval = Duration::from_secs(900); // 15 minutes

            loop {
                sleep(refresh_interval).await;

                tracing::debug!("Starting background JWKS refresh");

                for provider in self.providers.iter() {
                    match self.fetch_keys_from_provider(&provider.name).await {
                        Ok(keys) => {
                            tracing::info!(
                                provider = %provider.name,
                                keys_count = keys.len(),
                                "Background JWKS refresh successful"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                provider = %provider.name,
                                error = %e,
                                "Background JWKS refresh failed, using cached keys"
                            );
                        }
                    }
                }
            }
        })
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> Vec<CacheStats> {
        self.cache
            .iter()
            .map(|entry| {
                let provider = entry.key();
                let cached = entry.value();
                let age = Instant::now().duration_since(cached.fetched_at);
                let ttl_remaining = cached.expires_at.saturating_duration_since(Instant::now());

                CacheStats {
                    provider: provider.clone(),
                    keys_count: cached.keys.len(),
                    age_seconds: age.as_secs(),
                    ttl_remaining_seconds: ttl_remaining.as_secs(),
                    is_expired: cached.expires_at <= Instant::now(),
                }
            })
            .collect()
    }

    /// Find provider by issuer URL
    pub fn find_provider_by_issuer(&self, issuer: &str) -> Option<&JwksProvider> {
        self.providers.iter().find(|p| p.issuer == issuer)
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub provider: String,
    pub keys_count: usize,
    pub age_seconds: u64,
    pub ttl_remaining_seconds: u64,
    pub is_expired: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwk_deserialization() {
        let json = r#"{
            "kid": "test-key-1",
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "n": "0vx7agoebGcQ",
            "e": "AQAB"
        }"#;

        let jwk: JsonWebKey = serde_json::from_str(json).unwrap();
        assert_eq!(jwk.kid, "test-key-1");
        assert_eq!(jwk.kty, "RSA");
        assert_eq!(jwk.alg, "RS256");
    }

    #[test]
    fn test_jwks_response_deserialization() {
        let json = r#"{
            "keys": [
                {
                    "kid": "key-1",
                    "kty": "RSA",
                    "alg": "RS256",
                    "use": "sig",
                    "n": "0vx7agoebGcQ",
                    "e": "AQAB"
                },
                {
                    "kid": "key-2",
                    "kty": "RSA",
                    "alg": "RS384",
                    "use": "sig",
                    "n": "xjlbGcQ0vx7a",
                    "e": "AQAB"
                }
            ]
        }"#;

        let response: JwksResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.keys.len(), 2);
        assert_eq!(response.keys[0].kid, "key-1");
        assert_eq!(response.keys[1].kid, "key-2");
    }
}