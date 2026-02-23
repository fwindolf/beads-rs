//! `bd stats` -- show project statistics.

use anyhow::{bail, Context, Result};

use crate::cli::StatsArgs;
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd stats` command.
pub fn run(ctx: &RuntimeContext, _args: &StatsArgs) -> Result<()> {
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

    let base_where = "WHERE COALESCE(is_template, 0) = 0 AND issue_type != 'gate'";

    // Total counts by status
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM issues {}", base_where),
        [],
        |row| row.get(0),
    )?;

    let count_status = |status: &str| -> i64 {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM issues {} AND status = ?1",
                base_where
            ),
            rusqlite::params![status],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };

    let open = count_status("open");
    let closed = count_status("closed");
    let in_progress = count_status("in_progress");
    let blocked = count_status("blocked");
    let deferred = count_status("deferred");

    // Issues by type
    let mut type_stmt = conn.prepare(&format!(
        "SELECT issue_type, COUNT(*) FROM issues {} GROUP BY issue_type ORDER BY COUNT(*) DESC",
        base_where
    ))?;
    let by_type: Vec<(String, i64)> = type_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Issues by priority
    let mut pri_stmt = conn.prepare(&format!(
        "SELECT priority, COUNT(*) FROM issues {} GROUP BY priority ORDER BY priority ASC",
        base_where
    ))?;
    let by_priority: Vec<(i32, i64)> = pri_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Top 10 assignees
    let mut assignee_stmt = conn.prepare(&format!(
        "SELECT CASE WHEN assignee = '' THEN '(unassigned)' ELSE assignee END, COUNT(*) \
         FROM issues {} GROUP BY assignee ORDER BY COUNT(*) DESC LIMIT 10",
        base_where
    ))?;
    let by_assignee: Vec<(String, i64)> = assignee_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let type_map: serde_json::Map<String, serde_json::Value> = by_type
            .iter()
            .map(|(t, c)| (t.clone(), serde_json::json!(c)))
            .collect();
        let priority_map: serde_json::Map<String, serde_json::Value> = by_priority
            .iter()
            .map(|(p, c)| (format!("P{}", p), serde_json::json!(c)))
            .collect();
        let assignee_map: serde_json::Map<String, serde_json::Value> = by_assignee
            .iter()
            .map(|(a, c)| (a.clone(), serde_json::json!(c)))
            .collect();

        output_json(&serde_json::json!({
            "total": total,
            "open": open,
            "closed": closed,
            "in_progress": in_progress,
            "blocked": blocked,
            "deferred": deferred,
            "by_type": type_map,
            "by_priority": priority_map,
            "by_assignee": assignee_map,
        }));
    } else {
        println!("Project Statistics");
        println!("==================");
        println!();
        println!("Total issues: {}", total);
        println!("  Open:        {}", open);
        println!("  In Progress: {}", in_progress);
        println!("  Blocked:     {}", blocked);
        println!("  Deferred:    {}", deferred);
        println!("  Closed:      {}", closed);

        if !by_type.is_empty() {
            println!();
            println!("By Type:");
            let headers = &["TYPE", "COUNT"];
            let rows: Vec<Vec<String>> = by_type
                .iter()
                .map(|(t, c)| vec![t.clone(), c.to_string()])
                .collect();
            output_table(headers, &rows);
        }

        if !by_priority.is_empty() {
            println!();
            println!("By Priority:");
            let headers = &["PRIORITY", "COUNT"];
            let rows: Vec<Vec<String>> = by_priority
                .iter()
                .map(|(p, c)| vec![format!("P{}", p), c.to_string()])
                .collect();
            output_table(headers, &rows);
        }

        if !by_assignee.is_empty() {
            println!();
            println!("By Assignee (top 10):");
            let headers = &["ASSIGNEE", "COUNT"];
            let rows: Vec<Vec<String>> = by_assignee
                .iter()
                .map(|(a, c)| vec![a.clone(), c.to_string()])
                .collect();
            output_table(headers, &rows);
        }
    }

    Ok(())
}
