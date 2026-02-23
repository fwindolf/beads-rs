//! `bd agent` -- AI/automation agent state tracking.
//!
//! Agents are issues labeled `gt:agent` that self-report their state.
//! This module implements list, show, state, and run subcommands.

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::cli::{AgentArgs, AgentCommands};
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Valid agent states.
const VALID_STATES: &[&str] = &[
    "idle", "spawning", "running", "working", "stuck", "done", "stopped", "dead",
];

/// Execute the `bd agent` command.
pub fn run(ctx: &RuntimeContext, args: &AgentArgs) -> Result<()> {
    match &args.command {
        AgentCommands::List => cmd_list(ctx),
        AgentCommands::Show(a) => cmd_show(ctx, &a.name),
        AgentCommands::State(a) => cmd_state(ctx, &a.name, &a.new_state),
        AgentCommands::Run(a) => cmd_run(ctx, &a.name, &a.hook_bead),
        AgentCommands::Route(a) => {
            println!("bd agent route {}: not yet implemented", a.name);
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

fn cmd_list(ctx: &RuntimeContext) -> Result<()> {
    let conn = open_db(ctx, false)?;

    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.agent_state, i.role_type, i.rig, i.last_activity, i.hook_bead \
         FROM issues i \
         JOIN labels l ON i.id = l.issue_id \
         WHERE l.label = 'gt:agent' \
         ORDER BY i.updated_at DESC",
    )?;

    let agents: Vec<AgentRow> = stmt
        .query_map([], |row| {
            Ok(AgentRow {
                id: row.get(0)?,
                title: row.get::<_, String>(1).unwrap_or_default(),
                agent_state: row.get::<_, String>(2).unwrap_or_default(),
                role_type: row.get::<_, String>(3).unwrap_or_default(),
                rig: row.get::<_, String>(4).unwrap_or_default(),
                last_activity: row.get::<_, String>(5).unwrap_or_default(),
                hook_bead: row.get::<_, String>(6).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_agents: Vec<serde_json::Value> = agents
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "title": a.title,
                    "agent_state": non_empty(&a.agent_state),
                    "role_type": non_empty(&a.role_type),
                    "rig": non_empty(&a.rig),
                    "last_activity": non_empty(&a.last_activity),
                    "hook_bead": non_empty(&a.hook_bead),
                })
            })
            .collect();
        output_json(&json_agents);
    } else if agents.is_empty() {
        println!("No agents found.");
    } else {
        let headers = &["ID", "STATE", "ROLE", "RIG", "LAST ACTIVITY"];
        let rows: Vec<Vec<String>> = agents
            .iter()
            .map(|a| {
                vec![
                    a.id.clone(),
                    if a.agent_state.is_empty() {
                        "(none)".to_string()
                    } else {
                        a.agent_state.clone()
                    },
                    if a.role_type.is_empty() {
                        "-".to_string()
                    } else {
                        a.role_type.clone()
                    },
                    if a.rig.is_empty() {
                        "-".to_string()
                    } else {
                        a.rig.clone()
                    },
                    format_activity(&a.last_activity),
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

fn cmd_show(ctx: &RuntimeContext, agent_id: &str) -> Result<()> {
    let conn = open_db(ctx, false)?;

    // Load agent
    let agent = load_agent(&conn, agent_id)?;

    // Verify it has gt:agent label
    verify_agent_label(&conn, &agent.id)?;

    if ctx.json {
        output_json(&serde_json::json!({
            "id": agent.id,
            "title": agent.title,
            "agent_state": non_empty(&agent.agent_state),
            "role_type": non_empty(&agent.role_type),
            "rig": non_empty(&agent.rig),
            "last_activity": non_empty(&agent.last_activity),
            "hook_bead": non_empty(&agent.hook_bead),
            "role_bead": non_empty(&agent.role_bead),
        }));
    } else {
        println!("Agent: {}", agent.id);
        println!("Title: {}", agent.title);
        println!();
        println!("State:");
        println!(
            "  agent_state: {}",
            if agent.agent_state.is_empty() {
                "(not set)"
            } else {
                &agent.agent_state
            }
        );
        println!("  last_activity: {}", format_activity(&agent.last_activity));
        println!();
        println!("Identity:");
        println!(
            "  role_type: {}",
            if agent.role_type.is_empty() {
                "(not set)"
            } else {
                &agent.role_type
            }
        );
        println!(
            "  rig: {}",
            if agent.rig.is_empty() {
                "(not set)"
            } else {
                &agent.rig
            }
        );
        println!();
        println!("Slots:");
        println!(
            "  hook: {}",
            if agent.hook_bead.is_empty() {
                "(empty)"
            } else {
                &agent.hook_bead
            }
        );
        println!(
            "  role: {}",
            if agent.role_bead.is_empty() {
                "(empty)"
            } else {
                &agent.role_bead
            }
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

fn cmd_state(ctx: &RuntimeContext, agent_id: &str, new_state: &str) -> Result<()> {
    if ctx.readonly {
        bail!("cannot update agent state in read-only mode");
    }

    let state = new_state.to_lowercase();
    if !VALID_STATES.contains(&state.as_str()) {
        bail!(
            "invalid state {:?}; valid states: {}",
            state,
            VALID_STATES.join(", ")
        );
    }

    let conn = open_db(ctx, true)?;
    let now_str = Utc::now().to_rfc3339();

    // Check if agent exists
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            rusqlite::params![agent_id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !exists {
        // Auto-create the agent issue
        conn.execute(
            "INSERT INTO issues (id, title, status, priority, issue_type, agent_state, last_activity, created_at, created_by, updated_at) \
             VALUES (?1, ?2, 'open', 2, 'task', ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                agent_id,
                &format!("Agent: {}", agent_id),
                &state,
                &now_str,
                &now_str,
                &ctx.actor,
                &now_str,
            ],
        )
        .with_context(|| format!("failed to create agent {}", agent_id))?;

        // Add gt:agent label
        conn.execute(
            "INSERT OR IGNORE INTO labels (issue_id, label, created_at, created_by) VALUES (?1, 'gt:agent', ?2, ?3)",
            rusqlite::params![agent_id, &now_str, &ctx.actor],
        )?;

        // Record event
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, 'created', ?2, ?3, ?4)",
            rusqlite::params![agent_id, &ctx.actor, &format!("Agent: {}", agent_id), &now_str],
        )?;
    } else {
        // Verify it's an agent
        verify_agent_label(&conn, agent_id)?;

        // Update state
        conn.execute(
            "UPDATE issues SET agent_state = ?1, last_activity = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![&state, &now_str, &now_str, agent_id],
        )?;
    }

    // Record state change event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, 'agent_state', ?2, ?3, ?4)",
        rusqlite::params![agent_id, &ctx.actor, &state, &now_str],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "agent": agent_id,
            "agent_state": state,
            "last_activity": now_str,
        }));
    } else {
        println!("{} state={}", agent_id, state);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

fn cmd_run(ctx: &RuntimeContext, agent_id: &str, hook_bead: &str) -> Result<()> {
    if ctx.readonly {
        bail!("cannot update agent in read-only mode");
    }

    let conn = open_db(ctx, true)?;
    let now_str = Utc::now().to_rfc3339();

    // Verify agent exists and has gt:agent label
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            rusqlite::params![agent_id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !exists {
        bail!("agent '{}' not found", agent_id);
    }

    verify_agent_label(&conn, agent_id)?;

    // Update state to running and set hook_bead
    conn.execute(
        "UPDATE issues SET agent_state = 'running', hook_bead = ?1, last_activity = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![hook_bead, &now_str, &now_str, agent_id],
    )?;

    // Record event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, comment, created_at) \
         VALUES (?1, 'agent_state', ?2, 'running', ?3, ?4)",
        rusqlite::params![agent_id, &ctx.actor, &format!("hook={}", hook_bead), &now_str],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "agent": agent_id,
            "agent_state": "running",
            "hook_bead": hook_bead,
            "last_activity": now_str,
        }));
    } else {
        println!("{} state=running hook={}", agent_id, hook_bead);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct AgentRow {
    id: String,
    title: String,
    agent_state: String,
    role_type: String,
    rig: String,
    last_activity: String,
    hook_bead: String,
}

#[derive(Debug)]
struct AgentFull {
    id: String,
    title: String,
    agent_state: String,
    role_type: String,
    rig: String,
    last_activity: String,
    hook_bead: String,
    role_bead: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Open the beads database.
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

/// Load full agent details by ID.
fn load_agent(conn: &rusqlite::Connection, id: &str) -> Result<AgentFull> {
    conn.query_row(
        "SELECT id, title, agent_state, role_type, rig, last_activity, hook_bead, role_bead \
         FROM issues WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(AgentFull {
                id: row.get(0)?,
                title: row.get::<_, String>(1).unwrap_or_default(),
                agent_state: row.get::<_, String>(2).unwrap_or_default(),
                role_type: row.get::<_, String>(3).unwrap_or_default(),
                rig: row.get::<_, String>(4).unwrap_or_default(),
                last_activity: row.get::<_, String>(5).unwrap_or_default(),
                hook_bead: row.get::<_, String>(6).unwrap_or_default(),
                role_bead: row.get::<_, String>(7).unwrap_or_default(),
            })
        },
    )
    .with_context(|| format!("agent '{}' not found", id))
}

/// Verify that an issue has the gt:agent label.
fn verify_agent_label(conn: &rusqlite::Connection, id: &str) -> Result<()> {
    let has_label: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM labels WHERE issue_id = ?1 AND label = 'gt:agent')",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_label {
        bail!("{} is not an agent bead (missing gt:agent label)", id);
    }

    Ok(())
}

/// Format a last_activity timestamp for display.
fn format_activity(activity: &str) -> String {
    if activity.is_empty() {
        return "(not set)".to_string();
    }
    // Try to parse and show relative time
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(activity) {
        let now = Utc::now();
        let dur = now.signed_duration_since(dt.with_timezone(&Utc));
        let secs = dur.num_seconds();
        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    } else {
        activity.to_string()
    }
}

/// Return `None` for empty strings (for JSON null output).
fn non_empty(s: &str) -> serde_json::Value {
    if s.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(s.to_string())
    }
}
