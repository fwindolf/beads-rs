//! Issue struct -- the central domain model for the beads system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::dependency::Dependency;
use crate::entity::{BondRef, EntityRef, Validation};
use crate::enums::{AgentState, IssueType, MolType, Status, WispType, WorkType};

/// Helper for `skip_serializing_if` on `bool` fields.
fn is_false(b: &bool) -> bool {
    !b
}

/// Helper for `skip_serializing_if` on `i32` fields (priority: 0 is valid, never skip).
fn is_zero_priority(_p: &i32) -> bool {
    false
}

/// Helper for `skip_serializing_if` on `Vec` fields.
fn is_empty_vec<T>(v: &Vec<T>) -> bool {
    v.is_empty()
}

/// Helper for `skip_serializing_if` on duration fields.
fn is_zero_duration(d: &Option<std::time::Duration>) -> bool {
    d.is_none()
}

/// Represents a trackable work item.
///
/// Fields are organised into logical groups for maintainability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    // ===== Core Identification =====
    #[serde(default)]
    pub id: String,

    /// Internal: SHA256 of canonical content -- NOT exported to JSONL.
    #[serde(skip)]
    pub content_hash: String,

    // ===== Issue Content =====
    #[serde(default)]
    pub title: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub design: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub acceptance_criteria: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub spec_id: String,

    // ===== Status & Workflow =====
    #[serde(default, skip_serializing_if = "Status::is_default")]
    pub status: Status,

    /// Priority 0-4. No skip: 0 is valid (P0/critical).
    #[serde(default, skip_serializing_if = "is_zero_priority")]
    pub priority: i32,

    #[serde(default, skip_serializing_if = "IssueType::is_default")]
    pub issue_type: IssueType,

    // ===== Assignment =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assignee: String,

    /// Human owner for CV attribution (git author email).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<i32>,

    // ===== Timestamps =====
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_by: String,

    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub close_reason: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub closed_by_session: String,

    // ===== Time-Based Scheduling =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defer_until: Option<DateTime<Utc>>,

    // ===== External Integration =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_system: String,

    // ===== Custom Metadata =====
    /// Arbitrary JSON data for extension points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Box<serde_json::value::RawValue>>,

    // ===== Compaction Metadata =====
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub compaction_level: i32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted_at: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted_at_commit: Option<String>,

    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub original_size: i32,

    // ===== Internal Routing (not exported to JSONL) =====
    /// Which repo owns this issue (multi-repo support).
    #[serde(skip)]
    pub source_repo: String,

    /// Override prefix for ID generation (appends to config prefix).
    #[serde(skip)]
    pub id_prefix: String,

    /// Completely replace config prefix (for cross-rig creation).
    #[serde(skip)]
    pub prefix_override: String,

    // ===== Relational Data (populated for export/import) =====
    #[serde(default, skip_serializing_if = "is_empty_vec")]
    pub labels: Vec<String>,

    #[serde(default, skip_serializing_if = "is_empty_vec")]
    pub dependencies: Vec<Dependency>,

    #[serde(default, skip_serializing_if = "is_empty_vec")]
    pub comments: Vec<crate::comment::Comment>,

    // ===== Messaging Fields =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sender: String,

    #[serde(default, skip_serializing_if = "is_false")]
    pub ephemeral: bool,

    #[serde(default, skip_serializing_if = "WispType::is_default")]
    pub wisp_type: WispType,

    // ===== Context Markers =====
    #[serde(default, skip_serializing_if = "is_false")]
    pub pinned: bool,

    #[serde(default, skip_serializing_if = "is_false")]
    pub is_template: bool,

    // ===== Bonding Fields =====
    #[serde(default, skip_serializing_if = "is_empty_vec")]
    pub bonded_from: Vec<BondRef>,

    // ===== HOP Fields =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<EntityRef>,

    #[serde(default, skip_serializing_if = "is_empty_vec")]
    pub validations: Vec<Validation>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<f32>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub crystallizes: bool,

    // ===== Gate Fields (async coordination) =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub await_type: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub await_id: String,

    /// Max wait time before escalation.
    #[serde(
        default,
        skip_serializing_if = "is_zero_duration",
        with = "duration_serde"
    )]
    pub timeout: Option<std::time::Duration>,

    #[serde(default, skip_serializing_if = "is_empty_vec")]
    pub waiters: Vec<String>,

    // ===== Slot Fields =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub holder: String,

    // ===== Source Tracing Fields =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_formula: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_location: String,

    // ===== Agent Identity Fields =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hook_bead: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role_bead: String,

    #[serde(default, skip_serializing_if = "AgentState::is_default")]
    pub agent_state: AgentState,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role_type: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rig: String,

    // ===== Molecule Type Fields =====
    #[serde(default, skip_serializing_if = "MolType::is_default")]
    pub mol_type: MolType,

    // ===== Work Type Fields =====
    #[serde(default, skip_serializing_if = "WorkType::is_default")]
    pub work_type: WorkType,

    // ===== Event Fields =====
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub event_kind: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub actor: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub payload: String,
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

