//! `bd config` -- manage configuration (set/get/list/unset).

use anyhow::{bail, Context, Result};

use crate::cli::{ConfigArgs, ConfigCommands};
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd config` command.
pub fn run(ctx: &RuntimeContext, args: &ConfigArgs) -> Result<()> {
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
        ConfigCommands::Set(set_args) => {
            if ctx.readonly {
                bail!("cannot set config in read-only mode");
            }

            let conn = rusqlite::Connection::open(&db_path)
                .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            conn.execute(
                "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
                rusqlite::params![&set_args.key, &set_args.value],
            )
            .with_context(|| format!("failed to set config key '{}'", set_args.key))?;

            if ctx.json {
                output_json(&serde_json::json!({
                    "key": set_args.key,
                    "value": set_args.value,
                }));
            } else if !ctx.quiet {
                println!("Set {} = {}", set_args.key, set_args.value);
            }
        }

        ConfigCommands::Get(get_args) => {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            let value: Option<String> = conn
                .query_row(
                    "SELECT value FROM config WHERE key = ?1",
                    rusqlite::params![&get_args.key],
                    |row| row.get(0),
                )
                .ok();

            if ctx.json {
                output_json(&serde_json::json!({
                    "key": get_args.key,
                    "value": value,
                }));
            } else {
                match value {
                    Some(v) => println!("{}", v),
                    None => {
                        eprintln!("Key '{}' not found", get_args.key);
                        std::process::exit(1);
                    }
                }
            }
        }

        ConfigCommands::List => {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
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
                println!("No configuration values set");
            } else {
                for (key, value) in &entries {
                    println!("{} = {}", key, value);
                }
            }
        }

        ConfigCommands::Unset(unset_args) => {
            if ctx.readonly {
                bail!("cannot unset config in read-only mode");
            }

            let conn = rusqlite::Connection::open(&db_path)
                .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            let changes = conn.execute(
                "DELETE FROM config WHERE key = ?1",
                rusqlite::params![&unset_args.key],
            )?;

            if ctx.json {
                output_json(&serde_json::json!({
                    "key": unset_args.key,
                    "deleted": changes > 0,
                }));
            } else if changes > 0 {
                if !ctx.quiet {
                    println!("Unset {}", unset_args.key);
                }
            } else {
                eprintln!("Key '{}' not found", unset_args.key);
            }
        }
    }

    Ok(())
}
