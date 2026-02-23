//! `bd refile` -- refile an issue (stub).

use anyhow::Result;

use crate::cli::RefileArgs;
use crate::context::RuntimeContext;

/// Execute the `bd refile` command.
pub fn run(_ctx: &RuntimeContext, _args: &RefileArgs) -> Result<()> {
    println!("bd refile: not yet implemented");
    Ok(())
}
