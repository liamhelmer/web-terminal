// Middleware modules
// Per spec-kit/003-backend-spec.md section 1.3

pub mod auth;
pub mod cors;
pub mod rate_limit;
pub mod security_headers;
pub mod websocket_rate_limit;

// Re-export middleware components
pub use auth::{JwtAuthMiddleware, UserContext};
pub use cors::CorsConfig;
pub use rate_limit::{RateLimitConfig, RateLimitMetrics, RateLimitMiddleware};
pub use security_headers::{SecurityHeadersMiddleware, SecurityHeadersConfig};
pub use websocket_rate_limit::{
    RateLimitResult, RateLimitWarning, RateLimitedWebSocket, WebSocketRateLimitConfig,
    WebSocketRateLimiter,
};