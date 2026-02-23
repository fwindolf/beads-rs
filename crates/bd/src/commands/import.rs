//! `bd import` -- import issues from external sources (stub).

use anyhow::Result;

use crate::cli::ImportArgs;
use crate::context::RuntimeContext;

/// Execute the `bd import` command (stub).
pub fn run(_ctx: &RuntimeContext, _args: &ImportArgs) -> Result<()> {
    println!("bd import: not yet implemented");
    Ok(())
}
