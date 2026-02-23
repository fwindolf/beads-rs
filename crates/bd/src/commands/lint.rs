//! `bd lint` -- lint issues for common problems.
//!
//! Checks for:
//! - Issues with empty titles
//! - Issues with invalid priority values (outside 0-4)
//! - Orphaned labels (labels referencing non-existent issues)
//! - Orphaned dependencies (dependencies referencing non-existent issues)
//! - Issues with status values not in the known set

use anyhow::{Context, Result, bail};

use crate::cli::LintArgs;
use crate::context::RuntimeContext;

/// Known valid status values.
const VALID_STATUSES: &[&str] = &[
    "open",
    "in_progress",
    "blocked",
    "deferred",
    "closed",
    "pinned",
    "hooked",
];

/// Known valid issue types.
const VALID_TYPES: &[&str] = &[
    "bug", "feature", "task", "epic", "chore", "decision", "gate", "event", "wisp",
];

/// Execute the `bd lint` command.
pub fn run(ctx: &RuntimeContext, _args: &LintArgs) -> Result<()> {
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

    let mut warnings = 0u32;
    let mut errors = 0u32;

    println!("bd lint: checking issues for problems...");
    println!();

    // 1. Empty titles
    {
        let mut stmt = conn.prepare("SELECT id FROM issues WHERE title IS NULL OR title = ''")?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        if !ids.is_empty() {
            errors += ids.len() as u32;
            println!("[ERROR] {} issue(s) with empty titles:", ids.len());
            for id in &ids {
                println!("  - {}", id);
            }
            println!();
        }
    }

    // 2. Invalid priorities (outside 0-4)
    {
        let mut stmt =
            conn.prepare("SELECT id, priority FROM issues WHERE priority < 0 OR priority > 4")?;
        let bad: Vec<(String, i32)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        if !bad.is_empty() {
            warnings += bad.len() as u32;
            println!(
                "[WARN] {} issue(s) with invalid priority (expected 0-4):",
                bad.len()
            );
            for (id, pri) in &bad {
                println!("  - {}: priority={}", id, pri);
            }
            println!();
        }
    }

    // 3. Invalid status values
    {
        let placeholders: String = VALID_STATUSES
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT id, status FROM issues WHERE status NOT IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&query)?;
        let bad: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        if !bad.is_empty() {
            warnings += bad.len() as u32;
            println!("[WARN] {} issue(s) with unrecognized status:", bad.len());
            for (id, status) in &bad {
                println!("  - {}: status=\"{}\"", id, status);
            }
            println!();
        }
    }

    // 4. Invalid issue types
    {
        let placeholders: String = VALID_TYPES
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT id, issue_type FROM issues WHERE issue_type NOT IN ({}) AND issue_type != ''",
            placeholders
        );
        let mut stmt = conn.prepare(&query)?;
        let bad: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        if !bad.is_empty() {
            warnings += bad.len() as u32;
            println!("[WARN] {} issue(s) with unrecognized type:", bad.len());
            for (id, itype) in &bad {
                println!("  - {}: type=\"{}\"", id, itype);
            }
            println!();
        }
    }

    // 5. Orphaned labels (labels for non-existent issues)
    {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM labels WHERE issue_id NOT IN (SELECT id FROM issues)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            warnings += 1;
            println!(
                "[WARN] {} orphaned label record(s) (referencing non-existent issues)",
                count
            );
            println!();
        }
    }

    // 6. Orphaned dependencies
    {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dependencies \
                 WHERE issue_id NOT IN (SELECT id FROM issues) \
                    OR depends_on_id NOT IN (SELECT id FROM issues)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            warnings += 1;
            println!(
                "[WARN] {} orphaned dependency record(s) (referencing non-existent issues)",
                count
            );
            println!();
        }
    }

    // 7. Orphaned comments
    {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM comments WHERE issue_id NOT IN (SELECT id FROM issues)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            warnings += 1;
            println!(
                "[WARN] {} orphaned comment record(s) (referencing non-existent issues)",
                count
            );
            println!();
        }
    }

    // Summary
    let total = warnings + errors;
    if total == 0 {
        println!("Lint passed: no problems found");
    } else {
        println!(
            "Lint completed: {} error(s), {} warning(s)",
            errors, warnings
        );
    }

    Ok(())
}
