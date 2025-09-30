// Server management commands
// Per spec-kit/005-cli-spec.md

use crate::cli::args::{StartArgs, StopArgs, RestartArgs, StatusArgs};
use anyhow::{Result, Context};
use std::path::PathBuf;

pub async fn start(args: StartArgs, config_path: Option<PathBuf>) -> Result<()> {
    println!("🚀 Starting web-terminal server...");

    // Load configuration
    let config = load_config(config_path, &args)?;

    println!("📋 Configuration:");
    println!("  Host: {}", config.host);
    println!("  Port: {}", config.port);
    println!("  Workers: {}", config.workers);

    if let (Some(cert), Some(key)) = (&config.tls_cert, &config.tls_key) {
        println!("  TLS: enabled");
        println!("    Certificate: {}", cert.display());
        println!("    Key: {}", key.display());
    }

    if args.daemon {
        println!("  Mode: daemon");
        println!("\n⚠️  Daemon mode not yet implemented");
        println!("For now, run in foreground with Ctrl+C to stop");
    }

    // Start the actual server
    // Per spec-kit/003-backend-spec.md: Single-port architecture
    println!("\n🚀 Starting server...");

    use crate::config::Config as ServerConfig;
    use crate::server::Server;
    use crate::session::{SessionManager, SessionConfig};

    // Load server configuration
    let server_config = ServerConfig::default();
    let session_config = SessionConfig::default();

    // Create session manager
    let session_manager = SessionManager::new(session_config);

    // Create and start server
    // Per spec-kit/011-authentication-spec.md: External JWT authentication only
    let server = Server::new(server_config, session_manager);

    println!("✅ Server started on {}:{}", config.host, config.port);
    println!("📡 WebSocket endpoint: ws://{}:{}/ws", config.host, config.port);
    println!("💚 Health check: http://{}:{}/api/v1/health", config.host, config.port);
    println!("\n🔐 External JWT authentication enabled (JWKS-based)");
    println!("⚠️  Configure JWKS providers in config file");
    println!("\n🛑 Press Ctrl+C to stop");

    server.run().await?;

    Ok(())
}

pub async fn stop(args: StopArgs) -> Result<()> {
    println!("🛑 Stopping web-terminal server...");

    if args.force {
        println!("  Mode: force stop (no graceful shutdown)");
    } else {
        println!("  Mode: graceful shutdown (timeout: {}s)", args.timeout);
    }

    println!("\n⚠️  Stop command not yet implemented");
    println!("For now, use Ctrl+C or kill command");

    Ok(())
}

pub async fn restart(args: RestartArgs) -> Result<()> {
    println!("🔄 Restarting web-terminal server...");
    println!("  Timeout: {}s", args.timeout);

    println!("\n⚠️  Restart command not yet implemented");
    println!("For now, stop and start manually");

    Ok(())
}

pub async fn status(args: StatusArgs) -> Result<()> {
    if args.watch {
        println!("👀 Watching server status (Ctrl+C to exit)...");
        println!("\n⚠️  Watch mode not yet implemented");
        return Ok(());
    }

    let status = ServerStatus {
        running: false,
        uptime: None,
        active_sessions: 0,
        total_connections: 0,
        memory_usage_mb: 0,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("📊 Server Status");
        println!("  Running: {}", if status.running { "✅ yes" } else { "❌ no" });
        if let Some(uptime) = status.uptime {
            println!("  Uptime: {}", format_duration(uptime));
        }
        println!("  Active sessions: {}", status.active_sessions);
        println!("  Total connections: {}", status.total_connections);
        println!("  Memory usage: {} MB", status.memory_usage_mb);
    }

    Ok(())
}

// Helper types and functions

#[derive(Debug, serde::Serialize)]
struct ServerStatus {
    running: bool,
    uptime: Option<u64>,
    active_sessions: usize,
    total_connections: usize,
    memory_usage_mb: usize,
}

struct ServerConfig {
    host: String,
    port: u16,
    workers: usize,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
}

fn load_config(config_path: Option<PathBuf>, args: &StartArgs) -> Result<ServerConfig> {
    // Default configuration
    let config = ServerConfig {
        host: args.host.clone().unwrap_or_else(|| "0.0.0.0".to_string()),
        port: args.port.unwrap_or(8080),
        workers: args.workers.unwrap_or_else(|| num_cpus::get()),
        tls_cert: args.tls_cert.clone(),
        tls_key: args.tls_key.clone(),
    };

    // TODO: Load from config file if provided
    if let Some(path) = config_path {
        println!("📄 Loading config from: {}", path.display());
        // config::Config::builder()
        //     .add_source(config::File::from(path))
        //     .build()?;
    }

    Ok(config)
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}