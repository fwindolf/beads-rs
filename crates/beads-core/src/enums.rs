//! Enum types for the beads system.
//!
//! Each enum has:
//! - Custom Serialize (as snake_case string)
//! - Custom Deserialize (known variants + catch-all Custom/Other(String))
//! - `as_str()`, `is_default()`, `Display` impl

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ---------------------------------------------------------------------------
// Macro: defines an enum with known string variants + a Custom(String) fallback.
// ---------------------------------------------------------------------------
macro_rules! define_enum {
    (
        $(#[$meta:meta])*
        $name:ident, default = $default:ident, custom_variant = $custom_variant:ident,
        variants: [
            $( ($variant:ident, $str:expr) ),+ $(,)?
        ]
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $variant, )+
            $custom_variant(String),
        }

        impl $name {
            /// Returns the string representation.
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $str, )+
                    Self::$custom_variant(s) => s.as_str(),
                }
            }

            /// Returns `true` if this is the default variant.
            pub fn is_default(&self) -> bool {
                *self == Self::$default
            }

            /// Returns `true` if this is a built-in (non-custom) variant.
            pub fn is_builtin(&self) -> bool {
                !matches!(self, Self::$custom_variant(_))
            }

            /// Returns `true` if this is a known valid variant or any non-empty custom string.
            pub fn is_valid(&self) -> bool {
                match self {
                    Self::$custom_variant(s) => !s.is_empty(),
                    _ => true,
                }
            }

            /// Returns `true` if this is valid, also accepting the given custom values.
            pub fn is_valid_with_custom(&self, custom_values: &[&str]) -> bool {
                if self.is_builtin() {
                    return true;
                }
                if let Self::$custom_variant(s) = self {
                    return custom_values.contains(&s.as_str());
                }
                false
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::$default
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Ok(Self::from(s.as_str()))
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s {
                    $( $str => Self::$variant, )+
                    other => Self::$custom_variant(other.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                // Check known variants first to avoid allocation in common case.
                match s.as_str() {
                    $( $str => Self::$variant, )+
                    _ => Self::$custom_variant(s),
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Macro variant for enums that use an empty-string "None" default.
// ---------------------------------------------------------------------------
macro_rules! define_enum_with_none {
    (
        $(#[$meta:meta])*
        $name:ident, custom_variant = $custom_variant:ident,
        variants: [
            $( ($variant:ident, $str:expr) ),+ $(,)?
        ]
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            /// Empty / unset.
            None,
            $( $variant, )+
            $custom_variant(String),
        }

        impl $name {
            /// Returns the string representation.
            pub fn as_str(&self) -> &str {
                match self {
                    Self::None => "",
                    $( Self::$variant => $str, )+
                    Self::$custom_variant(s) => s.as_str(),
                }
            }

            /// Returns `true` if this is the default variant (None).
            pub fn is_default(&self) -> bool {
                *self == Self::None
            }

            /// Returns `true` if this is a built-in (non-custom) variant.
            pub fn is_builtin(&self) -> bool {
                !matches!(self, Self::$custom_variant(_))
            }

            /// Returns `true` if this is a known valid variant (including None).
            pub fn is_valid(&self) -> bool {
                !matches!(self, Self::$custom_variant(_))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::None
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Ok(Self::from(s.as_str()))
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s {
                    "" => Self::None,
                    $( $str => Self::$variant, )+
                    other => Self::$custom_variant(other.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                match s.as_str() {
                    "" => Self::None,
                    $( $str => Self::$variant, )+
                    _ => Self::$custom_variant(s),
                }
            }
        }
    };
}

// ===========================================================================
// Status
// ===========================================================================

define_enum! {
    /// Current state of an issue.
    Status, default = Open, custom_variant = Custom,
    variants: [
        (Open, "open"),
        (InProgress, "in_progress"),
        (Blocked, "blocked"),
        (Deferred, "deferred"),
        (Closed, "closed"),
        (Pinned, "pinned"),
        (Hooked, "hooked"),
    ]
}

// ===========================================================================
// IssueType
// ===========================================================================

define_enum! {
    /// Categorises the kind of work.
    IssueType, default = Task, custom_variant = Custom,
    variants: [
        (Bug, "bug"),
        (Feature, "feature"),
        (Task, "task"),
        (Epic, "epic"),
        (Chore, "chore"),
        (Decision, "decision"),
        (Message, "message"),
        (Molecule, "molecule"),
        (Event, "event"),
    ]
}

impl IssueType {
    /// Normalises aliases to their canonical form.
    pub fn normalize(&self) -> Self {
        match self.as_str() {
            "enhancement" | "feat" => Self::Feature,
            "dec" | "adr" => Self::Decision,
            _ => self.clone(),
        }
    }

    /// Returns `true` for core work types and the Event internal type.
    pub fn is_builtin_or_event(&self) -> bool {
        self.is_builtin()
    }
}

// ===========================================================================
// DependencyType
// ===========================================================================

define_enum! {
    /// Relationship type between issues.
    DependencyType, default = Blocks, custom_variant = Custom,
    variants: [
        (Blocks, "blocks"),
        (ParentChild, "parent-child"),
        (ConditionalBlocks, "conditional-blocks"),
        (WaitsFor, "waits-for"),
        (Related, "related"),
        (DiscoveredFrom, "discovered-from"),
        (RepliesTo, "replies-to"),
        (RelatesTo, "relates-to"),
        (Duplicates, "duplicates"),
        (Supersedes, "supersedes"),
        (AuthoredBy, "authored-by"),
        (AssignedTo, "assigned-to"),
        (ApprovedBy, "approved-by"),
        (Attests, "attests"),
        (Tracks, "tracks"),
        (Until, "until"),
        (CausedBy, "caused-by"),
        (Validates, "validates"),
        (DelegatedFrom, "delegated-from"),
    ]
}

impl DependencyType {
    /// Returns `true` if this dependency type blocks work (affects ready calculation).
    pub fn affects_ready_work(&self) -> bool {
        matches!(
            self,
            Self::Blocks | Self::ParentChild | Self::ConditionalBlocks | Self::WaitsFor
        )
    }

    /// Returns `true` if this is a well-known built-in dependency type.
    pub fn is_well_known(&self) -> bool {
        self.is_builtin()
    }
}

// ===========================================================================
// AgentState
// ===========================================================================

define_enum_with_none! {
    /// Self-reported state of an agent.
    AgentState, custom_variant = Custom,
    variants: [
        (Idle, "idle"),
        (Spawning, "spawning"),
        (Running, "running"),
        (Working, "working"),
        (Stuck, "stuck"),
        (Done, "done"),
        (Stopped, "stopped"),
        (Dead, "dead"),
    ]
}

// ===========================================================================
// MolType
// ===========================================================================

define_enum_with_none! {
    /// Molecule type for swarm coordination.
    MolType, custom_variant = Custom,
    variants: [
        (Swarm, "swarm"),
        (Patrol, "patrol"),
        (Work, "work"),
    ]
}

// ===========================================================================
// WispType
// ===========================================================================

define_enum_with_none! {
    /// Classification for TTL-based wisp compaction.
    WispType, custom_variant = Custom,
    variants: [
        (Heartbeat, "heartbeat"),
        (Ping, "ping"),
        (Patrol, "patrol"),
        (GcReport, "gc_report"),
        (Recovery, "recovery"),
        (Error, "error"),
        (Escalation, "escalation"),
    ]
}

// ===========================================================================
// WorkType
// ===========================================================================

define_enum! {
    /// How work assignment operates for a bead.
    WorkType, default = Mutex, custom_variant = Custom,
    variants: [
        (Mutex, "mutex"),
        (OpenCompetition, "open_competition"),
    ]
}

// ===========================================================================
// SortPolicy
// ===========================================================================

define_enum! {
    /// Determines how ready work is ordered.
    SortPolicy, default = Hybrid, custom_variant = Custom,
    variants: [
        (Hybrid, "hybrid"),
        (Priority, "priority"),
        (Oldest, "oldest"),
    ]
}

// ===========================================================================
// EventType
// ===========================================================================

/// Categorises audit trail events.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Created,
    Updated,
    StatusChanged,
    Commented,
    Closed,
    Reopened,
    DependencyAdded,
    DependencyRemoved,
    LabelAdded,
    LabelRemoved,
    Compacted,
    /// Catch-all for unknown / future event types.
    Other(String),
}

impl EventType {
    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::StatusChanged => "status_changed",
            Self::Commented => "commented",
            Self::Closed => "closed",
            Self::Reopened => "reopened",
            Self::DependencyAdded => "dependency_added",
            Self::DependencyRemoved => "dependency_removed",
            Self::LabelAdded => "label_added",
            Self::LabelRemoved => "label_removed",
            Self::Compacted => "compacted",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Returns `true` if this is the default variant.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Created)
    }
}

impl Default for EventType {
    fn default() -> Self {
        Self::Created
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for EventType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "created" => Self::Created,
            "updated" => Self::Updated,
            "status_changed" => Self::StatusChanged,
            "commented" => Self::Commented,
            "closed" => Self::Closed,
            "reopened" => Self::Reopened,
            "dependency_added" => Self::DependencyAdded,
            "dependency_removed" => Self::DependencyRemoved,
            "label_added" => Self::LabelAdded,
            "label_removed" => Self::LabelRemoved,
            "compacted" => Self::Compacted,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl From<String> for EventType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "created" => Self::Created,
            "updated" => Self::Updated,
            "status_changed" => Self::StatusChanged,
            "commented" => Self::Commented,
            "closed" => Self::Closed,
            "reopened" => Self::Reopened,
            "dependency_added" => Self::DependencyAdded,
            "dependency_removed" => Self::DependencyRemoved,
            "label_added" => Self::LabelAdded,
            "label_removed" => Self::LabelRemoved,
            "compacted" => Self::Compacted,
            _ => Self::Other(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_default_is_open() {
        assert_eq!(Status::default(), Status::Open);
        assert!(Status::Open.is_default());
        assert!(!Status::Closed.is_default());
    }

    #[test]
    fn status_roundtrip_serde() {
        let s = Status::InProgress;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#""in_progress""#);
        let back: Status = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn status_custom_roundtrip() {
        let json = r#""my_custom_status""#;
        let s: Status = serde_json::from_str(json).unwrap();
        assert_eq!(s, Status::Custom("my_custom_status".into()));
        assert_eq!(serde_json::to_string(&s).unwrap(), json);
    }

    #[test]
    fn dependency_type_as_str() {
        assert_eq!(DependencyType::ParentChild.as_str(), "parent-child");
        assert_eq!(
            DependencyType::ConditionalBlocks.as_str(),
            "conditional-blocks"
        );
    }

    #[test]
    fn dependency_type_affects_ready_work() {
        assert!(DependencyType::Blocks.affects_ready_work());
        assert!(DependencyType::ParentChild.affects_ready_work());
        assert!(!DependencyType::Related.affects_ready_work());
        assert!(!DependencyType::RepliesTo.affects_ready_work());
    }

    #[test]
    fn agent_state_none_default() {
        assert_eq!(AgentState::default(), AgentState::None);
        assert!(AgentState::None.is_default());
        assert_eq!(AgentState::None.as_str(), "");
    }

    #[test]
    fn agent_state_roundtrip() {
        let s = AgentState::Running;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#""running""#);
        let back: AgentState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn event_type_other_variant() {
        let json = r#""custom_event""#;
        let e: EventType = serde_json::from_str(json).unwrap();
        assert_eq!(e, EventType::Other("custom_event".into()));
    }

    #[test]
    fn issue_type_normalize() {
        assert_eq!(
            IssueType::Custom("enhancement".into()).normalize(),
            IssueType::Feature
        );
        assert_eq!(
            IssueType::Custom("feat".into()).normalize(),
            IssueType::Feature
        );
        assert_eq!(
            IssueType::Custom("adr".into()).normalize(),
            IssueType::Decision
        );
        assert_eq!(IssueType::Bug.normalize(), IssueType::Bug);
    }

    #[test]
    fn sort_policy_default() {
        assert_eq!(SortPolicy::default(), SortPolicy::Hybrid);
    }

    #[test]
    fn wisp_type_none_is_empty_string() {
        let json = r#""""#;
        let w: WispType = serde_json::from_str(json).unwrap();
        assert_eq!(w, WispType::None);
    }

    #[test]
    fn work_type_default() {
        assert_eq!(WorkType::default(), WorkType::Mutex);
        assert_eq!(WorkType::Mutex.as_str(), "mutex");
        assert_eq!(WorkType::OpenCompetition.as_str(), "open_competition");
    }
}
