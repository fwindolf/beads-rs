//! `bd vc` -- version-control operations for beads data (stubs).

use anyhow::Result;

use crate::cli::{VcArgs, VcCommands};
use crate::context::RuntimeContext;

/// Execute the `bd vc` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &VcArgs) -> Result<()> {
    match &args.command {
        VcCommands::Commit(_) => println!("bd vc commit: not yet implemented"),
        VcCommands::Push => println!("bd vc push: not yet implemented"),
        VcCommands::Pull => println!("bd vc pull: not yet implemented"),
        VcCommands::Status => println!("bd vc status: not yet implemented"),
    }
    Ok(())
}
