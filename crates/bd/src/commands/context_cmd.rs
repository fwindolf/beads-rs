//! `bd context` -- working context management (stubs).

use anyhow::Result;

use crate::cli::{ContextCmdArgs, ContextCmdCommands};
use crate::context::RuntimeContext;

/// Execute the `bd context` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &ContextCmdArgs) -> Result<()> {
    match &args.command {
        ContextCmdCommands::Set(a) => {
            println!("bd context set {}: not yet implemented", a.value)
        }
        ContextCmdCommands::Get => println!("bd context get: not yet implemented"),
        ContextCmdCommands::Clear => println!("bd context clear: not yet implemented"),
    }
    Ok(())
}
