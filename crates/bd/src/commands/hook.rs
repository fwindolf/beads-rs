//! `bd hook` -- hook management operations (stubs).

use anyhow::Result;

use crate::cli::{HookArgs, HookCommands};
use crate::context::RuntimeContext;

/// Execute the `bd hook` command (stub).
pub fn run(_ctx: &RuntimeContext, args: &HookArgs) -> Result<()> {
    match &args.command {
        HookCommands::Install(a) => {
            println!("bd hook install {}: not yet implemented", a.name)
        }
        HookCommands::Uninstall(a) => {
            println!("bd hook uninstall {}: not yet implemented", a.name)
        }
        HookCommands::List => println!("bd hook list: not yet implemented"),
        HookCommands::Test(a) => {
            println!("bd hook test {}: not yet implemented", a.name)
        }
    }
    Ok(())
}
