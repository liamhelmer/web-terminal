// Integration tests for rate limiting
// Per spec-kit/002-architecture.md Layer 1 Network Security: Rate Limiting

use actix_web::{test, web, App, HttpResponse};
use std::time::Duration;
use tokio::time::sleep;
use web_terminal::server::middleware::{RateLimitConfig, RateLimitMiddleware};

// Simple test handler
async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::test]
async fn test_ip_rate_limit_enforcement() {
    // Create rate limit config with low limits for testing
    let config = RateLimitConfig {
        ip_requests_per_minute: 5,
        user_requests_per_hour: 1000,
        lockout_threshold: 3,
        lockout_duration_minutes: 1,
    };

    let middleware = RateLimitMiddleware::new(config);
    let app = test::init_service(
        App::new()
            .wrap(middleware)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // First 5 requests should succeed
    for i in 0..5 {
        let req = test::TestRequest::get()
            .uri("/test")
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "Request {} should succeed",
            i + 1
        );
    }

    // 6th request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::TOO_MANY_REQUESTS,
        "6th request should be rate limited"
    );

    // Check rate limit headers
    let headers = resp.headers();
    assert!(headers.contains_key("x-ratelimit-limit"));
    assert!(headers.contains_key("x-ratelimit-remaining"));
    assert!(headers.contains_key("x-ratelimit-reset"));
}

#[actix_web::test]
async fn test_rate_limit_headers() {
    let config = RateLimitConfig {
        ip_requests_per_minute: 10,
        user_requests_per_hour: 1000,
        lockout_threshold: 5,
        lockout_duration_minutes: 1,
    };

    let middleware = RateLimitMiddleware::new(config);
    let app = test::init_service(
        App::new()
            .wrap(middleware)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;

    // Check for rate limit headers
    let headers = resp.headers();
    assert!(
        headers.contains_key("x-ratelimit-limit"),
        "X-RateLimit-Limit header should be present"
    );
    assert!(
        headers.contains_key("x-ratelimit-remaining"),
        "X-RateLimit-Remaining header should be present"
    );
    assert!(
        headers.contains_key("x-ratelimit-reset"),
        "X-RateLimit-Reset header should be present"
    );

    // Verify limit value
    let limit = headers
        .get("x-ratelimit-limit")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(limit, "10");
}

#[actix_web::test]
async fn test_lockout_mechanism() {
    let config = RateLimitConfig {
        ip_requests_per_minute: 2,
        user_requests_per_hour: 1000,
        lockout_threshold: 3,
        lockout_duration_minutes: 1,
    };

    let middleware = RateLimitMiddleware::new(config);
    let app = test::init_service(
        App::new()
            .wrap(middleware)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Exhaust rate limit and trigger violations
    for _ in 0..10 {
        let req = test::TestRequest::get()
            .uri("/test")
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .to_request();
        let _ = test::call_service(&app, req).await;
    }

    // Next request should show lockout
    let req = test::TestRequest::get()
        .uri("/test")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::TOO_MANY_REQUESTS,
        "Request should be blocked due to lockout"
    );

    // Check for lockout-specific headers
    let headers = resp.headers();
    assert!(
        headers.contains_key("x-ratelimit-reset"),
        "X-RateLimit-Reset header should indicate lockout end time"
    );
}

#[actix_web::test]
async fn test_metrics_tracking() {
    let config = RateLimitConfig {
        ip_requests_per_minute: 3,
        user_requests_per_hour: 1000,
        lockout_threshold: 2,
        lockout_duration_minutes: 1,
    };

    let middleware = RateLimitMiddleware::new(config);
    let metrics = middleware.metrics();

    let app = test::init_service(
        App::new()
            .wrap(middleware)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Make several requests to trigger violations
    for _ in 0..6 {
        let req = test::TestRequest::get()
            .uri("/test")
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .to_request();
        let _ = test::call_service(&app, req).await;
    }

    // Check metrics
    let (total_requests, violations, lockouts) = metrics.get_stats();
    assert!(
        total_requests >= 6,
        "Should have tracked at least 6 requests"
    );
    assert!(violations > 0, "Should have tracked violations");
    // Note: lockouts may be 0 or more depending on timing
}

#[actix_web::test]
async fn test_different_ips_independent_limits() {
    let config = RateLimitConfig {
        ip_requests_per_minute: 3,
        user_requests_per_hour: 1000,
        lockout_threshold: 5,
        lockout_duration_minutes: 1,
    };

    let middleware = RateLimitMiddleware::new(config);
    let app = test::init_service(
        App::new()
            .wrap(middleware)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Exhaust limit for first IP
    for _ in 0..5 {
        let req = test::TestRequest::get()
            .uri("/test")
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .to_request();
        let _ = test::call_service(&app, req).await;
    }

    // Request from different IP should still work
    let req = test::TestRequest::get()
        .uri("/test")
        .peer_addr("192.168.1.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Different IP should have independent rate limit"
    );
}

#[actix_web::test]
async fn test_websocket_rate_limit() {
    use web_terminal::server::middleware::{WebSocketRateLimitConfig, WebSocketRateLimiter};

    let config = WebSocketRateLimitConfig {
        max_messages_per_second: 5,
        warning_threshold_percent: 80,
        grace_period_seconds: 1,
    };

    let mut limiter = WebSocketRateLimiter::new(config);

    // First 5 messages should be allowed
    for i in 0..5 {
        let result = limiter.check_message();
        assert_eq!(
            result,
            web_terminal::server::middleware::RateLimitResult::Allowed,
            "Message {} should be allowed",
            i + 1
        );
    }

    // Additional messages should be throttled
    let result = limiter.check_message();
    assert!(
        matches!(
            result,
            web_terminal::server::middleware::RateLimitResult::Throttled
                | web_terminal::server::middleware::RateLimitResult::Warning { .. }
        ),
        "Excess message should be throttled or warned"
    );
}

#[actix_web::test]
async fn test_rate_limit_reset() {
    let config = RateLimitConfig {
        ip_requests_per_minute: 2,
        user_requests_per_hour: 1000,
        lockout_threshold: 5,
        lockout_duration_minutes: 1,
    };

    let middleware = RateLimitMiddleware::new(config);
    let app = test::init_service(
        App::new()
            .wrap(middleware)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Exhaust rate limit
    for _ in 0..3 {
        let req = test::TestRequest::get()
            .uri("/test")
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .to_request();
        let _ = test::call_service(&app, req).await;
    }

    // Next request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::TOO_MANY_REQUESTS);

    // Wait for rate limit window to reset (61 seconds to be safe)
    sleep(Duration::from_secs(61)).await;

    // New request should succeed after reset
    let req = test::TestRequest::get()
        .uri("/test")
        .peer_addr("127.0.0.1:8080".parse().unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Request should succeed after rate limit reset"
    );
}