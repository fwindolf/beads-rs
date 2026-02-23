//! Storage and Transaction traits -- the public API for issue persistence.
//!
//! Consumers depend on these traits rather than on concrete implementations so
//! that alternative backends (mocks, proxies, etc.) can be substituted.

use chrono::{DateTime, Utc};

use beads_core::comment::{Comment, Event};
use beads_core::dependency::Dependency;
use beads_core::enums::{DependencyType, IssueType, Status};
use beads_core::filter::{IssueFilter, WorkFilter};
use beads_core::issue::Issue;

use crate::error::Result;

// ---------------------------------------------------------------------------
// View / helper types
// ---------------------------------------------------------------------------

/// Typed partial-update struct for issues.
///
/// Only `Some` fields are applied; `None` fields are left unchanged. This
/// avoids the untyped `map[string]interface{}` pattern from Go.
#[derive(Debug, Clone, Default)]
pub struct IssueUpdates {
    pub title: Option<String>,
    pub description: Option<String>,
    pub design: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub notes: Option<String>,
    pub status: Option<Status>,
    pub priority: Option<i32>,
    pub issue_type: Option<IssueType>,
    pub assignee: Option<String>,
    pub owner: Option<String>,
    pub estimated_minutes: Option<Option<i32>>,
    pub spec_id: Option<String>,
    pub external_ref: Option<Option<String>>,
    pub source_system: Option<String>,
    pub close_reason: Option<String>,
    pub closed_by_session: Option<String>,
    pub due_at: Option<Option<DateTime<Utc>>>,
    pub defer_until: Option<Option<DateTime<Utc>>>,
    pub pinned: Option<bool>,
    pub is_template: Option<bool>,
    pub ephemeral: Option<bool>,
    pub sender: Option<String>,
    pub metadata: Option<Option<String>>,
    pub crystallizes: Option<bool>,
    pub quality_score: Option<Option<f32>>,
    pub mol_type: Option<String>,
    pub work_type: Option<String>,
    pub wisp_type: Option<String>,
    pub await_type: Option<String>,
    pub await_id: Option<String>,
    pub timeout: Option<Option<std::time::Duration>>,
    pub waiters: Option<Vec<String>>,
    pub hook_bead: Option<String>,
    pub role_bead: Option<String>,
    pub agent_state: Option<String>,
    pub last_activity: Option<Option<DateTime<Utc>>>,
    pub role_type: Option<String>,
    pub rig: Option<String>,
    pub event_kind: Option<String>,
    pub actor: Option<String>,
    pub target: Option<String>,
    pub payload: Option<String>,
}

/// A node in a dependency tree traversal.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// The issue at this node.
    pub issue: Issue,
    /// Depth from the root (0 = root).
    pub depth: i32,
    /// The dependency type of the edge leading to this node.
    pub dep_type: DependencyType,
    /// Whether this node was reached via a reverse traversal.
    pub reverse: bool,
}

/// An issue with its associated dependency edge metadata.
#[derive(Debug, Clone)]
pub struct IssueWithDependencyMetadata {
    /// The related issue.
    pub issue: Issue,
    /// The dependency edge connecting the issue.
    pub dependency: Dependency,
}

/// An issue that is blocked, along with the count of open blockers.
#[derive(Debug, Clone)]
pub struct BlockedIssue {
    /// The blocked issue.
    pub issue: Issue,
    /// Number of open blocking dependencies.
    pub blocked_by_count: i32,
}

/// Status of an epic with respect to its children.
#[derive(Debug, Clone)]
pub struct EpicStatus {
    /// The epic issue.
    pub epic: Issue,
    /// Total number of child issues.
    pub total_children: i32,
    /// Number of closed child issues.
    pub closed_children: i32,
}

/// Aggregate statistics about the issue database.
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub total_issues: i64,
    pub open_issues: i64,
    pub closed_issues: i64,
    pub in_progress_issues: i64,
    pub blocked_issues: i64,
    pub deferred_issues: i64,

    /// Breakdown by issue type: `(type_name, count)`.
    pub by_type: Vec<(String, i64)>,
    /// Breakdown by priority: `(priority, count)`.
    pub by_priority: Vec<(i32, i64)>,
    /// Breakdown by assignee: `(assignee, count)`.
    pub by_assignee: Vec<(String, i64)>,
}

