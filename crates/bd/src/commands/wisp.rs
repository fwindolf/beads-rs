//! `bd wisp` -- ephemeral formula execution (Phase 4 stub).

use anyhow::Result;

use crate::cli::WispArgs;
use crate::context::RuntimeContext;

/// Execute the `bd wisp` command (stub).
pub fn run(_ctx: &RuntimeContext, _args: &WispArgs) -> Result<()> {
    println!("bd wisp: ephemeral formula execution not yet implemented");
    Ok(())
}
