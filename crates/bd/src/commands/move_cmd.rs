//! `bd move` -- move an issue to a new prefix (stub).

use anyhow::Result;

use crate::cli::MoveCmdArgs;
use crate::context::RuntimeContext;

/// Execute the `bd move` command.
pub fn run(_ctx: &RuntimeContext, _args: &MoveCmdArgs) -> Result<()> {
    println!("bd move: not yet implemented");
    Ok(())
}
