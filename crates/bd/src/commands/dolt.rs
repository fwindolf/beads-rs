//! `bd dolt` -- Dolt-compatible database operations (stubs).
//!
//! beads-rs uses SQLite, not Dolt. These stubs exist for CLI compatibility
//! with the original beads Go implementation.

use anyhow::Result;

use crate::cli::{DoltArgs, DoltCommands};
use crate::context::RuntimeContext;

/// Execute the `bd dolt` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &DoltArgs) -> Result<()> {
    match &args.command {
        DoltCommands::Sql(_) => {
            println!("bd dolt sql: not yet implemented (beads-rs uses SQLite, not Dolt)");
        }
        DoltCommands::Status => {
            println!("bd dolt status: not yet implemented (beads-rs uses SQLite, not Dolt)");
        }
        DoltCommands::Log => {
            println!("bd dolt log: not yet implemented (beads-rs uses SQLite, not Dolt)");
        }
        DoltCommands::Commit(_) => {
            println!("bd dolt commit: not yet implemented (beads-rs uses SQLite, not Dolt)");
        }
        DoltCommands::Push => {
            println!("bd dolt push: not yet implemented (beads-rs uses SQLite, not Dolt)");
        }
        DoltCommands::Pull => {
            println!("bd dolt pull: not yet implemented (beads-rs uses SQLite, not Dolt)");
        }
    }
    Ok(())
}
