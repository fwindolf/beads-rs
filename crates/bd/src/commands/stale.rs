//! `bd stale` -- show issues not updated in N days.

use anyhow::{bail, Context, Result};

use crate::cli::StaleArgs;
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd stale` command.
pub fn run(ctx: &RuntimeContext, args: &StaleArgs) -> Result<()> {
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

    let days = args.days;
    let threshold = format!("-{} days", days);

    let mut stmt = conn.prepare(
        "SELECT id, title, status, priority, assignee, updated_at \
         FROM issues \
         WHERE status NOT IN ('closed') \
           AND COALESCE(is_template, 0) = 0 \
           AND issue_type != 'gate' \
           AND updated_at < datetime('now', ?1) \
         ORDER BY updated_at ASC",
    )?;

    let issues: Vec<(String, String, String, i32, String, String)> = stmt
        .query_map(rusqlite::params![&threshold], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_issues: Vec<serde_json::Value> = issues
            .iter()
            .map(|(id, title, status, priority, assignee, updated_at)| {
                serde_json::json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                    "assignee": assignee,
                    "updated_at": updated_at,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "days": days,
            "count": issues.len(),
            "issues": json_issues,
        }));
    } else {
        if issues.is_empty() {
            println!("No stale issues (updated within last {} days)", days);
            return Ok(());
        }

        println!(
            "Stale issues (not updated in {} days): {}\n",
            days,
            issues.len()
        );
        let headers = &["ID", "PRI", "STATUS", "TITLE", "ASSIGNEE", "UPDATED"];
        let rows: Vec<Vec<String>> = issues
            .iter()
            .map(|(id, title, status, priority, assignee, updated_at)| {
                // Truncate updated_at to date only
                let date = if updated_at.len() >= 10 {
                    &updated_at[..10]
                } else {
                    updated_at
                };
                vec![
                    id.clone(),
                    format!("P{}", priority),
                    status.clone(),
                    title.clone(),
                    assignee.clone(),
                    date.to_string(),
                ]
            })
            .collect();
        output_table(headers, &rows);
    }

    Ok(())
}
