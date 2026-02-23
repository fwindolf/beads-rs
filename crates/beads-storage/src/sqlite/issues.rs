//! Issue CRUD operations for [`SqliteStore`].

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use beads_core::content_hash::compute_content_hash;
use beads_core::entity::{BondRef, Validation};
use beads_core::enums::{AgentState, EventType, IssueType, MolType, Status, WispType, WorkType};
use beads_core::filter::IssueFilter;
use beads_core::issue::Issue;

use crate::error::{Result, StorageError};
use crate::sqlite::store::SqliteStore;
use crate::traits::IssueUpdates;

// ---------------------------------------------------------------------------
// Column list (shared between INSERT and SELECT)
// ---------------------------------------------------------------------------

/// All issue columns in a deterministic order for SELECT queries.
pub(crate) const ISSUE_COLUMNS: &str = r#"
    id, content_hash, title, description, design, acceptance_criteria, notes,
    status, priority, issue_type, assignee, estimated_minutes,
    created_at, created_by, owner, updated_at, closed_at, closed_by_session,
    external_ref, spec_id,
    compaction_level, compacted_at, compacted_at_commit, original_size,
    sender, ephemeral, wisp_type,
    pinned, is_template, crystallizes,
    mol_type, work_type, quality_score,
    source_system, metadata, source_repo, close_reason,
    event_kind, actor, target, payload,
    await_type, await_id, timeout_ns, waiters,
    hook_bead, role_bead, agent_state, last_activity, role_type, rig,
    due_at, defer_until,
    bonded_from, validations
"#;

/// Same as [`ISSUE_COLUMNS`] but prefixed with `issues.` for use in JOIN queries
/// to avoid ambiguous column names (e.g. `created_at` exists in both `issues` and `dependencies`).
pub(crate) const ISSUE_COLUMNS_PREFIXED: &str = r#"
    issues.id, issues.content_hash, issues.title, issues.description, issues.design, issues.acceptance_criteria, issues.notes,
    issues.status, issues.priority, issues.issue_type, issues.assignee, issues.estimated_minutes,
    issues.created_at, issues.created_by, issues.owner, issues.updated_at, issues.closed_at, issues.closed_by_session,
    issues.external_ref, issues.spec_id,
    issues.compaction_level, issues.compacted_at, issues.compacted_at_commit, issues.original_size,
    issues.sender, issues.ephemeral, issues.wisp_type,
    issues.pinned, issues.is_template, issues.crystallizes,
    issues.mol_type, issues.work_type, issues.quality_score,
    issues.source_system, issues.metadata, issues.source_repo, issues.close_reason,
    issues.event_kind, issues.actor, issues.target, issues.payload,
    issues.await_type, issues.await_id, issues.timeout_ns, issues.waiters,
    issues.hook_bead, issues.role_bead, issues.agent_state, issues.last_activity, issues.role_type, issues.rig,
    issues.due_at, issues.defer_until,
    issues.bonded_from, issues.validations
"#;

// ---------------------------------------------------------------------------
// Row scanning
// ---------------------------------------------------------------------------

