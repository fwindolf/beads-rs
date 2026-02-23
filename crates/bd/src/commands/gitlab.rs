//! `bd gitlab` -- GitLab integration (stub).

use anyhow::Result;

use crate::cli::{GitlabArgs, GitlabCommands};
use crate::context::RuntimeContext;

/// Execute the `bd gitlab` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &GitlabArgs) -> Result<()> {
    match &args.command {
        GitlabCommands::Config => println!("bd gitlab config: not yet implemented"),
        GitlabCommands::Sync => println!("bd gitlab sync: not yet implemented"),
        GitlabCommands::Import => println!("bd gitlab import: not yet implemented"),
    }
    Ok(())
}
