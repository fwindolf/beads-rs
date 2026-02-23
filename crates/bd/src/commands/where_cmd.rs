//! `bd where` -- show where an issue lives (stub).

use anyhow::Result;

use crate::cli::WhereCmdArgs;
use crate::context::RuntimeContext;

/// Execute the `bd where` command.
pub fn run(_ctx: &RuntimeContext, _args: &WhereCmdArgs) -> Result<()> {
    println!("bd where: not yet implemented");
    Ok(())
}