/// Deserialises a row into an [`Issue`].
///
/// The column order MUST match [`ISSUE_COLUMNS`].
pub(crate) fn scan_issue(row: &Row<'_>) -> rusqlite::Result<Issue> {
    let id: String = row.get("id")?;
    let content_hash: String = row.get("content_hash")?;
    let title: String = row.get("title")?;
    let description: String = row.get("description")?;
    let design: String = row.get("design")?;
    let acceptance_criteria: String = row.get("acceptance_criteria")?;
    let notes: String = row.get("notes")?;

    let status_str: String = row.get("status")?;
    let priority: i32 = row.get("priority")?;
    let issue_type_str: String = row.get("issue_type")?;
    let assignee: String = row.get("assignee")?;
    let estimated_minutes: Option<i32> = row.get("estimated_minutes")?;

    let created_at_str: String = row.get("created_at")?;
    let created_by: String = row.get("created_by")?;
    let owner: String = row.get("owner")?;
    let updated_at_str: String = row.get("updated_at")?;
    let closed_at_str: Option<String> = row.get("closed_at")?;
    let closed_by_session: String = row.get("closed_by_session")?;

    let external_ref: Option<String> = row.get("external_ref")?;
    let spec_id: String = row.get::<_, Option<String>>("spec_id")?.unwrap_or_default();

    let compaction_level: i32 = row.get("compaction_level")?;
    let compacted_at_str: Option<String> = row.get("compacted_at")?;
    let compacted_at_commit: Option<String> = row.get("compacted_at_commit")?;
    let original_size: i32 = row.get("original_size")?;

    let sender: String = row.get("sender")?;
    let ephemeral_int: i32 = row.get("ephemeral")?;
    let wisp_type_str: String = row.get("wisp_type")?;

    let pinned_int: i32 = row.get("pinned")?;
    let is_template_int: i32 = row.get("is_template")?;
    let crystallizes_int: i32 = row.get("crystallizes")?;

    let mol_type_str: String = row.get("mol_type")?;
    let work_type_str: String = row.get("work_type")?;
    let quality_score: Option<f64> = row.get("quality_score")?;

    let source_system: String = row.get("source_system")?;
    let metadata_str: String = row.get("metadata")?;
    let source_repo: String = row.get("source_repo")?;
    let close_reason: String = row.get("close_reason")?;

    let event_kind: String = row.get("event_kind")?;
    let actor: String = row.get("actor")?;
    let target: String = row.get("target")?;
    let payload: String = row.get("payload")?;

    let await_type: String = row.get("await_type")?;
    let await_id: String = row.get("await_id")?;
    let timeout_ns: i64 = row.get("timeout_ns")?;
    let waiters_str: String = row.get("waiters")?;

    let hook_bead: String = row.get("hook_bead")?;
    let role_bead: String = row.get("role_bead")?;
    let agent_state_str: String = row.get("agent_state")?;
    let last_activity_str: Option<String> = row.get("last_activity")?;
    let role_type: String = row.get("role_type")?;
    let rig: String = row.get("rig")?;

    let due_at_str: Option<String> = row.get("due_at")?;
    let defer_until_str: Option<String> = row.get("defer_until")?;

    let bonded_from_str: String = row.get("bonded_from")?;
    let validations_str: String = row.get("validations")?;

    // Parse timestamps.
    let created_at = parse_datetime(&created_at_str);
    let updated_at = parse_datetime(&updated_at_str);
    let closed_at = closed_at_str.as_deref().map(parse_datetime);
    let compacted_at = compacted_at_str.as_deref().map(parse_datetime);
    let last_activity = last_activity_str.as_deref().map(parse_datetime);
    let due_at = due_at_str.as_deref().map(parse_datetime);
    let defer_until = defer_until_str.as_deref().map(parse_datetime);

    // Parse JSON fields.
    let metadata = if metadata_str.is_empty() || metadata_str == "{}" {
        None
    } else {
        serde_json::value::RawValue::from_string(metadata_str).ok()
    };

    let bonded_from: Vec<BondRef> = serde_json::from_str(&bonded_from_str).unwrap_or_default();
    let validations: Vec<Validation> = serde_json::from_str(&validations_str).unwrap_or_default();
    let waiters: Vec<String> = serde_json::from_str(&waiters_str).unwrap_or_default();

    let timeout = if timeout_ns > 0 {
        Some(std::time::Duration::from_nanos(timeout_ns as u64))
    } else {
        None
    };

    Ok(Issue {
        id,
        content_hash,
        title,
        description,
        design,
        acceptance_criteria,
        notes,
        status: Status::from(status_str),
        priority,
        issue_type: IssueType::from(issue_type_str),
        assignee,
        estimated_minutes,
        created_at,
        created_by,
        owner,
        updated_at,
        closed_at,
        closed_by_session,
        external_ref,
        spec_id,
        compaction_level,
        compacted_at,
        compacted_at_commit,
        original_size,
        sender,
        ephemeral: ephemeral_int != 0,
        wisp_type: WispType::from(wisp_type_str),
        pinned: pinned_int != 0,
        is_template: is_template_int != 0,
        crystallizes: crystallizes_int != 0,
        mol_type: MolType::from(mol_type_str),
        work_type: WorkType::from(work_type_str),
        quality_score: quality_score.map(|v| v as f32),
        source_system,
        metadata,
        source_repo,
        close_reason,
        event_kind,
        actor,
        target,
        payload,
        await_type,
        await_id,
        timeout,
        waiters,
        hook_bead,
        role_bead,
        agent_state: AgentState::from(agent_state_str),
        last_activity,
        role_type,
        rig,
        due_at,
        defer_until,
        bonded_from,
        validations,
        // Fields not stored in DB:
        labels: Vec::new(),
        dependencies: Vec::new(),
        comments: Vec::new(),
        id_prefix: String::new(),
        prefix_override: String::new(),
        creator: None,
        holder: String::new(),
        source_formula: String::new(),
        source_location: String::new(),
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Formats a `DateTime<Utc>` as ISO 8601 TEXT for SQLite.
pub(crate) fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Parses an ISO 8601 TEXT string from SQLite into a `DateTime<Utc>`.
pub(crate) fn parse_datetime(s: &str) -> DateTime<Utc> {
    // Try full RFC 3339 first, then common SQLite formats.
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ"))
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
            .map(|ndt| ndt.and_utc())
            .unwrap_or_else(|_| Utc::now())
    })
}

// ---------------------------------------------------------------------------
// Issue insert helper (shared between store and transaction)
// ---------------------------------------------------------------------------

/// Inserts a single issue into the database using the provided connection.
pub(crate) fn insert_issue(conn: &Connection, issue: &Issue, actor: &str) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);
    let content_hash = compute_content_hash(issue);

    let metadata_str = issue
        .metadata
        .as_ref()
        .map(|m| m.get().to_string())
        .unwrap_or_else(|| "{}".to_string());
    let bonded_from_str =
        serde_json::to_string(&issue.bonded_from).unwrap_or_else(|_| "[]".to_string());
    let validations_str =
        serde_json::to_string(&issue.validations).unwrap_or_else(|_| "[]".to_string());
    let waiters_str = serde_json::to_string(&issue.waiters).unwrap_or_else(|_| "[]".to_string());
    let timeout_ns = issue.timeout.map(|d| d.as_nanos() as i64).unwrap_or(0);

    let created_at_str = format_datetime(&issue.created_at);
    let updated_at_str = format_datetime(&issue.updated_at);
    let closed_at_str = issue.closed_at.as_ref().map(format_datetime);
    let compacted_at_str = issue.compacted_at.as_ref().map(format_datetime);
    let last_activity_str = issue.last_activity.as_ref().map(format_datetime);
    let due_at_str = issue.due_at.as_ref().map(format_datetime);
    let defer_until_str = issue.defer_until.as_ref().map(format_datetime);

    conn.execute(
        &format!(
            "INSERT INTO issues ({ISSUE_COLUMNS}) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18,
                ?19, ?20,
                ?21, ?22, ?23, ?24,
                ?25, ?26, ?27,
                ?28, ?29, ?30,
                ?31, ?32, ?33,
                ?34, ?35, ?36, ?37,
                ?38, ?39, ?40, ?41,
                ?42, ?43, ?44, ?45,
                ?46, ?47, ?48, ?49, ?50, ?51,
                ?52, ?53,
                ?54, ?55
            )"
        ),
        params![
            issue.id,                              // 1
            content_hash,                          // 2
            issue.title,                           // 3
            issue.description,                     // 4
            issue.design,                          // 5
            issue.acceptance_criteria,             // 6
            issue.notes,                           // 7
            issue.status.as_str(),                 // 8
            issue.priority,                        // 9
            issue.issue_type.as_str(),             // 10
            issue.assignee,                        // 11
            issue.estimated_minutes,               // 12
            created_at_str,                        // 13
            issue.created_by,                      // 14
            issue.owner,                           // 15
            updated_at_str,                        // 16
            closed_at_str,                         // 17
            issue.closed_by_session,               // 18
            issue.external_ref,                    // 19
            issue.spec_id,                         // 20
            issue.compaction_level,                // 21
            compacted_at_str,                      // 22
            issue.compacted_at_commit,             // 23
            issue.original_size,                   // 24
            issue.sender,                          // 25
            issue.ephemeral as i32,                // 26
            issue.wisp_type.as_str(),              // 27
            issue.pinned as i32,                   // 28
            issue.is_template as i32,              // 29
            issue.crystallizes as i32,             // 30
            issue.mol_type.as_str(),               // 31
            issue.work_type.as_str(),              // 32
            issue.quality_score.map(|v| v as f64), // 33
            issue.source_system,                   // 34
            metadata_str,                          // 35
            issue.source_repo,                     // 36
            issue.close_reason,                    // 37
            issue.event_kind,                      // 38
            issue.actor,                           // 39
            issue.target,                          // 40
            issue.payload,                         // 41
            issue.await_type,                      // 42
            issue.await_id,                        // 43
            timeout_ns,                            // 44
            waiters_str,                           // 45
            issue.hook_bead,                       // 46
            issue.role_bead,                       // 47
            issue.agent_state.as_str(),            // 48
            last_activity_str,                     // 49
            issue.role_type,                       // 50
            issue.rig,                             // 51
            due_at_str,                            // 52
            defer_until_str,                       // 53
            bonded_from_str,                       // 54
            validations_str,                       // 55
        ],
    )?;

    // Emit "created" event.
    emit_event(
        conn,
        &issue.id,
        EventType::Created,
        actor,
        None,
        None,
        None,
        &now_str,
    )?;

    Ok(())
}

