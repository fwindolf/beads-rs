//! Issue validation rules.

use crate::enums::Status;
use crate::issue::Issue;

/// Error type for validation failures.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("title is required")]
    TitleRequired,

    #[error("title must be 500 characters or less (got {0})")]
    TitleTooLong(usize),

    #[error("priority must be between 0 and 4 (got {0})")]
    InvalidPriority(i32),

    #[error("invalid status: {0}")]
    InvalidStatus(String),

    #[error("invalid issue type: {0}")]
    InvalidIssueType(String),

    #[error("estimated_minutes cannot be negative")]
    NegativeEstimate,

    #[error("closed issues must have closed_at timestamp")]
    ClosedWithoutTimestamp,

    #[error("non-closed issues cannot have closed_at timestamp")]
    NotClosedWithTimestamp,

    #[error("invalid agent state: {0}")]
    InvalidAgentState(String),

    #[error("metadata must be valid JSON")]
    InvalidMetadata,
}

/// Validates an issue using built-in rules only.
pub fn validate(issue: &Issue) -> Result<(), ValidationError> {
    validate_with_custom(issue, &[], &[])
}

/// Validates an issue, allowing custom statuses.
pub fn validate_with_custom_statuses(
    issue: &Issue,
    custom_statuses: &[&str],
) -> Result<(), ValidationError> {
    validate_with_custom(issue, custom_statuses, &[])
}

/// Validates an issue, allowing custom statuses and types.
pub fn validate_with_custom(
    issue: &Issue,
    custom_statuses: &[&str],
    custom_types: &[&str],
) -> Result<(), ValidationError> {
    // Title required.
    if issue.title.is_empty() {
        return Err(ValidationError::TitleRequired);
    }
    // Title max 500 chars.
    if issue.title.len() > 500 {
        return Err(ValidationError::TitleTooLong(issue.title.len()));
    }
    // Priority 0-4.
    if issue.priority < 0 || issue.priority > 4 {
        return Err(ValidationError::InvalidPriority(issue.priority));
    }
    // Status must be valid.
    if !issue.status.is_valid_with_custom(custom_statuses) {
        return Err(ValidationError::InvalidStatus(
            issue.status.as_str().to_owned(),
        ));
    }
    // IssueType must be valid.
    if !issue.issue_type.is_valid_with_custom(custom_types) {
        return Err(ValidationError::InvalidIssueType(
            issue.issue_type.as_str().to_owned(),
        ));
    }
    // Estimated minutes cannot be negative.
    if let Some(est) = issue.estimated_minutes {
        if est < 0 {
            return Err(ValidationError::NegativeEstimate);
        }
    }
    // Closed-at invariant.
    if issue.status == Status::Closed && issue.closed_at.is_none() {
        return Err(ValidationError::ClosedWithoutTimestamp);
    }
    if issue.status != Status::Closed && issue.closed_at.is_some() {
        return Err(ValidationError::NotClosedWithTimestamp);
    }
    // Agent state must be valid.
    if !issue.agent_state.is_valid() {
        return Err(ValidationError::InvalidAgentState(
            issue.agent_state.as_str().to_owned(),
        ));
    }
    // Metadata must be valid JSON if set.
    if let Some(ref meta) = issue.metadata {
        if serde_json::from_str::<serde_json::Value>(meta.get()).is_err() {
            return Err(ValidationError::InvalidMetadata);
        }
    }

    Ok(())
}

