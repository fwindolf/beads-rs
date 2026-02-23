//! `bd gate` -- quality gate management (list, show, create, close, check).
//!
//! Gates are issues with `issue_type='gate'` that block workflow until a
//! condition is met. They support several `await_type` values:
//! - `human`: must be manually closed
//! - `timer`: auto-close when `created_at + timeout < now`
//! - `gh:run`: auto-close when a GitHub Actions run succeeds
//! - `gh:pr`: auto-close when a GitHub PR is merged

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};

use beads_core::idgen;

use crate::cli::{GateArgs, GateCommands};
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd gate` command.
pub fn run(ctx: &RuntimeContext, args: &GateArgs) -> Result<()> {
    match &args.command {
        GateCommands::List => cmd_list(ctx),
        GateCommands::Show(a) => cmd_show(ctx, &a.id),
        GateCommands::Create(a) => cmd_create(ctx, a),
        GateCommands::Close(a) => cmd_close(ctx, &a.id, a.reason.as_deref()),
        GateCommands::Check => cmd_check(ctx),
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

fn cmd_list(ctx: &RuntimeContext) -> Result<()> {
    let conn = open_db(ctx, false)?;

    let mut stmt = conn.prepare(
        "SELECT id, title, status, await_type, await_id, timeout_ns, waiters, created_at \
         FROM issues WHERE issue_type = 'gate' AND status != 'closed' \
         ORDER BY created_at DESC",
    )?;

    let gates: Vec<GateRow> = stmt
        .query_map([], |row| {
            Ok(GateRow {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get::<_, String>(2).unwrap_or_default(),
                await_type: row.get::<_, String>(3).unwrap_or_default(),
                await_id: row.get::<_, String>(4).unwrap_or_default(),
                timeout_ns: row.get::<_, i64>(5).unwrap_or(0),
                waiters_json: row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string()),
                created_at: row.get::<_, String>(7).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_gates: Vec<serde_json::Value> = gates.iter().map(|g| gate_to_json(g)).collect();
        output_json(&json_gates);
    } else if gates.is_empty() {
        println!("No open gates found.");
    } else {
        let headers = &["ID", "AWAIT", "AWAIT_ID", "STATUS", "TITLE"];
        let rows: Vec<Vec<String>> = gates
            .iter()
            .map(|g| {
                vec![
                    g.id.clone(),
                    g.await_type.clone(),
                    truncate(&g.await_id, 20),
                    g.status.clone(),
                    g.title.clone(),
                ]
            })
            .collect();
        output_table(headers, &rows);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Show
// ---------------------------------------------------------------------------

fn cmd_show(ctx: &RuntimeContext, id: &str) -> Result<()> {
    let conn = open_db(ctx, false)?;
    let gate = load_gate(&conn, id)?;

    let waiters: Vec<String> = serde_json::from_str(&gate.waiters_json).unwrap_or_default();
    let timeout_display = format_duration_ns(gate.timeout_ns);

    if ctx.json {
        output_json(&gate_to_json(&gate));
    } else {
        println!("{}: {}", gate.id, gate.title);
        println!("Status: {}", gate.status);
        println!("Await type: {}", if gate.await_type.is_empty() { "none" } else { &gate.await_type });
        if !gate.await_id.is_empty() {
            println!("Await ID: {}", gate.await_id);
        }
        if gate.timeout_ns > 0 {
            println!("Timeout: {}", timeout_display);
        }
        if !waiters.is_empty() {
            println!("Waiters: {}", waiters.join(", "));
        }
        println!("Created: {}", gate.created_at);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

fn cmd_create(ctx: &RuntimeContext, args: &crate::cli::GateCreateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot create gates in read-only mode");
    }

    let conn = open_db(ctx, true)?;

    let await_type = args.await_type.as_deref().unwrap_or("human");
    let await_id = args.await_id.as_deref().unwrap_or("");

    // Parse timeout
    let timeout_ns: i64 = if let Some(ref t) = args.timeout {
        parse_duration_to_ns(t)?
    } else {
        0
    };

    let waiters_json = serde_json::to_string(&args.waiters)?;

    // Get prefix
    let prefix: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'issue_prefix'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "bd".to_string());

    // Generate ID
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))
        .unwrap_or(0);

    let hash_length = idgen::compute_adaptive_length(
        count as usize,
        idgen::adaptive_defaults::MIN_LENGTH,
        idgen::adaptive_defaults::MAX_LENGTH,
        idgen::adaptive_defaults::MAX_COLLISION_PROB,
    );

    let now = Utc::now();
    let now_str = now.to_rfc3339();

    let mut issue_id = String::new();
    for nonce in 0..10 {
        let candidate = idgen::generate_hash_id(
            &prefix,
            &args.title,
            "",
            &ctx.actor,
            now,
            hash_length,
            nonce,
        );
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                rusqlite::params![&candidate],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if !exists {
            issue_id = candidate;
            break;
        }
    }

    if issue_id.is_empty() {
        bail!("failed to generate unique ID after 10 attempts");
    }

    conn.execute(
        "INSERT INTO issues (id, title, status, priority, issue_type, await_type, await_id, timeout_ns, waiters, created_at, created_by, updated_at) \
         VALUES (?1, ?2, 'open', 2, 'gate', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &issue_id,
            &args.title,
            await_type,
            await_id,
            timeout_ns,
            &waiters_json,
            &now_str,
            &ctx.actor,
            &now_str,
        ],
    )
    .with_context(|| format!("failed to create gate {}", issue_id))?;

    // Record event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&issue_id, "created", &ctx.actor, &args.title, &now_str],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "id": issue_id,
            "title": args.title,
            "await_type": await_type,
            "await_id": await_id,
            "timeout_ns": timeout_ns,
            "waiters": args.waiters,
        }));
    } else {
        println!("Created gate: {}", issue_id);
        println!("  Title: {}", args.title);
        println!("  Await type: {}", await_type);
        if !await_id.is_empty() {
            println!("  Await ID: {}", await_id);
        }
        if timeout_ns > 0 {
            println!("  Timeout: {}", format_duration_ns(timeout_ns));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Close
// ---------------------------------------------------------------------------

fn cmd_close(ctx: &RuntimeContext, id: &str, reason: Option<&str>) -> Result<()> {
    if ctx.readonly {
        bail!("cannot close gates in read-only mode");
    }

    let conn = open_db(ctx, true)?;
    let gate = load_gate(&conn, id)?;

    if gate.status == "closed" {
        bail!("gate '{}' is already closed", id);
    }

    let reason = reason.unwrap_or("Manually closed");
    let now_str = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE issues SET status = 'closed', close_reason = ?1, closed_at = ?2, updated_at = ?3 \
         WHERE id = ?4",
        rusqlite::params![reason, &now_str, &now_str, id],
    )?;

    // Record event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, comment, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            id,
            "closed",
            &ctx.actor,
            &gate.status,
            "closed",
            reason,
            &now_str,
        ],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "id": id,
            "status": "closed",
            "reason": reason,
        }));
    } else {
        println!("Closed gate {}: {}", id, reason);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Check
// ---------------------------------------------------------------------------

fn cmd_check(ctx: &RuntimeContext) -> Result<()> {
    let conn = open_db(ctx, true)?;

    let mut stmt = conn.prepare(
        "SELECT id, title, status, await_type, await_id, timeout_ns, waiters, created_at \
         FROM issues WHERE issue_type = 'gate' AND status != 'closed' \
         ORDER BY created_at ASC",
    )?;

    let gates: Vec<GateRow> = stmt
        .query_map([], |row| {
            Ok(GateRow {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get::<_, String>(2).unwrap_or_default(),
                await_type: row.get::<_, String>(3).unwrap_or_default(),
                await_id: row.get::<_, String>(4).unwrap_or_default(),
                timeout_ns: row.get::<_, i64>(5).unwrap_or(0),
                waiters_json: row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string()),
                created_at: row.get::<_, String>(7).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if gates.is_empty() {
        if ctx.json {
            output_json(&serde_json::json!({ "checked": 0, "closed": [] }));
        } else {
            println!("No open gates to check.");
        }
        return Ok(());
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let mut closed_gates: Vec<serde_json::Value> = Vec::new();

    for gate in &gates {
        let result = check_gate(gate, now);
        match result {
            GateResult::Resolved(reason) => {
                // Auto-close the gate
                conn.execute(
                    "UPDATE issues SET status = 'closed', close_reason = ?1, closed_at = ?2, updated_at = ?3 \
                     WHERE id = ?4",
                    rusqlite::params![&reason, &now_str, &now_str, &gate.id],
                )?;

                conn.execute(
                    "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, comment, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        &gate.id,
                        "closed",
                        &ctx.actor,
                        &gate.status,
                        "closed",
                        &reason,
                        &now_str,
                    ],
                )?;

                closed_gates.push(serde_json::json!({
                    "id": gate.id,
                    "reason": reason,
                }));

                if !ctx.json {
                    println!("Closed gate {}: {}", gate.id, reason);
                }
            }
            GateResult::Pending => {
                if !ctx.json && !ctx.quiet {
                    println!("Gate {} ({}): pending", gate.id, gate.await_type);
                }
            }
            GateResult::Error(msg) => {
                if !ctx.json {
                    eprintln!("Gate {} ({}): error -- {}", gate.id, gate.await_type, msg);
                }
            }
        }
    }

    if ctx.json {
        output_json(&serde_json::json!({
            "checked": gates.len(),
            "closed": closed_gates,
        }));
    } else if closed_gates.is_empty() {
        println!("\nChecked {} gates, none resolved.", gates.len());
    } else {
        println!(
            "\nChecked {} gates, {} auto-closed.",
            gates.len(),
            closed_gates.len()
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Gate check logic
// ---------------------------------------------------------------------------

enum GateResult {
    Resolved(String),
    Pending,
    Error(String),
}

fn check_gate(gate: &GateRow, now: DateTime<Utc>) -> GateResult {
    match gate.await_type.as_str() {
        "timer" => check_timer_gate(gate, now),
        "human" => GateResult::Pending, // Must be manually closed
        "gh:run" => check_gh_run_gate(gate),
        "gh:pr" => check_gh_pr_gate(gate),
        _ => GateResult::Pending,
    }
}

fn check_timer_gate(gate: &GateRow, now: DateTime<Utc>) -> GateResult {
    if gate.timeout_ns <= 0 {
        return GateResult::Pending;
    }

    let created_at = DateTime::parse_from_rfc3339(&gate.created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now);

    let elapsed = now.signed_duration_since(created_at);
    let timeout = chrono::Duration::nanoseconds(gate.timeout_ns);

    if elapsed >= timeout {
        GateResult::Resolved("timer expired".to_string())
    } else {
        GateResult::Pending
    }
}

fn check_gh_run_gate(gate: &GateRow) -> GateResult {
    if gate.await_id.is_empty() {
        return GateResult::Error("no await_id set for gh:run gate".to_string());
    }

    // Shell out to: gh run view <await_id> --json status,conclusion
    match std::process::Command::new("gh")
        .args(["run", "view", &gate.await_id, "--json", "status,conclusion"])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return GateResult::Error(format!("gh run view failed: {}", stderr.trim()));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(val) => {
                    let status = val["status"].as_str().unwrap_or("");
                    let conclusion = val["conclusion"].as_str().unwrap_or("");
                    if status == "completed" && conclusion == "success" {
                        GateResult::Resolved(format!(
                            "GitHub Actions run {} completed successfully",
                            gate.await_id
                        ))
                    } else if status == "completed" {
                        GateResult::Error(format!(
                            "run {} completed with conclusion: {}",
                            gate.await_id, conclusion
                        ))
                    } else {
                        GateResult::Pending
                    }
                }
                Err(e) => GateResult::Error(format!("failed to parse gh output: {}", e)),
            }
        }
        Err(e) => GateResult::Error(format!("failed to run gh: {}", e)),
    }
}

fn check_gh_pr_gate(gate: &GateRow) -> GateResult {
    if gate.await_id.is_empty() {
        return GateResult::Error("no await_id set for gh:pr gate".to_string());
    }

    // Shell out to: gh pr view <await_id> --json state,merged
    match std::process::Command::new("gh")
        .args(["pr", "view", &gate.await_id, "--json", "state,merged"])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return GateResult::Error(format!("gh pr view failed: {}", stderr.trim()));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<serde_json::Value>(&stdout) {
                Ok(val) => {
                    let state = val["state"].as_str().unwrap_or("");
                    let merged = val["merged"].as_bool().unwrap_or(false);
                    if state == "MERGED" || merged {
                        GateResult::Resolved(format!(
                            "GitHub PR {} merged",
                            gate.await_id
                        ))
                    } else if state == "CLOSED" {
                        GateResult::Error(format!(
                            "PR {} was closed without merging",
                            gate.await_id
                        ))
                    } else {
                        GateResult::Pending
                    }
                }
                Err(e) => GateResult::Error(format!("failed to parse gh output: {}", e)),
            }
        }
        Err(e) => GateResult::Error(format!("failed to run gh: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct GateRow {
    id: String,
    title: String,
    status: String,
    await_type: String,
    await_id: String,
    timeout_ns: i64,
    waiters_json: String,
    created_at: String,
}

fn gate_to_json(gate: &GateRow) -> serde_json::Value {
    let waiters: Vec<String> = serde_json::from_str(&gate.waiters_json).unwrap_or_default();
    serde_json::json!({
        "id": gate.id,
        "title": gate.title,
        "status": gate.status,
        "await_type": gate.await_type,
        "await_id": gate.await_id,
        "timeout_ns": gate.timeout_ns,
        "timeout": format_duration_ns(gate.timeout_ns),
        "waiters": waiters,
        "created_at": gate.created_at,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load a gate issue by ID.
fn load_gate(conn: &rusqlite::Connection, id: &str) -> Result<GateRow> {
    conn.query_row(
        "SELECT id, title, status, await_type, await_id, timeout_ns, waiters, created_at \
         FROM issues WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(GateRow {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get::<_, String>(2).unwrap_or_default(),
                await_type: row.get::<_, String>(3).unwrap_or_default(),
                await_id: row.get::<_, String>(4).unwrap_or_default(),
                timeout_ns: row.get::<_, i64>(5).unwrap_or(0),
                waiters_json: row.get::<_, String>(6).unwrap_or_else(|_| "[]".to_string()),
                created_at: row.get::<_, String>(7).unwrap_or_default(),
            })
        },
    )
    .with_context(|| format!("gate '{}' not found", id))
}

/// Parse a human-readable duration string into nanoseconds.
///
/// Supports: `30s`, `5m`, `2h`, `1d`, `30m`, `1h30m`, etc.
fn parse_duration_to_ns(s: &str) -> Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(0);
    }

    let mut total_ns: i64 = 0;
    let mut current_num = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            if current_num.is_empty() {
                bail!("invalid duration '{}': expected number before unit", s);
            }
            let num: i64 = current_num
                .parse()
                .with_context(|| format!("invalid number in duration '{}'", s))?;
            current_num.clear();

            let multiplier: i64 = match ch {
                's' => 1_000_000_000,                     // seconds
                'm' => 60 * 1_000_000_000,                // minutes
                'h' => 60 * 60 * 1_000_000_000,           // hours
                'd' => 24 * 60 * 60 * 1_000_000_000,      // days
                _ => bail!("invalid duration unit '{}' in '{}' (valid: s, m, h, d)", ch, s),
            };

            total_ns += num * multiplier;
        }
    }

    // If there's a trailing number without unit, treat as seconds
    if !current_num.is_empty() {
        let num: i64 = current_num
            .parse()
            .with_context(|| format!("invalid number in duration '{}'", s))?;
        total_ns += num * 1_000_000_000;
    }

    if total_ns == 0 {
        bail!("invalid duration '{}': parsed to zero", s);
    }

    Ok(total_ns)
}

/// Format nanoseconds as human-readable duration string.
fn format_duration_ns(ns: i64) -> String {
    if ns <= 0 {
        return "none".to_string();
    }

    let total_secs = ns / 1_000_000_000;
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if secs > 0 {
        parts.push(format!("{}s", secs));
    }

    if parts.is_empty() {
        "0s".to_string()
    } else {
        parts.join("")
    }
}

/// Truncate a string to max length, appending "..." if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

/// Open the beads database (read-only or read-write).
fn open_db(ctx: &RuntimeContext, writable: bool) -> Result<rusqlite::Connection> {
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

    if writable {
        rusqlite::Connection::open(&db_path)
            .with_context(|| format!("failed to open database: {}", db_path.display()))
    } else {
        rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("failed to open database: {}", db_path.display()))
    }
}
