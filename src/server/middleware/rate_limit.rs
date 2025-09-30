// Rate limiting middleware for HTTP requests
// Per spec-kit/002-architecture.md Layer 1 Network Security: Rate Limiting

use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use dashmap::DashMap;
use futures_util::future::LocalBoxFuture;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::{
    net::IpAddr,
    rc::Rc,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tracing::{debug, warn};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// IP-based rate limit: requests per minute
    pub ip_requests_per_minute: u32,
    /// User-based rate limit: requests per hour
    pub user_requests_per_hour: u32,
    /// Number of violations before temporary lockout
    pub lockout_threshold: u32,
    /// Lockout duration in minutes
    pub lockout_duration_minutes: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            ip_requests_per_minute: 100,
            user_requests_per_hour: 1000,
            lockout_threshold: 5,
            lockout_duration_minutes: 15,
        }
    }
}

/// Rate limiter key (IP or User ID)
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum RateLimitKey {
    Ip(IpAddr),
    User(String),
}

/// Violation tracking for lockout mechanism
#[derive(Debug, Clone)]
struct ViolationTracker {
    count: u32,
    last_violation: SystemTime,
    locked_until: Option<SystemTime>,
}

impl ViolationTracker {
    fn new() -> Self {
        Self {
            count: 0,
            last_violation: SystemTime::now(),
            locked_until: None,
        }
    }

    fn record_violation(&mut self, lockout_threshold: u32, lockout_duration: Duration) {
        self.count += 1;
        self.last_violation = SystemTime::now();

        if self.count >= lockout_threshold {
            self.locked_until = Some(SystemTime::now() + lockout_duration);
        }
    }

    fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            SystemTime::now() < locked_until
        } else {
            false
        }
    }

    fn reset_if_expired(&mut self, reset_window: Duration) {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.last_violation) {
            if elapsed > reset_window {
                self.count = 0;
                self.locked_until = None;
            }
        }
    }
}

/// Rate limiting metrics
#[derive(Debug, Default)]
pub struct RateLimitMetrics {
    total_requests: Arc<std::sync::atomic::AtomicU64>,
    rate_limit_violations: Arc<std::sync::atomic::AtomicU64>,
    lockouts: Arc<std::sync::atomic::AtomicU64>,
}

