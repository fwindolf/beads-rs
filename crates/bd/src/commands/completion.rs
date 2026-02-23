//! `bd completion` -- generate shell completions.
//!
//! Uses `clap_complete` to generate shell completion scripts for
//! Bash, Zsh, Fish, and PowerShell.

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};

use crate::cli::{Cli, CompletionArgs, CompletionCommands};
use crate::context::RuntimeContext;

/// Execute the `bd completion` command.
pub fn run(_ctx: &RuntimeContext, args: &CompletionArgs) -> Result<()> {
    let shell = match &args.command {
        CompletionCommands::Bash => Shell::Bash,
        CompletionCommands::Zsh => Shell::Zsh,
        CompletionCommands::Fish => Shell::Fish,
        CompletionCommands::Powershell => Shell::PowerShell,
    };

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "bd", &mut std::io::stdout());

    Ok(())
}
