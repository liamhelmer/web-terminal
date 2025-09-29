// CLI module for web-terminal
// Per spec-kit/005-cli-spec.md

pub mod commands;
pub mod args;

pub use args::Cli;
pub use commands::execute;