//! `bd kv` -- key-value metadata operations.
//!
//! Provides CRUD operations on the `metadata` table in the beads database.
//! Internal keys (prefixed with `migration:`) are excluded from `list` output.

use anyhow::{bail, Context, Result};

use crate::cli::{KvArgs, KvCommands};
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd kv` command.
pub fn run(ctx: &RuntimeContext, args: &KvArgs) -> Result<()> {
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

    match &args.command {
        KvCommands::Get(get_args) => run_get(ctx, &db_path, &get_args.key),
        KvCommands::Set(set_args) => run_set(ctx, &db_path, &set_args.key, &set_args.value),
        KvCommands::List => run_list(ctx, &db_path),
        KvCommands::Delete(del_args) => run_delete(ctx, &db_path, &del_args.key),
    }
}

fn run_get(ctx: &RuntimeContext, db_path: &std::path::Path, key: &str) -> Result<()> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .ok();

    if ctx.json {
        output_json(&serde_json::json!({
            "key": key,
            "value": value,
        }));
    } else {
        match value {
            Some(v) => println!("{}", v),
            None => {
                eprintln!("Key '{}' not found", key);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn run_set(
    ctx: &RuntimeContext,
    db_path: &std::path::Path,
    key: &str,
    value: &str,
) -> Result<()> {
    if ctx.readonly {
        bail!("cannot set metadata in read-only mode");
    }

    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .with_context(|| format!("failed to set metadata key '{}'", key))?;

    if ctx.json {
        output_json(&serde_json::json!({
            "key": key,
            "value": value,
        }));
    } else if !ctx.quiet {
        println!("Set {} = {}", key, value);
    }

    Ok(())
}

fn run_list(ctx: &RuntimeContext, db_path: &std::path::Path) -> Result<()> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let mut stmt =
        conn.prepare("SELECT key, value FROM metadata WHERE key NOT LIKE 'migration:%' ORDER BY key")?;
    let entries: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let map: serde_json::Map<String, serde_json::Value> = entries
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        output_json(&map);
    } else if entries.is_empty() {
        println!("No metadata entries");
    } else {
        for (key, value) in &entries {
            println!("{} = {}", key, value);
        }
    }

    Ok(())
}

fn run_delete(ctx: &RuntimeContext, db_path: &std::path::Path, key: &str) -> Result<()> {
    if ctx.readonly {
        bail!("cannot delete metadata in read-only mode");
    }

    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let changes = conn.execute(
        "DELETE FROM metadata WHERE key = ?1",
        rusqlite::params![key],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "key": key,
            "deleted": changes > 0,
        }));
    } else if changes > 0 {
        if !ctx.quiet {
            println!("Deleted {}", key);
        }
    } else {
        eprintln!("Key '{}' not found", key);
    }

    Ok(())
}
