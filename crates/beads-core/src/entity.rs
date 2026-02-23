//! Entity references for HOP entity tracking and CV chains.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A structured reference to an entity (human, agent, or org).
///
/// Can be rendered as a URI: `hop://<platform>/<org>/<id>`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRef {
    /// Human-readable identifier (e.g., "polecat/Nux", "mayor").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,

    /// Execution context (e.g., "gastown", "github").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub platform: String,

    /// Organisation (e.g., "steveyegge", "anthropics").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub org: String,

    /// Unique identifier within the platform/org.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
}

impl EntityRef {
    /// Returns `true` if all fields are empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.platform.is_empty()
            && self.org.is_empty()
            && self.id.is_empty()
    }

    /// Returns the entity as a HOP URI: `hop://<platform>/<org>/<id>`.
    ///
    /// Returns `None` if Platform, Org, or ID is missing.
    pub fn uri(&self) -> Option<String> {
        if self.platform.is_empty() || self.org.is_empty() || self.id.is_empty() {
            return None;
        }
        Some(format!("hop://{}/{}/{}", self.platform, self.org, self.id))
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.name.is_empty() {
            return f.write_str(&self.name);
        }
        if let Some(uri) = self.uri() {
            return f.write_str(&uri);
        }
        f.write_str(&self.id)
    }
}

/// Records who validated/approved work completion.
///
/// Core to HOP's proof-of-stake concept -- validators stake their reputation
/// on approvals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Validation {
    /// Who approved/rejected the work.
    pub validator: Option<EntityRef>,

    /// Validation result: `accepted`, `rejected`, `revision_requested`.
    pub outcome: String,

    /// When the validation occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Optional quality score (0.0 -- 1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

/// Validation outcome constants.
pub mod validation_outcome {
    pub const ACCEPTED: &str = "accepted";
    pub const REJECTED: &str = "rejected";
    pub const REVISION_REQUESTED: &str = "revision_requested";
}

impl Validation {
    /// Checks if the outcome is a known validation outcome.
    pub fn is_valid_outcome(&self) -> bool {
        matches!(
            self.outcome.as_str(),
            validation_outcome::ACCEPTED
                | validation_outcome::REJECTED
                | validation_outcome::REVISION_REQUESTED
        )
    }
}

/// Tracks compound molecule lineage.
///
/// When protos or molecules are bonded together, `BondRef` records which
/// sources were combined and how they were attached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BondRef {
    /// Source proto or molecule ID.
    pub source_id: String,

    /// Bond type: sequential, parallel, conditional.
    pub bond_type: String,

    /// Attachment site (issue ID or empty for root).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bond_point: String,
}

/// Bond type constants for compound molecules.
pub mod bond_type {
    pub const SEQUENTIAL: &str = "sequential";
    pub const PARALLEL: &str = "parallel";
    pub const CONDITIONAL: &str = "conditional";
    pub const ROOT: &str = "root";
}

/// Parses a HOP entity URI into an [`EntityRef`].
///
/// Supported formats:
/// - `hop://<platform>/<org>/<id>`
/// - `entity://hop/<platform>/<org>/<id>` (legacy)
///
/// Returns an error if the URI is invalid.
pub fn parse_entity_uri(uri: &str) -> Result<EntityRef, ParseEntityUriError> {
    const HOP_PREFIX: &str = "hop://";
    const LEGACY_PREFIX: &str = "entity://hop/";

    let rest = if let Some(rest) = uri.strip_prefix(HOP_PREFIX) {
        rest
    } else if let Some(rest) = uri.strip_prefix(LEGACY_PREFIX) {
        rest
    } else {
        return Err(ParseEntityUriError::InvalidPrefix(uri.to_owned()));
    };

    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.len() != 3 || parts[0].is_empty() || parts[1].is_empty() || parts[2].is_empty() {
        return Err(ParseEntityUriError::InvalidFormat(uri.to_owned()));
    }

    Ok(EntityRef {
        name: String::new(),
        platform: parts[0].to_owned(),
        org: parts[1].to_owned(),
        id: parts[2].to_owned(),
    })
}

/// Errors returned when parsing an entity URI.
#[derive(Debug, thiserror::Error)]
pub enum ParseEntityUriError {
    #[error(
        "invalid entity URI: must start with \"hop://\" (or legacy \"entity://hop/\"), got {0:?}"
    )]
    InvalidPrefix(String),

    #[error("invalid entity URI: expected hop://<platform>/<org>/<id>, got {0:?}")]
    InvalidFormat(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ref_uri() {
        let e = EntityRef {
            name: "polecat/Nux".into(),
            platform: "gastown".into(),
            org: "steveyegge".into(),
            id: "polecat-nux".into(),
        };
        assert_eq!(e.uri(), Some("hop://gastown/steveyegge/polecat-nux".into()));
    }

    #[test]
    fn entity_ref_display_prefers_name() {
        let e = EntityRef {
            name: "mayor".into(),
            platform: "gastown".into(),
            org: "steveyegge".into(),
            id: "mayor-1".into(),
        };
        assert_eq!(e.to_string(), "mayor");
    }

    #[test]
    fn entity_ref_display_falls_back_to_uri() {
        let e = EntityRef {
            name: String::new(),
            platform: "gastown".into(),
            org: "steveyegge".into(),
            id: "mayor-1".into(),
        };
        assert_eq!(e.to_string(), "hop://gastown/steveyegge/mayor-1");
    }

    #[test]
    fn parse_hop_uri() {
        let e = parse_entity_uri("hop://github/anthropic/claude-1").unwrap();
        assert_eq!(e.platform, "github");
        assert_eq!(e.org, "anthropic");
        assert_eq!(e.id, "claude-1");
    }

    #[test]
    fn parse_legacy_uri() {
        let e = parse_entity_uri("entity://hop/github/anthropic/claude-1").unwrap();
        assert_eq!(e.platform, "github");
        assert_eq!(e.org, "anthropic");
        assert_eq!(e.id, "claude-1");
    }

    #[test]
    fn parse_invalid_uri() {
        assert!(parse_entity_uri("http://example.com").is_err());
        assert!(parse_entity_uri("hop://").is_err());
        assert!(parse_entity_uri("hop://a/b").is_err());
        assert!(parse_entity_uri("hop:///b/c").is_err());
    }

    #[test]
    fn validation_outcome_check() {
        let v = Validation {
            validator: None,
            outcome: "accepted".into(),
            timestamp: chrono::Utc::now(),
            score: None,
        };
        assert!(v.is_valid_outcome());

        let v2 = Validation {
            validator: None,
            outcome: "unknown".into(),
            timestamp: chrono::Utc::now(),
            score: None,
        };
        assert!(!v2.is_valid_outcome());
    }
}
