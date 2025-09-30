// TLS and security headers integration tests
// Per spec-kit/009-deployment-spec.md: TLS requirements
// Per spec-kit/002-architecture.md: Security architecture

#![cfg(feature = "tls")]

use actix_web::{test, web, App, HttpResponse};
use web_terminal::server::middleware::{
    cors::CorsConfig,
    security_headers::{SecurityHeadersMiddleware, SecurityHeadersConfig},
};

#[actix_web::test]
async fn test_security_headers_present() {
    // Per spec-kit/002-architecture.md: Defense in depth strategy
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeadersMiddleware::default())
            .route("/", web::get().to(|| async { HttpResponse::Ok().body("test") }))
    ).await;

    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;

    // Verify all required security headers are present
    let headers = resp.headers();

    // HSTS: Enforce HTTPS
    assert!(
        headers.contains_key(actix_web::http::header::STRICT_TRANSPORT_SECURITY),
        "Missing HSTS header"
    );
    let hsts = headers.get(actix_web::http::header::STRICT_TRANSPORT_SECURITY).unwrap();
    assert!(hsts.to_str().unwrap().contains("max-age=31536000"), "HSTS max-age incorrect");

    // CSP: Prevent XSS
    assert!(
        headers.contains_key(actix_web::http::header::CONTENT_SECURITY_POLICY),
        "Missing CSP header"
    );
    let csp = headers.get(actix_web::http::header::CONTENT_SECURITY_POLICY).unwrap();
    assert!(csp.to_str().unwrap().contains("default-src 'self'"), "CSP policy incorrect");

    // X-Frame-Options: Prevent clickjacking
    assert!(
        headers.contains_key(actix_web::http::header::X_FRAME_OPTIONS),
        "Missing X-Frame-Options header"
    );
    let frame_options = headers.get(actix_web::http::header::X_FRAME_OPTIONS).unwrap();
    assert_eq!(frame_options.to_str().unwrap(), "DENY", "X-Frame-Options incorrect");

    // X-Content-Type-Options: Prevent MIME sniffing
    assert!(
        headers.contains_key(actix_web::http::header::X_CONTENT_TYPE_OPTIONS),
        "Missing X-Content-Type-Options header"
    );
    let content_type_options = headers.get(actix_web::http::header::X_CONTENT_TYPE_OPTIONS).unwrap();
    assert_eq!(content_type_options.to_str().unwrap(), "nosniff", "X-Content-Type-Options incorrect");

    // X-XSS-Protection: Enable XSS filter
    assert!(
        headers.contains_key(actix_web::http::header::X_XSS_PROTECTION),
        "Missing X-XSS-Protection header"
    );
    let xss_protection = headers.get(actix_web::http::header::X_XSS_PROTECTION).unwrap();
    assert_eq!(xss_protection.to_str().unwrap(), "1; mode=block", "X-XSS-Protection incorrect");

    // Referrer-Policy: Control referrer information
    assert!(
        headers.contains_key(actix_web::http::header::REFERRER_POLICY),
        "Missing Referrer-Policy header"
    );
    let referrer_policy = headers.get(actix_web::http::header::REFERRER_POLICY).unwrap();
    assert_eq!(
        referrer_policy.to_str().unwrap(),
        "strict-origin-when-cross-origin",
        "Referrer-Policy incorrect"
    );
}