/// Serde helper module for `Option<std::time::Duration>` stored as nanoseconds.
mod duration_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        dur: &Option<std::time::Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match dur {
            Some(d) => serializer.serialize_u64(d.as_nanos() as u64),
            None => serializer.serialize_u64(0),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<std::time::Duration>, D::Error> {
        let ns = u64::deserialize(deserializer)?;
        if ns == 0 {
            Ok(None)
        } else {
            Ok(Some(std::time::Duration::from_nanos(ns)))
        }
    }
}

impl Default for Issue {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(),
            content_hash: String::new(),
            title: String::new(),
            description: String::new(),
            design: String::new(),
            acceptance_criteria: String::new(),
            notes: String::new(),
            spec_id: String::new(),
            status: Status::Open,
            priority: 0,
            issue_type: IssueType::Task,
            assignee: String::new(),
            owner: String::new(),
            estimated_minutes: None,
            created_at: now,
            created_by: String::new(),
            updated_at: now,
            closed_at: None,
            close_reason: String::new(),
            closed_by_session: String::new(),
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: String::new(),
            metadata: None,
            compaction_level: 0,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: 0,
            source_repo: String::new(),
            id_prefix: String::new(),
            prefix_override: String::new(),
            labels: Vec::new(),
            dependencies: Vec::new(),
            comments: Vec::new(),
            sender: String::new(),
            ephemeral: false,
            wisp_type: WispType::default(),
            pinned: false,
            is_template: false,
            bonded_from: Vec::new(),
            creator: None,
            validations: Vec::new(),
            quality_score: None,
            crystallizes: false,
            await_type: String::new(),
            await_id: String::new(),
            timeout: None,
            waiters: Vec::new(),
            holder: String::new(),
            source_formula: String::new(),
            source_location: String::new(),
            hook_bead: String::new(),
            role_bead: String::new(),
            agent_state: AgentState::default(),
            last_activity: None,
            role_type: String::new(),
            rig: String::new(),
            mol_type: MolType::default(),
            work_type: WorkType::default(),
            event_kind: String::new(),
            actor: String::new(),
            target: String::new(),
            payload: String::new(),
        }
    }
}

impl Issue {
    /// Applies default values for fields omitted during JSONL import.
    ///
    /// - Status defaults to Open if empty
    /// - IssueType defaults to Task if empty
    pub fn set_defaults(&mut self) {
        if self.status == Status::Custom(String::new()) || self.status.as_str().is_empty() {
            self.status = Status::Open;
        }
        if self.issue_type == IssueType::Custom(String::new())
            || self.issue_type.as_str().is_empty()
        {
            self.issue_type = IssueType::Task;
        }
    }

    /// Returns `true` if this issue is a compound (bonded from multiple sources).
    pub fn is_compound(&self) -> bool {
        !self.bonded_from.is_empty()
    }

    /// Returns the BondRefs for this compound's constituent protos.
    pub fn get_constituents(&self) -> &[BondRef] {
        &self.bonded_from
    }
}

