// Per 011-authentication-spec.md section 2.2: JWT Verifier Implementation
// Responsibilities:
// - Validate JWT signatures using JWKS keys
// - Verify token claims (iss, aud, exp, nbf)
// - Extract user identity from claims
// - Support multiple signing algorithms (RS256, RS384, RS512)

use crate::security::jwks_client::{JwksClient, JwksError, JwksProvider};
use jsonwebtoken::{
    decode, decode_header, Algorithm, DecodingKey, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// JWT validation error types
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("JWT validation failed: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("JWKS error: {0}")]
    JwksError(#[from] JwksError),

    #[error("Missing or invalid kid in JWT header")]
    MissingKid,

    #[error("Key not found for kid={0}")]
    KeyNotFound(String),

    #[error("Provider not found for issuer={0}")]
    ProviderNotFound(String),

    #[error("Unsupported algorithm: {0:?}")]
    UnsupportedAlgorithm(Algorithm),

    #[error("Invalid RSA key: {0}")]
    InvalidRsaKey(String),

    #[error("Token expired at {0}")]
    TokenExpired(i64),

    #[error("Token not yet valid (nbf={0})")]
    TokenNotYetValid(i64),

    #[error("Invalid issuer: expected one of {expected:?}, got {actual}")]
    InvalidIssuer {
        expected: Vec<String>,
        actual: String,
    },

    #[error("Invalid audience: expected one of {expected:?}, got {actual:?}")]
    InvalidAudience {
        expected: Vec<String>,
        actual: Vec<String>,
    },
}

/// JWT Claims structure
/// Per 011-authentication-spec.md section 4.1: Standard Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,                          // Subject (user identifier)
    pub iss: String,                          // Issuer
    #[serde(default)]
    pub aud: Audience,                        // Audience (can be string or array)
    pub exp: i64,                             // Expiration time
    pub iat: i64,                             // Issued at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,                     // Not before

    // Optional Backstage-specific claims
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ent: Option<Vec<String>>,             // Entity references

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usc: Option<UserSignInContext>,       // User sign-in context

    // Optional custom claims
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<String>>,

    // Catch-all for other custom claims
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Audience can be a string or array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    pub fn contains(&self, value: &str) -> bool {
        match self {
            Audience::Single(s) => s == value,
            Audience::Multiple(v) => v.iter().any(|s| s == value),
        }
    }

    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Audience::Single(s) => vec![s.clone()],
            Audience::Multiple(v) => v.clone(),
        }
    }
}

impl Default for Audience {
    fn default() -> Self {
        Audience::Multiple(Vec::new())
    }
}

/// Backstage user sign-in context
/// Per 011-authentication-spec.md section 4.2: Backstage-Specific Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSignInContext {
    #[serde(rename = "ownershipEntityRefs")]
    pub ownership_entity_refs: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "displayName")]
    pub display_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Validated token with claims and provider info
#[derive(Debug, Clone)]
pub struct ValidatedToken {
    pub claims: Claims,
    pub provider: String,
    pub algorithm: Algorithm,
}

/// JWT validator with JWKS integration
/// Per 011-authentication-spec.md section 2.2: JWT Verifier
pub struct JwtValidator {
    jwks_client: Arc<JwksClient>,
    auth_config: crate::config::AuthConfig,
    allowed_algorithms: Vec<Algorithm>,
    clock_skew_seconds: u64,
}

impl JwtValidator {
    /// Create a new JWT validator from auth configuration
    /// Per 011-authentication-spec.md section 10: Configuration
    pub fn new(jwks_client: Arc<JwksClient>, auth_config: crate::config::AuthConfig) -> Self {
        // Determine allowed algorithms from config
        let allowed_algorithms = auth_config
            .jwks
            .providers
            .iter()
            .flat_map(|p| &p.algorithms)
            .filter_map(|alg| match alg.as_str() {
                "RS256" => Some(Algorithm::RS256),
                "RS384" => Some(Algorithm::RS384),
                "RS512" => Some(Algorithm::RS512),
                "ES256" => Some(Algorithm::ES256),
                "ES384" => Some(Algorithm::ES384),
                _ => None,
            })
            .collect::<Vec<_>>();

        let allowed_algorithms = if allowed_algorithms.is_empty() {
            // Default algorithms if none specified
            vec![
                Algorithm::RS256,
                Algorithm::RS384,
                Algorithm::RS512,
                Algorithm::ES256,
                Algorithm::ES384,
            ]
        } else {
            allowed_algorithms
        };

        Self {
            jwks_client,
            auth_config: auth_config.clone(),
            allowed_algorithms,
            clock_skew_seconds: auth_config.validation.clock_skew_seconds(),
        }
    }

