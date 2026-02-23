//! `bd delete` -- delete issues from the database.

use anyhow::{bail, Context, Result};

use crate::cli::DeleteArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd delete` command.
pub fn run(ctx: &RuntimeContext, args: &DeleteArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot delete issues in read-only mode");
    }

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

    // Safety: require --force for deletion
    if !args.force {
        bail!(
            "deletion is destructive and cannot be undone.\n\
            Use --force to confirm deletion of {} issue(s): {}",
            args.ids.len(),
            args.ids.join(", ")
        );
    }

    let conn = rusqlite::Connection::open(&db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let mut deleted_ids: Vec<String> = Vec::new();

    for id in &args.ids {
        // Check if issue exists
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            eprintln!("Issue {} not found", id);
            continue;
        }

        // Delete in correct order to respect foreign keys
        // 1. Delete labels
        conn.execute(
            "DELETE FROM labels WHERE issue_id = ?1",
            rusqlite::params![id],
        )?;

        // 2. Delete comments
        conn.execute(
            "DELETE FROM comments WHERE issue_id = ?1",
            rusqlite::params![id],
        )?;

        // 3. Delete events
        conn.execute(
            "DELETE FROM events WHERE issue_id = ?1",
            rusqlite::params![id],
        )?;

        // 4. Delete dependencies (both directions)
        conn.execute(
            "DELETE FROM dependencies WHERE issue_id = ?1 OR depends_on_id = ?1",
            rusqlite::params![id],
        )?;

        // 5. Delete the issue itself
        conn.execute("DELETE FROM issues WHERE id = ?1", rusqlite::params![id])?;

        deleted_ids.push(id.clone());

        if !ctx.json {
            println!("Deleted {}", id);
        }
    }

    if ctx.json {
        output_json(&deleted_ids);
    }

    Ok(())
}