// ---------------------------------------------------------------------------
// Storage trait
// ---------------------------------------------------------------------------

/// Primary storage interface for issue persistence.
///
/// Mirrors the Go `Storage` interface. All methods return [`Result`] to
/// propagate [`StorageError`]s.
pub trait Storage: Send + Sync {
    // -- Issue CRUD ----------------------------------------------------------

    /// Creates a new issue and emits a "created" event.
    fn create_issue(&self, issue: &Issue, actor: &str) -> Result<()>;

    /// Creates multiple issues in a single batch.
    fn create_issues(&self, issues: &[Issue], actor: &str) -> Result<()>;

    /// Retrieves an issue by its ID.
    fn get_issue(&self, id: &str) -> Result<Issue>;

    /// Retrieves an issue by its external reference.
    fn get_issue_by_external_ref(&self, external_ref: &str) -> Result<Issue>;

    /// Retrieves multiple issues by their IDs.
    fn get_issues_by_ids(&self, ids: &[String]) -> Result<Vec<Issue>>;

    /// Applies partial updates to an issue and emits an "updated" event.
    fn update_issue(&self, id: &str, updates: &IssueUpdates, actor: &str) -> Result<()>;

    /// Closes an issue (sets status=closed, closed_at=now) and emits a
    /// "closed" event.
    fn close_issue(&self, id: &str, reason: &str, actor: &str, session: &str) -> Result<()>;

    /// Permanently deletes an issue and its related data.
    fn delete_issue(&self, id: &str) -> Result<()>;

    /// Searches issues by text query and optional filter.
    fn search_issues(&self, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>>;

    // -- Dependencies --------------------------------------------------------

    /// Adds a dependency edge between two issues.
    fn add_dependency(&self, dep: &Dependency, actor: &str) -> Result<()>;

    /// Removes a dependency edge.
    fn remove_dependency(&self, issue_id: &str, depends_on_id: &str, actor: &str) -> Result<()>;

    /// Returns the issues that the given issue depends on.
    fn get_dependencies(&self, issue_id: &str) -> Result<Vec<Issue>>;

    /// Returns the issues that depend on the given issue.
    fn get_dependents(&self, issue_id: &str) -> Result<Vec<Issue>>;

    /// Returns dependencies with their edge metadata.
    fn get_dependencies_with_metadata(
        &self,
        issue_id: &str,
    ) -> Result<Vec<IssueWithDependencyMetadata>>;

    /// Returns dependents with their edge metadata.
    fn get_dependents_with_metadata(
        &self,
        issue_id: &str,
    ) -> Result<Vec<IssueWithDependencyMetadata>>;

    /// Traverses the dependency tree from the given root.
    fn get_dependency_tree(
        &self,
        issue_id: &str,
        max_depth: i32,
        show_all_paths: bool,
        reverse: bool,
    ) -> Result<Vec<TreeNode>>;

    // -- Labels --------------------------------------------------------------

    /// Adds a label to an issue.
    fn add_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()>;

    /// Removes a label from an issue.
    fn remove_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()>;

    /// Returns all labels for an issue.
    fn get_labels(&self, issue_id: &str) -> Result<Vec<String>>;

    /// Returns all issues with the given label.
    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>>;

    // -- Work queries --------------------------------------------------------

    /// Returns issues that are ready to work on (open, not blocked, not
    /// deferred, not template).
    fn get_ready_work(&self, filter: &WorkFilter) -> Result<Vec<Issue>>;

    /// Returns issues that have at least one open blocking dependency.
    fn get_blocked_issues(&self, filter: &WorkFilter) -> Result<Vec<BlockedIssue>>;

    /// Returns epics where all children are closed.
    fn get_epics_eligible_for_closure(&self) -> Result<Vec<EpicStatus>>;

    // -- Comments and events -------------------------------------------------

    /// Adds a comment to an issue and returns the created comment.
    fn add_comment(&self, issue_id: &str, author: &str, text: &str) -> Result<Comment>;

    /// Returns all comments for an issue.
    fn get_comments(&self, issue_id: &str) -> Result<Vec<Comment>>;

    /// Returns recent events for an issue.
    fn get_events(&self, issue_id: &str, limit: i32) -> Result<Vec<Event>>;

    /// Returns all events with id > `since_id`.
    fn get_all_events_since(&self, since_id: i64) -> Result<Vec<Event>>;

    // -- Statistics -----------------------------------------------------------

    /// Returns aggregate statistics about the issue database.
    fn get_statistics(&self) -> Result<Statistics>;

    // -- Configuration -------------------------------------------------------

    /// Sets a configuration key-value pair.
    fn set_config(&self, key: &str, value: &str) -> Result<()>;

    /// Gets a configuration value by key.
    fn get_config(&self, key: &str) -> Result<String>;

    /// Returns all configuration key-value pairs.
    fn get_all_config(&self) -> Result<std::collections::HashMap<String, String>>;

    // -- Transactions --------------------------------------------------------

    /// Executes a closure within a database transaction.
    ///
    /// If the closure returns `Ok`, the transaction is committed.
    /// If it returns `Err` or panics, the transaction is rolled back.
    fn run_in_transaction(&self, f: &dyn Fn(&dyn Transaction) -> Result<()>) -> Result<()>;

    // -- Lifecycle -----------------------------------------------------------

    /// Closes the database connection and releases resources.
    fn close(&self) -> Result<()>;
}

// ---------------------------------------------------------------------------
// Transaction trait
// ---------------------------------------------------------------------------

/// Subset of [`Storage`] methods available inside a transaction.
///
/// All operations share a single database connection and are committed or
/// rolled back atomically.
pub trait Transaction {
    // -- Issue operations ----------------------------------------------------

