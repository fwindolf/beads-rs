//! Comment and Event CRUD operations for [`SqliteStore`].

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use beads_core::comment::{Comment, Event};
use beads_core::enums::EventType;

use crate::error::Result;
use crate::sqlite::issues::{emit_event, format_datetime, parse_datetime};
use crate::sqlite::store::SqliteStore;

// ---------------------------------------------------------------------------
// Connection-level helpers (shared with Transaction)
// ---------------------------------------------------------------------------

/// Adds a comment on the given connection, returning the created comment.
pub(crate) fn add_comment_on_conn(
    conn: &Connection,
    issue_id: &str,
    author: &str,
    text: &str,
) -> Result<Comment> {
    let now = Utc::now();
    let now_str = format_datetime(&now);

    conn.execute(
        "INSERT INTO comments (issue_id, author, text, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![issue_id, author, text, now_str],
    )?;

    let id = conn.last_insert_rowid();

    // Emit a "commented" event.
    emit_event(
        conn,
        issue_id,
        EventType::Commented,
        author,
        None,
        None,
        Some(text),
        &now_str,
    )?;

    Ok(Comment {
        id,
        issue_id: issue_id.to_string(),
        author: author.to_string(),
        text: text.to_string(),
        created_at: now,
    })
}

/// Adds a comment with an explicit event-only record (no "commented" event
/// emitted -- used inside transactions that manage events separately).
pub(crate) fn add_comment_no_event(
    conn: &Connection,
    issue_id: &str,
    author: &str,
    text: &str,
) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);
    conn.execute(
        "INSERT INTO comments (issue_id, author, text, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![issue_id, author, text, now_str],
    )?;
    Ok(())
}

/// Imports a comment with a specific created_at timestamp (for import/migration).
pub(crate) fn import_comment_on_conn(
    conn: &Connection,
    issue_id: &str,
    author: &str,
    text: &str,
    created_at: DateTime<Utc>,
) -> Result<Comment> {
    let created_at_str = format_datetime(&created_at);

    conn.execute(
        "INSERT INTO comments (issue_id, author, text, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![issue_id, author, text, created_at_str],
    )?;

    let id = conn.last_insert_rowid();

    Ok(Comment {
        id,
        issue_id: issue_id.to_string(),
        author: author.to_string(),
        text: text.to_string(),
        created_at,
    })
}

/// Returns all comments for an issue on the given connection.
pub(crate) fn get_comments_on_conn(conn: &Connection, issue_id: &str) -> Result<Vec<Comment>> {
    let mut stmt = conn.prepare(
        "SELECT id, issue_id, author, text, created_at
         FROM comments WHERE issue_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![issue_id], |row: &rusqlite::Row<'_>| {
        let created_at_str: String = row.get(4)?;
        Ok(Comment {
            id: row.get(0)?,
            issue_id: row.get(1)?,
            author: row.get(2)?,
            text: row.get(3)?,
            created_at: parse_datetime(&created_at_str),
        })
    })?;
    let mut comments = Vec::new();
    for row in rows {
        comments.push(row?);
    }
    Ok(comments)
}

// ---------------------------------------------------------------------------
// SqliteStore methods
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Adds a comment and returns it.
    pub fn add_comment_impl(&self, issue_id: &str, author: &str, text: &str) -> Result<Comment> {
        let conn = self.lock_conn()?;
        add_comment_on_conn(&conn, issue_id, author, text)
    }

    /// Returns all comments for an issue.
    pub fn get_comments_impl(&self, issue_id: &str) -> Result<Vec<Comment>> {
        let conn = self.lock_conn()?;
        get_comments_on_conn(&conn, issue_id)
    }

    /// Returns recent events for an issue.
    pub fn get_events_impl(&self, issue_id: &str, limit: i32) -> Result<Vec<Event>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, issue_id, event_type, actor, old_value, new_value, comment, created_at
             FROM events WHERE issue_id = ?1
             ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![issue_id, limit], scan_event)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Returns all events with id greater than `since_id`.
    pub fn get_all_events_since_impl(&self, since_id: i64) -> Result<Vec<Event>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, issue_id, event_type, actor, old_value, new_value, comment, created_at
             FROM events WHERE id > ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![since_id], scan_event)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }
}

/// Scans a row from the events table into an [`Event`].
fn scan_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    let created_at_str: String = row.get(7)?;
    let event_type_str: String = row.get(2)?;
    Ok(Event {
        id: row.get(0)?,
        issue_id: row.get(1)?,
        event_type: EventType::from(event_type_str),
        actor: row.get(3)?,
        old_value: row.get(4)?,
        new_value: row.get(5)?,
        comment: row.get(6)?,
        created_at: parse_datetime(&created_at_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::issue::IssueBuilder;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn add_and_get_comment() {
        let store = test_store();
        let issue = IssueBuilder::new("Issue").id("bd-cmt1").build();
        store.create_issue_impl(&issue, "alice").unwrap();

        let comment = store
            .add_comment_impl("bd-cmt1", "alice", "Looks good")
            .unwrap();
        assert_eq!(comment.author, "alice");
        assert_eq!(comment.text, "Looks good");
        assert!(comment.id > 0);

        let comments = store.get_comments_impl("bd-cmt1").unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "Looks good");
    }

    #[test]
    fn get_events() {
        let store = test_store();
        let issue = IssueBuilder::new("Issue").id("bd-evt1").build();
        store.create_issue_impl(&issue, "alice").unwrap();
        store
            .add_comment_impl("bd-evt1", "bob", "A comment")
            .unwrap();

        let events = store.get_events_impl("bd-evt1", 10).unwrap();
        // At minimum: created + commented.
        assert!(events.len() >= 2);
    }

    #[test]
    fn get_all_events_since() {
        let store = test_store();
        let issue = IssueBuilder::new("Issue").id("bd-evt2").build();
        store.create_issue_impl(&issue, "alice").unwrap();

        let events = store.get_all_events_since_impl(0).unwrap();
        assert!(!events.is_empty());

        let last_id = events.last().unwrap().id;
        let empty = store.get_all_events_since_impl(last_id).unwrap();
        assert!(empty.is_empty());
    }
}
