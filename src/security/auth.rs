// Authentication service with JWT
// Per spec-kit/003-backend-spec.md section 4.1

use std::time::Duration;

use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::session::UserId;

/// Authentication service
/// Per spec-kit/003-backend-spec.md: JWT token validation
pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    token_expiry: Duration,
}

impl AuthService {
    /// Create a new authentication service
    /// Per spec-kit/003-backend-spec.md: jsonwebtoken 9.x
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation: Validation::default(),
            token_expiry: Duration::from_secs(8 * 3600), // 8 hours
        }
    }

    /// Authenticate user and create JWT token
    /// Per spec-kit/003-backend-spec.md section 4.1
    pub fn create_token(&self, user_id: UserId) -> Result<AuthToken> {
        let now = Utc::now();
        let exp = now + chrono::Duration::seconds(self.token_expiry.as_secs() as i64);

        let claims = Claims {
            sub: user_id.as_str().to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| Error::Internal(format!("Failed to encode JWT: {}", e)))?;

        Ok(AuthToken {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_at: exp,
            user_id,
        })
    }

    /// Validate JWT token and extract user ID
    /// Per spec-kit/003-backend-spec.md: JWT token validation
    pub fn validate_token(&self, token: &str) -> Result<UserId> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                tracing::warn!("Invalid JWT token: {}", e);
                Error::InvalidToken
            })?;

        Ok(UserId::new(token_data.claims.sub))
    }

    /// Check if token is expired
    pub fn is_token_expired(&self, token: &str) -> bool {
        match decode::<Claims>(token, &self.decoding_key, &self.validation) {
            Ok(token_data) => {
                let exp = token_data.claims.exp;
                let now = Utc::now().timestamp() as usize;
                now > exp
            }
            Err(_) => true,
        }
    }
}

/// JWT claims structure
/// Per spec-kit/003-backend-spec.md: Standard JWT claims
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject (user ID)
    sub: String,
    /// Expiration time (as UTC timestamp)
    exp: usize,
    /// Issued at (as UTC timestamp)
    iat: usize,
}

/// Authentication token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub user_id: UserId,
}

/// User credentials for authentication
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_service_creation() {
        let secret = b"test_secret_key_at_least_32_bytes_long";
        let auth = AuthService::new(secret);
        assert_eq!(auth.token_expiry, Duration::from_secs(8 * 3600));
    }

    #[test]
    fn test_create_and_validate_token() {
        let secret = b"test_secret_key_at_least_32_bytes_long";
        let auth = AuthService::new(secret);
        let user_id = UserId::new("test_user".to_string());

        // Create token
        let token = auth.create_token(user_id.clone()).unwrap();
        assert!(!token.access_token.is_empty());
        assert_eq!(token.token_type, "Bearer");

        // Validate token
        let validated_user_id = auth.validate_token(&token.access_token).unwrap();
        assert_eq!(validated_user_id.as_str(), user_id.as_str());
    }

    #[test]
    fn test_invalid_token() {
        let secret = b"test_secret_key_at_least_32_bytes_long";
        let auth = AuthService::new(secret);

        let result = auth.validate_token("invalid_token");
        assert!(result.is_err());
    }
}