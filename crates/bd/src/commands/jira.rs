//! `bd jira` -- Jira integration (stub).

use anyhow::Result;

use crate::cli::{JiraArgs, JiraCommands};
use crate::context::RuntimeContext;

/// Execute the `bd jira` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &JiraArgs) -> Result<()> {
    match &args.command {
        JiraCommands::Config => println!("bd jira config: not yet implemented"),
        JiraCommands::Sync => println!("bd jira sync: not yet implemented"),
        JiraCommands::Import => println!("bd jira import: not yet implemented"),
    }
    Ok(())
}
