// Integration tests for JWT validation
// Per 011-authentication-spec.md section 12: Testing Requirements

use std::sync::Arc;
use std::time::Duration;
use web_terminal::security::{Claims, JwksClient, JwksProvider, JwtValidator, ValidationError};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Create a test JWKS response
fn create_test_jwks() -> serde_json::Value {
    serde_json::json!({
        "keys": [
            {
                "kid": "test-key-1",
                "kty": "RSA",
                "alg": "RS256",
                "use": "sig",
                "n": "0vx7agoebGcQVeq4...",
                "e": "AQAB"
            },
            {
                "kid": "test-key-2",
                "kty": "RSA",
                "alg": "RS384",
                "use": "sig",
                "n": "xGQ7agoeblVeq4cv...",
                "e": "AQAB"
            }
        ]
    })
}

/// Create a test JWT token (for structure testing only, not cryptographically valid)
fn create_test_token_header(kid: &str, alg: &str) -> String {
    let header = serde_json::json!({
        "typ": "JWT",
        "alg": alg,
        "kid": kid
    });

    let header_b64 = base64::encode_config(
        serde_json::to_string(&header).unwrap(),
        base64::URL_SAFE_NO_PAD,
    );

    format!("{}.payload.signature", header_b64)
}

#[tokio::test]
async fn test_jwks_fetch_success() {
    // Start mock server
    let mock_server = MockServer::start().await;

    // Configure mock endpoint
    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server)
        .await;

    // Create JWKS client
    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Fetch keys
    let keys = client
        .fetch_keys("test-provider")
        .await
        .expect("Failed to fetch keys");

    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].kid, "test-key-1");
    assert_eq!(keys[0].alg, "RS256");
    assert_eq!(keys[1].kid, "test-key-2");
    assert_eq!(keys[1].alg, "RS384");
}

#[tokio::test]
async fn test_jwks_cache_behavior() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .expect(1) // Should only be called once due to caching
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // First fetch - should hit the server
    let keys1 = client
        .fetch_keys("test-provider")
        .await
        .expect("First fetch failed");

    // Second fetch - should use cache
    let keys2 = client
        .fetch_keys("test-provider")
        .await
        .expect("Second fetch failed");

    assert_eq!(keys1.len(), keys2.len());
    assert_eq!(keys1[0].kid, keys2[0].kid);
}

#[tokio::test]
async fn test_jwks_cache_expiration() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .expect(2) // Should be called twice: initial + after expiration
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_millis(100), // Very short TTL for testing
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // First fetch
    client
        .fetch_keys("test-provider")
        .await
        .expect("First fetch failed");

    // Wait for cache to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Second fetch - should hit server again
    client
        .fetch_keys("test-provider")
        .await
        .expect("Second fetch failed");
}

#[tokio::test]
async fn test_jwks_get_specific_key() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Get specific key
    let key = client
        .get_key("test-key-1", "test-provider")
        .await
        .expect("Failed to get key")
        .expect("Key not found");

    assert_eq!(key.kid, "test-key-1");
    assert_eq!(key.alg, "RS256");

    // Try to get non-existent key
    let missing_key = client
        .get_key("non-existent", "test-provider")
        .await
        .expect("Failed to query key");

    assert!(missing_key.is_none());
}

#[tokio::test]
async fn test_jwks_http_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Should fail with error
    let result = client.fetch_keys("test-provider").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwks_invalid_json() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Should fail with JSON error
    let result = client.fetch_keys("test-provider").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwks_empty_keys() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "keys": []
        })))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Should fail with empty keys error
    let result = client.fetch_keys("test-provider").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwks_cache_stats() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Initially no stats
    let stats = client.cache_stats();
    assert_eq!(stats.len(), 0);

    // Fetch keys
    client
        .fetch_keys("test-provider")
        .await
        .expect("Failed to fetch keys");

    // Should have stats now
    let stats = client.cache_stats();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].provider, "test-provider");
    assert_eq!(stats[0].keys_count, 2);
    assert!(!stats[0].is_expired);
}

