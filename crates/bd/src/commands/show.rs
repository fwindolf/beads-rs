//! `bd show` -- display issue details.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::ShowArgs;
use crate::context::RuntimeContext;
use crate::output::{format_issue_compact, format_issue_detail, output_json};

/// Execute the `bd show` command.
pub fn run(ctx: &RuntimeContext, args: &ShowArgs) -> Result<()> {
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

    let mut found_count = 0;
    let mut all_issues: Vec<Issue> = Vec::new();

    for id in &args.ids {
        match load_issue(&conn, id)? {
            Some(issue) => {
                found_count += 1;
                all_issues.push(issue);
            }
            None => {
                eprintln!("Issue {} not found", id);
            }
        }
    }

    if ctx.json {
        if all_issues.is_empty() {
            bail!("no issues found matching the provided IDs");
        }
        // Go outputs [Issue, ...] array (even for single ID).
        // Labels are already populated by load_issue().
        output_json(&all_issues);
    } else if args.short {
        for issue in &all_issues {
            println!("{}", format_issue_compact(issue));
        }
    } else {
        for (i, issue) in all_issues.iter().enumerate() {
            if i > 0 {
                println!("\n{}\n", "-".repeat(60));
            }
            println!("{}", format_issue_detail(issue));
            println!();
        }
    }

    if found_count == 0 && !ctx.json {
        std::process::exit(1);
    }

    Ok(())
}

/// Load an issue from the database by ID.
///
/// Returns `None` if the issue is not found.
fn load_issue(conn: &rusqlite::Connection, id: &str) -> Result<Option<Issue>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, design, acceptance_criteria, notes, spec_id, \
         status, priority, issue_type, assignee, owner, estimated_minutes, \
         created_at, created_by, updated_at, closed_at, close_reason, \
         due_at, defer_until, external_ref \
         FROM issues WHERE id = ?1",
    )?;

    let issue = stmt.query_row(rusqlite::params![id], |row| {
        let status_str: String = row.get(7)?;
        let type_str: String = row.get(9)?;
        let created_at_str: String = row.get(13)?;
        let updated_at_str: String = row.get(15)?;
        let closed_at_str: Option<String> = row.get(16)?;
        let due_at_str: Option<String> = row.get(18)?;
        let defer_until_str: Option<String> = row.get(19)?;

        Ok(Issue {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get::<_, String>(2).unwrap_or_default(),
            design: row.get::<_, String>(3).unwrap_or_default(),
            acceptance_criteria: row.get::<_, String>(4).unwrap_or_default(),
            notes: row.get::<_, String>(5).unwrap_or_default(),
            spec_id: row.get::<_, String>(6).unwrap_or_default(),
            status: Status::from(status_str.as_str()),
            priority: row.get(8)?,
            issue_type: IssueType::from(type_str.as_str()),
            assignee: row.get::<_, String>(10).unwrap_or_default(),
            owner: row.get::<_, String>(11).unwrap_or_default(),
            estimated_minutes: row.get(12)?,
            created_at: parse_datetime(&created_at_str),
            created_by: row.get::<_, String>(14).unwrap_or_default(),
            updated_at: parse_datetime(&updated_at_str),
            closed_at: closed_at_str.as_deref().map(parse_datetime),
            close_reason: row.get::<_, String>(17).unwrap_or_default(),
            due_at: due_at_str.as_deref().map(parse_datetime),
            defer_until: defer_until_str.as_deref().map(parse_datetime),
            external_ref: row.get(20)?,
            ..Issue::default()
        })
    });

    match issue {
        Ok(mut issue) => {
            // Load labels
            let mut label_stmt =
                conn.prepare("SELECT label FROM labels WHERE issue_id = ?1 ORDER BY label")?;
            let labels: Vec<String> = label_stmt
                .query_map(rusqlite::params![id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            issue.labels = labels;

            Ok(Some(issue))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Parse a datetime string (RFC3339) into a `DateTime<Utc>`.
///
/// Falls back to `Utc::now()` on parse failure to avoid crashing on corrupt data.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
