//! `bd ready` -- show ready work items (open, no active blockers).

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::ReadyArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd ready` command.
pub fn run(ctx: &RuntimeContext, args: &ReadyArgs) -> Result<()> {
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

    // Ready work = open issues with no open blocking dependencies.
    // This query:
    // 1. Selects open issues
    // 2. Excludes issues that have open blockers (via LEFT JOIN + IS NULL)
    // 3. Excludes templates, gates, and deferred issues
    let mut conditions: Vec<String> = vec![
        "i.status = 'open'".to_string(),
        "COALESCE(i.is_template, 0) = 0".to_string(),
        "i.issue_type != 'gate'".to_string(),
        "COALESCE(i.ephemeral, 0) = 0".to_string(),
        "(i.defer_until IS NULL OR i.defer_until <= datetime('now'))".to_string(),
    ];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    // Assignee filter
    if args.unassigned {
        conditions.push("(i.assignee = '' OR i.assignee IS NULL)".to_string());
    } else if let Some(ref assignee) = args.assignee {
        conditions.push(format!("i.assignee = ?{}", params.len() + 1));
        params.push(Box::new(assignee.clone()));
    }

    // Priority filter
    if let Some(priority) = args.priority {
        conditions.push(format!("i.priority = ?{}", params.len() + 1));
        params.push(Box::new(priority));
    }

    // Type filter
    if let Some(ref t) = args.issue_type {
        let normalized = IssueType::from(t.as_str()).normalize();
        conditions.push(format!("i.issue_type = ?{}", params.len() + 1));
        params.push(Box::new(normalized.as_str().to_string()));
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));

    // Sort policy
    let order_clause = match args.sort.as_str() {
        "priority" => "ORDER BY i.priority ASC, i.created_at ASC",
        "oldest" => "ORDER BY i.created_at ASC",
        _ => "ORDER BY i.priority ASC, i.created_at ASC",
    };

    let limit_clause = if args.limit > 0 {
        format!("LIMIT {}", args.limit)
    } else {
        String::new()
    };

    // Query: open issues with no open blocking dependencies
    let sql = format!(
        "SELECT i.id, i.title, i.description, i.design, i.acceptance_criteria, i.notes, i.spec_id, \
         i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes, \
         i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason, \
         i.due_at, i.defer_until, i.external_ref \
         FROM issues i \
         LEFT JOIN (\
             SELECT d.issue_id \
             FROM dependencies d \
             JOIN issues blocker ON d.depends_on_id = blocker.id \
             WHERE d.type IN ('blocks', 'parent-child') AND blocker.status != 'closed'\
         ) blocked ON i.id = blocked.issue_id \
         {} AND blocked.issue_id IS NULL \
         {} {}",
        where_clause, order_clause, limit_clause
    );

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let issues: Vec<Issue> = stmt
        .query_map(param_refs.as_slice(), |row| {
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
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Apply label filter in-memory
    let issues = if args.labels.is_empty() {
        issues
    } else {
        let filter_labels: Vec<String> = args
            .labels
            .iter()
            .flat_map(|l| l.split(','))
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        if filter_labels.is_empty() {
            issues
        } else {
            issues
                .into_iter()
                .filter(|issue| {
                    let labels: Vec<String> = conn
                        .prepare("SELECT label FROM labels WHERE issue_id = ?1")
                        .and_then(|mut stmt| {
                            stmt.query_map(rusqlite::params![&issue.id], |row| row.get(0))
                                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                        })
                        .unwrap_or_default();
                    filter_labels.iter().all(|fl| labels.contains(fl))
                })
                .collect()
        }
    };

    // Output
    if ctx.json {
        // Go outputs [Issue, ...] array.
        output_json(&issues);
        return Ok(());
    }

    if issues.is_empty() {
        println!("\nNo ready work found\n");
        return Ok(());
    }

    println!(
        "\nReady work ({} issues with no active blockers):\n",
        issues.len()
    );

    for (i, issue) in issues.iter().enumerate() {
        println!(
            "{}. [P{}] [{}] {}: {}",
            i + 1,
            issue.priority,
            issue.issue_type,
            issue.id,
            issue.title
        );
        if let Some(ref est) = issue.estimated_minutes {
            println!("   Estimate: {} min", est);
        }
        if !issue.assignee.is_empty() {
            println!("   Assignee: {}", issue.assignee);
        }
    }
    println!();

    Ok(())
}

/// Parse a datetime string (RFC3339) into a `DateTime<Utc>`.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
