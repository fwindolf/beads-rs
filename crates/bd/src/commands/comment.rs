//! `bd comment` and `bd comments` -- add and list comments on issues.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::cli::{CommentArgs, CommentsArgs};
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd comment` command (add a comment).
pub fn run_add(ctx: &RuntimeContext, args: &CommentArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot add comments in read-only mode");
    }

    let text = match &args.text {
        Some(t) => t.clone(),
        None => bail!("comment text required (editor mode not yet implemented)"),
    };

    if text.trim().is_empty() {
        bail!("comment text cannot be empty");
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

    let now = Utc::now();
    let now_str = now.to_rfc3339();

    conn.execute(
        "INSERT INTO comments (issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&args.id, &ctx.actor, &text, &now_str],
    )
    .with_context(|| format!("failed to add comment to {}", args.id))?;

    // Update the issue's updated_at timestamp
    conn.execute(
        "UPDATE issues SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now_str, &args.id],
    )?;

    // Record comment event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, comment, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&args.id, "commented", &ctx.actor, &text, &now_str],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "issue_id": args.id,
            "author": ctx.actor,
            "text": text,
            "created_at": now_str,
        }));
    } else if !ctx.quiet {
        println!("Added comment to {}", args.id);
    }

    Ok(())
}

/// Execute the `bd comments` command (list comments).
pub fn run_list(ctx: &RuntimeContext, args: &CommentsArgs) -> Result<()> {
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

    let mut stmt = conn.prepare(
        "SELECT id, author, text, created_at FROM comments WHERE issue_id = ?1 ORDER BY created_at ASC",
    )?;

    let comments: Vec<(i64, String, String, String)> = stmt
        .query_map(rusqlite::params![&args.id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_comments: Vec<serde_json::Value> = comments
            .iter()
            .map(|(id, author, text, created_at)| {
                serde_json::json!({
                    "id": id,
                    "issue_id": args.id,
                    "author": author,
                    "text": text,
                    "created_at": created_at,
                })
            })
            .collect();
        output_json(&json_comments);
    } else if comments.is_empty() {
        println!("No comments on {}", args.id);
    } else {
        println!("Comments on {}:\n", args.id);
        for (_, author, text, created_at) in &comments {
            let time_display = DateTime::parse_from_rfc3339(created_at)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|_| created_at.clone());
            println!("  {} {}", time_display, author);
            for line in text.lines() {
                println!("    {}", line);
            }
            println!();
        }
    }

    Ok(())
}
