//! `bd close` -- close one or more issues.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::CloseArgs;
use crate::context::RuntimeContext;
use crate::output::{load_labels, output_json};

/// Execute the `bd close` command.
pub fn run(ctx: &RuntimeContext, args: &CloseArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot close issues in read-only mode");
    }

    if args.ids.is_empty() {
        bail!("no issue ID provided");
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

    let reason = args.reason.as_deref().unwrap_or("Closed");
    let now = Utc::now();
    let now_str = now.to_rfc3339();

    let mut closed_ids: Vec<String> = Vec::new();

    for id in &args.ids {
        // Check if issue exists
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            eprintln!("Issue {} not found", id);
            continue;
        }

        // Check current status
        let current_status: String = conn
            .query_row(
                "SELECT status FROM issues WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or_default();

        if current_status == "closed" {
            eprintln!("Issue {} is already closed", id);
            continue;
        }

        // Check if pinned (requires --force)
        let is_pinned: bool = conn
            .query_row(
                "SELECT COALESCE(pinned, 0) FROM issues WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if is_pinned && !args.force {
            eprintln!(
                "cannot close {}: issue is pinned (use --force to override)",
                id
            );
            continue;
        }

        // Check for open blockers (unless --force)
        if !args.force {
            let blocker_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM dependencies d \
                     JOIN issues i ON d.depends_on_id = i.id \
                     WHERE d.issue_id = ?1 AND d.type = 'blocks' AND i.status != 'closed'",
                    rusqlite::params![id],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if blocker_count > 0 {
                eprintln!(
                    "cannot close {}: blocked by {} open dependencies (use --force to override)",
                    id, blocker_count
                );
                continue;
            }
        }

        // Close the issue
        conn.execute(
            "UPDATE issues SET status = 'closed', close_reason = ?1, closed_at = ?2, updated_at = ?3 \
             WHERE id = ?4",
            rusqlite::params![reason, &now_str, &now_str, id],
        )
        .with_context(|| format!("failed to close issue {}", id))?;

        // Record close event
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, comment, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                "closed",
                &ctx.actor,
                &current_status,
                "closed",
                reason,
                &now_str,
            ],
        )?;

        closed_ids.push(id.clone());

        if !ctx.json {
            println!("Closed {}: {}", id, reason);
        }
    }

    if ctx.json {
        // Go outputs [Issue, ...] array of the closed issues, re-fetched after close.
        let mut issues: Vec<Issue> = Vec::new();
        for id in &closed_ids {
            if let Ok(issue) = load_issue_by_id(&conn, id) {
                issues.push(issue);
            }
        }
        output_json(&issues);
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
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