#[actix_web::test]
async fn test_cors_headers_present() {
    // Per spec-kit/002-architecture.md: CORS policy
    let cors_config = CorsConfig {
        allowed_origins: vec!["https://example.com".to_string()],
        allowed_methods: vec!["GET".to_string(), "POST".to_string()],
        allowed_headers: vec!["Authorization".to_string(), "Content-Type".to_string()],
        max_age: 3600,
        supports_credentials: true,
    };

    let app = test::init_service(
        App::new()
            .wrap(cors_config.build())
            .route("/", web::get().to(|| async { HttpResponse::Ok().body("test") }))
    ).await;

    // Test preflight request
    let req = test::TestRequest::with_uri("/")
        .method(actix_web::http::Method::OPTIONS)
        .insert_header(("Origin", "https://example.com"))
        .insert_header(("Access-Control-Request-Method", "GET"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    // Verify CORS headers
    assert!(
        headers.contains_key(actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            || headers.contains_key(actix_web::http::header::ACCESS_CONTROL_ALLOW_CREDENTIALS),
        "Missing CORS headers"
    );
}

#[actix_web::test]
async fn test_security_headers_custom_config() {
    // Test custom security headers configuration
    let config = SecurityHeadersConfig {
        enable_hsts: false,  // Disable HSTS for this test
        hsts_max_age: 31536000,
        enable_csp: true,
        csp_policy: "default-src 'none'".to_string(),
        enable_frame_options: true,
        frame_options: "SAMEORIGIN".to_string(),
    };

    let app = test::init_service(
        App::new()
            .wrap(SecurityHeadersMiddleware::new(config))
            .route("/", web::get().to(|| async { HttpResponse::Ok().body("test") }))
    ).await;

    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;

    let headers = resp.headers();

    // HSTS should be absent (disabled)
    assert!(
        !headers.contains_key(actix_web::http::header::STRICT_TRANSPORT_SECURITY),
        "HSTS should be disabled"
    );

    // Custom CSP policy
    let csp = headers.get(actix_web::http::header::CONTENT_SECURITY_POLICY).unwrap();
    assert_eq!(csp.to_str().unwrap(), "default-src 'none'", "Custom CSP policy not applied");

    // Custom X-Frame-Options
    let frame_options = headers.get(actix_web::http::header::X_FRAME_OPTIONS).unwrap();
    assert_eq!(frame_options.to_str().unwrap(), "SAMEORIGIN", "Custom X-Frame-Options not applied");
}

#[test]
fn test_tls_config_validation() {
    use web_terminal::server::tls::validate_tls_files;

    // Test with non-existent files
    let result = validate_tls_files("nonexistent_cert.pem", "nonexistent_key.pem");
    assert!(result.is_err(), "Should fail with non-existent files");
}

#[actix_web::test]
async fn test_cors_wildcard_origin() {
    // Test CORS with wildcard origin (development mode)
    let cors_config = CorsConfig {
        allowed_origins: vec!["*".to_string()],
        allowed_methods: vec!["GET".to_string()],
        allowed_headers: vec!["Content-Type".to_string()],
        max_age: 3600,
        supports_credentials: true,
    };

    let app = test::init_service(
        App::new()
            .wrap(cors_config.build())
            .route("/", web::get().to(|| async { HttpResponse::Ok().body("test") }))
    ).await;

    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Origin", "http://any-origin.com"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Request should succeed with wildcard CORS");
}

#[actix_web::test]
async fn test_security_headers_applied_to_all_routes() {
    // Verify security headers are applied to all routes
    let app = test::init_service(
        App::new()
            .wrap(SecurityHeadersMiddleware::default())
            .route("/api/health", web::get().to(|| async { HttpResponse::Ok().json("OK") }))
            .route("/api/data", web::post().to(|| async { HttpResponse::Created().json("Created") }))
    ).await;

    // Test GET request
    let req = test::TestRequest::get().uri("/api/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.headers().contains_key(actix_web::http::header::STRICT_TRANSPORT_SECURITY),
        "Security headers should be on GET"
    );

    // Test POST request
    let req = test::TestRequest::post().uri("/api/data").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.headers().contains_key(actix_web::http::header::STRICT_TRANSPORT_SECURITY),
        "Security headers should be on POST"
    );
}

#[test]
fn test_cors_config_production() {
    // Test production CORS configuration
    let config = CorsConfig::production(vec!["https://example.com".to_string()]);
    assert_eq!(config.allowed_origins, vec!["https://example.com"]);
    assert!(config.supports_credentials);
    assert_eq!(config.max_age, 3600);
}

#[test]
fn test_cors_config_development() {
    // Test development CORS configuration
    let config = CorsConfig::development();
    assert_eq!(config.allowed_origins, vec!["*"]);
    assert!(config.supports_credentials);
}

#[test]
fn test_security_headers_config_defaults() {
    // Test security headers configuration defaults
    let config = SecurityHeadersConfig::default();
    assert!(config.enable_hsts);
    assert_eq!(config.hsts_max_age, 31536000);
    assert!(config.enable_csp);
    assert!(config.enable_frame_options);
    assert_eq!(config.frame_options, "DENY");
}