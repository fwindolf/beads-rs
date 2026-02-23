//! `bd rename` -- rename an issue's title.

use anyhow::{Context, Result, bail};
use chrono::Utc;

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::RenameArgs;
use crate::context::RuntimeContext;
use crate::output::{load_labels, output_json};

/// Execute the `bd rename` command.
pub fn run(ctx: &RuntimeContext, args: &RenameArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot rename issues in read-only mode");
    }

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

    // Check issue exists
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

    let now_str = Utc::now().to_rfc3339();

    // Update the title
    conn.execute(
        "UPDATE issues SET title = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![&args.new_title, &now_str, &args.id],
    )
    .with_context(|| format!("failed to rename issue {}", args.id))?;

    // Record an "updated" event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            &args.id,
            "updated",
            &ctx.actor,
            format!("title -> {}", args.new_title),
            &now_str,
        ],
    )?;

    if ctx.json {
        let issue = load_issue_by_id(&conn, &args.id)?;
        output_json(&vec![issue]);
    } else if !ctx.quiet {
        println!("Renamed {}: {}", args.id, args.new_title);
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
