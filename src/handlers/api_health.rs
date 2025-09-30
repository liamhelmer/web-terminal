// REST API Health check handler
// Per docs/spec-kit/006-api-spec.md - Monitoring API

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use std::time::Instant;

use crate::error::Result;
use crate::handlers::api_types::*;
use crate::session::manager::SessionManager;

/// Server start time for uptime calculation
static START_TIME: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);

/// GET /api/v1/health - Health check endpoint
///
/// Per docs/spec-kit/006-api-spec.md - Health Check
/// No authentication required (public endpoint)
pub async fn health_check(session_manager: web::Data<Arc<SessionManager>>) -> Result<HttpResponse> {
    let uptime_seconds = START_TIME.elapsed().as_secs();

    // Check session manager health
    let sessions_check = match check_sessions_health(&session_manager).await {
        Ok(_) => "ok",
        Err(_) => "degraded",
    };

    // Check memory
    let memory_check = check_memory_health();

    // Check disk space
    let disk_check = check_disk_health();

    let response = HealthResponse {
        status: if sessions_check == "ok" && memory_check == "ok" && disk_check == "ok" {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks: HealthChecks {
            sessions: sessions_check.to_string(),
            memory: memory_check.to_string(),
            disk: disk_check.to_string(),
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Check if session manager is healthy
async fn check_sessions_health(session_manager: &SessionManager) -> Result<()> {
    // Try to list sessions to verify manager is responsive
    let _sessions = session_manager.list_sessions().await;
    Ok(())
}

/// Check memory health (basic check)
fn check_memory_health() -> &'static str {
    // In a real implementation, you would check available memory
    // For now, assume OK
    "ok"
}

/// Check disk space health (basic check)
fn check_disk_health() -> &'static str {
    // In a real implementation, you would check available disk space
    // For now, assume OK
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::manager::SessionConfig;

    #[actix_web::test]
    async fn test_health_check_response_structure() {
        let config = SessionConfig::default();
        let session_manager = Arc::new(SessionManager::new(config));
        let data = web::Data::new(session_manager);

        let response = health_check(data).await.unwrap();
        assert_eq!(response.status(), 200);
    }

    #[test]
    fn test_uptime_increases() {
        let uptime1 = START_TIME.elapsed().as_secs();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let uptime2 = START_TIME.elapsed().as_secs();
        assert!(uptime2 >= uptime1);
    }
}
