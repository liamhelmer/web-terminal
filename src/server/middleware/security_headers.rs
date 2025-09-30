// Security headers middleware
// Per spec-kit/002-architecture.md Layer 1: Network Security

use std::future::{ready, Ready};
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;

/// Security headers middleware
/// Per spec-kit/002-architecture.md: Defense in depth strategy
///
/// Adds security headers to all HTTP responses:
/// - Strict-Transport-Security (HSTS)
/// - Content-Security-Policy (CSP)
/// - X-Frame-Options
/// - X-Content-Type-Options
/// - X-XSS-Protection
/// - Referrer-Policy
#[derive(Clone)]
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
}

#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS header
    pub enable_hsts: bool,

    /// HSTS max-age in seconds (default: 1 year)
    pub hsts_max_age: u32,

    /// Enable CSP header
    pub enable_csp: bool,

    /// CSP policy (default: default-src 'self')
    pub csp_policy: String,

    /// Enable X-Frame-Options
    pub enable_frame_options: bool,

    /// X-Frame-Options value (default: DENY)
    pub frame_options: String,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            enable_csp: true,
            csp_policy: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".to_string(),
            enable_frame_options: true,
            frame_options: "DENY".to_string(),
        }
    }
}

impl SecurityHeadersMiddleware {
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }
}

impl Default for SecurityHeadersMiddleware {
    fn default() -> Self {
        Self::new(SecurityHeadersConfig::default())
    }
}

impl<S, B> Transform<S, ServiceRequest> for SecurityHeadersMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersMiddlewareService {
            service,
            config: self.config.clone(),
        }))
    }
}

pub struct SecurityHeadersMiddlewareService<S> {
    service: S,
    config: SecurityHeadersConfig,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let config = self.config.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;

            // Add security headers to response
            let headers = res.headers_mut();

            // HSTS: Enforce HTTPS
            if config.enable_hsts {
                headers.insert(
                    actix_web::http::header::STRICT_TRANSPORT_SECURITY,
                    format!("max-age={}", config.hsts_max_age).parse().unwrap(),
                );
            }

            // CSP: Prevent XSS attacks
            if config.enable_csp {
                headers.insert(
                    actix_web::http::header::CONTENT_SECURITY_POLICY,
                    config.csp_policy.parse().unwrap(),
                );
            }

            // X-Frame-Options: Prevent clickjacking
            if config.enable_frame_options {
                headers.insert(
                    actix_web::http::header::X_FRAME_OPTIONS,
                    config.frame_options.parse().unwrap(),
                );
            }

            // X-Content-Type-Options: Prevent MIME sniffing
            headers.insert(
                actix_web::http::header::X_CONTENT_TYPE_OPTIONS,
                "nosniff".parse().unwrap(),
            );

            // X-XSS-Protection: Enable XSS filter
            headers.insert(
                actix_web::http::header::X_XSS_PROTECTION,
                "1; mode=block".parse().unwrap(),
            );

            // Referrer-Policy: Control referrer information
            headers.insert(
                actix_web::http::header::REFERRER_POLICY,
                "strict-origin-when-cross-origin".parse().unwrap(),
            );

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().body("test")
    }

    #[actix_web::test]
    async fn test_security_headers_applied() {
        let app = test::init_service(
            App::new()
                .wrap(SecurityHeadersMiddleware::default())
                .route("/", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;

        // Verify security headers present
        let headers = resp.headers();

        assert!(headers.contains_key(actix_web::http::header::STRICT_TRANSPORT_SECURITY));
        assert!(headers.contains_key(actix_web::http::header::CONTENT_SECURITY_POLICY));
        assert!(headers.contains_key(actix_web::http::header::X_FRAME_OPTIONS));
        assert!(headers.contains_key(actix_web::http::header::X_CONTENT_TYPE_OPTIONS));
        assert!(headers.contains_key(actix_web::http::header::X_XSS_PROTECTION));
        assert!(headers.contains_key(actix_web::http::header::REFERRER_POLICY));
    }

    #[test]
    fn test_security_headers_config_default() {
        let config = SecurityHeadersConfig::default();
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31536000);
        assert!(config.enable_csp);
        assert!(config.enable_frame_options);
    }
}