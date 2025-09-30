// Shell completions command
// Per spec-kit/005-cli-spec.md

use crate::cli::args::{Cli, CompletionsArgs, Shell};
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, shells};
use std::io;

pub fn execute(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = "web-terminal";

    match args.shell {
        Shell::Bash => {
            generate(shells::Bash, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Zsh => {
            generate(shells::Zsh, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Fish => {
            generate(shells::Fish, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Powershell => {
            generate(shells::PowerShell, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Elvish => {
            generate(shells::Elvish, &mut cmd, bin_name, &mut io::stdout());
        }
    }

    Ok(())
}
