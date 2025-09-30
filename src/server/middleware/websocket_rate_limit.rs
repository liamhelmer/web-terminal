// WebSocket-specific rate limiting
// Per spec-kit/002-architecture.md Layer 1 Network Security: Rate Limiting

use actix::{Actor, ActorContext, Handler, Message, StreamHandler};
use actix_web_actors::ws;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, warn};

/// WebSocket rate limit configuration
#[derive(Debug, Clone)]
pub struct WebSocketRateLimitConfig {
    /// Maximum messages per second
    pub max_messages_per_second: u32,
    /// Warning threshold (percentage of limit)
    pub warning_threshold_percent: u8,
    /// Grace period before disconnection (seconds)
    pub grace_period_seconds: u64,
}

impl Default for WebSocketRateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_second: 100,
            warning_threshold_percent: 80,
            grace_period_seconds: 5,
        }
    }
}

/// WebSocket rate limiter
pub struct WebSocketRateLimiter {
    config: WebSocketRateLimitConfig,
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    violations: u32,
    last_violation: Option<Instant>,
    warning_sent: bool,
}

impl WebSocketRateLimiter {
    pub fn new(config: WebSocketRateLimitConfig) -> Self {
        let quota =
            Quota::per_second(std::num::NonZeroU32::new(config.max_messages_per_second).unwrap());

        Self {
            config,
            limiter: Arc::new(GovernorRateLimiter::direct(quota)),
            violations: 0,
            last_violation: None,
            warning_sent: false,
        }
    }

    /// Check if message is allowed
    pub fn check_message(&mut self) -> RateLimitResult {
        match self.limiter.check() {
            Ok(_) => {
                // Reset violations if grace period has passed
                if let Some(last_violation) = self.last_violation {
                    if last_violation.elapsed()
                        > Duration::from_secs(self.config.grace_period_seconds)
                    {
                        self.violations = 0;
                        self.warning_sent = false;
                    }
                }

                debug!("WebSocket message rate limit check passed");
                RateLimitResult::Allowed
            }
            Err(_) => {
                self.violations += 1;
                self.last_violation = Some(Instant::now());

                warn!(
                    "WebSocket rate limit exceeded (violation {})",
                    self.violations
                );

                // Calculate warning threshold
                let warning_threshold = (self.config.max_messages_per_second as f32
                    * (self.config.warning_threshold_percent as f32 / 100.0))
                    as u32;

                // Send warning if threshold reached but not yet sent
                if self.violations >= warning_threshold && !self.warning_sent {
                    self.warning_sent = true;
                    return RateLimitResult::Warning {
                        violations: self.violations,
                        max_violations: warning_threshold * 2, // Disconnect after 2x threshold
                    };
                }

                // Disconnect if too many violations
                if self.violations >= warning_threshold * 2 {
                    return RateLimitResult::Disconnect;
                }

                RateLimitResult::Throttled
            }
        }
    }

    /// Reset rate limiter (e.g., after successful processing)
    pub fn reset(&mut self) {
        self.violations = 0;
        self.last_violation = None;
        self.warning_sent = false;
    }
}

/// Rate limit check result
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitResult {
    /// Message is allowed
    Allowed,
    /// Message is throttled (rate limit exceeded temporarily)
    Throttled,
    /// Warning sent to client
    Warning {
        violations: u32,
        max_violations: u32,
    },
    /// Client should be disconnected
    Disconnect,
}

/// Warning message to client
#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct RateLimitWarning {
    pub violations: u32,
    pub max_violations: u32,
}

/// Example WebSocket actor with rate limiting
pub struct RateLimitedWebSocket {
    rate_limiter: WebSocketRateLimiter,
}

impl RateLimitedWebSocket {
    pub fn new(config: WebSocketRateLimitConfig) -> Self {
        Self {
            rate_limiter: WebSocketRateLimiter::new(config),
        }
    }

    fn handle_rate_limit_result(
        &mut self,
        result: RateLimitResult,
        ctx: &mut ws::WebsocketContext<Self>,
    ) {
        match result {
            RateLimitResult::Allowed => {
                // Message processing allowed
            }
            RateLimitResult::Throttled => {
                // Drop message silently (client is sending too fast)
                debug!("WebSocket message throttled");
            }
            RateLimitResult::Warning {
                violations,
                max_violations,
            } => {
                // Send warning to client
                warn!(
                    "Sending rate limit warning to client ({}/{})",
                    violations, max_violations
                );
                ctx.text(format!(
                    r#"{{"type":"rate_limit_warning","violations":{},"max_violations":{}}}"#,
                    violations, max_violations
                ));
            }
            RateLimitResult::Disconnect => {
                // Disconnect client
                warn!("Disconnecting client due to rate limit violations");
                ctx.text(r#"{"type":"rate_limit_exceeded","reason":"Too many messages"}"#);
                ctx.stop();
            }
        }
    }
}

impl Actor for RateLimitedWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("WebSocket connection started with rate limiting");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("WebSocket connection stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RateLimitedWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                // Check rate limit
                let result = self.rate_limiter.check_message();
                self.handle_rate_limit_result(result.clone(), ctx);

                if result == RateLimitResult::Allowed {
                    // Process message (echo in this example)
                    ctx.text(text);
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                // Check rate limit
                let result = self.rate_limiter.check_message();
                self.handle_rate_limit_result(result.clone(), ctx);

                if result == RateLimitResult::Allowed {
                    // Process message (echo in this example)
                    ctx.binary(bin);
                }
            }
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                // Pong received
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(e) => {
                warn!("WebSocket error: {:?}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl Handler<RateLimitWarning> for RateLimitedWebSocket {
    type Result = ();

    fn handle(&mut self, msg: RateLimitWarning, ctx: &mut Self::Context) {
        ctx.text(format!(
            r#"{{"type":"rate_limit_warning","violations":{},"max_violations":{}}}"#,
            msg.violations, msg.max_violations
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_rate_limit_config_defaults() {
        let config = WebSocketRateLimitConfig::default();
        assert_eq!(config.max_messages_per_second, 100);
        assert_eq!(config.warning_threshold_percent, 80);
        assert_eq!(config.grace_period_seconds, 5);
    }

    #[test]
    fn test_rate_limiter_allows_messages_within_limit() {
        let config = WebSocketRateLimitConfig {
            max_messages_per_second: 10,
            warning_threshold_percent: 80,
            grace_period_seconds: 5,
        };
        let mut limiter = WebSocketRateLimiter::new(config);

        // First few messages should be allowed
        for _ in 0..5 {
            assert_eq!(limiter.check_message(), RateLimitResult::Allowed);
        }
    }

    #[test]
    fn test_rate_limiter_throttles_excess_messages() {
        let config = WebSocketRateLimitConfig {
            max_messages_per_second: 5,
            warning_threshold_percent: 80,
            grace_period_seconds: 5,
        };
        let mut limiter = WebSocketRateLimiter::new(config);

        // Exhaust rate limit
        for _ in 0..10 {
            let _ = limiter.check_message();
        }

        // Additional messages should be throttled
        let result = limiter.check_message();
        assert!(matches!(
            result,
            RateLimitResult::Throttled | RateLimitResult::Warning { .. }
        ));
    }
}
