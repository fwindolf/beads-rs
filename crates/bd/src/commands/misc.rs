//! Miscellaneous stub commands.
//!
//! Groups simple stubs that don't warrant their own file.

use anyhow::Result;

use crate::context::RuntimeContext;

/// Execute the `bd sql` command (stub).
pub fn run_sql(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd sql: interactive SQL shell not yet implemented. Use 'bd query' instead.");
    Ok(())
}

/// Execute the `bd bootstrap` command.
///
/// With SQLite storage, bootstrap is automatic: `bd init` creates the database
/// and schema. This command explains how initialization works.
pub fn run_bootstrap(_ctx: &RuntimeContext) -> Result<()> {
    println!("bd bootstrap");
    println!();
    println!("With SQLite storage, bootstrap is automatic:");
    println!("  1. Run 'bd init' to create .beads/ with a fresh database");
    println!("  2. Issues are stored in .beads/beads.db");
    println!("  3. JSONL export (.beads/issues.jsonl) syncs via git");
    println!();
    println!("On a fresh clone with an existing .beads/issues.jsonl:");
    println!("  - Run 'bd init' to create the database");
    println!("  - Then 'bd sync' to import issues from JSONL");
    println!();
    println!("No manual bootstrap step is needed.");
    Ok(())
}
