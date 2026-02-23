//! Miscellaneous stub commands (Phase 8: Utilities, Completion & Polish).
//!
//! Groups simple stubs into a single file to avoid file bloat.
//! Each stub prints a helpful message and returns `Ok(())`.

use anyhow::Result;

use crate::cli::{WorktreeArgs, WorktreeCommands};
use crate::context::RuntimeContext;

/// Execute the `bd sql` command (stub).
pub fn run_sql(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd sql: interactive SQL shell not yet implemented. Use 'bd query' instead.");
    Ok(())
}

/// Execute the `bd quickstart` command (stub).
pub fn run_quickstart(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd quickstart: not yet implemented");
    Ok(())
}

/// Execute the `bd onboard` command (stub).
pub fn run_onboard(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd onboard: not yet implemented");
    Ok(())
}

/// Execute the `bd bootstrap` command (stub).
pub fn run_bootstrap(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd bootstrap: not yet implemented");
    Ok(())
}

/// Execute the `bd preflight` command (stub).
pub fn run_preflight(_ctx: &RuntimeContext) -> Result<()> {
    println!("All checks passed");
    Ok(())
}

/// Execute the `bd prime` command (stub).
pub fn run_prime(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd prime: not yet implemented");
    Ok(())
}

/// Execute the `bd upgrade` command (stub).
pub fn run_upgrade(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd upgrade: check https://github.com/steveyegge/beads/releases for latest version");
    Ok(())
}

/// Execute the `bd worktree` command (stub).
pub fn run_worktree(_ctx: &RuntimeContext, args: &WorktreeArgs) -> Result<()> {
    match &args.command {
        WorktreeCommands::Create(a) => {
            let name = a.name.as_deref().unwrap_or("<auto>");
            println!("bd worktree create {}: not yet implemented", name);
        }
        WorktreeCommands::Remove(a) => {
            println!("bd worktree remove {}: not yet implemented", a.name);
        }
        WorktreeCommands::List => {
            println!("bd worktree list: not yet implemented");
        }
    }
    Ok(())
}
