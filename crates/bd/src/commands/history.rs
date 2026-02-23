//! `bd history` -- show event history for an issue.

use anyhow::{bail, Context, Result};

use crate::cli::HistoryArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd history` command.
pub fn run(ctx: &RuntimeContext, args: &HistoryArgs) -> Result<()> {
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

    // Verify issue exists
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            rusqlite::params![&args.id],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if !exists {
        bail!("issue '{}' not found", args.id);
    }

    let mut stmt = conn.prepare(
        "SELECT event_type, actor, old_value, new_value, created_at \
         FROM events \
         WHERE issue_id = ?1 \
         ORDER BY created_at ASC",
    )?;

    let events: Vec<(String, String, String, String, String)> = stmt
        .query_map(rusqlite::params![&args.id], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_events: Vec<serde_json::Value> = events
            .iter()
            .map(|(event_type, actor, old_value, new_value, created_at)| {
                let mut obj = serde_json::json!({
                    "event_type": event_type,
                    "actor": actor,
                    "created_at": created_at,
                });
                if !old_value.is_empty() {
                    obj["old_value"] = serde_json::json!(old_value);
                }
                if !new_value.is_empty() {
                    obj["new_value"] = serde_json::json!(new_value);
                }
                obj
            })
            .collect();
        output_json(&serde_json::json!({
            "issue_id": args.id,
            "events": json_events,
        }));
    } else {
        if events.is_empty() {
            println!("No history for {}", args.id);
            return Ok(());
        }

        println!("History for {}:\n", args.id);
        for (event_type, actor, old_value, new_value, created_at) in &events {
            // Truncate timestamp to a readable format
            let timestamp = if created_at.len() >= 19 {
                &created_at[..19]
            } else {
                created_at
            };

            let actor_str = if actor.is_empty() {
                String::new()
            } else {
                format!(" by {}", actor)
            };

            let detail = match event_type.as_str() {
                "status_changed" => {
                    if !old_value.is_empty() && !new_value.is_empty() {
                        format!("{} -> {}", old_value, new_value)
                    } else if !new_value.is_empty() {
                        new_value.clone()
                    } else {
                        String::new()
                    }
                }
                "dependency_added" | "dependency_removed" => {
                    if !new_value.is_empty() {
                        new_value.clone()
                    } else if !old_value.is_empty() {
                        old_value.clone()
                    } else {
                        String::new()
                    }
                }
                _ => {
                    if !new_value.is_empty() {
                        new_value.clone()
                    } else if !old_value.is_empty() {
                        old_value.clone()
                    } else {
                        String::new()
                    }
                }
            };

            if detail.is_empty() {
                println!("  {} {}{}", timestamp, event_type, actor_str);
            } else {
                println!("  {} {}{}: {}", timestamp, event_type, actor_str, detail);
            }
        }
    }

    Ok(())
}
