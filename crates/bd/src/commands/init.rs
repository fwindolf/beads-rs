//! `bd init` -- initialize a beads database in the current directory.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use crate::cli::InitArgs;
use crate::context::RuntimeContext;

/// Default gitignore content for the `.beads` directory.
const GITIGNORE_CONTENT: &str = r#"# Beads database files
*.db
*.db-journal
*.db-wal
*.db-shm
dolt/

# Local state
.local_version
interactions.jsonl
"#;

/// Execute the `bd init` command.
pub fn run(ctx: &RuntimeContext, args: &InitArgs) -> Result<()> {
    let cwd = env::current_dir().context("failed to get current directory")?;

    let beads_dir = cwd.join(".beads");

    // Safety guard: check for existing data unless --force
    if !args.force && beads_dir.is_dir() {
        // Check for existing database file
        let db_path = beads_dir.join("beads.db");
        let dolt_path = beads_dir.join("dolt");
        if db_path.exists() || dolt_path.exists() {
            bail!(
                "Found existing database in {}\n\n\
                This workspace is already initialized.\n\n\
                To use the existing database:\n  \
                Just run bd commands normally (e.g., bd list)\n\n\
                To completely reinitialize (data loss warning):\n  \
                rm -rf {} && bd init\n\n\
                Or use --force to re-initialize.",
                beads_dir.display(),
                beads_dir.display()
            );
        }
    }

    // Determine prefix
    let prefix = match &args.prefix {
        Some(p) => p.trim_end_matches('-').to_string(),
        None => {
            // Auto-detect from directory name
            let dir_name = cwd
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "bd".to_string());
            dir_name.trim_end_matches('-').to_string()
        }
    };

    // Create .beads directory
    fs::create_dir_all(&beads_dir)
        .with_context(|| format!("failed to create directory: {}", beads_dir.display()))?;

    // Create .gitignore
    let gitignore_path = beads_dir.join(".gitignore");
    if !gitignore_path.exists() {
        fs::write(&gitignore_path, GITIGNORE_CONTENT).with_context(|| {
            format!("failed to create .gitignore: {}", gitignore_path.display())
        })?;
    }

    // Create metadata.json
    let metadata_path = beads_dir.join("metadata.json");
    if !metadata_path.exists() {
        let metadata = serde_json::json!({
            "backend": "sqlite",
            "database": "beads.db",
            "jsonl_export": "issues.jsonl",
        });
        let content =
            serde_json::to_string_pretty(&metadata).context("failed to serialize metadata.json")?;
        fs::write(&metadata_path, content).with_context(|| {
            format!(
                "failed to create metadata.json: {}",
                metadata_path.display()
            )
        })?;
    }

    // Create the SQLite database
    let db_path = beads_dir.join("beads.db");
    create_database(&db_path, &prefix, &ctx.actor)?;

    // Create empty issues.jsonl
    let jsonl_path = beads_dir.join("issues.jsonl");
    if !jsonl_path.exists() {
        fs::write(&jsonl_path, "")
            .with_context(|| format!("failed to create issues.jsonl: {}", jsonl_path.display()))?;
    }

    if !args.quiet {
        println!();
        println!("bd initialized successfully!");
        println!();
        println!("  Database: {}", db_path.display());
        println!("  Issue prefix: {}", prefix);
        println!(
            "  Issues will be named: {}-<hash> (e.g., {}-a3f2dd)",
            prefix, prefix
        );
        println!();
        println!("Run `bd create \"My first issue\"` to get started.");
        println!();
    }

    Ok(())
}

/// Create and initialize the SQLite database with schema and config.
fn create_database(db_path: &PathBuf, prefix: &str, actor: &str) -> Result<()> {
    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("failed to create database: {}", db_path.display()))?;

    // Enable WAL mode for better concurrent read performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    // Create schema
    conn.execute_batch(SCHEMA_SQL)?;

    // Set issue prefix
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        rusqlite::params!["issue_prefix", prefix],
    )?;

    // Set initial bd version
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        rusqlite::params!["bd_version", env!("CARGO_PKG_VERSION")],
    )?;

    // Set initialization timestamp
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        rusqlite::params!["last_import_time", &now],
    )?;

    // Record the actor who initialized
    if !actor.is_empty() {
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            rusqlite::params!["init_actor", actor],
        )?;
    }

    Ok(())
}

