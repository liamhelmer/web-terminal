// Unit tests for Authentication Service
// Per spec-kit/008-testing-spec.md - Unit Tests
// Per spec-kit/003-backend-spec.md section 4.1 - JWT authentication

use web_terminal::security::AuthService;
use web_terminal::session::UserId;
use std::time::Duration;

/// Test authentication service creation
#[test]
fn test_auth_service_creation() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long";

    // Act
    let auth = AuthService::new(secret);

    // Assert - service should be created successfully
    // Token expiry is internal, just verify creation works
}

/// Test create token with valid user ID
///
/// Per spec-kit/003-backend-spec.md: JWT token creation
#[test]
fn test_create_token_valid_user() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Act
    let result = auth.create_token(user_id.clone());

    // Assert
    assert!(result.is_ok());
    let token = result.unwrap();
    assert!(!token.access_token.is_empty());
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.user_id.as_str(), "test_user");
}

/// Test validate token with valid token
///
/// Per spec-kit/003-backend-spec.md: JWT token validation
#[test]
fn test_validate_token_valid() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());
    let token = auth.create_token(user_id.clone()).unwrap();

    // Act
    let result = auth.validate_token(&token.access_token);

    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), user_id.as_str());
}

/// Test validate token with invalid token
#[test]
fn test_validate_token_invalid() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);

    // Act
    let result = auth.validate_token("invalid.token.here");

    // Assert
    assert!(result.is_err());
}

/// Test validate token with empty token
#[test]
fn test_validate_token_empty() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);

    // Act
    let result = auth.validate_token("");

    // Assert
    assert!(result.is_err());
}

/// Test validate token with wrong secret
#[test]
fn test_validate_token_wrong_secret() {
    // Arrange
    let secret1 = b"test_secret_key_at_least_32_bytes_long_for_security";
    let secret2 = b"different_secret_key_at_least_32_bytes_long_here";
    let auth1 = AuthService::new(secret1);
    let auth2 = AuthService::new(secret2);
    let user_id = UserId::new("test_user".to_string());
    let token = auth1.create_token(user_id).unwrap();

    // Act
    let result = auth2.validate_token(&token.access_token);

    // Assert
    assert!(result.is_err());
}

/// Test token expiration check
#[test]
fn test_is_token_expired_valid() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());
    let token = auth.create_token(user_id).unwrap();

    // Act
    let is_expired = auth.is_token_expired(&token.access_token);

    // Assert
    assert!(!is_expired, "Token should not be expired immediately after creation");
}

/// Test token expiration check with invalid token
#[test]
fn test_is_token_expired_invalid() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);

    // Act
    let is_expired = auth.is_token_expired("invalid.token.here");

    // Assert
    assert!(is_expired, "Invalid token should be considered expired");
}

/// Test multiple user tokens
#[test]
fn test_multiple_user_tokens() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user1 = UserId::new("user1".to_string());
    let user2 = UserId::new("user2".to_string());

    // Act
    let token1 = auth.create_token(user1.clone()).unwrap();
    let token2 = auth.create_token(user2.clone()).unwrap();

    // Assert
    assert_ne!(token1.access_token, token2.access_token);

    let validated1 = auth.validate_token(&token1.access_token).unwrap();
    let validated2 = auth.validate_token(&token2.access_token).unwrap();

    assert_eq!(validated1.as_str(), user1.as_str());
    assert_eq!(validated2.as_str(), user2.as_str());
}

/// Test token contains expected claims
#[test]
fn test_token_claims_structure() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Act
    let token = auth.create_token(user_id).unwrap();

    // Assert - token should be a valid JWT with 3 parts
    let parts: Vec<&str> = token.access_token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts (header.payload.signature)");
}

/// Test token type is Bearer
#[test]
fn test_token_type() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Act
    let token = auth.create_token(user_id).unwrap();

    // Assert
    assert_eq!(token.token_type, "Bearer");
}

/// Test token expires_at is in the future
#[test]
fn test_token_expires_at_future() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Act
    let token = auth.create_token(user_id).unwrap();

    // Assert
    let now = chrono::Utc::now();
    assert!(token.expires_at > now, "Token should expire in the future");
}

/// Test user ID with special characters
#[test]
fn test_user_id_special_characters() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("user@example.com".to_string());

    // Act
    let token = auth.create_token(user_id.clone()).unwrap();
    let validated = auth.validate_token(&token.access_token).unwrap();

    // Assert
    assert_eq!(validated.as_str(), user_id.as_str());
}

/// Test user ID with Unicode characters
#[test]
fn test_user_id_unicode() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("用户123".to_string());

    // Act
    let token = auth.create_token(user_id.clone()).unwrap();
    let validated = auth.validate_token(&token.access_token).unwrap();

    // Assert
    assert_eq!(validated.as_str(), user_id.as_str());
}

/// Test concurrent token creation and validation
#[test]
fn test_concurrent_token_operations() {
    use std::sync::Arc;
    use std::thread;

    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = Arc::new(AuthService::new(secret));
    let mut handles = vec![];

    // Act - create and validate tokens concurrently
    for i in 0..10 {
        let auth_clone = Arc::clone(&auth);
        let handle = thread::spawn(move || {
            let user_id = UserId::new(format!("user{}", i));
            let token = auth_clone.create_token(user_id.clone()).unwrap();
            let validated = auth_clone.validate_token(&token.access_token).unwrap();
            assert_eq!(validated.as_str(), user_id.as_str());
        });
        handles.push(handle);
    }

    // Assert - all operations should complete successfully
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

/// Test malformed JWT token
#[test]
fn test_malformed_jwt_token() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);

    // Act & Assert - various malformed tokens
    assert!(auth.validate_token("not.a.valid.jwt.token").is_err());
    assert!(auth.validate_token("onlyonepart").is_err());
    assert!(auth.validate_token("two.parts").is_err());
    assert!(auth.validate_token("").is_err());
}

/// Test token reuse is idempotent
#[test]
fn test_token_reuse_idempotent() {
    // Arrange
    let secret = b"test_secret_key_at_least_32_bytes_long_for_security";
    let auth = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());
    let token = auth.create_token(user_id.clone()).unwrap();

    // Act - validate the same token multiple times
    let result1 = auth.validate_token(&token.access_token);
    let result2 = auth.validate_token(&token.access_token);
    let result3 = auth.validate_token(&token.access_token);

    // Assert - all validations should succeed with same user
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert!(result3.is_ok());
    assert_eq!(result1.unwrap().as_str(), user_id.as_str());
    assert_eq!(result2.unwrap().as_str(), user_id.as_str());
    assert_eq!(result3.unwrap().as_str(), user_id.as_str());
}