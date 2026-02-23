//! `bd compact` -- compact the database (stub).

use anyhow::Result;

use crate::context::RuntimeContext;

/// Execute the `bd compact` command (stub).
pub fn run(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd compact: not yet implemented");
    Ok(())
}
