//! `bd types` -- list all known issue types (built-in + custom from config).

use anyhow::{Context, Result};

use crate::cli::TypesArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Built-in issue types, matching the `IssueType` enum in beads-core.
const BUILTIN_TYPES: &[&str] = &[
    "bug", "feature", "task", "epic", "chore", "decision", "message", "molecule", "event",
];

/// Execute the `bd types` command.
///
/// Lists all known issue types: built-in types from the `IssueType` enum
/// plus any custom types defined via `config set custom_types ...`.
pub fn run(ctx: &RuntimeContext, _args: &TypesArgs) -> Result<()> {
    let mut types: Vec<String> = BUILTIN_TYPES.iter().map(|s| s.to_string()).collect();

    // Try to load custom types from config (best-effort: if no DB, just show built-in).
    if let Some(beads_dir) = ctx.resolve_db_path() {
        let db_path = beads_dir.join("beads.db");
        if db_path.exists() {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            // Look for custom types in config table.
            // Convention: "custom_types" key holds comma-separated type names.
            let custom: Option<String> = conn
                .query_row(
                    "SELECT value FROM config WHERE key = 'custom_types'",
                    [],
                    |row| row.get(0),
                )
                .ok();

            if let Some(custom_str) = custom {
                for t in custom_str.split(',') {
                    let t = t.trim();
                    if !t.is_empty() && !types.contains(&t.to_string()) {
                        types.push(t.to_string());
                    }
                }
            }
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "builtin": BUILTIN_TYPES,
            "types": types,
        }));
    } else {
        for t in &types {
            println!("{}", t);
        }
    }

    Ok(())
}
