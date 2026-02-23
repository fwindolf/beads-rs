//! `bd reset` -- reset the database (stub).

use anyhow::Result;

use crate::context::RuntimeContext;

/// Execute the `bd reset` command (stub).
pub fn run(_ctx: &RuntimeContext) -> Result<()> {
    eprintln!("WARNING: 'bd reset' would delete all data in the beads database.");
    eprintln!("This command is not yet implemented.");
    eprintln!();
    eprintln!("To manually reset, remove the .beads/ directory and run 'bd init'.");
    Ok(())
}
