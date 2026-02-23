//! `bd doctor` -- check and repair database health.
//!
//! The `health` subcommand (default when no subcommand given) is a real
//! implementation that checks:
//! - `.beads/` directory exists
//! - Database file can be opened
//! - SQLite integrity check passes
//! - Schema tables are present and valid
//! - Counts of issues, dependencies, labels, comments, events
//! - Data quality issues (empty titles, orphaned records)
//!
//! Other subcommands (fix, validate, pollution, artifacts) are stubs.

use anyhow::{Context, Result};

use crate::cli::{DoctorArgs, DoctorCommands};
use crate::context::RuntimeContext;

/// Expected tables in the beads database schema.
const EXPECTED_TABLES: &[&str] = &[
    "issues",
    "dependencies",
    "labels",
    "comments",
    "events",
    "config",
    "metadata",
];

/// Execute the `bd doctor` command.
pub fn run(ctx: &RuntimeContext, args: &DoctorArgs) -> Result<()> {
    match &args.command {
        Some(DoctorCommands::Health) | None => run_health(ctx),
        Some(DoctorCommands::Fix) => {
            println!("bd doctor fix: not yet implemented");
            Ok(())
        }
        Some(DoctorCommands::Validate) => {
            println!("bd doctor validate: not yet implemented");
            Ok(())
        }
        Some(DoctorCommands::Pollution) => {
            println!("bd doctor pollution: not yet implemented");
            Ok(())
        }
        Some(DoctorCommands::Artifacts) => {
            println!("bd doctor artifacts: not yet implemented");
            Ok(())
        }
    }
}

/// Run the `bd doctor health` check.
fn run_health(ctx: &RuntimeContext) -> Result<()> {
    let mut issues_found = 0u32;

    println!("bd doctor: checking database health...");
    println!();

    // 1. Check .beads/ directory exists
    let beads_dir = match ctx.resolve_db_path() {
        Some(dir) => {
            println!("[OK] .beads/ directory found: {}", dir.display());
            dir
        }
        None => {
            println!("[FAIL] .beads/ directory not found");
            println!();
            println!("Hint: run 'bd init' to create a database");
            issues_found += 1;
            print_summary(issues_found);
            return Ok(());
        }
    };

    // 2. Check database file exists
    let db_path = beads_dir.join("beads.db");
    if !db_path.exists() {
        println!("[FAIL] Database file not found: {}", db_path.display());
        issues_found += 1;
        print_summary(issues_found);
        return Ok(());
    }
    println!("[OK] Database file exists: {}", db_path.display());

    // 3. Try to open the database
    let conn = match rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(conn) => {
            println!("[OK] Database opens successfully");
            conn
        }
        Err(e) => {
            println!("[FAIL] Cannot open database: {}", e);
            issues_found += 1;
            print_summary(issues_found);
            return Ok(());
        }
    };

    // 4. Check integrity
    match conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0)) {
        Ok(result) if result == "ok" => {
            println!("[OK] SQLite integrity check passed");
        }
        Ok(result) => {
            println!("[WARN] SQLite integrity check: {}", result);
            issues_found += 1;
        }
        Err(e) => {
            println!("[FAIL] SQLite integrity check failed: {}", e);
            issues_found += 1;
        }
    }

    // 5. Check schema: verify expected tables exist
    let existing_tables = get_table_names(&conn).context("failed to query table list")?;

    for table in EXPECTED_TABLES {
        if existing_tables.contains(&table.to_string()) {
            println!("[OK] Table '{}' exists", table);
        } else {
            println!("[FAIL] Table '{}' is missing", table);
            issues_found += 1;
        }
    }

    // 6. Count records in key tables
    println!();
    println!("Record counts:");

    let count = |table: &str| -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
            row.get(0)
        })
        .unwrap_or(-1)
    };

    let issue_count = count("issues");
    let dep_count = count("dependencies");
    let label_count = count("labels");
    let comment_count = count("comments");
    let event_count = count("events");

    println!("  Issues:       {}", issue_count);
    println!("  Dependencies: {}", dep_count);
    println!("  Labels:       {}", label_count);
    println!("  Comments:     {}", comment_count);
    println!("  Events:       {}", event_count);

    // 7. Check for empty-title issues
    let empty_titles: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM issues WHERE title IS NULL OR title = ''",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if empty_titles > 0 {
        println!();
        println!(
            "[WARN] {} issue(s) with empty titles detected",
            empty_titles
        );
        issues_found += 1;
    }

    // 8. Check for orphaned dependencies (referencing non-existent issues)
    let orphaned_deps: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM dependencies \
             WHERE issue_id NOT IN (SELECT id FROM issues) \
                OR depends_on_id NOT IN (SELECT id FROM issues)",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if orphaned_deps > 0 {
        println!(
            "[WARN] {} orphaned dependency record(s) detected",
            orphaned_deps
        );
        issues_found += 1;
    }

    // 9. Check for orphaned labels
    let orphaned_labels: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM labels WHERE issue_id NOT IN (SELECT id FROM issues)",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if orphaned_labels > 0 {
        println!(
            "[WARN] {} orphaned label record(s) detected",
            orphaned_labels
        );
        issues_found += 1;
    }

    print_summary(issues_found);
    Ok(())
}

/// Print the final summary line.
fn print_summary(issues_found: u32) {
    println!();
    if issues_found == 0 {
        println!("Health check passed: no issues found");
    } else {
        println!("Health check completed: {} issue(s) found", issues_found);
    }
}

/// Get all user table names from the database.
fn get_table_names(conn: &rusqlite::Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}
