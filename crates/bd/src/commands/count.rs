//! `bd count` -- count issues by status.

use anyhow::{bail, Context, Result};

use crate::cli::CountArgs;
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd count` command.
pub fn run(ctx: &RuntimeContext, args: &CountArgs) -> Result<()> {
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

    // If --by-status, group by status
    if args.by_status {
        return run_by_status(ctx, &conn);
    }

    // Otherwise, count with optional filters
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    // Exclude templates
    conditions.push("COALESCE(is_template, 0) = 0".to_string());
    conditions.push("issue_type != 'gate'".to_string());

    if let Some(ref status) = args.status {
        conditions.push(format!("status = ?{}", params.len() + 1));
        params.push(Box::new(status.clone()));
    }

    if let Some(ref issue_type) = args.issue_type {
        conditions.push(format!("issue_type = ?{}", params.len() + 1));
        params.push(Box::new(issue_type.clone()));
    }

    if let Some(ref assignee) = args.assignee {
        conditions.push(format!("assignee = ?{}", params.len() + 1));
        params.push(Box::new(assignee.clone()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!("SELECT COUNT(*) FROM issues {}", where_clause);
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let count: i64 = conn.query_row(&sql, param_refs.as_slice(), |row| row.get(0))?;

    if ctx.json {
        output_json(&serde_json::json!({ "count": count }));
    } else {
        println!("{}", count);
    }

    Ok(())
}

/// Count issues grouped by status.
fn run_by_status(ctx: &RuntimeContext, conn: &rusqlite::Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT status, COUNT(*) as cnt FROM issues \
         WHERE COALESCE(is_template, 0) = 0 AND issue_type != 'gate' \
         GROUP BY status ORDER BY cnt DESC",
    )?;

    let counts: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let mut map = serde_json::Map::new();
        for (status, count) in &counts {
            map.insert(status.clone(), serde_json::json!(count));
        }
        output_json(&serde_json::Value::Object(map));
    } else {
        let headers = &["STATUS", "COUNT"];
        let rows: Vec<Vec<String>> = counts
            .iter()
            .map(|(status, count)| vec![status.clone(), count.to_string()])
            .collect();
        output_table(headers, &rows);
    }

    Ok(())
}