#[tokio::test]
async fn test_jwks_background_refresh() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .expect(2) // Initial fetch + background refresh
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = Arc::new(JwksClient::new(providers).expect("Failed to create client"));

    // Initial fetch
    client
        .fetch_keys("test-provider")
        .await
        .expect("Initial fetch failed");

    // Start background refresh task
    let _refresh_handle = client.clone().start_refresh_task();

    // Wait for background refresh (15 minutes in production, but the task runs immediately on start)
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Note: In a real test, you'd configure a shorter refresh interval
}

#[tokio::test]
async fn test_jwt_validator_missing_kid() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = Arc::new(JwksClient::new(providers).expect("Failed to create client"));
    let validator = JwtValidator::new(client);

    // Create token without kid in header
    let header = serde_json::json!({
        "typ": "JWT",
        "alg": "RS256"
    });

    let header_b64 = base64::encode_config(
        serde_json::to_string(&header).unwrap(),
        base64::URL_SAFE_NO_PAD,
    );

    let token = format!("{}.payload.signature", header_b64);

    // Should fail with missing kid error
    let result = validator.validate(&token).await;
    assert!(matches!(result, Err(ValidationError::MissingKid)));
}

#[tokio::test]
async fn test_jwt_validator_key_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server)
        .await;

    let providers = vec![JwksProvider {
        name: "test-provider".to_string(),
        url: format!("{}/.well-known/jwks.json", mock_server.uri()),
        issuer: "https://test.example.com".to_string(),
        cache_ttl: Duration::from_secs(3600),
    }];

    let client = Arc::new(JwksClient::new(providers).expect("Failed to create client"));
    let validator = JwtValidator::new(client);

    // Create token with non-existent kid
    let header = serde_json::json!({
        "typ": "JWT",
        "alg": "RS256",
        "kid": "non-existent-key"
    });

    let claims = serde_json::json!({
        "sub": "user:default/test",
        "iss": "https://test.example.com",
        "aud": "web-terminal",
        "exp": 9999999999i64,
        "iat": 1735563600i64
    });

    let header_b64 = base64::encode_config(
        serde_json::to_string(&header).unwrap(),
        base64::URL_SAFE_NO_PAD,
    );

    let claims_b64 = base64::encode_config(
        serde_json::to_string(&claims).unwrap(),
        base64::URL_SAFE_NO_PAD,
    );

    let token = format!("{}.{}.signature", header_b64, claims_b64);

    // Should fail with key not found error
    let result = validator.validate(&token).await;
    assert!(matches!(result, Err(ValidationError::KeyNotFound(_))));
}

#[tokio::test]
async fn test_multiple_providers() {
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server1)
        .await;

    Mock::given(method("GET"))
        .and(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_test_jwks()))
        .mount(&mock_server2)
        .await;

    let providers = vec![
        JwksProvider {
            name: "provider1".to_string(),
            url: format!("{}/.well-known/jwks.json", mock_server1.uri()),
            issuer: "https://provider1.example.com".to_string(),
            cache_ttl: Duration::from_secs(3600),
        },
        JwksProvider {
            name: "provider2".to_string(),
            url: format!("{}/.well-known/jwks.json", mock_server2.uri()),
            issuer: "https://provider2.example.com".to_string(),
            cache_ttl: Duration::from_secs(3600),
        },
    ];

    let client = JwksClient::new(providers).expect("Failed to create client");

    // Fetch from both providers
    let keys1 = client
        .fetch_keys("provider1")
        .await
        .expect("Provider 1 fetch failed");
    let keys2 = client
        .fetch_keys("provider2")
        .await
        .expect("Provider 2 fetch failed");

    assert_eq!(keys1.len(), 2);
    assert_eq!(keys2.len(), 2);

    // Check cache stats
    let stats = client.cache_stats();
    assert_eq!(stats.len(), 2);
}
