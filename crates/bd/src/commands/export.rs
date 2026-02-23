//! `bd export` -- export issues to external formats (stub).

use anyhow::Result;

use crate::cli::{ExportArgs, ExportCommands};
use crate::context::RuntimeContext;

/// Execute the `bd export` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &ExportArgs) -> Result<()> {
    match &args.command {
        Some(ExportCommands::Obsidian(_)) => {
            println!("bd export obsidian: not yet implemented");
        }
        None => {
            println!("bd export: not yet implemented");
        }
    }
    Ok(())
}
