//! `bd edit` -- interactive editing (stub).

use anyhow::Result;

use crate::cli::EditArgs;
use crate::context::RuntimeContext;

/// Execute the `bd edit` command.
pub fn run(_ctx: &RuntimeContext, _args: &EditArgs) -> Result<()> {
    println!("bd edit: interactive editing not yet implemented. Use 'bd update' instead.");
    Ok(())
}
