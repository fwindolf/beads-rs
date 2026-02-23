//! `bd relate` / `bd unrelate` -- add/remove "related" dependencies.

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::cli::{RelateArgs, UnrelateArgs};
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd relate` command -- add a "related" dependency.
pub fn run_relate(ctx: &RuntimeContext, args: &RelateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot add dependencies in read-only mode");
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
    for id in [&args.from, &args.to] {
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

    conn.execute(
        "INSERT OR IGNORE INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) \
         VALUES (?1, ?2, 'related', ?3, ?4)",
        rusqlite::params![&args.from, &args.to, &now_str, &ctx.actor],
    )
    .with_context(|| format!("failed to add related dependency {} -> {}", args.from, args.to))?;

    // Record event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            &args.from,
            "dependency_added",
            &ctx.actor,
            format!("related:{}", args.to),
            &now_str,
        ],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "from": args.from,
            "to": args.to,
            "type": "related",
        }));
    } else if !ctx.quiet {
        println!("Added related dependency: {} <-> {}", args.from, args.to);
    }

    Ok(())
}

/// Execute the `bd unrelate` command -- remove a "related" dependency.
pub fn run_unrelate(ctx: &RuntimeContext, args: &UnrelateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot remove dependencies in read-only mode");
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

    let now_str = Utc::now().to_rfc3339();

    // Try both directions for "related" since it's bidirectional
    let changes1 = conn.execute(
        "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2 AND type = 'related'",
        rusqlite::params![&args.from, &args.to],
    )?;
    let changes2 = conn.execute(
        "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2 AND type = 'related'",
        rusqlite::params![&args.to, &args.from],
    )?;

    let total_changes = changes1 + changes2;

    if total_changes > 0 {
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, old_value, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &args.from,
                "dependency_removed",
                &ctx.actor,
                format!("related:{}", args.to),
                &now_str,
            ],
        )?;
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "from": args.from,
            "to": args.to,
            "removed": total_changes > 0,
        }));
    } else if total_changes > 0 {
        if !ctx.quiet {
            println!("Removed related dependency: {} <-> {}", args.from, args.to);
        }
    } else {
        eprintln!("No related dependency found: {} <-> {}", args.from, args.to);
    }

    Ok(())
}
