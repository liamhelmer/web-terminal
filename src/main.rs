// Web-Terminal: Browser-based terminal emulator
// Per spec-kit/005-cli-spec.md

mod cli;
mod config;
mod server;
mod session;
mod execution;
mod filesystem;
mod security;
mod protocol;
mod monitoring;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    // Execute command
    if let Err(e) = cli::execute(cli).await {
        eprintln!("❌ Error: {}", e);

        // Exit with appropriate error code
        // Per spec-kit/005-cli-spec.md exit codes
        let exit_code = match e.to_string().as_str() {
            s if s.contains("config") => 2,
            s if s.contains("auth") => 3,
            s if s.contains("permission") => 4,
            s if s.contains("not found") => 5,
            s if s.contains("already running") => 10,
            s if s.contains("not running") => 11,
            s if s.contains("cannot connect") => 12,
            _ => 1,
        };

        std::process::exit(exit_code);
    }
}

// Build metadata injected by build.rs
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_COMMIT: &str = env!("GIT_COMMIT");
pub const GIT_BRANCH: &str = env!("GIT_BRANCH");
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const BUILD_PROFILE: &str = env!("BUILD_PROFILE");
