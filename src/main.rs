// Web Terminal - Main Entry Point
// Per spec-kit/003-backend-spec.md

mod config;
mod server;
mod session;
mod execution;
mod filesystem;
mod security;
mod protocol;
mod monitoring;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing/logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    tracing::info!("Web Terminal starting...");

    // TODO: Load configuration
    // TODO: Initialize server
    // TODO: Start HTTP/WebSocket server on single port

    tracing::info!("Web Terminal initialized - ready for implementation");

    Ok(())
}
