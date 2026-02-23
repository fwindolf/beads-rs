//! `bd query` -- execute raw SQL against the beads database.
//!
//! Runs an arbitrary SQL query in read-only mode and prints results
//! as an aligned table (or JSON with `--json`).

use anyhow::{bail, Context, Result};
use rusqlite::types::Value;

use crate::cli::QueryArgs;
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd query` command.
pub fn run(ctx: &RuntimeContext, args: &QueryArgs) -> Result<()> {
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

    let mut stmt = conn
        .prepare(&args.sql)
        .with_context(|| format!("failed to prepare SQL: {}", args.sql))?;

    // Get column names
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let col_count = col_names.len();

    // Execute and collect rows
    let rows_result = stmt.query_map([], |row| {
        let mut values = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let val: Value = row.get(i)?;
            values.push(val);
        }
        Ok(values)
    })?;

    let mut all_rows: Vec<Vec<Value>> = Vec::new();
    for row in rows_result {
        all_rows.push(row?);
    }

    if ctx.json {
        // Build JSON array of objects
        let json_rows: Vec<serde_json::Value> = all_rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                for (i, val) in row.iter().enumerate() {
                    let json_val = match val {
                        Value::Null => serde_json::Value::Null,
                        Value::Integer(n) => serde_json::json!(n),
                        Value::Real(f) => serde_json::json!(f),
                        Value::Text(s) => serde_json::Value::String(s.clone()),
                        Value::Blob(b) => {
                            serde_json::Value::String(format!("<blob {} bytes>", b.len()))
                        }
                    };
                    obj.insert(col_names[i].clone(), json_val);
                }
                serde_json::Value::Object(obj)
            })
            .collect();

        output_json(&serde_json::json!({
            "columns": col_names,
            "rows": json_rows,
            "count": all_rows.len(),
        }));
    } else {
        if all_rows.is_empty() {
            println!("(0 rows)");
            return Ok(());
        }

        // Convert to string rows for table output
        let str_rows: Vec<Vec<String>> = all_rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|val| match val {
                        Value::Null => "NULL".to_string(),
                        Value::Integer(n) => n.to_string(),
                        Value::Real(f) => f.to_string(),
                        Value::Text(s) => s.clone(),
                        Value::Blob(b) => format!("<blob {} bytes>", b.len()),
                    })
                    .collect()
            })
            .collect();

        let headers: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();
        output_table(&headers, &str_rows);
        println!("({} row{})", all_rows.len(), if all_rows.len() == 1 { "" } else { "s" });
    }

    Ok(())
}
