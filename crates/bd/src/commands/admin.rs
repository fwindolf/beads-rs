//! `bd admin` -- administrative operations (stubs).

use anyhow::Result;

use crate::cli::{AdminArgs, AdminCommands};
use crate::context::RuntimeContext;

/// Execute the `bd admin` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &AdminArgs) -> Result<()> {
    match &args.command {
        AdminCommands::Aliases => println!("bd admin aliases: not yet implemented"),
        AdminCommands::Cleanup => println!("bd admin cleanup: not yet implemented"),
        AdminCommands::Compact => println!("bd admin compact: not yet implemented"),
        AdminCommands::Reset => {
            eprintln!("WARNING: 'bd admin reset' would delete all data in the beads database.");
            eprintln!("This command is not yet implemented.");
        }
    }
    Ok(())
}