/// Builder for constructing an [`Issue`] with a fluent API.
pub struct IssueBuilder {
    issue: Issue,
}

impl IssueBuilder {
    /// Creates a new builder with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut issue = Issue::default();
        issue.title = title.into();
        Self { issue }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.issue.id = id.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.issue.description = description.into();
        self
    }

    pub fn design(mut self, design: impl Into<String>) -> Self {
        self.issue.design = design.into();
        self
    }

    pub fn acceptance_criteria(mut self, ac: impl Into<String>) -> Self {
        self.issue.acceptance_criteria = ac.into();
        self
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.issue.notes = notes.into();
        self
    }

    pub fn spec_id(mut self, spec_id: impl Into<String>) -> Self {
        self.issue.spec_id = spec_id.into();
        self
    }

    pub fn status(mut self, status: Status) -> Self {
        self.issue.status = status;
        self
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.issue.priority = priority;
        self
    }

    pub fn issue_type(mut self, issue_type: IssueType) -> Self {
        self.issue.issue_type = issue_type;
        self
    }

    pub fn assignee(mut self, assignee: impl Into<String>) -> Self {
        self.issue.assignee = assignee.into();
        self
    }

    pub fn owner(mut self, owner: impl Into<String>) -> Self {
        self.issue.owner = owner.into();
        self
    }

    pub fn estimated_minutes(mut self, minutes: i32) -> Self {
        self.issue.estimated_minutes = Some(minutes);
        self
    }

    pub fn created_at(mut self, t: DateTime<Utc>) -> Self {
        self.issue.created_at = t;
        self
    }

    pub fn created_by(mut self, by: impl Into<String>) -> Self {
        self.issue.created_by = by.into();
        self
    }

    pub fn updated_at(mut self, t: DateTime<Utc>) -> Self {
        self.issue.updated_at = t;
        self
    }

    pub fn closed_at(mut self, t: DateTime<Utc>) -> Self {
        self.issue.closed_at = Some(t);
        self
    }

    pub fn close_reason(mut self, reason: impl Into<String>) -> Self {
        self.issue.close_reason = reason.into();
        self
    }

    pub fn due_at(mut self, t: DateTime<Utc>) -> Self {
        self.issue.due_at = Some(t);
        self
    }

    pub fn defer_until(mut self, t: DateTime<Utc>) -> Self {
        self.issue.defer_until = Some(t);
        self
    }

    pub fn external_ref(mut self, ext: impl Into<String>) -> Self {
        self.issue.external_ref = Some(ext.into());
        self
    }

    pub fn source_system(mut self, sys: impl Into<String>) -> Self {
        self.issue.source_system = sys.into();
        self
    }

    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.issue.labels = labels;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.issue.pinned = pinned;
        self
    }

    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.issue.ephemeral = ephemeral;
        self
    }

    pub fn sender(mut self, sender: impl Into<String>) -> Self {
        self.issue.sender = sender.into();
        self
    }

    pub fn wisp_type(mut self, wt: WispType) -> Self {
        self.issue.wisp_type = wt;
        self
    }

    pub fn is_template(mut self, is_template: bool) -> Self {
        self.issue.is_template = is_template;
        self
    }

    pub fn creator(mut self, creator: EntityRef) -> Self {
        self.issue.creator = Some(creator);
        self
    }

    pub fn crystallizes(mut self, crystallizes: bool) -> Self {
        self.issue.crystallizes = crystallizes;
        self
    }

    pub fn quality_score(mut self, score: f32) -> Self {
        self.issue.quality_score = Some(score);
        self
    }

    pub fn await_type(mut self, t: impl Into<String>) -> Self {
        self.issue.await_type = t.into();
        self
    }

    pub fn await_id(mut self, id: impl Into<String>) -> Self {
        self.issue.await_id = id.into();
        self
    }

    pub fn timeout(mut self, d: std::time::Duration) -> Self {
        self.issue.timeout = Some(d);
        self
    }

    pub fn holder(mut self, holder: impl Into<String>) -> Self {
        self.issue.holder = holder.into();
        self
    }

    pub fn hook_bead(mut self, hb: impl Into<String>) -> Self {
        self.issue.hook_bead = hb.into();
        self
    }

    pub fn role_bead(mut self, rb: impl Into<String>) -> Self {
        self.issue.role_bead = rb.into();
        self
    }

    pub fn agent_state(mut self, state: AgentState) -> Self {
        self.issue.agent_state = state;
        self
    }

    pub fn role_type(mut self, rt: impl Into<String>) -> Self {
        self.issue.role_type = rt.into();
        self
    }

    pub fn rig(mut self, rig: impl Into<String>) -> Self {
        self.issue.rig = rig.into();
        self
    }

    pub fn mol_type(mut self, mt: MolType) -> Self {
        self.issue.mol_type = mt;
        self
    }

    pub fn work_type(mut self, wt: WorkType) -> Self {
        self.issue.work_type = wt;
        self
    }

    pub fn event_kind(mut self, ek: impl Into<String>) -> Self {
        self.issue.event_kind = ek.into();
        self
    }

    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.issue.actor = actor.into();
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.issue.target = target.into();
        self
    }

    pub fn payload(mut self, payload: impl Into<String>) -> Self {
        self.issue.payload = payload.into();
        self
    }

    pub fn source_formula(mut self, sf: impl Into<String>) -> Self {
        self.issue.source_formula = sf.into();
        self
    }

    pub fn source_location(mut self, sl: impl Into<String>) -> Self {
        self.issue.source_location = sl.into();
        self
    }

    /// Consumes the builder and returns the constructed [`Issue`].
    pub fn build(self) -> Issue {
        self.issue
    }
}

