// CLI command implementations
// Per spec-kit/005-cli-spec.md

mod completions;
mod config;
mod health;
mod logs;
mod metrics;
mod server;
mod sessions;
mod users;

use crate::cli::args::{Cli, Commands};
use anyhow::Result;

/// Execute CLI command
pub async fn execute(cli: Cli) -> Result<()> {
    // Set up logging level based on verbose/quiet flags
    setup_logging(&cli);

    // Execute the command
    match cli.command {
        Commands::Start(args) => server::start(args, cli.config).await,
        Commands::Stop(args) => server::stop(args).await,
        Commands::Restart(args) => server::restart(args).await,
        Commands::Status(args) => server::status(args).await,
        Commands::Sessions(cmd) => sessions::execute(cmd).await,
        Commands::Config(cmd) => config::execute(cmd, cli.config).await,
        Commands::Users(cmd) => users::execute(cmd).await,
        Commands::Logs(args) => logs::execute(args).await,
        Commands::Metrics(args) => metrics::execute(args).await,
        Commands::Health(args) => health::execute(args).await,
        Commands::Completions(args) => completions::execute(args),
    }
}

fn setup_logging(cli: &Cli) {
    use tracing_subscriber::{fmt, EnvFilter};

    let level = if cli.verbose {
        "debug"
    } else if cli.quiet {
        "error"
    } else {
        "info"
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    fmt().with_env_filter(filter).with_target(false).init();
}
