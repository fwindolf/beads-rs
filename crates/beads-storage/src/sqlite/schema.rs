//! DDL statements and migrations for the SQLite schema.
//!
//! Ported from the Go Dolt schema (`schema.go`), adapted for SQLite types.
//! Timestamps are stored as TEXT in ISO 8601 format (SQLite has no native
//! datetime type). Booleans are stored as INTEGER (0/1). JSON blobs are TEXT.

/// Current schema version. Bumped whenever DDL or migrations change.
pub const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Core DDL statements executed during `init_schema`.
pub const SCHEMA_STATEMENTS: &[&str] = &[
    // -- Issues table --------------------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS issues (
        id                  TEXT PRIMARY KEY,
        content_hash        TEXT DEFAULT '',
        title               TEXT NOT NULL,
        description         TEXT NOT NULL DEFAULT '',
        design              TEXT NOT NULL DEFAULT '',
        acceptance_criteria TEXT NOT NULL DEFAULT '',
        notes               TEXT NOT NULL DEFAULT '',
        status              TEXT NOT NULL DEFAULT 'open',
        priority            INTEGER NOT NULL DEFAULT 2,
        issue_type          TEXT NOT NULL DEFAULT 'task',
        assignee            TEXT DEFAULT '',
        estimated_minutes   INTEGER,
        created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        created_by          TEXT DEFAULT '',
        owner               TEXT DEFAULT '',
        updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        closed_at           TEXT,
        closed_by_session   TEXT DEFAULT '',
        external_ref        TEXT,
        spec_id             TEXT,
        compaction_level    INTEGER DEFAULT 0,
        compacted_at        TEXT,
        compacted_at_commit TEXT,
        original_size       INTEGER DEFAULT 0,
        -- Messaging fields
        sender              TEXT DEFAULT '',
        ephemeral           INTEGER DEFAULT 0,
        wisp_type           TEXT DEFAULT '',
        -- Pinned / template
        pinned              INTEGER DEFAULT 0,
        is_template         INTEGER DEFAULT 0,
        -- Work economics (HOP Decision 006)
        crystallizes        INTEGER DEFAULT 0,
        -- Molecule type
        mol_type            TEXT DEFAULT '',
        -- Work type (mutex / open_competition)
        work_type           TEXT DEFAULT 'mutex',
        -- HOP quality score (0.0-1.0)
        quality_score       REAL,
        -- Federation source system
        source_system       TEXT DEFAULT '',
        -- Custom metadata (JSON blob)
        metadata            TEXT DEFAULT '{}',
        -- Source repo for multi-repo
        source_repo         TEXT DEFAULT '',
        -- Close reason
        close_reason        TEXT DEFAULT '',
        -- Event fields
        event_kind          TEXT DEFAULT '',
        actor               TEXT DEFAULT '',
        target              TEXT DEFAULT '',
        payload             TEXT DEFAULT '',
        -- Gate fields
        await_type          TEXT DEFAULT '',
        await_id            TEXT DEFAULT '',
        timeout_ns          INTEGER DEFAULT 0,
        waiters             TEXT DEFAULT '[]',
        -- Agent fields
        hook_bead           TEXT DEFAULT '',
        role_bead           TEXT DEFAULT '',
        agent_state         TEXT DEFAULT '',
        last_activity       TEXT,
        role_type           TEXT DEFAULT '',
        rig                 TEXT DEFAULT '',
        -- Time-based scheduling
        due_at              TEXT,
        defer_until         TEXT,
        -- Bonded-from lineage (JSON array of BondRef)
        bonded_from         TEXT DEFAULT '[]',
        -- HOP validations (JSON array of Validation)
        validations         TEXT DEFAULT '[]'
    )
    "#,
    // -- Indexes on issues ---------------------------------------------------
    "CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)",
    "CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority)",
    "CREATE INDEX IF NOT EXISTS idx_issues_issue_type ON issues(issue_type)",
    "CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee)",
    "CREATE INDEX IF NOT EXISTS idx_issues_created_at ON issues(created_at)",
    "CREATE INDEX IF NOT EXISTS idx_issues_spec_id ON issues(spec_id)",
    "CREATE INDEX IF NOT EXISTS idx_issues_external_ref ON issues(external_ref)",
    // -- Dependencies table --------------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS dependencies (
        issue_id      TEXT NOT NULL,
        depends_on_id TEXT NOT NULL,
        type          TEXT NOT NULL DEFAULT 'blocks',
        created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        created_by    TEXT NOT NULL,
        metadata      TEXT DEFAULT '{}',
        thread_id     TEXT DEFAULT '',
        PRIMARY KEY (issue_id, depends_on_id),
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    )
    "#,
    "CREATE INDEX IF NOT EXISTS idx_dependencies_issue ON dependencies(issue_id)",
    "CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on ON dependencies(depends_on_id)",
    "CREATE INDEX IF NOT EXISTS idx_dependencies_depends_on_type ON dependencies(depends_on_id, type)",
    "CREATE INDEX IF NOT EXISTS idx_dependencies_thread ON dependencies(thread_id)",
    // -- Labels table --------------------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS labels (
        issue_id TEXT NOT NULL,
        label    TEXT NOT NULL,
        PRIMARY KEY (issue_id, label),
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    )
    "#,
    "CREATE INDEX IF NOT EXISTS idx_labels_label ON labels(label)",
    // -- Comments table ------------------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS comments (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        issue_id   TEXT NOT NULL,
        author     TEXT NOT NULL,
        text       TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    )
    "#,
    "CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id)",
    "CREATE INDEX IF NOT EXISTS idx_comments_created_at ON comments(created_at)",
    // -- Events table (audit trail) ------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS events (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        issue_id    TEXT NOT NULL,
        event_type  TEXT NOT NULL,
        actor       TEXT NOT NULL,
        old_value   TEXT,
        new_value   TEXT,
        comment     TEXT,
        created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
    )
    "#,
    "CREATE INDEX IF NOT EXISTS idx_events_issue ON events(issue_id)",
    "CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at)",
    // -- Config table --------------------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS config (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    )
    "#,
    // -- Metadata table ------------------------------------------------------
    r#"
    CREATE TABLE IF NOT EXISTS metadata (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    )
    "#,
];

/// Default configuration values inserted on first init.
pub const DEFAULT_CONFIG: &[(&str, &str)] = &[
    ("compaction_enabled", "false"),
    ("compact_tier1_days", "30"),
    ("compact_tier1_dep_levels", "2"),
    ("compact_tier2_days", "90"),
    ("compact_tier2_dep_levels", "5"),
    ("compact_tier2_commits", "100"),
    ("compact_model", "claude-haiku-4-5-20251001"),
    ("compact_batch_size", "50"),
    ("compact_parallel_workers", "5"),
    ("auto_compact_enabled", "false"),
    (
        "types.custom",
        "molecule,gate,convoy,merge-request,slot,agent,role,rig,message",
    ),
];

/// Schema migrations applied after initial DDL.
///
/// Each migration is a `(name, sql)` pair. Migrations are tracked in the
/// `metadata` table under the key `migration:<name>` so they run at most once.
pub const MIGRATIONS: &[(&str, &str)] = &[
    // Future migrations go here, e.g.:
    // ("001_add_foo_column", "ALTER TABLE issues ADD COLUMN foo TEXT DEFAULT ''"),
];