/// ID prefix constants for molecule/wisp instantiation.
pub mod id_prefix {
    /// Persistent molecules (bd-mol-xxx).
    pub const MOL: &str = "mol";
    /// Ephemeral wisps (bd-wisp-xxx).
    pub const WISP: &str = "wisp";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_issue() {
        let issue = Issue::default();
        assert_eq!(issue.status, Status::Open);
        assert_eq!(issue.issue_type, IssueType::Task);
        assert_eq!(issue.priority, 0);
    }

    #[test]
    fn builder_basic() {
        let issue = IssueBuilder::new("Fix the bug")
            .priority(2)
            .status(Status::InProgress)
            .issue_type(IssueType::Bug)
            .assignee("alice")
            .build();

        assert_eq!(issue.title, "Fix the bug");
        assert_eq!(issue.priority, 2);
        assert_eq!(issue.status, Status::InProgress);
        assert_eq!(issue.issue_type, IssueType::Bug);
        assert_eq!(issue.assignee, "alice");
    }

    #[test]
    fn issue_serde_roundtrip() {
        let issue = IssueBuilder::new("Test issue")
            .id("bd-abc123")
            .priority(1)
            .description("A test description")
            .build();

        let json = serde_json::to_string(&issue).unwrap();
        let back: Issue = serde_json::from_str(&json).unwrap();

        assert_eq!(back.title, "Test issue");
        assert_eq!(back.id, "bd-abc123");
        assert_eq!(back.priority, 1);
        assert_eq!(back.description, "A test description");
    }

    #[test]
    fn issue_set_defaults() {
        let json = r#"{"title": "hello"}"#;
        let mut issue: Issue = serde_json::from_str(json).unwrap();
        // After deserialization of empty enum, they will be Custom("") -- set_defaults fixes this
        issue.set_defaults();
        assert_eq!(issue.status, Status::Open);
        assert_eq!(issue.issue_type, IssueType::Task);
    }

    #[test]
    fn issue_is_compound() {
        let mut issue = Issue::default();
        assert!(!issue.is_compound());

        issue.bonded_from.push(BondRef {
            source_id: "src-1".into(),
            bond_type: "sequential".into(),
            bond_point: String::new(),
        });
        assert!(issue.is_compound());
    }
}
