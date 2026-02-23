//! `bd github` -- GitHub integration (stub).

use anyhow::Result;

use crate::cli::{GithubArgs, GithubCommands};
use crate::context::RuntimeContext;

/// Execute the `bd github` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &GithubArgs) -> Result<()> {
    match &args.command {
        GithubCommands::Config => println!("bd github config: not yet implemented"),
        GithubCommands::Sync => println!("bd github sync: not yet implemented"),
        GithubCommands::Import => println!("bd github import: not yet implemented"),
    }
    Ok(())
}
