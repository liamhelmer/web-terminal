// Integration tests for JWT authentication flow
// Per spec-kit/008-testing-spec.md - Integration Tests
//
// Tests authentication with JWT tokens

use std::sync::Arc;
use std::time::Duration;
use web_terminal::security::auth::AuthService;
use web_terminal::session::UserId;

/// Test JWT token creation and validation
///
/// Per FR-5.1: Require authentication for all connections
#[tokio::test]
async fn test_token_creation_and_validation() {
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_service = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Create token
    let token = auth_service
        .create_token(user_id.clone())
        .expect("Failed to create token");

    assert!(!token.access_token.is_empty());
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.user_id, user_id);

    // Validate token
    let validated_user_id = auth_service
        .validate_token(&token.access_token)
        .expect("Failed to validate token");

    assert_eq!(validated_user_id, user_id);
}

/// Test invalid token rejection
///
/// Per FR-5.1: Require authentication for all connections
#[tokio::test]
async fn test_invalid_token_rejection() {
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_service = AuthService::new(secret);

    // Test with invalid token
    let result = auth_service.validate_token("invalid_token_string");
    assert!(result.is_err(), "Expected invalid token to be rejected");

    // Test with empty token
    let result = auth_service.validate_token("");
    assert!(result.is_err(), "Expected empty token to be rejected");

    // Test with malformed token
    let result = auth_service.validate_token("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid");
    assert!(result.is_err(), "Expected malformed token to be rejected");
}

/// Test token expiration
///
/// Per spec-kit/003-backend-spec.md section 4.1
#[tokio::test]
async fn test_token_expiration() {
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_service = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Create token
    let token = auth_service
        .create_token(user_id.clone())
        .expect("Failed to create token");

    // Verify token is not expired immediately
    assert!(
        !auth_service.is_token_expired(&token.access_token),
        "Token should not be expired immediately"
    );

    // Note: Full expiration testing requires either:
    // 1. Creating a token with a very short expiration (requires modifying AuthService)
    // 2. Mocking time (requires additional dependencies)
    // For now, we just verify the is_token_expired method works
}

/// Test token validation with different secret
///
/// Per security requirements: tokens should not validate with wrong secret
#[tokio::test]
async fn test_token_validation_with_wrong_secret() {
    let secret1 = b"test_secret_key_at_least_32_bytes_long";
    let secret2 = b"different_secret_key_also_32_bytes_x";

    let auth_service1 = AuthService::new(secret1);
    let auth_service2 = AuthService::new(secret2);

    let user_id = UserId::new("test_user".to_string());

    // Create token with first secret
    let token = auth_service1
        .create_token(user_id.clone())
        .expect("Failed to create token");

    // Try to validate with different secret (should fail)
    let result = auth_service2.validate_token(&token.access_token);
    assert!(
        result.is_err(),
        "Token created with one secret should not validate with different secret"
    );
}

/// Test multiple tokens for same user
///
/// Verify that multiple tokens can be created and validated independently
#[tokio::test]
async fn test_multiple_tokens_same_user() {
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_service = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    // Create multiple tokens
    let token1 = auth_service
        .create_token(user_id.clone())
        .expect("Failed to create token 1");

    let token2 = auth_service
        .create_token(user_id.clone())
        .expect("Failed to create token 2");

    // Tokens should be different
    assert_ne!(
        token1.access_token, token2.access_token,
        "Each token should be unique"
    );

    // Both should validate successfully
    let validated1 = auth_service
        .validate_token(&token1.access_token)
        .expect("Failed to validate token 1");
    let validated2 = auth_service
        .validate_token(&token2.access_token)
        .expect("Failed to validate token 2");

    assert_eq!(validated1, user_id);
    assert_eq!(validated2, user_id);
}

/// Test tokens for different users
///
/// Verify that tokens correctly identify different users
#[tokio::test]
async fn test_tokens_different_users() {
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_service = AuthService::new(secret);

    let user1 = UserId::new("user1".to_string());
    let user2 = UserId::new("user2".to_string());

    // Create tokens for different users
    let token1 = auth_service
        .create_token(user1.clone())
        .expect("Failed to create token for user1");

    let token2 = auth_service
        .create_token(user2.clone())
        .expect("Failed to create token for user2");

    // Validate tokens
    let validated1 = auth_service
        .validate_token(&token1.access_token)
        .expect("Failed to validate token 1");
    let validated2 = auth_service
        .validate_token(&token2.access_token)
        .expect("Failed to validate token 2");

    // Verify correct user IDs
    assert_eq!(validated1, user1);
    assert_eq!(validated2, user2);
    assert_ne!(validated1, validated2);
}

/// Test token format
///
/// Verify tokens follow JWT format (three base64-encoded parts separated by dots)
#[tokio::test]
async fn test_token_format() {
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_service = AuthService::new(secret);
    let user_id = UserId::new("test_user".to_string());

    let token = auth_service
        .create_token(user_id.clone())
        .expect("Failed to create token");

    // JWT tokens should have 3 parts separated by dots
    let parts: Vec<&str> = token.access_token.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "JWT token should have 3 parts (header.payload.signature)"
    );

    // Each part should be non-empty
    for (i, part) in parts.iter().enumerate() {
        assert!(!part.is_empty(), "JWT token part {} should not be empty", i);
    }
}
