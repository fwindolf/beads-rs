//! `bd orphans` -- show issues with no dependencies.

use anyhow::{bail, Context, Result};

use crate::cli::OrphansArgs;
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd orphans` command.
pub fn run(ctx: &RuntimeContext, _args: &OrphansArgs) -> Result<()> {
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

    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.status, i.priority, i.assignee \
         FROM issues i \
         WHERE i.id NOT IN (SELECT issue_id FROM dependencies) \
           AND i.id NOT IN (SELECT depends_on_id FROM dependencies) \
           AND COALESCE(i.is_template, 0) = 0 \
           AND i.issue_type != 'gate' \
           AND i.status != 'closed' \
         ORDER BY i.priority ASC, i.created_at DESC",
    )?;

    let orphans: Vec<(String, String, String, i32, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_orphans: Vec<serde_json::Value> = orphans
            .iter()
            .map(|(id, title, status, priority, assignee)| {
                serde_json::json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                    "assignee": assignee,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "count": orphans.len(),
            "issues": json_orphans,
        }));
    } else {
        if orphans.is_empty() {
            println!("No orphan issues found");
            return Ok(());
        }

        println!("Orphan issues (no dependencies): {}\n", orphans.len());
        let headers = &["ID", "PRI", "STATUS", "TITLE", "ASSIGNEE"];
        let rows: Vec<Vec<String>> = orphans
            .iter()
            .map(|(id, title, status, priority, assignee)| {
                vec![
                    id.clone(),
                    format!("P{}", priority),
                    status.clone(),
                    title.clone(),
                    assignee.clone(),
                ]
            })
            .collect();
        output_table(headers, &rows);
    }

    Ok(())
}