/// Emits an event row into the events table.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_event(
    conn: &Connection,
    issue_id: &str,
    event_type: EventType,
    actor: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    comment: Option<&str>,
    created_at: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, comment, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            issue_id,
            event_type.as_str(),
            actor,
            old_value,
            new_value,
            comment,
            created_at,
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// SqliteStore issue methods
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Creates a single issue.
    pub fn create_issue_impl(&self, issue: &Issue, actor: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        insert_issue(&conn, issue, actor)
    }

    /// Creates multiple issues in a single transaction.
    pub fn create_issues_impl(&self, issues: &[Issue], actor: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| StorageError::Transaction(format!("failed to begin: {e}")))?;
        for issue in issues {
            insert_issue(&tx, issue, actor)?;
        }
        tx.commit()
            .map_err(|e| StorageError::Transaction(format!("failed to commit: {e}")))?;
        Ok(())
    }

    /// Retrieves an issue by ID.
    pub fn get_issue_impl(&self, id: &str) -> Result<Issue> {
        let conn = self.lock_conn()?;
        get_issue_on_conn(&conn, id)
    }

    /// Retrieves an issue by external reference.
    pub fn get_issue_by_external_ref_impl(&self, external_ref: &str) -> Result<Issue> {
        let conn = self.lock_conn()?;
        let sql = format!("SELECT {ISSUE_COLUMNS} FROM issues WHERE external_ref = ?1");
        conn.query_row(&sql, params![external_ref], scan_issue)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    StorageError::not_found("issue", format!("external_ref={external_ref}"))
                }
                other => StorageError::Query(other),
            })
    }

    /// Retrieves multiple issues by their IDs.
    pub fn get_issues_by_ids_impl(&self, ids: &[String]) -> Result<Vec<Issue>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.lock_conn()?;
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT {ISSUE_COLUMNS} FROM issues WHERE id IN ({placeholders})");
        let mut stmt = conn.prepare(&sql)?;
        let params = rusqlite::params_from_iter(ids.iter());
        let rows = stmt.query_map(params, scan_issue)?;
        let mut issues = Vec::new();
        for row in rows {
            issues.push(row?);
        }
        Ok(issues)
    }

    /// Applies partial updates to an issue.
    pub fn update_issue_impl(&self, id: &str, updates: &IssueUpdates, actor: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        update_issue_on_conn(&conn, id, updates, actor)
    }

    /// Closes an issue.
    pub fn close_issue_impl(
        &self,
        id: &str,
        reason: &str,
        actor: &str,
        session: &str,
    ) -> Result<()> {
        let conn = self.lock_conn()?;
        close_issue_on_conn(&conn, id, reason, actor, session)
    }

    /// Deletes an issue and all its related data (cascading FKs).
    pub fn delete_issue_impl(&self, id: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        delete_issue_on_conn(&conn, id)
    }

    /// Searches issues by text query and filter.
    pub fn search_issues_impl(&self, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>> {
        let conn = self.lock_conn()?;
        search_issues_on_conn(&conn, query, filter)
    }
}

