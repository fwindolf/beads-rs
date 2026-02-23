//! `bd formula` -- formula operations (Phase 4 stub).

use anyhow::Result;

use crate::cli::{FormulaArgs, FormulaCommands};
use crate::context::RuntimeContext;

/// Execute the `bd formula` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &FormulaArgs) -> Result<()> {
    let sub = match &args.command {
        FormulaCommands::List => "list",
        FormulaCommands::Show(_) => "show",
        FormulaCommands::Create(_) => "create",
        FormulaCommands::Delete(_) => "delete",
    };
    println!("bd formula {}: formula support not yet implemented", sub);
    Ok(())
}
