//! `bd repo` -- repository management (stubs).

use anyhow::Result;

use crate::cli::{RepoArgs, RepoCommands};
use crate::context::RuntimeContext;

/// Execute the `bd repo` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &RepoArgs) -> Result<()> {
    match &args.command {
        RepoCommands::List => println!("bd repo list: not yet implemented"),
        RepoCommands::Info(_) => println!("bd repo info: not yet implemented"),
    }
    Ok(())
}
