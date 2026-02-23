//! Formula data model -- a minimal subset of the Go formula type system.
//!
//! Covers: steps, variables (with defaults & required), conditions,
//! dependencies between steps, and gate definitions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Default formula type.
fn default_type() -> String {
    "workflow".to_string()
}

/// Default step type.
fn default_step_type() -> String {
    "task".to_string()
}

/// Default priority.
fn default_priority() -> i32 {
    2
}

/// Root structure for `.formula.json` / `.formula.toml` files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    /// Unique identifier / name for this formula.
    pub formula: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Formula type: "workflow", "expansion", "aspect".
    #[serde(default = "default_type")]
    pub r#type: String,

    /// Schema version (currently 1).
    #[serde(default)]
    pub version: i32,

    /// Template variables with optional defaults and validation.
    #[serde(default)]
    pub vars: HashMap<String, VarDef>,

    /// Steps that become issues when the formula is cooked.
    #[serde(default)]
    pub steps: Vec<Step>,

    /// Where this formula was loaded from (set by the parser).
    #[serde(skip)]
    pub source: String,
}

/// Variable definition with optional default and required flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDef {
    /// What this variable is for.
    #[serde(default)]
    pub description: String,

    /// Whether the variable must be provided (no default).
    #[serde(default)]
    pub required: bool,

    /// Default value (None = no default).
    #[serde(default)]
    pub default: Option<String>,
}

/// A work-item step that becomes an issue when cooked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Unique identifier within this formula.
    pub id: String,

    /// Issue title (supports `{{variable}}` substitution).
    pub title: String,

    /// Issue description (supports substitution).
    #[serde(default)]
    pub description: String,

    /// Issue type: "task", "bug", "feature", "epic", "chore".
    #[serde(default = "default_step_type")]
    pub r#type: String,

    /// Issue priority (0-4).
    #[serde(default = "default_priority")]
    pub priority: i32,

    /// Step IDs this step depends on.
    #[serde(default)]
    pub needs: Vec<String>,

    /// Condition for including this step, e.g. `"{{type}} == feature"`.
    #[serde(default)]
    pub condition: Option<String>,

    /// Gate configuration (async wait condition).
    #[serde(default)]
    pub gate: Option<StepGate>,

    /// Default assignee (supports substitution).
    #[serde(default)]
    pub assignee: Option<String>,

    /// Labels applied to the created issue.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Gate defines an async wait condition for a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepGate {
    /// Condition type: "human", "timer", "gh:run", "gh:pr".
    pub r#type: String,

    /// Condition identifier (e.g. workflow name for gh:run).
    #[serde(default)]
    pub id: String,

    /// How long to wait before escalation (e.g. "30m", "1h").
    #[serde(default)]
    pub timeout: String,
}

/// A fully-resolved step ready for issue creation.
#[derive(Debug, Clone, Serialize)]
pub struct CookedStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub issue_type: String,
    pub priority: i32,
    pub needs: Vec<String>,
    pub gate: Option<StepGate>,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
}

/// Errors that can occur during formula parsing and cooking.
#[derive(Debug, thiserror::Error)]
pub enum FormulaError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("missing required variable: {0}")]
    MissingVariable(String),

    #[error("unknown variable in condition: {0}")]
    UnknownVariable(String),

    #[error("invalid condition: {0}")]
    InvalidCondition(String),

    #[error("step not found: {0}")]
    StepNotFound(String),

    #[error("cycle detected in step dependencies")]
    CycleDetected,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
