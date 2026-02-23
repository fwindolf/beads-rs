//! `bd restore` -- restore a deleted or archived issue (stub).

use anyhow::Result;

use crate::cli::RestoreArgs;
use crate::context::RuntimeContext;

/// Execute the `bd restore` command (stub).
pub fn run(_ctx: &RuntimeContext, _args: &RestoreArgs) -> Result<()> {
    println!("bd restore: not yet implemented");
    Ok(())
}
