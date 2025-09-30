// Integration tests for authentication middleware
// Per spec-kit/011-authentication-spec.md: Testing requirements

use actix_web::{test, web, App, HttpResponse};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use web_terminal::server::middleware::auth::{JwtAuthMiddleware, JwtClaims};

/// Test JWT claims
fn create_test_claims(exp_offset: i64) -> JwtClaims {
    let now = Utc::now().timestamp() as usize;
    JwtClaims {
        sub: "user:default/test".to_string(),
        iss: "https://test.example.com".to_string(),
        aud: vec!["web-terminal".to_string()],
        exp: (now as i64 + exp_offset) as usize,
        iat: now,
        nbf: Some(now),
        ent: Some(vec![
            "user:default/test".to_string(),
            "group:default/developers".to_string(),
        ]),
    }
}

/// Create test JWT token
fn create_test_token(secret: &[u8], exp_offset: i64) -> String {
    let claims = create_test_claims(exp_offset);
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .expect("Failed to encode test token")
}

/// Test endpoint handler
async fn protected_endpoint() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Success",
        "protected": true
    }))
}

#[actix_web::test]
async fn test_http_endpoint_without_token() {
    // Per spec-kit/011-authentication-spec.md: Missing token returns 401
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_middleware = JwtAuthMiddleware::new(secret);

    let app = test::init_service(
        App::new()
            .wrap(auth_middleware)
            .route("/protected", web::get().to(protected_endpoint)),
    )
    .await;

    let req = test::TestRequest::get().uri("/protected").to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_http_endpoint_with_invalid_token() {
    // Per spec-kit/011-authentication-spec.md: Invalid token returns 401
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_middleware = JwtAuthMiddleware::new(secret);

    let app = test::init_service(
        App::new()
            .wrap(auth_middleware)
            .route("/protected", web::get().to(protected_endpoint)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_http_endpoint_with_valid_token() {
    // Per spec-kit/011-authentication-spec.md: Valid token returns 200
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_middleware = JwtAuthMiddleware::new(secret);
    let token = create_test_token(secret, 3600);

    let app = test::init_service(
        App::new()
            .wrap(auth_middleware)
            .route("/protected", web::get().to(protected_endpoint)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 200 OK
    assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
}

#[actix_web::test]
async fn test_http_endpoint_with_expired_token() {
    // Per spec-kit/011-authentication-spec.md: Expired token returns 401
    let secret = b"test_secret_key_at_least_32_bytes_long";
    let auth_middleware = JwtAuthMiddleware::new(secret);
    let token = create_test_token(secret, -3600); // Expired 1 hour ago

    let app = test::init_service(
        App::new()
            .wrap(auth_middleware)
            .route("/protected", web::get().to(protected_endpoint)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 401 Unauthorized
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

// WebSocket authentication tests
// Note: Full WebSocket testing requires more setup with actix-web-actors
// These tests verify the protocol message types are correct

#[test]
fn test_authenticate_message_serialization() {
    use web_terminal::protocol::ClientMessage;

    let msg = ClientMessage::Authenticate {
        token: "test_token".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"authenticate""#));
    assert!(json.contains(r#""token":"test_token""#));
}

#[test]
fn test_authenticated_message_serialization() {
    use web_terminal::protocol::ServerMessage;

    let msg = ServerMessage::Authenticated {
        user_id: "user:default/test".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"authenticated""#));
    assert!(json.contains(r#""user_id":"user:default/test""#));
}

#[test]
fn test_authenticate_message_deserialization() {
    use web_terminal::protocol::ClientMessage;

    let json = r#"{"type":"authenticate","token":"test_token"}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();

    match msg {
        ClientMessage::Authenticate { token } => {
            assert_eq!(token, "test_token");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_user_context_extraction() {
    // Test that UserContext correctly extracts groups from Backstage ent claim
    use web_terminal::server::middleware::auth::UserContext;

    let claims = create_test_claims(3600);
    let context = UserContext::from_claims(claims);

    assert_eq!(context.user_id.as_str(), "user:default/test");
    assert_eq!(context.groups.len(), 1);
    assert!(context
        .groups
        .contains(&"group:default/developers".to_string()));
    assert_eq!(context.provider, "https://test.example.com");
}
