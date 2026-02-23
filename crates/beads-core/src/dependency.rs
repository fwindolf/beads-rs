//! Dependency types -- relationships between issues.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::enums::DependencyType;

/// Represents a relationship between issues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub issue_id: String,

    pub depends_on_id: String,

    /// Dependency type (serialised as "type" in JSON).
    #[serde(rename = "type")]
    pub dep_type: DependencyType,

    pub created_at: DateTime<Utc>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_by: String,

    /// Type-specific edge data (JSON blob).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub metadata: String,

    /// Groups conversation edges for efficient thread queries.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub thread_id: String,
}

/// Counts for dependencies and dependents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyCounts {
    /// Number of issues this issue depends on.
    pub dependency_count: i32,
    /// Number of issues that depend on this issue.
    pub dependent_count: i32,
}

/// Metadata for waits-for dependencies (fanout gates).
///
/// Stored as JSON in the `Dependency.metadata` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitsForMeta {
    /// Gate type: "all-children" or "any-children".
    pub gate: String,

    /// Which step/issue spawns the children to wait for.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub spawner_id: String,
}

/// Gate constants for waits-for dependencies.
pub mod waits_for_gate {
    /// Wait for all dynamic children to complete.
    pub const ALL_CHILDREN: &str = "all-children";
    /// Proceed when first child completes (future).
    pub const ANY_CHILDREN: &str = "any-children";
}

/// Metadata for attests dependencies (skill attestations).
///
/// Stored as JSON in the `Dependency.metadata` field.
/// Enables: Entity X attests that Entity Y has skill Z at level N.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestsMeta {
    /// Skill being attested (e.g., "go", "rust", "code-review").
    pub skill: String,

    /// Proficiency level (e.g., "beginner", "intermediate", "expert").
    pub level: String,

    /// When the attestation was made (RFC3339 format).
    pub date: String,

    /// Optional reference to supporting evidence.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub evidence: String,

    /// Optional free-form notes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// Keywords that indicate an issue was closed due to failure.
///
/// Used by conditional-blocks dependencies to determine if the condition is met.
pub const FAILURE_CLOSE_KEYWORDS: &[&str] = &[
    "failed",
    "rejected",
    "wontfix",
    "won't fix",
    "canceled",
    "cancelled",
    "abandoned",
    "blocked",
    "error",
    "timeout",
    "aborted",
];

/// Returns `true` if the close reason indicates the issue failed.
///
/// Used by conditional-blocks dependencies: B runs only if A fails.
pub fn is_failure_close(close_reason: &str) -> bool {
    if close_reason.is_empty() {
        return false;
    }
    let lower = close_reason.to_lowercase();
    FAILURE_CLOSE_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_serde_roundtrip() {
        let dep = Dependency {
            issue_id: "bd-abc".into(),
            depends_on_id: "bd-def".into(),
            dep_type: DependencyType::Blocks,
            created_at: Utc::now(),
            created_by: "alice".into(),
            metadata: String::new(),
            thread_id: String::new(),
        };

        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains(r#""type":"blocks""#));

        let back: Dependency = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dep_type, DependencyType::Blocks);
        assert_eq!(back.issue_id, "bd-abc");
    }

    #[test]
    fn failure_close_detection() {
        assert!(is_failure_close("Build failed"));
        assert!(is_failure_close("wontfix"));
        assert!(is_failure_close("REJECTED by reviewer"));
        assert!(is_failure_close("Cancelled by user"));
        assert!(!is_failure_close(""));
        assert!(!is_failure_close("Completed successfully"));
    }

    #[test]
    fn waits_for_meta_serde() {
        let m = WaitsForMeta {
            gate: waits_for_gate::ALL_CHILDREN.into(),
            spawner_id: String::new(),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("all-children"));
    }

    #[test]
    fn attests_meta_serde() {
        let m = AttestsMeta {
            skill: "rust".into(),
            level: "expert".into(),
            date: "2024-01-15T00:00:00Z".into(),
            evidence: "bd-xyz".into(),
            notes: String::new(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: AttestsMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.skill, "rust");
        assert_eq!(back.level, "expert");
    }
}
