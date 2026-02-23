//! `bd update` -- update issue fields.

use anyhow::{Context, Result, bail};
use chrono::Utc;

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::UpdateArgs;
use crate::context::RuntimeContext;
use crate::output::{load_labels, output_json};

/// Execute the `bd update` command.
pub fn run(ctx: &RuntimeContext, args: &UpdateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot update issues in read-only mode");
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
    let mut updates: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut changes: Vec<String> = Vec::new();

    // Build SET clause dynamically
    if let Some(ref title) = args.title {
        updates.push(format!("title = ?{}", params.len() + 1));
        params.push(Box::new(title.clone()));
        changes.push(format!("title -> {}", title));
    }

    if let Some(ref desc) = args.description {
        updates.push(format!("description = ?{}", params.len() + 1));
        params.push(Box::new(desc.clone()));
        changes.push("description updated".to_string());
    }

    if let Some(ref t) = args.issue_type {
        let normalized = IssueType::from(t.as_str()).normalize();
        updates.push(format!("issue_type = ?{}", params.len() + 1));
        params.push(Box::new(normalized.as_str().to_string()));
        changes.push(format!("type -> {}", normalized));
    }

    if let Some(ref p) = args.priority {
        let priority = parse_priority(p)?;
        updates.push(format!("priority = ?{}", params.len() + 1));
        params.push(Box::new(priority));
        changes.push(format!("priority -> P{}", priority));
    }

    if let Some(ref assignee) = args.assignee {
        updates.push(format!("assignee = ?{}", params.len() + 1));
        params.push(Box::new(assignee.clone()));
        changes.push(format!("assignee -> {}", assignee));
    }

    if let Some(ref status) = args.status {
        updates.push(format!("status = ?{}", params.len() + 1));
        params.push(Box::new(status.clone()));
        changes.push(format!("status -> {}", status));

        // If closing, set closed_at
        if status == "closed" {
            updates.push(format!("closed_at = ?{}", params.len() + 1));
            params.push(Box::new(now_str.clone()));
        }
    }

    if updates.is_empty() && args.add_labels.is_empty() && args.remove_labels.is_empty() {
        bail!(
            "no fields to update. Specify at least one field flag (--title, --description, --type, --priority, --assignee, --status, --add-label, --remove-label)"
        );
    }

    // Always update updated_at
    updates.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(Box::new(now_str.clone()));

    // Execute update if there are field changes
    if !updates.is_empty() {
        let id_param_idx = params.len() + 1;
        params.push(Box::new(args.id.clone()));

        let sql = format!(
            "UPDATE issues SET {} WHERE id = ?{}",
            updates.join(", "),
            id_param_idx
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .with_context(|| format!("failed to update issue {}", args.id))?;
    }

    // Handle label additions
    for label in &args.add_labels {
        for l in label.split(',') {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                rusqlite::params![&args.id, l],
            )?;
            changes.push(format!("+label:{}", l));
        }
    }

    // Handle label removals
    for label in &args.remove_labels {
        for l in label.split(',') {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            conn.execute(
                "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
                rusqlite::params![&args.id, l],
            )?;
            changes.push(format!("-label:{}", l));
        }
    }

    // Record update event
    if !changes.is_empty() {
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &args.id,
                "updated",
                &ctx.actor,
                changes.join(", "),
                &now_str,
            ],
        )?;
    }

    if ctx.json {
        // Go outputs [Issue] array after update -- re-fetch from DB.
        let issue = load_issue_by_id(&conn, &args.id)?;
        output_json(&vec![issue]);
    } else if !ctx.quiet {
        println!("Updated {}", args.id);
        for change in &changes {
            println!("  {}", change);
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

/// Parse a priority string.
fn parse_priority(s: &str) -> Result<i32> {
    let s = s.trim();
    let num_str = if s.starts_with('P') || s.starts_with('p') {
        &s[1..]
    } else {
        s
    };
    let p: i32 = num_str
        .parse()
        .with_context(|| format!("invalid priority '{}': expected 0-4 or P0-P4", s))?;
    if !(0..=4).contains(&p) {
        anyhow::bail!("priority must be between 0 and 4 (got {})", p);
    }
    Ok(p)
}