/// Validates an issue for multi-repo import (federation trust model).
///
/// Built-in types are validated (to catch typos). Non-built-in types are
/// trusted since the source repo already validated them.
pub fn validate_for_import(issue: &Issue, custom_statuses: &[&str]) -> Result<(), ValidationError> {
    if issue.title.is_empty() {
        return Err(ValidationError::TitleRequired);
    }
    if issue.title.len() > 500 {
        return Err(ValidationError::TitleTooLong(issue.title.len()));
    }
    if issue.priority < 0 || issue.priority > 4 {
        return Err(ValidationError::InvalidPriority(issue.priority));
    }
    if !issue.status.is_valid_with_custom(custom_statuses) {
        return Err(ValidationError::InvalidStatus(
            issue.status.as_str().to_owned(),
        ));
    }
    // Issue type: federation trust model. Custom types from source repos are trusted.
    if let Some(est) = issue.estimated_minutes {
        if est < 0 {
            return Err(ValidationError::NegativeEstimate);
        }
    }
    if issue.status == Status::Closed && issue.closed_at.is_none() {
        return Err(ValidationError::ClosedWithoutTimestamp);
    }
    if issue.status != Status::Closed && issue.closed_at.is_some() {
        return Err(ValidationError::NotClosedWithTimestamp);
    }
    if !issue.agent_state.is_valid() {
        return Err(ValidationError::InvalidAgentState(
            issue.agent_state.as_str().to_owned(),
        ));
    }
    if let Some(ref meta) = issue.metadata {
        if serde_json::from_str::<serde_json::Value>(meta.get()).is_err() {
            return Err(ValidationError::InvalidMetadata);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::{IssueType, Status};
    use crate::issue::IssueBuilder;

    #[test]
    fn valid_issue_passes() {
        let issue = IssueBuilder::new("Valid issue").priority(2).build();
        assert!(validate(&issue).is_ok());
    }

    #[test]
    fn empty_title_fails() {
        let issue = IssueBuilder::new("").build();
        match validate(&issue) {
            Err(ValidationError::TitleRequired) => {}
            other => panic!("expected TitleRequired, got {:?}", other),
        }
    }

    #[test]
    fn long_title_fails() {
        let title = "x".repeat(501);
        let issue = IssueBuilder::new(title).build();
        match validate(&issue) {
            Err(ValidationError::TitleTooLong(n)) => assert_eq!(n, 501),
            other => panic!("expected TitleTooLong, got {:?}", other),
        }
    }

    #[test]
    fn invalid_priority_fails() {
        let issue = IssueBuilder::new("Test").priority(5).build();
        match validate(&issue) {
            Err(ValidationError::InvalidPriority(5)) => {}
            other => panic!("expected InvalidPriority(5), got {:?}", other),
        }
    }

    #[test]
    fn negative_priority_fails() {
        let issue = IssueBuilder::new("Test").priority(-1).build();
        assert!(matches!(
            validate(&issue),
            Err(ValidationError::InvalidPriority(-1))
        ));
    }

    #[test]
    fn custom_status_rejected_without_config() {
        let issue = IssueBuilder::new("Test")
            .status(Status::Custom("my_status".into()))
            .build();
        assert!(matches!(
            validate(&issue),
            Err(ValidationError::InvalidStatus(_))
        ));
    }

    #[test]
    fn custom_status_accepted_with_config() {
        let issue = IssueBuilder::new("Test")
            .status(Status::Custom("my_status".into()))
            .build();
        assert!(validate_with_custom_statuses(&issue, &["my_status"]).is_ok());
    }

    #[test]
    fn custom_type_rejected_without_config() {
        let issue = IssueBuilder::new("Test")
            .issue_type(IssueType::Custom("my_type".into()))
            .build();
        assert!(matches!(
            validate(&issue),
            Err(ValidationError::InvalidIssueType(_))
        ));
    }

    #[test]
    fn custom_type_accepted_with_config() {
        let issue = IssueBuilder::new("Test")
            .issue_type(IssueType::Custom("my_type".into()))
            .build();
        assert!(validate_with_custom(&issue, &[], &["my_type"]).is_ok());
    }

    #[test]
    fn closed_without_timestamp_fails() {
        let issue = IssueBuilder::new("Test").status(Status::Closed).build();
        assert!(matches!(
            validate(&issue),
            Err(ValidationError::ClosedWithoutTimestamp)
        ));
    }

    #[test]
    fn closed_with_timestamp_passes() {
        let issue = IssueBuilder::new("Test")
            .status(Status::Closed)
            .closed_at(chrono::Utc::now())
            .build();
        assert!(validate(&issue).is_ok());
    }

    #[test]
    fn not_closed_with_timestamp_fails() {
        let issue = IssueBuilder::new("Test")
            .status(Status::Open)
            .closed_at(chrono::Utc::now())
            .build();
        assert!(matches!(
            validate(&issue),
            Err(ValidationError::NotClosedWithTimestamp)
        ));
    }

    #[test]
    fn negative_estimate_fails() {
        let mut issue = IssueBuilder::new("Test").build();
        issue.estimated_minutes = Some(-5);
        assert!(matches!(
            validate(&issue),
            Err(ValidationError::NegativeEstimate)
        ));
    }
}
