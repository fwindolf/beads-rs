//! Storage backend for the beads system.
//!
//! Provides the [`Storage`] trait and a SQLite implementation ([`SqliteStore`]).

pub mod error;
pub mod sqlite;
pub mod traits;

// Re-exports for convenience.
pub use error::StorageError;
pub use sqlite::SqliteStore;
pub use traits::{
    BlockedIssue, EpicStatus, IssueUpdates, IssueWithDependencyMetadata, Statistics, Storage,
    Transaction, TreeNode,
};

// ---------------------------------------------------------------------------
// Storage trait implementation for SqliteStore
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use beads_core::comment::{Comment, Event};
use beads_core::dependency::Dependency;
use beads_core::filter::{IssueFilter, WorkFilter};
use beads_core::issue::Issue;

use crate::error::Result;

impl Storage for SqliteStore {
    fn create_issue(&self, issue: &Issue, actor: &str) -> Result<()> {
        self.create_issue_impl(issue, actor)
    }

    fn create_issues(&self, issues: &[Issue], actor: &str) -> Result<()> {
        self.create_issues_impl(issues, actor)
    }

    fn get_issue(&self, id: &str) -> Result<Issue> {
        self.get_issue_impl(id)
    }

    fn get_issue_by_external_ref(&self, external_ref: &str) -> Result<Issue> {
        self.get_issue_by_external_ref_impl(external_ref)
    }

    fn get_issues_by_ids(&self, ids: &[String]) -> Result<Vec<Issue>> {
        self.get_issues_by_ids_impl(ids)
    }

    fn update_issue(&self, id: &str, updates: &IssueUpdates, actor: &str) -> Result<()> {
        self.update_issue_impl(id, updates, actor)
    }

    fn close_issue(&self, id: &str, reason: &str, actor: &str, session: &str) -> Result<()> {
        self.close_issue_impl(id, reason, actor, session)
    }

    fn delete_issue(&self, id: &str) -> Result<()> {
        self.delete_issue_impl(id)
    }

    fn search_issues(&self, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>> {
        self.search_issues_impl(query, filter)
    }

    fn add_dependency(&self, dep: &Dependency, actor: &str) -> Result<()> {
        self.add_dependency_impl(dep, actor)
    }

    fn remove_dependency(&self, issue_id: &str, depends_on_id: &str, actor: &str) -> Result<()> {
        self.remove_dependency_impl(issue_id, depends_on_id, actor)
    }

    fn get_dependencies(&self, issue_id: &str) -> Result<Vec<Issue>> {
        self.get_dependencies_impl(issue_id)
    }

    fn get_dependents(&self, issue_id: &str) -> Result<Vec<Issue>> {
        self.get_dependents_impl(issue_id)
    }

    fn get_dependencies_with_metadata(
        &self,
        issue_id: &str,
    ) -> Result<Vec<IssueWithDependencyMetadata>> {
        self.get_dependencies_with_metadata_impl(issue_id)
    }

    fn get_dependents_with_metadata(
        &self,
        issue_id: &str,
    ) -> Result<Vec<IssueWithDependencyMetadata>> {
        self.get_dependents_with_metadata_impl(issue_id)
    }

    fn get_dependency_tree(
        &self,
        issue_id: &str,
        max_depth: i32,
        show_all_paths: bool,
        reverse: bool,
    ) -> Result<Vec<TreeNode>> {
        self.get_dependency_tree_impl(issue_id, max_depth, show_all_paths, reverse)
    }

    fn add_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()> {
        self.add_label_impl(issue_id, label, actor)
    }

    fn remove_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()> {
        self.remove_label_impl(issue_id, label, actor)
    }

    fn get_labels(&self, issue_id: &str) -> Result<Vec<String>> {
        self.get_labels_impl(issue_id)
    }

    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>> {
        self.get_issues_by_label_impl(label)
    }

    fn get_ready_work(&self, filter: &WorkFilter) -> Result<Vec<Issue>> {
        self.get_ready_work_impl(filter)
    }

    fn get_blocked_issues(&self, filter: &WorkFilter) -> Result<Vec<BlockedIssue>> {
        self.get_blocked_issues_impl(filter)
    }

    fn get_epics_eligible_for_closure(&self) -> Result<Vec<EpicStatus>> {
        self.get_epics_eligible_for_closure_impl()
    }

    fn add_comment(&self, issue_id: &str, author: &str, text: &str) -> Result<Comment> {
        self.add_comment_impl(issue_id, author, text)
    }

    fn get_comments(&self, issue_id: &str) -> Result<Vec<Comment>> {
        self.get_comments_impl(issue_id)
    }

    fn get_events(&self, issue_id: &str, limit: i32) -> Result<Vec<Event>> {
        self.get_events_impl(issue_id, limit)
    }

    fn get_all_events_since(&self, since_id: i64) -> Result<Vec<Event>> {
        self.get_all_events_since_impl(since_id)
    }

    fn get_statistics(&self) -> Result<Statistics> {
        self.get_statistics_impl()
    }

    fn set_config(&self, key: &str, value: &str) -> Result<()> {
        self.set_config_impl(key, value)
    }

    fn get_config(&self, key: &str) -> Result<String> {
        self.get_config_impl(key)
    }

    fn get_all_config(&self) -> Result<HashMap<String, String>> {
        self.get_all_config_impl()
    }

    fn run_in_transaction(&self, f: &dyn Fn(&dyn Transaction) -> Result<()>) -> Result<()> {
        self.run_in_transaction_impl(f)
    }

    fn close(&self) -> Result<()> {
        // SQLite connections are closed when the Connection is dropped.
        // The Mutex wrapper ensures thread safety.
        Ok(())
    }
}