// ---------------------------------------------------------------------------
// Connection-level helpers (used by both SqliteStore and Transaction)
// ---------------------------------------------------------------------------

/// Retrieves a single issue by ID on the given connection.
pub(crate) fn get_issue_on_conn(conn: &Connection, id: &str) -> Result<Issue> {
    let sql = format!("SELECT {ISSUE_COLUMNS} FROM issues WHERE id = ?1");
    conn.query_row(&sql, params![id], scan_issue)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => StorageError::not_found("issue", id),
            other => StorageError::Query(other),
        })
}

/// Applies partial updates on the given connection.
pub(crate) fn update_issue_on_conn(
    conn: &Connection,
    id: &str,
    updates: &IssueUpdates,
    actor: &str,
) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);

    // Build SET clause dynamically from non-None fields.
    let mut set_clauses: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    macro_rules! add_field {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = updates.$field {
                set_clauses.push(format!("{} = ?", $col));
                param_values.push(Box::new(val.clone()));
            }
        };
    }

    macro_rules! add_bool_field {
        ($field:ident, $col:expr) => {
            if let Some(val) = updates.$field {
                set_clauses.push(format!("{} = ?", $col));
                param_values.push(Box::new(val as i32));
            }
        };
    }

    add_field!(title, "title");
    add_field!(description, "description");
    add_field!(design, "design");
    add_field!(acceptance_criteria, "acceptance_criteria");
    add_field!(notes, "notes");
    add_field!(spec_id, "spec_id");
    add_field!(assignee, "assignee");
    add_field!(owner, "owner");
    add_field!(source_system, "source_system");
    add_field!(close_reason, "close_reason");
    add_field!(closed_by_session, "closed_by_session");
    add_field!(sender, "sender");
    add_field!(mol_type, "mol_type");
    add_field!(work_type, "work_type");
    add_field!(wisp_type, "wisp_type");
    add_field!(await_type, "await_type");
    add_field!(await_id, "await_id");
    add_field!(hook_bead, "hook_bead");
    add_field!(role_bead, "role_bead");
    add_field!(agent_state, "agent_state");
    add_field!(role_type, "role_type");
    add_field!(rig, "rig");
    add_field!(event_kind, "event_kind");
    add_field!(actor, "actor");
    add_field!(target, "target");
    add_field!(payload, "payload");

    if let Some(ref status) = updates.status {
        set_clauses.push("status = ?".to_string());
        param_values.push(Box::new(status.as_str().to_string()));
    }
    if let Some(ref issue_type) = updates.issue_type {
        set_clauses.push("issue_type = ?".to_string());
        param_values.push(Box::new(issue_type.as_str().to_string()));
    }
    if let Some(priority) = updates.priority {
        set_clauses.push("priority = ?".to_string());
        param_values.push(Box::new(priority));
    }

    // Option<Option<T>> fields: outer Some means "update", inner Option is the new value.
    if let Some(ref ext) = updates.external_ref {
        set_clauses.push("external_ref = ?".to_string());
        param_values.push(Box::new(ext.clone()));
    }
    if let Some(ref est) = updates.estimated_minutes {
        set_clauses.push("estimated_minutes = ?".to_string());
        param_values.push(Box::new(*est));
    }
    if let Some(ref meta) = updates.metadata {
        set_clauses.push("metadata = ?".to_string());
        param_values.push(Box::new(meta.clone().unwrap_or_else(|| "{}".to_string())));
    }
    if let Some(ref qs) = updates.quality_score {
        set_clauses.push("quality_score = ?".to_string());
        param_values.push(Box::new(qs.map(|v| v as f64)));
    }
    if let Some(ref timeout) = updates.timeout {
        set_clauses.push("timeout_ns = ?".to_string());
        let ns = timeout.map(|d| d.as_nanos() as i64).unwrap_or(0);
        param_values.push(Box::new(ns));
    }
    if let Some(ref waiters) = updates.waiters {
        set_clauses.push("waiters = ?".to_string());
        param_values.push(Box::new(
            serde_json::to_string(waiters).unwrap_or_else(|_| "[]".to_string()),
        ));
    }

    // DateTime Option<Option<DateTime>> fields.
    if let Some(ref due) = updates.due_at {
        set_clauses.push("due_at = ?".to_string());
        param_values.push(Box::new(due.as_ref().map(format_datetime)));
    }
    if let Some(ref defer) = updates.defer_until {
        set_clauses.push("defer_until = ?".to_string());
        param_values.push(Box::new(defer.as_ref().map(format_datetime)));
    }
    if let Some(ref la) = updates.last_activity {
        set_clauses.push("last_activity = ?".to_string());
        param_values.push(Box::new(la.as_ref().map(format_datetime)));
    }

    add_bool_field!(pinned, "pinned");
    add_bool_field!(is_template, "is_template");
    add_bool_field!(ephemeral, "ephemeral");
    add_bool_field!(crystallizes, "crystallizes");

    if set_clauses.is_empty() {
        return Ok(()); // Nothing to update.
    }

    // Always update updated_at.
    set_clauses.push("updated_at = ?".to_string());
    param_values.push(Box::new(now_str.clone()));

    let sql = format!("UPDATE issues SET {} WHERE id = ?", set_clauses.join(", "));
    param_values.push(Box::new(id.to_string()));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    let affected = conn.execute(&sql, param_refs.as_slice())?;
    if affected == 0 {
        return Err(StorageError::not_found("issue", id));
    }

    // Emit "updated" event.
    emit_event(
        conn,
        id,
        EventType::Updated,
        actor,
        None,
        None,
        None,
        &now_str,
    )?;

    Ok(())
}

