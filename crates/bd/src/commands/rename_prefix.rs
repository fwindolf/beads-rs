//! `bd rename-prefix` -- rename issue ID prefix (stub).

use anyhow::Result;

use crate::cli::RenamePrefixArgs;
use crate::context::RuntimeContext;

/// Execute the `bd rename-prefix` command.
pub fn run(_ctx: &RuntimeContext, _args: &RenamePrefixArgs) -> Result<()> {
    println!("bd rename-prefix: not yet implemented");
    Ok(())
}
