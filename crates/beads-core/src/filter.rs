//! Filter types for querying issues.

use chrono::{DateTime, Utc};

use crate::enums::{IssueType, MolType, SortPolicy, Status, WispType};

/// Filter for issue queries.
#[derive(Debug, Clone, Default)]
pub struct IssueFilter {
    pub status: Option<Status>,
    pub priority: Option<i32>,
    pub issue_type: Option<IssueType>,
    pub assignee: Option<String>,

    /// AND semantics: issue must have ALL these labels.
    pub labels: Vec<String>,
    /// OR semantics: issue must have AT LEAST ONE of these labels.
    pub labels_any: Vec<String>,
    /// Glob pattern for label matching (e.g., "tech-*").
    pub label_pattern: Option<String>,
    /// Regex pattern for label matching (e.g., "tech-(debt|legacy)").
    pub label_regex: Option<String>,

    pub title_search: Option<String>,

    /// Filter by specific issue IDs.
    pub ids: Vec<String>,
    /// Filter by ID prefix (e.g., "bd-" to match "bd-abc123").
    pub id_prefix: Option<String>,
    /// Filter by spec_id prefix.
    pub spec_id_prefix: Option<String>,

    pub limit: Option<i32>,

    // Pattern matching
    pub title_contains: Option<String>,
    pub description_contains: Option<String>,
    pub notes_contains: Option<String>,

    // Date ranges
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    pub closed_after: Option<DateTime<Utc>>,
    pub closed_before: Option<DateTime<Utc>>,

    // Empty/null checks
    pub empty_description: bool,
    pub no_assignee: bool,
    pub no_labels: bool,

    // Numeric ranges
    pub priority_min: Option<i32>,
    pub priority_max: Option<i32>,

    /// Filter by source_repo field (None = any).
    pub source_repo: Option<String>,

    /// Filter by ephemeral flag (None = any).
    pub ephemeral: Option<bool>,

    /// Filter by pinned flag (None = any).
    pub pinned: Option<bool>,

    /// Filter by template flag (None = any).
    pub is_template: Option<bool>,

    /// Filter by parent issue (via parent-child dependency).
    pub parent_id: Option<String>,
    /// Exclude issues that are children of another issue.
    pub no_parent: bool,

    /// Filter by molecule type (None = any).
    pub mol_type: Option<MolType>,

    /// Filter by wisp type (None = any).
    pub wisp_type: Option<WispType>,

    /// Exclude issues with these statuses.
    pub exclude_status: Vec<Status>,

    /// Exclude issues with these types.
    pub exclude_types: Vec<IssueType>,

    // Time-based scheduling filters
    /// Filter issues with defer_until set (any value).
    pub deferred: bool,
    pub defer_after: Option<DateTime<Utc>>,
    pub defer_before: Option<DateTime<Utc>>,
    pub due_after: Option<DateTime<Utc>>,
    pub due_before: Option<DateTime<Utc>>,
    /// Filter issues where due_at < now AND status != closed.
    pub overdue: bool,
}

/// Filter for ready work queries.
#[derive(Debug, Clone, Default)]
pub struct WorkFilter {
    pub status: Option<Status>,
    /// Filter by issue type string.
    pub issue_type: Option<String>,
    pub priority: Option<i32>,
    pub assignee: Option<String>,
    /// Filter for issues with no assignee.
    pub unassigned: bool,

    /// AND semantics: issue must have ALL these labels.
    pub labels: Vec<String>,
    /// OR semantics: issue must have AT LEAST ONE of these labels.
    pub labels_any: Vec<String>,
    /// Glob pattern for label matching.
    pub label_pattern: Option<String>,
    /// Regex pattern for label matching.
    pub label_regex: Option<String>,

    pub limit: Option<i32>,
    pub sort_policy: SortPolicy,

    /// Filter to descendants of a bead/epic (recursive).
    pub parent_id: Option<String>,

    /// Filter by molecule type (None = any).
    pub mol_type: Option<MolType>,
    /// Filter by wisp type (None = any).
    pub wisp_type: Option<WispType>,

    /// If true, include issues with future defer_until timestamps.
    pub include_deferred: bool,
    /// If true, include ephemeral issues (wisps).
    pub include_ephemeral: bool,
    /// If true, include mol/wisp steps.
    pub include_mol_steps: bool,
}

/// Filter for stale issue queries.
#[derive(Debug, Clone)]
pub struct StaleFilter {
    /// Issues not updated in this many days.
    pub days: i32,
    /// Filter by status (open|in_progress|blocked), empty = all non-closed.
    pub status: Option<String>,
    /// Maximum issues to return.
    pub limit: Option<i32>,
}

impl Default for StaleFilter {
    fn default() -> Self {
        Self {
            days: 30,
            status: None,
            limit: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_filter_defaults() {
        let f = IssueFilter::default();
        assert!(f.status.is_none());
        assert!(f.priority.is_none());
        assert!(f.labels.is_empty());
        assert!(!f.overdue);
    }

    #[test]
    fn work_filter_defaults() {
        let f = WorkFilter::default();
        assert_eq!(f.sort_policy, SortPolicy::Hybrid);
        assert!(!f.unassigned);
        assert!(!f.include_deferred);
    }

    #[test]
    fn stale_filter_defaults() {
        let f = StaleFilter::default();
        assert_eq!(f.days, 30);
        assert!(f.status.is_none());
    }
}
