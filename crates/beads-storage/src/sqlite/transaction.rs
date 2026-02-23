//! Transaction wrapper for [`SqliteStore`].

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use beads_core::comment::Comment;
use beads_core::dependency::Dependency;
use beads_core::filter::IssueFilter;
use beads_core::issue::Issue;

use crate::error::{Result, StorageError};
use crate::sqlite::comments;
use crate::sqlite::config;
use crate::sqlite::dependencies;
use crate::sqlite::issues;
use crate::sqlite::labels;
use crate::sqlite::store::SqliteStore;
use crate::traits::{IssueUpdates, Transaction};

/// A thin wrapper around a SQLite connection that is inside a transaction.
///
/// The [`SqliteTx`] holds a reference to the connection (which already has an
/// active transaction via `BEGIN`). It implements [`Transaction`] by delegating
/// to the same connection-level helpers used by [`SqliteStore`].
pub(crate) struct SqliteTx<'a> {
    pub(crate) conn: &'a Connection,
}

impl Transaction for SqliteTx<'_> {
    fn create_issue(&self, issue: &Issue, actor: &str) -> Result<()> {
        issues::insert_issue(self.conn, issue, actor)
    }

    fn create_issues(&self, issue_list: &[Issue], actor: &str) -> Result<()> {
        for issue in issue_list {
            issues::insert_issue(self.conn, issue, actor)?;
        }
        Ok(())
    }

    fn update_issue(&self, id: &str, updates: &IssueUpdates, actor: &str) -> Result<()> {
        issues::update_issue_on_conn(self.conn, id, updates, actor)
    }

    fn close_issue(&self, id: &str, reason: &str, actor: &str, session: &str) -> Result<()> {
        issues::close_issue_on_conn(self.conn, id, reason, actor, session)
    }

    fn delete_issue(&self, id: &str) -> Result<()> {
        issues::delete_issue_on_conn(self.conn, id)
    }

    fn get_issue(&self, id: &str) -> Result<Issue> {
        issues::get_issue_on_conn(self.conn, id)
    }

    fn search_issues(&self, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>> {
        issues::search_issues_on_conn(self.conn, query, filter)
    }

    fn add_dependency(&self, dep: &Dependency, actor: &str) -> Result<()> {
        dependencies::add_dependency_on_conn(self.conn, dep, actor)
    }

    fn remove_dependency(&self, issue_id: &str, depends_on_id: &str, actor: &str) -> Result<()> {
        dependencies::remove_dependency_on_conn(self.conn, issue_id, depends_on_id, actor)
    }

    fn get_dependency_records(&self, issue_id: &str) -> Result<Vec<Dependency>> {
        dependencies::get_dependency_records_on_conn(self.conn, issue_id)
    }

    fn add_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()> {
        labels::add_label_on_conn(self.conn, issue_id, label, actor)
    }

    fn remove_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()> {
        labels::remove_label_on_conn(self.conn, issue_id, label, actor)
    }

    fn get_labels(&self, issue_id: &str) -> Result<Vec<String>> {
        labels::get_labels_on_conn(self.conn, issue_id)
    }

    fn set_config(&self, key: &str, value: &str) -> Result<()> {
        config::set_config_on_conn(self.conn, key, value)
    }

    fn get_config(&self, key: &str) -> Result<String> {
        config::get_config_on_conn(self.conn, key)
    }

    fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        config::set_metadata_on_conn(self.conn, key, value)
    }

    fn get_metadata(&self, key: &str) -> Result<String> {
        config::get_metadata_on_conn(self.conn, key)
    }

    fn add_comment(&self, issue_id: &str, actor: &str, comment: &str) -> Result<()> {
        comments::add_comment_no_event(self.conn, issue_id, actor, comment)
    }

    fn import_comment(
        &self,
        issue_id: &str,
        author: &str,
        text: &str,
        created_at: DateTime<Utc>,
    ) -> Result<Comment> {
        comments::import_comment_on_conn(self.conn, issue_id, author, text, created_at)
    }

    fn get_comments(&self, issue_id: &str) -> Result<Vec<Comment>> {
        comments::get_comments_on_conn(self.conn, issue_id)
    }
}

// ---------------------------------------------------------------------------
// SqliteStore::run_in_transaction
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Runs a closure inside a database transaction.
    pub fn run_in_transaction_impl(
        &self,
        f: &dyn Fn(&dyn Transaction) -> Result<()>,
    ) -> Result<()> {
        let conn = self.lock_conn()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| StorageError::Transaction(format!("failed to begin: {e}")))?;

        let sqlite_tx = SqliteTx { conn: &tx };
        match f(&sqlite_tx) {
            Ok(()) => {
                tx.commit()
                    .map_err(|e| StorageError::Transaction(format!("failed to commit: {e}")))?;
                Ok(())
            }
            Err(e) => {
                // Transaction is rolled back on drop.
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::dependency::Dependency;
    use beads_core::enums::DependencyType;
    use beads_core::issue::IssueBuilder;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn transaction_commit() {
        let store = test_store();

        store
            .run_in_transaction_impl(&|tx| {
                let issue = IssueBuilder::new("In transaction").id("bd-tx1").build();
                tx.create_issue(&issue, "alice")?;
                tx.add_label("bd-tx1", "transacted", "alice")?;
                Ok(())
            })
            .unwrap();

        // Verify committed.
        let issue = store.get_issue_impl("bd-tx1").unwrap();
        assert_eq!(issue.title, "In transaction");
        let labels = store.get_labels_impl("bd-tx1").unwrap();
        assert_eq!(labels, vec!["transacted"]);
    }

    #[test]
    fn transaction_rollback_on_error() {
        let store = test_store();

        let result = store.run_in_transaction_impl(&|tx| {
            let issue = IssueBuilder::new("Should rollback").id("bd-tx2").build();
            tx.create_issue(&issue, "alice")?;
            // Force an error.
            Err(StorageError::Internal("test rollback".into()))
        });

        assert!(result.is_err());

        // Issue should NOT exist.
        let err = store.get_issue_impl("bd-tx2").unwrap_err();
        assert!(err.is_not_found());
    }

    #[test]
    fn transaction_with_dependencies() {
        let store = test_store();

        store
            .run_in_transaction_impl(&|tx| {
                let parent = IssueBuilder::new("Parent").id("bd-txp1").build();
                let child = IssueBuilder::new("Child").id("bd-txc1").build();
                tx.create_issue(&parent, "alice")?;
                tx.create_issue(&child, "alice")?;

                let dep = Dependency {
                    issue_id: "bd-txc1".into(),
                    depends_on_id: "bd-txp1".into(),
                    dep_type: DependencyType::ParentChild,
                    created_at: chrono::Utc::now(),
                    created_by: "alice".into(),
                    metadata: String::new(),
                    thread_id: String::new(),
                };
                tx.add_dependency(&dep, "alice")?;
                Ok(())
            })
            .unwrap();

        let deps = store.get_dependencies_impl("bd-txc1").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].id, "bd-txp1");
    }
}
