//! `bd linear` -- Linear integration (stub).

use anyhow::Result;

use crate::cli::{LinearArgs, LinearCommands};
use crate::context::RuntimeContext;

/// Execute the `bd linear` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &LinearArgs) -> Result<()> {
    match &args.command {
        LinearCommands::Config => println!("bd linear config: not yet implemented"),
        LinearCommands::Sync => println!("bd linear sync: not yet implemented"),
        LinearCommands::Import => println!("bd linear import: not yet implemented"),
    }
    Ok(())
}
