//! `bd migrate` -- run database migrations (stub).

use anyhow::Result;

use crate::context::RuntimeContext;

/// Execute the `bd migrate` command (stub).
pub fn run(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd migrate: not yet implemented");
    Ok(())
}
