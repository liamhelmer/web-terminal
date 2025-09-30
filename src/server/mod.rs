// HTTP/WebSocket server module
// Per spec-kit/003-backend-spec.md section 2.2

pub mod http;
pub mod middleware;
pub mod websocket;

#[cfg(feature = "tls")]
pub mod tls;

pub use http::Server;
pub use middleware::{JwtAuthMiddleware, RateLimitMiddleware};
pub use websocket::WebSocketSession;

#[cfg(feature = "tls")]
pub use tls::{load_tls_config, validate_tls_files, TlsConfig as TlsServerConfig};
