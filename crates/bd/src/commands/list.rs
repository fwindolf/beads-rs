//! `bd list` -- list issues with filtering and formatting.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;

use crate::cli::ListArgs;
use crate::context::RuntimeContext;
use crate::output::{format_issue_detail, format_issue_row, load_labels, output_json, output_table, populate_labels_bulk};

/// Execute the `bd list` command.
pub fn run(ctx: &RuntimeContext, args: &ListArgs) -> Result<()> {
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

    // Build SQL query with filters
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    // Status filter
    if let Some(ref status) = args.status {
        if status != "all" {
            conditions.push(format!("status = ?{}", params.len() + 1));
            params.push(Box::new(status.clone()));
        }
    } else if !args.all {
        // Default: exclude closed issues
        conditions.push("status != 'closed'".to_string());
    }

    // Type filter
    if let Some(ref t) = args.issue_type {
        let normalized = IssueType::from(t.as_str()).normalize();
        conditions.push(format!("issue_type = ?{}", params.len() + 1));
        params.push(Box::new(normalized.as_str().to_string()));
    }

    // Assignee filter
    if let Some(ref assignee) = args.assignee {
        conditions.push(format!("assignee = ?{}", params.len() + 1));
        params.push(Box::new(assignee.clone()));
    }

    // Priority filter
    if let Some(ref p) = args.priority {
        let priority = parse_priority(p)?;
        conditions.push(format!("priority = ?{}", params.len() + 1));
        params.push(Box::new(priority));
    }

    // Exclude templates and gates by default
    conditions.push("COALESCE(is_template, 0) = 0".to_string());
    conditions.push("issue_type != 'gate'".to_string());

    // Build WHERE clause
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Build ORDER BY clause
    let order_clause = match args.sort.as_deref() {
        Some("priority") => {
            if args.reverse {
                "ORDER BY priority DESC"
            } else {
                "ORDER BY priority ASC"
            }
        }
        Some("created") => {
            if args.reverse {
                "ORDER BY created_at ASC"
            } else {
                "ORDER BY created_at DESC"
            }
        }
        Some("updated") => {
            if args.reverse {
                "ORDER BY updated_at ASC"
            } else {
                "ORDER BY updated_at DESC"
            }
        }
        Some("status") => {
            if args.reverse {
                "ORDER BY status DESC"
            } else {
                "ORDER BY status ASC"
            }
        }
        Some("id") => {
            if args.reverse {
                "ORDER BY id DESC"
            } else {
                "ORDER BY id ASC"
            }
        }
        Some("title") => {
            if args.reverse {
                "ORDER BY title DESC"
            } else {
                "ORDER BY title ASC"
            }
        }
        Some("type") => {
            if args.reverse {
                "ORDER BY issue_type DESC"
            } else {
                "ORDER BY issue_type ASC"
            }
        }
        Some("assignee") => {
            if args.reverse {
                "ORDER BY assignee DESC"
            } else {
                "ORDER BY assignee ASC"
            }
        }
        _ => "ORDER BY priority ASC, created_at DESC",
    };

    // Build LIMIT clause
    let limit_clause = if args.limit > 0 {
        format!("LIMIT {}", args.limit)
    } else {
        String::new()
    };

    let sql = format!(
        "SELECT id, title, description, design, acceptance_criteria, notes, spec_id, \
         status, priority, issue_type, assignee, owner, estimated_minutes, \
         created_at, created_by, updated_at, closed_at, close_reason, \
         due_at, defer_until, external_ref \
         FROM issues {} {} {}",
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

    // Apply label filter in-memory (labels are in a separate table)
    let issues = if args.labels.is_empty() {
        issues
    } else {
        // Flatten comma-separated labels
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
                    let labels = load_labels(&conn, &issue.id);
                    // AND semantics: must have ALL filter labels
                    filter_labels.iter().all(|fl| labels.contains(fl))
                })
                .collect()
        }
    };

    // Apply --label-any filter (OR semantics)
    let issues = if args.label_any.is_empty() {
        issues
    } else {
        let filter_labels: Vec<String> = args
            .label_any
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
                    let labels = load_labels(&conn, &issue.id);
                    // OR semantics: must have ANY filter label
                    filter_labels.iter().any(|fl| labels.contains(fl))
                })
                .collect()
        }
    };

    // Output
    if ctx.json {
        // Go serializes Issue structs directly with labels populated.
        let mut issues = issues;
        populate_labels_bulk(&conn, &mut issues);
        output_json(&issues);
    } else if args.long {
        println!("\nFound {} issues:\n", issues.len());
        for issue in &issues {
            println!("{}", format_issue_detail(issue));
            println!();
        }
    } else {
        // Compact table format
        let headers = &["ID", "PRI", "TYPE", "STATUS", "TITLE", "ASSIGNEE"];
        let rows: Vec<Vec<String>> = issues.iter().map(|i| format_issue_row(i)).collect();
        output_table(headers, &rows);

        // Show truncation hint
        if args.limit > 0 && issues.len() == args.limit as usize {
            eprintln!("\nShowing {} issues (use --limit 0 for all)", args.limit);
        }
    }

    Ok(())
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

/// Parse a datetime string (RFC3339) into a `DateTime<Utc>`.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