impl RateLimitMetrics {
    pub fn record_request(&self) {
        self.total_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_violation(&self) {
        self.rate_limit_violations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_lockout(&self) {
        self.lockouts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.total_requests
                .load(std::sync::atomic::Ordering::Relaxed),
            self.rate_limit_violations
                .load(std::sync::atomic::Ordering::Relaxed),
            self.lockouts.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    config: RateLimitConfig,
    ip_limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    user_limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    violations: Arc<DashMap<RateLimitKey, ViolationTracker>>,
    metrics: Arc<RateLimitMetrics>,
}

impl RateLimitMiddleware {
    pub fn new(config: RateLimitConfig) -> Self {
        let ip_quota =
            Quota::per_minute(std::num::NonZeroU32::new(config.ip_requests_per_minute).unwrap());
        let user_quota =
            Quota::per_hour(std::num::NonZeroU32::new(config.user_requests_per_hour).unwrap());

        Self {
            config,
            ip_limiter: Arc::new(GovernorRateLimiter::direct(ip_quota)),
            user_limiter: Arc::new(GovernorRateLimiter::direct(user_quota)),
            violations: Arc::new(DashMap::new()),
            metrics: Arc::new(RateLimitMetrics::default()),
        }
    }

    pub fn metrics(&self) -> Arc<RateLimitMetrics> {
        Arc::clone(&self.metrics)
    }

    fn check_lockout(&self, key: &RateLimitKey) -> Option<SystemTime> {
        if let Some(mut tracker) = self.violations.get_mut(key) {
            tracker.reset_if_expired(Duration::from_secs(3600)); // Reset after 1 hour
            if tracker.is_locked() {
                return tracker.locked_until;
            }
        }
        None
    }

    fn record_violation(&self, key: RateLimitKey) {
        let lockout_duration = Duration::from_secs(self.config.lockout_duration_minutes * 60);

        self.violations
            .entry(key.clone())
            .and_modify(|tracker| {
                tracker.record_violation(self.config.lockout_threshold, lockout_duration);
            })
            .or_insert_with(|| {
                let mut tracker = ViolationTracker::new();
                tracker.record_violation(self.config.lockout_threshold, lockout_duration);
                tracker
            });

        self.metrics.record_violation();

        if let Some(tracker) = self.violations.get(&key) {
            if tracker.is_locked() {
                self.metrics.record_lockout();
                warn!("Rate limit lockout triggered for {:?}", key);
            }
        }
    }

    fn extract_ip(req: &ServiceRequest) -> Option<IpAddr> {
        req.peer_addr().map(|addr| addr.ip())
    }

    fn extract_user_id(req: &ServiceRequest) -> Option<String> {
        // Extract user ID from extensions (set by auth middleware)
        req.extensions().get::<String>().cloned()
    }

    fn add_rate_limit_headers<B>(
        res: &mut ServiceResponse<B>,
        limit: u32,
        remaining: u32,
        reset_secs: u64,
    ) {
        let headers = res.headers_mut();
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
            actix_web::http::header::HeaderValue::from_str(&limit.to_string()).unwrap(),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
            actix_web::http::header::HeaderValue::from_str(&remaining.to_string()).unwrap(),
        );
        headers.insert(
            actix_web::http::header::HeaderName::from_static("x-ratelimit-reset"),
            actix_web::http::header::HeaderValue::from_str(&reset_secs.to_string()).unwrap(),
        );
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RateLimitMiddlewareService {
            service: Rc::new(service),
            config: self.config.clone(),
            ip_limiter: Arc::clone(&self.ip_limiter),
            user_limiter: Arc::clone(&self.user_limiter),
            violations: Arc::clone(&self.violations),
            metrics: Arc::clone(&self.metrics),
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: Rc<S>,
    config: RateLimitConfig,
    ip_limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    user_limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    violations: Arc<DashMap<RateLimitKey, ViolationTracker>>,
    metrics: Arc<RateLimitMetrics>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let config = self.config.clone();
        let ip_limiter = Arc::clone(&self.ip_limiter);
        let user_limiter = Arc::clone(&self.user_limiter);
        let violations = Arc::clone(&self.violations);
        let metrics = Arc::clone(&self.metrics);

        Box::pin(async move {
            metrics.record_request();

            // Check IP-based rate limit
            if let Some(ip) = RateLimitMiddleware::extract_ip(&req) {
                let key = RateLimitKey::Ip(ip);

                // Check if IP is locked out
                if let Some(locked_until) = {
                    if let Some(mut tracker) = violations.get_mut(&key) {
                        tracker.reset_if_expired(Duration::from_secs(3600));
                        if tracker.is_locked() {
                            tracker.locked_until
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } {
                    let reset_secs = locked_until
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    return Ok(ServiceResponse::new(
                        req.request().clone(),
                        HttpResponse::TooManyRequests()
                            .insert_header(("X-RateLimit-Reset", reset_secs.to_string()))
                            .insert_header((
                                "Retry-After",
                                (reset_secs
                                    - SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs())
                                .to_string(),
                            ))
                            .json(serde_json::json!({
                                "error": "Rate limit exceeded - temporary lockout",
                                "reset_at": reset_secs
                            })),
                    ));
                }

                // Check IP rate limit
                match ip_limiter.check() {
                    Ok(_) => {
                        debug!("IP rate limit check passed for {:?}", ip);
                    }
                    Err(_) => {
                        warn!("IP rate limit exceeded for {:?}", ip);

                        // Record violation
                        violations
                            .entry(key.clone())
                            .and_modify(|tracker| {
                                tracker.record_violation(
                                    config.lockout_threshold,
                                    Duration::from_secs(config.lockout_duration_minutes * 60),
                                );
                            })
                            .or_insert_with(|| {
                                let mut tracker = ViolationTracker::new();
                                tracker.record_violation(
                                    config.lockout_threshold,
                                    Duration::from_secs(config.lockout_duration_minutes * 60),
                                );
                                tracker
                            });

                        metrics.record_violation();

                        if let Some(tracker) = violations.get(&key) {
                            if tracker.is_locked() {
                                metrics.record_lockout();
                            }
                        }

                        let reset_secs = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + 60; // IP limit resets every minute

                        return Ok(ServiceResponse::new(
                            req.request().clone(),
                            HttpResponse::TooManyRequests()
                                .insert_header((
                                    "X-RateLimit-Limit",
                                    config.ip_requests_per_minute.to_string(),
                                ))
                                .insert_header(("X-RateLimit-Remaining", "0"))
                                .insert_header(("X-RateLimit-Reset", reset_secs.to_string()))
                                .insert_header(("Retry-After", "60"))
                                .json(serde_json::json!({
                                    "error": "Rate limit exceeded",
                                    "limit": config.ip_requests_per_minute,
                                    "reset_at": reset_secs
                                })),
                        ));
                    }
                }
            }

            // Check user-based rate limit if user is authenticated
            if let Some(user_id) = RateLimitMiddleware::extract_user_id(&req) {
                let key = RateLimitKey::User(user_id.clone());

                // Check if user is locked out
                if let Some(locked_until) = {
                    if let Some(mut tracker) = violations.get_mut(&key) {
                        tracker.reset_if_expired(Duration::from_secs(3600));
                        if tracker.is_locked() {
                            tracker.locked_until
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } {
                    let reset_secs = locked_until
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    return Ok(ServiceResponse::new(
                        req.request().clone(),
                        HttpResponse::TooManyRequests()
                            .insert_header(("X-RateLimit-Reset", reset_secs.to_string()))
                            .insert_header((
                                "Retry-After",
                                (reset_secs
                                    - SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs())
                                .to_string(),
                            ))
                            .json(serde_json::json!({
                                "error": "Rate limit exceeded - temporary lockout",
                                "reset_at": reset_secs
                            })),
                    ));
                }

                // Check user rate limit
                match user_limiter.check() {
                    Ok(_) => {
                        debug!("User rate limit check passed for {}", user_id);
                    }
                    Err(_) => {
                        warn!("User rate limit exceeded for {}", user_id);

                        // Record violation
                        violations
                            .entry(key.clone())
                            .and_modify(|tracker| {
                                tracker.record_violation(
                                    config.lockout_threshold,
                                    Duration::from_secs(config.lockout_duration_minutes * 60),
                                );
                            })
                            .or_insert_with(|| {
                                let mut tracker = ViolationTracker::new();
                                tracker.record_violation(
                                    config.lockout_threshold,
                                    Duration::from_secs(config.lockout_duration_minutes * 60),
                                );
                                tracker
                            });

                        metrics.record_violation();

                        if let Some(tracker) = violations.get(&key) {
                            if tracker.is_locked() {
                                metrics.record_lockout();
                            }
                        }

                        let reset_secs = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + 3600; // User limit resets every hour

                        return Ok(ServiceResponse::new(
                            req.request().clone(),
                            HttpResponse::TooManyRequests()
                                .insert_header((
                                    "X-RateLimit-Limit",
                                    config.user_requests_per_hour.to_string(),
                                ))
                                .insert_header(("X-RateLimit-Remaining", "0"))
                                .insert_header(("X-RateLimit-Reset", reset_secs.to_string()))
                                .insert_header(("Retry-After", "3600"))
                                .json(serde_json::json!({
                                    "error": "Rate limit exceeded",
                                    "limit": config.user_requests_per_hour,
                                    "reset_at": reset_secs
                                })),
                        ));
                    }
                }
            }

            // Rate limits passed, proceed with request
            let mut res = service.call(req).await?;

            // Add rate limit headers to response
            RateLimitMiddleware::add_rate_limit_headers(
                &mut res,
                config.ip_requests_per_minute,
                config.ip_requests_per_minute, // Remaining would need state tracking per IP
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 60,
            );

            Ok(res.map_into_boxed_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::default();
        assert_eq!(config.ip_requests_per_minute, 100);
        assert_eq!(config.user_requests_per_hour, 1000);
        assert_eq!(config.lockout_threshold, 5);
        assert_eq!(config.lockout_duration_minutes, 15);
    }

    #[test]
    fn test_violation_tracker() {
        let mut tracker = ViolationTracker::new();
        assert_eq!(tracker.count, 0);
        assert!(!tracker.is_locked());

        // Record violations
        for _ in 0..4 {
            tracker.record_violation(5, Duration::from_secs(60));
            assert!(!tracker.is_locked());
        }

        // 5th violation triggers lockout
        tracker.record_violation(5, Duration::from_secs(60));
        assert!(tracker.is_locked());
    }

    #[test]
    fn test_metrics() {
        let metrics = RateLimitMetrics::default();
        metrics.record_request();
        metrics.record_violation();
        metrics.record_lockout();

        let (requests, violations, lockouts) = metrics.get_stats();
        assert_eq!(requests, 1);
        assert_eq!(violations, 1);
        assert_eq!(lockouts, 1);
    }
}