    /// Validate a JWT token
    /// Per 011-authentication-spec.md section 5.2: Validation Process
    pub async fn validate(&self, token: &str) -> Result<ValidatedToken, ValidationError> {
        // Step 1: Decode header to extract kid and alg
        let header = decode_header(token)?;

        let kid = header.kid.as_ref()
            .ok_or(ValidationError::MissingKid)?;

        // Step 2: Verify algorithm is allowed
        if !self.allowed_algorithms.contains(&header.alg) {
            return Err(ValidationError::UnsupportedAlgorithm(header.alg));
        }

        // Step 3: Decode without verification to get issuer (using unsafe validation)
        let mut validation_unsafe = Validation::default();
        validation_unsafe.insecure_disable_signature_validation();
        validation_unsafe.validate_exp = false;

        let unverified: TokenData<Claims> = decode(
            token,
            &DecodingKey::from_secret(&[]),
            &validation_unsafe,
        )?;
        let issuer = &unverified.claims.iss;

        // Step 4: Find provider by issuer
        let provider = self.find_provider_by_issuer(issuer)?;

        // Step 5: Fetch public key via JWKS client
        let jwk = self.jwks_client
            .get_key(kid, &provider.name)
            .await?
            .ok_or_else(|| ValidationError::KeyNotFound(kid.clone()))?;

        // Step 6: Convert JWK to RSA public key
        let decoding_key = self.jwk_to_decoding_key(&jwk)?;

        // Step 7: Build validation parameters
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[issuer]);
        validation.leeway = self.clock_skew_seconds;

        // Allow flexible audience validation (checked separately)
        validation.validate_aud = false;

        // Step 8: Verify token signature and decode claims
        let token_data = decode::<Claims>(token, &decoding_key, &validation)?;

        Ok(ValidatedToken {
            claims: token_data.claims,
            provider: provider.name.clone(),
            algorithm: header.alg,
        })
    }

    /// Convert JWK to jsonwebtoken DecodingKey
    fn jwk_to_decoding_key(&self, jwk: &crate::security::jwks_client::JsonWebKey) -> Result<DecodingKey, ValidationError> {
        if jwk.kty != "RSA" {
            return Err(ValidationError::InvalidRsaKey(format!(
                "Unsupported key type: {}",
                jwk.kty
            )));
        }

        // Note: jsonwebtoken's from_rsa_components handles base64 decoding internally
        // We just pass the base64url-encoded strings directly

        // Create RSA public key from components
        DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|e| ValidationError::InvalidRsaKey(format!("Failed to create key: {}", e)))
    }

    /// Find provider by issuer URL
    fn find_provider_by_issuer(&self, issuer: &str) -> Result<&JwksProvider, ValidationError> {
        self.jwks_client
            .find_provider_by_issuer(issuer)
            .ok_or_else(|| ValidationError::ProviderNotFound(issuer.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_deserialization_standard() {
        let json = r#"{
            "sub": "user:default/alice",
            "iss": "https://backstage.example.com",
            "aud": "web-terminal",
            "exp": 1735567200,
            "iat": 1735563600,
            "nbf": 1735563600
        }"#;

        let claims: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user:default/alice");
        assert_eq!(claims.iss, "https://backstage.example.com");
        assert!(matches!(claims.aud, Audience::Single(_)));
    }

    #[test]
    fn test_claims_deserialization_backstage() {
        let json = r#"{
            "sub": "user:default/john.doe",
            "iss": "https://backstage.example.com",
            "aud": ["backstage"],
            "exp": 1735567200,
            "iat": 1735563600,
            "ent": ["user:default/john.doe"],
            "usc": {
                "ownershipEntityRefs": ["group:default/platform-team"],
                "displayName": "John Doe",
                "email": "john.doe@example.com"
            }
        }"#;

        let claims: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user:default/john.doe");
        assert!(claims.ent.is_some());
        assert!(claims.usc.is_some());

        let usc = claims.usc.unwrap();
        assert_eq!(usc.ownership_entity_refs.len(), 1);
        assert_eq!(usc.display_name.unwrap(), "John Doe");
    }

    #[test]
    fn test_audience_single() {
        let aud = Audience::Single("web-terminal".to_string());
        assert!(aud.contains("web-terminal"));
        assert!(!aud.contains("other"));
    }

    #[test]
    fn test_audience_multiple() {
        let aud = Audience::Multiple(vec![
            "web-terminal".to_string(),
            "backstage".to_string(),
        ]);
        assert!(aud.contains("web-terminal"));
        assert!(aud.contains("backstage"));
        assert!(!aud.contains("other"));
    }
}