    fn create_issue(&self, issue: &Issue, actor: &str) -> Result<()>;
    fn create_issues(&self, issues: &[Issue], actor: &str) -> Result<()>;
    fn update_issue(&self, id: &str, updates: &IssueUpdates, actor: &str) -> Result<()>;
    fn close_issue(&self, id: &str, reason: &str, actor: &str, session: &str) -> Result<()>;
    fn delete_issue(&self, id: &str) -> Result<()>;
    fn get_issue(&self, id: &str) -> Result<Issue>;
    fn search_issues(&self, query: &str, filter: &IssueFilter) -> Result<Vec<Issue>>;

    // -- Dependency operations -----------------------------------------------

    fn add_dependency(&self, dep: &Dependency, actor: &str) -> Result<()>;
    fn remove_dependency(&self, issue_id: &str, depends_on_id: &str, actor: &str) -> Result<()>;
    fn get_dependency_records(&self, issue_id: &str) -> Result<Vec<Dependency>>;

    // -- Label operations ----------------------------------------------------

    fn add_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()>;
    fn remove_label(&self, issue_id: &str, label: &str, actor: &str) -> Result<()>;
    fn get_labels(&self, issue_id: &str) -> Result<Vec<String>>;

    // -- Config operations ---------------------------------------------------

    fn set_config(&self, key: &str, value: &str) -> Result<()>;
    fn get_config(&self, key: &str) -> Result<String>;

    // -- Metadata operations -------------------------------------------------

    fn set_metadata(&self, key: &str, value: &str) -> Result<()>;
    fn get_metadata(&self, key: &str) -> Result<String>;

    // -- Comment operations --------------------------------------------------

    fn add_comment(&self, issue_id: &str, actor: &str, comment: &str) -> Result<()>;
    fn import_comment(
        &self,
        issue_id: &str,
        author: &str,
        text: &str,
        created_at: DateTime<Utc>,
    ) -> Result<Comment>;
    fn get_comments(&self, issue_id: &str) -> Result<Vec<Comment>>;
}
