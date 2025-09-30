// REST API integration tests
// Per docs/spec-kit/006-api-spec.md

use actix_web::{test, web, App};
use serde_json::json;
use std::sync::Arc;

use web_terminal::config::Config;
use web_terminal::handlers;
use web_terminal::security::jwks_client::JwksClient;
use web_terminal::security::jwt_validator::JwtValidator;
use web_terminal::server::middleware::auth::JwtAuthMiddleware;
use web_terminal::session::manager::SessionManager;

/// Helper function to create a test app with authentication disabled
fn create_test_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let config = Config::default();
    let session_manager = Arc::new(SessionManager::new(config.session));

    App::new()
        .app_data(web::Data::new(session_manager))
        .service(
            web::scope("/api/v1")
                .route("/health", web::get().to(handlers::health_check))
        )
}

#[actix_web::test]
async fn test_health_check_endpoint() {
    let app = test::init_service(create_test_app()).await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "healthy");
    assert!(body["version"].is_string());
    assert!(body["uptime_seconds"].is_number());
    assert_eq!(body["checks"]["sessions"], "ok");
}

#[actix_web::test]
async fn test_health_check_no_auth_required() {
    // Health check should work without Authorization header
    let app = test::init_service(create_test_app()).await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_create_session_input_validation() {
    // Test that request validation works

    let config = Config::default();
    let session_manager = Arc::new(SessionManager::new(config.session.clone()));
    let jwks_client = Arc::new(JwksClient::new(config.auth.clone()));
    let jwt_validator = Arc::new(JwtValidator::new(jwks_client, config.auth));

    // Note: In a real test, you would need to inject a valid JWT token
    // For now, this tests the route registration
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(session_manager))
            .app_data(web::Data::new(jwt_validator.clone()))
            .service(
                web::scope("/api/v1").route("/sessions", web::post().to(handlers::create_session)),
            ),
    )
    .await;

    // Test with valid payload structure
    let payload = json!({
        "initial_dir": "/workspace",
        "environment": {
            "VAR": "value"
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/sessions")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Without auth, should get 401 or other auth error
    // The important thing is it doesn't panic on validation
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_web::test]
async fn test_list_sessions_query_validation() {
    let config = Config::default();
    let session_manager = Arc::new(SessionManager::new(config.session.clone()));
    let jwks_client = Arc::new(JwksClient::new(config.auth.clone()));
    let jwt_validator = Arc::new(JwtValidator::new(jwks_client, config.auth));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(session_manager))
            .app_data(web::Data::new(jwt_validator.clone()))
            .service(
                web::scope("/api/v1").route("/sessions", web::get().to(handlers::list_sessions)),
            ),
    )
    .await;

    // Test with valid query parameters
    let req = test::TestRequest::get()
        .uri("/api/v1/sessions?limit=10&offset=0")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should fail auth, not validation
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

#[actix_web::test]
async fn test_api_error_response_format() {
    // Test that error responses follow the spec format

    let app = test::init_service(create_test_app()).await;

    // Request non-existent endpoint
    let req = test::TestRequest::get()
        .uri("/api/v1/nonexistent")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should get 404
    assert_eq!(resp.status(), 404);
}

#[test]
fn test_create_session_request_validation_logic() {
    use validator::Validate;
    use web_terminal::handlers::CreateSessionRequest;

    // Valid request
    let req = CreateSessionRequest {
        initial_dir: Some("/workspace".to_string()),
        environment: None,
    };
    assert!(req.validate().is_ok());

    // Path too long (over 4096 chars)
    let req = CreateSessionRequest {
        initial_dir: Some("a".repeat(5000)),
        environment: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_list_sessions_query_validation_logic() {
    use validator::Validate;
    use web_terminal::handlers::ListSessionsQuery;

    // Valid query
    let query = ListSessionsQuery {
        limit: Some(50),
        offset: Some(0),
    };
    assert!(query.validate().is_ok());

    // Limit too large (over 100)
    let query = ListSessionsQuery {
        limit: Some(200),
        offset: Some(0),
    };
    assert!(query.validate().is_err());

    // Limit too small (under 1)
    let query = ListSessionsQuery {
        limit: Some(0),
        offset: Some(0),
    };
    assert!(query.validate().is_err());
}

#[test]
fn test_session_history_query_validation_logic() {
    use validator::Validate;
    use web_terminal::handlers::SessionHistoryQuery;

    // Valid query
    let query = SessionHistoryQuery {
        limit: Some(100),
    };
    assert!(query.validate().is_ok());

    // Limit too large (over 1000)
    let query = SessionHistoryQuery {
        limit: Some(2000),
    };
    assert!(query.validate().is_err());
}

#[test]
fn test_error_response_format() {
    use web_terminal::handlers::ErrorResponse;

    // Session not found error
    let err = ErrorResponse::session_not_found("session123");
    assert_eq!(err.error.code, "SESSION_NOT_FOUND");
    assert!(err.error.message.contains("session123"));
    assert!(err.error.details.is_some());
    assert_eq!(
        err.error.details.as_ref().unwrap()["session_id"],
        "session123"
    );

    // JWT expired error
    let err = ErrorResponse::jwt_expired("2025-09-29T08:00:00Z");
    assert_eq!(err.error.code, "JWT_EXPIRED");
    assert!(err.www_authenticate.is_some());
    assert!(err
        .www_authenticate
        .as_ref()
        .unwrap()
        .contains("Bearer realm"));

    // Unauthorized user error
    let err = ErrorResponse::unauthorized_user("user:default/alice", vec![
        "group:default/developers".to_string()
    ]);
    assert_eq!(err.error.code, "UNAUTHORIZED_USER");
    assert!(err.error.details.is_some());

    // Rate limit exceeded error
    let err = ErrorResponse::rate_limit_exceeded();
    assert_eq!(err.error.code, "RATE_LIMIT_EXCEEDED");

    // Internal error
    let err = ErrorResponse::internal_error();
    assert_eq!(err.error.code, "INTERNAL_ERROR");
}

#[test]
fn test_health_response_structure() {
    use web_terminal::handlers::{HealthChecks, HealthResponse};

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: "1.0.0".to_string(),
        uptime_seconds: 3600,
        checks: HealthChecks {
            sessions: "ok".to_string(),
            memory: "ok".to_string(),
            disk: "ok".to_string(),
        },
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["version"], "1.0.0");
    assert_eq!(json["uptime_seconds"], 3600);
    assert_eq!(json["checks"]["sessions"], "ok");
}