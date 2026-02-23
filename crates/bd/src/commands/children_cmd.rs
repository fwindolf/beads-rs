//! `bd children` -- top-level alias for `bd dep children`.

use anyhow::{bail, Context, Result};

use crate::cli::ChildrenArgs;
use crate::context::RuntimeContext;

/// Execute the `bd children` command (delegates to dep children).
pub fn run(ctx: &RuntimeContext, args: &ChildrenArgs) -> Result<()> {
    let beads_dir = ctx
        .resolve_db_path()
        .context("no beads database found. Run 'bd init' to create one.")?;
    let db_path = beads_dir.join("beads.db");

    if !db_path.exists() {
        bail!(
            "no beads database found at {}\nHint: run 'bd init' to create a database",
            db_path.display()
        );
    }

    super::dep::run_children(ctx, &db_path, &args.id)
}