/// Closes an issue on the given connection.
pub(crate) fn close_issue_on_conn(
    conn: &Connection,
    id: &str,
    reason: &str,
    actor: &str,
    session: &str,
) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);

    let affected = conn.execute(
        "UPDATE issues SET status = 'closed', closed_at = ?1, close_reason = ?2,
         closed_by_session = ?3, updated_at = ?1 WHERE id = ?4",
        params![now_str, reason, session, id],
    )?;
    if affected == 0 {
        return Err(StorageError::not_found("issue", id));
    }

    emit_event(
        conn,
        id,
        EventType::Closed,
        actor,
        None,
        Some(reason),
        None,
        &now_str,
    )?;

    Ok(())
}

/// Deletes an issue on the given connection.
pub(crate) fn delete_issue_on_conn(conn: &Connection, id: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM issues WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(StorageError::not_found("issue", id));
    }
    Ok(())
}

/// Searches issues on the given connection.
pub(crate) fn search_issues_on_conn(
    conn: &Connection,
    query: &str,
    filter: &IssueFilter,
) -> Result<Vec<Issue>> {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1;

    // Full-text search across title, description, notes.
    if !query.is_empty() {
        where_clauses.push(format!(
            "(title LIKE ?{pi} OR description LIKE ?{pi} OR notes LIKE ?{pi})",
            pi = param_idx
        ));
        param_values.push(Box::new(format!("%{query}%")));
        param_idx += 1;
    }

    // Filter fields.
    if let Some(ref status) = filter.status {
        where_clauses.push(format!("status = ?{param_idx}"));
        param_values.push(Box::new(status.as_str().to_string()));
        param_idx += 1;
    }
    if let Some(priority) = filter.priority {
        where_clauses.push(format!("priority = ?{param_idx}"));
        param_values.push(Box::new(priority));
        param_idx += 1;
    }
    if let Some(ref issue_type) = filter.issue_type {
        where_clauses.push(format!("issue_type = ?{param_idx}"));
        param_values.push(Box::new(issue_type.as_str().to_string()));
        param_idx += 1;
    }
    if let Some(ref assignee) = filter.assignee {
        where_clauses.push(format!("assignee = ?{param_idx}"));
        param_values.push(Box::new(assignee.clone()));
        param_idx += 1;
    }
    if let Some(ref title_contains) = filter.title_contains {
        where_clauses.push(format!("title LIKE ?{param_idx}"));
        param_values.push(Box::new(format!("%{title_contains}%")));
        param_idx += 1;
    }
    if let Some(ref desc_contains) = filter.description_contains {
        where_clauses.push(format!("description LIKE ?{param_idx}"));
        param_values.push(Box::new(format!("%{desc_contains}%")));
        param_idx += 1;
    }
    if let Some(ref notes_contains) = filter.notes_contains {
        where_clauses.push(format!("notes LIKE ?{param_idx}"));
        param_values.push(Box::new(format!("%{notes_contains}%")));
        param_idx += 1;
    }
    if let Some(ref created_after) = filter.created_after {
        where_clauses.push(format!("created_at >= ?{param_idx}"));
        param_values.push(Box::new(format_datetime(created_after)));
        param_idx += 1;
    }
    if let Some(ref created_before) = filter.created_before {
        where_clauses.push(format!("created_at <= ?{param_idx}"));
        param_values.push(Box::new(format_datetime(created_before)));
        param_idx += 1;
    }
    if let Some(ref updated_after) = filter.updated_after {
        where_clauses.push(format!("updated_at >= ?{param_idx}"));
        param_values.push(Box::new(format_datetime(updated_after)));
        param_idx += 1;
    }
    if let Some(ref updated_before) = filter.updated_before {
        where_clauses.push(format!("updated_at <= ?{param_idx}"));
        param_values.push(Box::new(format_datetime(updated_before)));
        param_idx += 1;
    }
    if filter.no_assignee {
        where_clauses.push("(assignee IS NULL OR assignee = '')".to_string());
    }
    if filter.empty_description {
        where_clauses.push("(description IS NULL OR description = '')".to_string());
    }
    if let Some(ref id_prefix) = filter.id_prefix {
        where_clauses.push(format!("id LIKE ?{param_idx}"));
        param_values.push(Box::new(format!("{id_prefix}%")));
        param_idx += 1;
    }
    if let Some(ref spec_prefix) = filter.spec_id_prefix {
        where_clauses.push(format!("spec_id LIKE ?{param_idx}"));
        param_values.push(Box::new(format!("{spec_prefix}%")));
        param_idx += 1;
    }
    if let Some(ephemeral) = filter.ephemeral {
        where_clauses.push(format!("ephemeral = ?{param_idx}"));
        param_values.push(Box::new(ephemeral as i32));
        param_idx += 1;
    }
    if let Some(pinned) = filter.pinned {
        where_clauses.push(format!("pinned = ?{param_idx}"));
        param_values.push(Box::new(pinned as i32));
        param_idx += 1;
    }
    if let Some(is_template) = filter.is_template {
        where_clauses.push(format!("is_template = ?{param_idx}"));
        param_values.push(Box::new(is_template as i32));
        param_idx += 1;
    }
    if let Some(ref mol_type) = filter.mol_type {
        where_clauses.push(format!("mol_type = ?{param_idx}"));
        param_values.push(Box::new(mol_type.as_str().to_string()));
        param_idx += 1;
    }
    if let Some(ref wisp_type) = filter.wisp_type {
        where_clauses.push(format!("wisp_type = ?{param_idx}"));
        param_values.push(Box::new(wisp_type.as_str().to_string()));
        param_idx += 1;
    }
    if let Some(ref source_repo) = filter.source_repo {
        where_clauses.push(format!("source_repo = ?{param_idx}"));
        param_values.push(Box::new(source_repo.clone()));
        param_idx += 1;
    }

    // Exclude statuses.
    for status in &filter.exclude_status {
        where_clauses.push(format!("status != ?{param_idx}"));
        param_values.push(Box::new(status.as_str().to_string()));
        param_idx += 1;
    }
    // Exclude types.
    for itype in &filter.exclude_types {
        where_clauses.push(format!("issue_type != ?{param_idx}"));
        param_values.push(Box::new(itype.as_str().to_string()));
        param_idx += 1;
    }

    // Filter by specific IDs.
    if !filter.ids.is_empty() {
        let placeholders: Vec<String> = filter
            .ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", param_idx + i))
            .collect();
        where_clauses.push(format!("id IN ({})", placeholders.join(",")));
        for id in &filter.ids {
            param_values.push(Box::new(id.clone()));
        }
        param_idx += filter.ids.len();
    }

    // Labels AND.
    for label in &filter.labels {
        where_clauses.push(format!(
            "EXISTS (SELECT 1 FROM labels WHERE labels.issue_id = issues.id AND labels.label = ?{param_idx})"
        ));
        param_values.push(Box::new(label.clone()));
        param_idx += 1;
    }

    // Labels OR.
    if !filter.labels_any.is_empty() {
        let placeholders: Vec<String> = filter
            .labels_any
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", param_idx + i))
            .collect();
        where_clauses.push(format!(
            "EXISTS (SELECT 1 FROM labels WHERE labels.issue_id = issues.id AND labels.label IN ({}))",
            placeholders.join(",")
        ));
        for label in &filter.labels_any {
            param_values.push(Box::new(label.clone()));
        }
        param_idx += filter.labels_any.len();
    }

    // No labels.
    if filter.no_labels {
        where_clauses.push(
            "NOT EXISTS (SELECT 1 FROM labels WHERE labels.issue_id = issues.id)".to_string(),
        );
    }

    // Build final SQL.
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let limit_sql = filter
        .limit
        .map(|l| format!(" LIMIT {l}"))
        .unwrap_or_default();

    let sql = format!(
        "SELECT {ISSUE_COLUMNS} FROM issues {where_sql} ORDER BY created_at DESC{limit_sql}"
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), scan_issue)?;

    let mut issues = Vec::new();
    for row in rows {
        issues.push(row?);
    }

    // Suppress the "unused" warning for param_idx.
    let _ = param_idx;

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::store::SqliteStore;
    use beads_core::issue::IssueBuilder;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn create_and_get_issue() {
        let store = test_store();
        let issue = IssueBuilder::new("Test issue")
            .id("bd-test1")
            .description("A test description")
            .priority(2)
            .build();

        store.create_issue_impl(&issue, "alice").unwrap();

        let got = store.get_issue_impl("bd-test1").unwrap();
        assert_eq!(got.title, "Test issue");
        assert_eq!(got.description, "A test description");
        assert_eq!(got.priority, 2);
        assert!(!got.content_hash.is_empty());
    }

    #[test]
    fn get_nonexistent_issue_returns_not_found() {
        let store = test_store();
        let err = store.get_issue_impl("bd-nope").unwrap_err();
        assert!(err.is_not_found());
    }

    #[test]
    fn update_issue_partial() {
        let store = test_store();
        let issue = IssueBuilder::new("Original title").id("bd-upd1").build();
        store.create_issue_impl(&issue, "alice").unwrap();

        let updates = IssueUpdates {
            title: Some("Updated title".into()),
            priority: Some(3),
            ..Default::default()
        };
        store.update_issue_impl("bd-upd1", &updates, "bob").unwrap();

        let got = store.get_issue_impl("bd-upd1").unwrap();
        assert_eq!(got.title, "Updated title");
        assert_eq!(got.priority, 3);
    }

    #[test]
    fn close_issue() {
        let store = test_store();
        let issue = IssueBuilder::new("To close").id("bd-close1").build();
        store.create_issue_impl(&issue, "alice").unwrap();

        store
            .close_issue_impl("bd-close1", "completed", "alice", "session-1")
            .unwrap();

        let got = store.get_issue_impl("bd-close1").unwrap();
        assert_eq!(got.status, Status::Closed);
        assert!(got.closed_at.is_some());
        assert_eq!(got.close_reason, "completed");
    }

    #[test]
    fn delete_issue() {
        let store = test_store();
        let issue = IssueBuilder::new("To delete").id("bd-del1").build();
        store.create_issue_impl(&issue, "alice").unwrap();

        store.delete_issue_impl("bd-del1").unwrap();

        let err = store.get_issue_impl("bd-del1").unwrap_err();
        assert!(err.is_not_found());
    }

    #[test]
    fn search_issues_by_text() {
        let store = test_store();
        let issue1 = IssueBuilder::new("Fix login bug")
            .id("bd-s1")
            .description("Users cannot log in")
            .build();
        let issue2 = IssueBuilder::new("Add dashboard")
            .id("bd-s2")
            .description("New dashboard feature")
            .build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();

        let results = store
            .search_issues_impl("login", &IssueFilter::default())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "bd-s1");
    }

    #[test]
    fn search_issues_by_status_filter() {
        let store = test_store();
        let issue1 = IssueBuilder::new("Open issue")
            .id("bd-sf1")
            .status(Status::Open)
            .build();
        let issue2 = IssueBuilder::new("Closed issue")
            .id("bd-sf2")
            .status(Status::Closed)
            .closed_at(Utc::now())
            .build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();

        let filter = IssueFilter {
            status: Some(Status::Open),
            ..Default::default()
        };
        let results = store.search_issues_impl("", &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "bd-sf1");
    }
}
