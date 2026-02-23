//! `bd create` -- create a new issue.

use anyhow::{bail, Context, Result};
use chrono::Utc;

use beads_core::enums::{IssueType, Status};
use beads_core::idgen;
use beads_core::issue::Issue;

use crate::cli::CreateArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd create` command.
pub fn run(ctx: &RuntimeContext, args: &CreateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot create issues in read-only mode");
    }

    // Resolve title from positional arg or --title flag
    let title = match (&args.title, &args.title_flag) {
        (Some(pos), Some(flag)) if pos != flag => {
            bail!(
                "cannot specify different titles as both positional argument and --title flag\n  \
                Positional: {:?}\n  --title:    {:?}",
                pos,
                flag
            );
        }
        (Some(t), _) => t.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => bail!("title required"),
    };

    // Parse priority
    let priority = parse_priority(&args.priority)?;

    // Normalize issue type
    let issue_type = IssueType::from(args.issue_type.as_str()).normalize();

    // Resolve the database path
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

    // Get issue prefix from config
    let prefix: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'issue_prefix'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "bd".to_string());

    // Generate ID
    let issue_id = if let Some(ref explicit_id) = args.id {
        explicit_id.clone()
    } else {
        // Get issue count for adaptive length
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))
            .unwrap_or(0);

        let hash_length = idgen::compute_adaptive_length(
            count as usize,
            idgen::adaptive_defaults::MIN_LENGTH,
            idgen::adaptive_defaults::MAX_LENGTH,
            idgen::adaptive_defaults::MAX_COLLISION_PROB,
        );

        let description = args.description.as_deref().unwrap_or("");
        let now = Utc::now();

        // Try up to 10 nonces to avoid collisions
        let mut id = String::new();
        for nonce in 0..10 {
            let candidate = idgen::generate_hash_id(
                &prefix,
                &title,
                description,
                &ctx.actor,
                now,
                hash_length,
                nonce,
            );

            // Check if ID already exists
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                    rusqlite::params![&candidate],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !exists {
                id = candidate;
                break;
            }
        }

        if id.is_empty() {
            bail!("failed to generate unique ID after 10 attempts");
        }

        id
    };

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let description = args.description.as_deref().unwrap_or("");

    // Handle --dry-run
    if args.dry_run {
        let issue = Issue {
            id: issue_id.clone(),
            title: title.clone(),
            description: description.to_string(),
            status: Status::Open,
            priority,
            issue_type: issue_type.clone(),
            assignee: args.assignee.clone().unwrap_or_default(),
            created_by: ctx.actor.clone(),
            created_at: now,
            updated_at: now,
            ..Issue::default()
        };

        if ctx.json {
            output_json(&issue);
        } else {
            println!("[DRY RUN] Would create issue:");
            println!("  ID: {}", issue.id);
            println!("  Title: {}", issue.title);
            println!("  Type: {}", issue.issue_type);
            println!("  Priority: P{}", issue.priority);
            println!("  Status: {}", issue.status);
            if !issue.assignee.is_empty() {
                println!("  Assignee: {}", issue.assignee);
            }
            if !issue.description.is_empty() {
                println!("  Description: {}", issue.description);
            }
            if !args.labels.is_empty() {
                println!("  Labels: {}", args.labels.join(", "));
            }
        }
        return Ok(());
    }

    // Insert the issue
    conn.execute(
        "INSERT INTO issues (id, title, description, status, priority, issue_type, assignee, created_at, created_by, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            &issue_id,
            &title,
            description,
            "open",
            priority,
            issue_type.as_str(),
            args.assignee.as_deref().unwrap_or(""),
            &now_str,
            &ctx.actor,
            &now_str,
        ],
    )
    .with_context(|| format!("failed to create issue {}", issue_id))?;

    // Add labels
    for label in &args.labels {
        // Handle comma-separated labels within a single argument
        for l in label.split(',') {
            let l = l.trim();
            if l.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                rusqlite::params![&issue_id, l],
            )
            .with_context(|| format!("failed to add label '{}' to {}", l, issue_id))?;
        }
    }

    // Record create event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&issue_id, "created", &ctx.actor, &title, &now_str],
    )?;

    // Output
    if ctx.json {
        let issue = Issue {
            id: issue_id.clone(),
            title,
            description: description.to_string(),
            status: Status::Open,
            priority,
            issue_type,
            assignee: args.assignee.clone().unwrap_or_default(),
            created_by: ctx.actor.clone(),
            created_at: now,
            updated_at: now,
            labels: args.labels.clone(),
            ..Issue::default()
        };
        output_json(&issue);
    } else if args.silent {
        println!("{}", issue_id);
    } else {
        println!("Created issue: {}", issue_id);
        println!("  Title: {}", args.title.as_deref().unwrap_or(""));
        println!("  Priority: P{}", priority);
        println!("  Status: open");
    }

    Ok(())
}

/// Parse a priority string that can be either a bare number ("2") or prefixed ("P2"/"p2").
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
        bail!("priority must be between 0 and 4 (got {})", p);
    }

    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_priority_bare_number() {
        assert_eq!(parse_priority("0").unwrap(), 0);
        assert_eq!(parse_priority("2").unwrap(), 2);
        assert_eq!(parse_priority("4").unwrap(), 4);
    }

    #[test]
    fn parse_priority_prefixed() {
        assert_eq!(parse_priority("P0").unwrap(), 0);
        assert_eq!(parse_priority("P3").unwrap(), 3);
        assert_eq!(parse_priority("p1").unwrap(), 1);
    }

    #[test]
    fn parse_priority_out_of_range() {
        assert!(parse_priority("5").is_err());
        assert!(parse_priority("-1").is_err());
        assert!(parse_priority("P5").is_err());
    }

    #[test]
    fn parse_priority_invalid() {
        assert!(parse_priority("high").is_err());
        assert!(parse_priority("").is_err());
    }
}
