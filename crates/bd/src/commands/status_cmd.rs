//! `bd status` -- get or set issue status.

use anyhow::{bail, Context, Result};
use chrono::Utc;

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::StatusCmdArgs;
use crate::context::RuntimeContext;
use crate::output::{load_labels, output_json};

/// Known valid statuses.
const KNOWN_STATUSES: &[&str] = &[
    "open",
    "in_progress",
    "blocked",
    "deferred",
    "closed",
    "pinned",
    "hooked",
];

/// Execute the `bd status` command.
pub fn run(ctx: &RuntimeContext, args: &StatusCmdArgs) -> Result<()> {
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

    let conn = rusqlite::Connection::open(&db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    // Get current status
    let current_status: String = conn
        .query_row(
            "SELECT status FROM issues WHERE id = ?1",
            rusqlite::params![&args.id],
            |row| row.get(0),
        )
        .with_context(|| format!("issue '{}' not found", args.id))?;

    match &args.new_status {
        None => {
            // Just print current status
            if ctx.json {
                output_json(&serde_json::json!({
                    "id": args.id,
                    "status": current_status,
                }));
            } else {
                println!("{}", current_status);
            }
        }
        Some(new_status) => {
            if ctx.readonly {
                bail!("cannot change status in read-only mode");
            }

            // Validate status (warn but allow custom statuses)
            if !KNOWN_STATUSES.contains(&new_status.as_str()) {
                eprintln!(
                    "Warning: '{}' is not a standard status ({})",
                    new_status,
                    KNOWN_STATUSES.join(", ")
                );
            }

            if new_status == &current_status {
                if !ctx.quiet {
                    println!("Issue {} is already '{}'", args.id, current_status);
                }
                return Ok(());
            }

            let now_str = Utc::now().to_rfc3339();

            // Update status
            let mut sql = String::from("UPDATE issues SET status = ?1, updated_at = ?2");

            // If closing, set closed_at
            if new_status == "closed" {
                sql.push_str(", closed_at = ?2");
            }

            // If reopening from closed, clear closed_at
            if current_status == "closed" && new_status != "closed" {
                sql.push_str(", closed_at = NULL, close_reason = ''");
            }

            sql.push_str(" WHERE id = ?3");

            conn.execute(
                &sql,
                rusqlite::params![new_status, &now_str, &args.id],
            )
            .with_context(|| format!("failed to update status for {}", args.id))?;

            // Record "status_changed" event
            conn.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &args.id,
                    "status_changed",
                    &ctx.actor,
                    &current_status,
                    new_status,
                    &now_str,
                ],
            )?;

            if ctx.json {
                let issue = load_issue_by_id(&conn, &args.id)?;
                output_json(&vec![issue]);
            } else if !ctx.quiet {
                println!("Status of {} changed: {} -> {}", args.id, current_status, new_status);
            }
        }
    }

    Ok(())
}

/// Load an issue by ID, including labels.
fn load_issue_by_id(conn: &rusqlite::Connection, id: &str) -> Result<Issue> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, design, acceptance_criteria, notes, spec_id, \
         status, priority, issue_type, assignee, owner, estimated_minutes, \
         created_at, created_by, updated_at, closed_at, close_reason, \
         due_at, defer_until, external_ref \
         FROM issues WHERE id = ?1",
    )?;

    let mut issue: Issue = stmt.query_row(rusqlite::params![id], |row| {
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
    })?;

    issue.labels = load_labels(conn, id);
    Ok(issue)
}

/// Parse a datetime string (RFC3339) into a `DateTime<Utc>`.
fn parse_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}
