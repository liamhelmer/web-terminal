// CLI argument definitions
// Per spec-kit/005-cli-spec.md

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Web-Terminal: Browser-based terminal emulator with WASM execution
#[derive(Parser, Debug)]
#[command(name = "web-terminal")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, global = true, env = "WEB_TERMINAL_CONFIG")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the web-terminal server
    Start(StartArgs),

    /// Stop the running server
    Stop(StopArgs),

    /// Restart the server
    Restart(RestartArgs),

    /// Check server status
    Status(StatusArgs),

    /// Session management commands
    #[command(subcommand)]
    Sessions(SessionCommands),

    /// Configuration commands
    #[command(subcommand)]
    Config(ConfigCommands),

    /// User management commands
    #[command(subcommand)]
    Users(UserCommands),

    /// View server logs
    Logs(LogsArgs),

    /// Display metrics
    Metrics(MetricsArgs),

    /// Health check
    Health(HealthArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),
}

// ============================================================================
// Server Management Commands
// ============================================================================

#[derive(Parser, Debug)]
pub struct StartArgs {
    /// Server port
    #[arg(short, long, env = "WEB_TERMINAL_PORT")]
    pub port: Option<u16>,

    /// Server host
    #[arg(long, env = "WEB_TERMINAL_HOST")]
    pub host: Option<String>,

    /// Number of worker threads (0 = auto-detect)
    #[arg(short, long)]
    pub workers: Option<usize>,

    /// TLS certificate file
    #[arg(long, env = "WEB_TERMINAL_TLS_CERT")]
    pub tls_cert: Option<PathBuf>,

    /// TLS private key file
    #[arg(long, env = "WEB_TERMINAL_TLS_KEY")]
    pub tls_key: Option<PathBuf>,

    /// Run as background daemon
    #[arg(short, long)]
    pub daemon: bool,
}

#[derive(Parser, Debug)]
pub struct StopArgs {
    /// Force stop without graceful shutdown
    #[arg(short, long)]
    pub force: bool,

    /// Graceful shutdown timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
}

#[derive(Parser, Debug)]
pub struct RestartArgs {
    /// Graceful shutdown timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
}

#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Output in JSON format
    #[arg(short, long)]
    pub json: bool,

    /// Continuously watch status
    #[arg(short, long)]
    pub watch: bool,
}

// ============================================================================
// Session Management Commands
// ============================================================================

#[derive(Subcommand, Debug)]
pub enum SessionCommands {
    /// List active sessions
    List(SessionListArgs),

    /// Terminate a session
    Kill(SessionKillArgs),

    /// Clean up expired sessions
    Cleanup(SessionCleanupArgs),
}

#[derive(Parser, Debug)]
pub struct SessionListArgs {
    /// Filter by user ID
    #[arg(short, long)]
    pub user: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Sort by field
    #[arg(short, long, value_enum, default_value = "created")]
    pub sort: SessionSortField,
}

#[derive(Parser, Debug)]
pub struct SessionKillArgs {
    /// Session ID to kill
    pub session_id: String,

    /// Force kill without cleanup
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct SessionCleanupArgs {
    /// Show what would be cleaned without cleaning
    #[arg(short, long)]
    pub dry_run: bool,

    /// Clean all sessions regardless of status
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SessionSortField {
    Id,
    User,
    Created,
    Activity,
}

// ============================================================================
// Configuration Commands
// ============================================================================

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Display configuration
    Show(ConfigShowArgs),

    /// Set configuration value
    Set(ConfigSetArgs),

    /// Validate configuration
    Validate(ConfigValidateArgs),
}

#[derive(Parser, Debug)]
pub struct ConfigShowArgs {
    /// Show specific section only
    #[arg(short, long)]
    pub section: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "toml")]
    pub format: ConfigFormat,
}

#[derive(Parser, Debug)]
pub struct ConfigSetArgs {
    /// Configuration key (e.g., server.port)
    pub key: String,

    /// Configuration value
    pub value: String,

    /// Configuration file to modify
    #[arg(short, long)]
    pub file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ConfigValidateArgs {
    /// Configuration file to validate
    #[arg(short, long)]
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConfigFormat {
    Toml,
    Json,
    Yaml,
}

// ============================================================================
// User Management Commands
// ============================================================================

#[derive(Subcommand, Debug)]
pub enum UserCommands {
    /// List users
    List(UserListArgs),

    /// Create user
    Create(UserCreateArgs),

    /// Delete user
    Delete(UserDeleteArgs),
}

#[derive(Parser, Debug)]
pub struct UserListArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Show only active users
    #[arg(short, long)]
    pub active: bool,
}

#[derive(Parser, Debug)]
pub struct UserCreateArgs {
    /// Username
    pub username: String,

    /// User email address
    #[arg(short, long)]
    pub email: Option<String>,

    /// Grant admin privileges
    #[arg(short, long)]
    pub admin: bool,

    /// Set password (interactive if omitted)
    #[arg(short, long)]
    pub password: Option<String>,
}

#[derive(Parser, Debug)]
pub struct UserDeleteArgs {
    /// Username to delete
    pub username: String,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub force: bool,
}

// ============================================================================
// Diagnostics Commands
// ============================================================================

#[derive(Parser, Debug)]
pub struct LogsArgs {
    /// Follow log output
    #[arg(short, long)]
    pub follow: bool,

    /// Show last N lines
    #[arg(short, long, default_value = "100")]
    pub tail: usize,

    /// Filter by level
    #[arg(short, long, value_enum)]
    pub level: Option<LogLevel>,

    /// Filter by session ID
    #[arg(short, long)]
    pub session: Option<String>,

    /// Output in JSON format
    #[arg(short, long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Parser, Debug)]
pub struct MetricsArgs {
    /// Continuously update metrics
    #[arg(short, long)]
    pub watch: bool,

    /// Update interval in seconds
    #[arg(short, long, default_value = "5")]
    pub interval: u64,

    /// Output format
    #[arg(short, long, value_enum, default_value = "human")]
    pub format: MetricsFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MetricsFormat {
    Human,
    Json,
    Prometheus,
}

#[derive(Parser, Debug)]
pub struct HealthArgs {
    /// Show detailed health information
    #[arg(short, long)]
    pub verbose: bool,

    /// Output in JSON format
    #[arg(short, long)]
    pub json: bool,
}

// ============================================================================
// Shell Completions
// ============================================================================

#[derive(Parser, Debug)]
pub struct CompletionsArgs {
    /// Shell type
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

// ============================================================================
// Common Types
// ============================================================================

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
}