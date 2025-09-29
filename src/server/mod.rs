// HTTP/WebSocket server module
// Per spec-kit/003-backend-spec.md section 2.2

pub mod http;
pub mod middleware;
pub mod websocket;

pub use http::Server;
pub use middleware::{AuthMiddleware, RateLimitMiddleware};
pub use websocket::WebSocketSession;