/// SQL schema for the beads database.
///
/// Matches the Go version's schema for compatibility.
const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS issues (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT DEFAULT '',
    design TEXT DEFAULT '',
    acceptance_criteria TEXT DEFAULT '',
    notes TEXT DEFAULT '',
    spec_id TEXT DEFAULT '',
    status TEXT DEFAULT 'open',
    priority INTEGER DEFAULT 2,
    issue_type TEXT DEFAULT 'task',
    assignee TEXT DEFAULT '',
    owner TEXT DEFAULT '',
    estimated_minutes INTEGER,
    created_at TEXT NOT NULL,
    created_by TEXT DEFAULT '',
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    close_reason TEXT DEFAULT '',
    closed_by_session TEXT DEFAULT '',
    due_at TEXT,
    defer_until TEXT,
    external_ref TEXT,
    source_system TEXT DEFAULT '',
    metadata TEXT,
    compaction_level INTEGER DEFAULT 0,
    compacted_at TEXT,
    compacted_at_commit TEXT,
    original_size INTEGER DEFAULT 0,
    source_repo TEXT DEFAULT '',
    sender TEXT DEFAULT '',
    ephemeral INTEGER DEFAULT 0,
    wisp_type TEXT DEFAULT '',
    pinned INTEGER DEFAULT 0,
    is_template INTEGER DEFAULT 0,
    bonded_from TEXT DEFAULT '[]',
    creator TEXT,
    validations TEXT DEFAULT '[]',
    quality_score REAL,
    crystallizes INTEGER DEFAULT 0,
    await_type TEXT DEFAULT '',
    await_id TEXT DEFAULT '',
    timeout_ns INTEGER DEFAULT 0,
    waiters TEXT DEFAULT '[]',
    holder TEXT DEFAULT '',
    source_formula TEXT DEFAULT '',
    source_location TEXT DEFAULT '',
    hook_bead TEXT DEFAULT '',
    role_bead TEXT DEFAULT '',
    agent_state TEXT DEFAULT '',
    last_activity TEXT,
    role_type TEXT DEFAULT '',
    rig TEXT DEFAULT '',
    mol_type TEXT DEFAULT '',
    work_type TEXT DEFAULT 'mutex',
    event_kind TEXT DEFAULT '',
    actor TEXT DEFAULT '',
    target TEXT DEFAULT '',
    payload TEXT DEFAULT '',
    content_hash TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS dependencies (
    issue_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    type TEXT DEFAULT 'blocks',
    created_at TEXT NOT NULL,
    created_by TEXT DEFAULT '',
    metadata TEXT DEFAULT '',
    thread_id TEXT DEFAULT '',
    PRIMARY KEY (issue_id, depends_on_id, type),
    FOREIGN KEY (issue_id) REFERENCES issues(id),
    FOREIGN KEY (depends_on_id) REFERENCES issues(id)
);

CREATE TABLE IF NOT EXISTS labels (
    issue_id TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (issue_id, label),
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);

CREATE TABLE IF NOT EXISTS comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,
    author TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    comment TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);

CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Indices for common queries
CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee);
CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority);
CREATE INDEX IF NOT EXISTS idx_issues_type ON issues(issue_type);
CREATE INDEX IF NOT EXISTS idx_issues_created ON issues(created_at);
CREATE INDEX IF NOT EXISTS idx_issues_updated ON issues(updated_at);
CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on ON dependencies(depends_on_id);
CREATE INDEX IF NOT EXISTS idx_labels_label ON labels(label);
CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id);
CREATE INDEX IF NOT EXISTS idx_events_issue ON events(issue_id);
"#;
