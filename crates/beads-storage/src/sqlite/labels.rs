//! Label CRUD operations for [`SqliteStore`].

use chrono::Utc;
use rusqlite::{Connection, params};

use beads_core::enums::EventType;
use beads_core::issue::Issue;

use crate::error::{Result, StorageError};
use crate::sqlite::issues::{ISSUE_COLUMNS_PREFIXED, emit_event, format_datetime, scan_issue};
use crate::sqlite::store::SqliteStore;

// ---------------------------------------------------------------------------
// Connection-level helpers (shared with Transaction)
// ---------------------------------------------------------------------------

pub(crate) fn add_label_on_conn(
    conn: &Connection,
    issue_id: &str,
    label: &str,
    actor: &str,
) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);

    conn.execute(
        "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
        params![issue_id, label],
    )?;

    emit_event(
        conn,
        issue_id,
        EventType::LabelAdded,
        actor,
        None,
        Some(label),
        None,
        &now_str,
    )?;

    Ok(())
}

pub(crate) fn remove_label_on_conn(
    conn: &Connection,
    issue_id: &str,
    label: &str,
    actor: &str,
) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);

    let affected = conn.execute(
        "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
        params![issue_id, label],
    )?;

    if affected == 0 {
        return Err(StorageError::not_found(
            "label",
            format!("{issue_id}:{label}"),
        ));
    }

    emit_event(
        conn,
        issue_id,
        EventType::LabelRemoved,
        actor,
        Some(label),
        None,
        None,
        &now_str,
    )?;

    Ok(())
}

pub(crate) fn get_labels_on_conn(conn: &Connection, issue_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT label FROM labels WHERE issue_id = ?1 ORDER BY label")?;
    let rows = stmt.query_map(params![issue_id], |row| row.get::<_, String>(0))?;
    let mut labels = Vec::new();
    for row in rows {
        labels.push(row?);
    }
    Ok(labels)
}

// ---------------------------------------------------------------------------
// SqliteStore methods
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Adds a label to an issue.
    pub fn add_label_impl(&self, issue_id: &str, label: &str, actor: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        add_label_on_conn(&conn, issue_id, label, actor)
    }

    /// Removes a label from an issue.
    pub fn remove_label_impl(&self, issue_id: &str, label: &str, actor: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        remove_label_on_conn(&conn, issue_id, label, actor)
    }

    /// Returns all labels for an issue.
    pub fn get_labels_impl(&self, issue_id: &str) -> Result<Vec<String>> {
        let conn = self.lock_conn()?;
        get_labels_on_conn(&conn, issue_id)
    }

    /// Returns all issues with the given label.
    pub fn get_issues_by_label_impl(&self, label: &str) -> Result<Vec<Issue>> {
        let conn = self.lock_conn()?;
        let sql = format!(
            "SELECT {ISSUE_COLUMNS_PREFIXED} FROM issues
             INNER JOIN labels ON issues.id = labels.issue_id
             WHERE labels.label = ?1
             ORDER BY issues.created_at DESC"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![label], scan_issue)?;
        let mut issues = Vec::new();
        for row in rows {
            issues.push(row?);
        }
        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::issue::IssueBuilder;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn add_and_get_labels() {
        let store = test_store();
        let issue = IssueBuilder::new("Labeled issue").id("bd-lbl1").build();
        store.create_issue_impl(&issue, "alice").unwrap();

        store.add_label_impl("bd-lbl1", "bug", "alice").unwrap();
        store
            .add_label_impl("bd-lbl1", "critical", "alice")
            .unwrap();

        let labels = store.get_labels_impl("bd-lbl1").unwrap();
        assert_eq!(labels, vec!["bug", "critical"]);
    }

    #[test]
    fn remove_label() {
        let store = test_store();
        let issue = IssueBuilder::new("Issue").id("bd-lbl2").build();
        store.create_issue_impl(&issue, "alice").unwrap();
        store
            .add_label_impl("bd-lbl2", "tech-debt", "alice")
            .unwrap();
        store
            .remove_label_impl("bd-lbl2", "tech-debt", "alice")
            .unwrap();

        let labels = store.get_labels_impl("bd-lbl2").unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn get_issues_by_label() {
        let store = test_store();
        let issue1 = IssueBuilder::new("A").id("bd-lbl3").build();
        let issue2 = IssueBuilder::new("B").id("bd-lbl4").build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();

        store.add_label_impl("bd-lbl3", "p0", "alice").unwrap();
        store.add_label_impl("bd-lbl4", "p0", "alice").unwrap();
        store.add_label_impl("bd-lbl4", "urgent", "alice").unwrap();

        let issues = store.get_issues_by_label_impl("p0").unwrap();
        assert_eq!(issues.len(), 2);
    }
}
