//! `bd thanks` -- thank a contributor (stub).

use anyhow::Result;

use crate::cli::ThanksArgs;
use crate::context::RuntimeContext;

/// Execute the `bd thanks` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &ThanksArgs) -> Result<()> {
    println!("bd thanks {}: not yet implemented", args.id);
    Ok(())
}
