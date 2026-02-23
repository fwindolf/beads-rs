//! `bd supersede` -- mark an issue as superseded by another.

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::cli::SupersedeArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd supersede` command.
pub fn run(ctx: &RuntimeContext, args: &SupersedeArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot mark issues as superseded in read-only mode");
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

    let conn = rusqlite::Connection::open(&db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    // Validate both issues exist
    for id in [&args.id, &args.superseded_by] {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if !exists {
            bail!("issue '{}' not found", id);
        }
    }

    let now_str = Utc::now().to_rfc3339();
    let close_reason = format!("superseded by {}", args.superseded_by);

    // Add a "supersedes" dependency (new issue supersedes old)
    conn.execute(
        "INSERT OR IGNORE INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            &args.superseded_by,
            &args.id,
            "supersedes",
            &now_str,
            &ctx.actor,
        ],
    )
    .with_context(|| {
        format!(
            "failed to add supersedes dependency {} -> {}",
            args.superseded_by, args.id
        )
    })?;

    // Close the superseded issue
    conn.execute(
        "UPDATE issues SET status = 'closed', close_reason = ?1, closed_at = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![&close_reason, &now_str, &now_str, &args.id],
    )
    .with_context(|| format!("failed to close issue {} as superseded", args.id))?;

    // Record close event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, comment, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &args.id,
            "closed",
            &ctx.actor,
            "closed",
            &close_reason,
            &now_str,
        ],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "id": args.id,
            "superseded_by": args.superseded_by,
            "status": "closed",
            "close_reason": close_reason,
        }));
    } else if !ctx.quiet {
        println!("Marked {} as superseded by {}", args.id, args.superseded_by);
    }

    Ok(())
}
