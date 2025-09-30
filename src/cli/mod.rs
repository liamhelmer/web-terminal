// CLI module for web-terminal
// Per spec-kit/005-cli-spec.md

pub mod args;
pub mod commands;

pub use args::Cli;
pub use commands::execute;
