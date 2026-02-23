//! Configuration types and loading for the beads system.
//!
//! The main entry point is [`BeadsConfig`], which represents the contents of
//! `.beads/config.yaml`. Configuration is loaded with [`load_config`] and
//! saved with [`save_config`].
//!
//! Ported from Go `internal/config/config.go`, `sync.go`, and `repos.go`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration file could not be read.
    #[error("failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    /// The configuration file contained invalid YAML.
    #[error("failed to parse config file: {0}")]
    ParseError(#[from] serde_yaml::Error),

    /// The `.beads/` directory was not found.
    #[error("no .beads directory found (run 'bd init' first)")]
    BeadsDirNotFound,

    /// A configuration value was invalid.
    #[error("invalid configuration value for key '{key}': {reason}")]
    InvalidValue {
        /// The configuration key that had an invalid value.
        key: String,
        /// A description of why the value is invalid.
        reason: String,
    },
}

/// A specialized `Result` type for configuration operations.
pub type Result<T> = std::result::Result<T, ConfigError>;

// ---------------------------------------------------------------------------
// Sync mode
// ---------------------------------------------------------------------------

/// The sync mode controlling how beads syncs data.
///
/// Currently only `DoltNative` is supported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    /// Use Dolt remote directly (the only supported mode).
    #[default]
    DoltNative,
}

// ---------------------------------------------------------------------------
// Conflict resolution
// ---------------------------------------------------------------------------

/// The global conflict resolution strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConflictStrategy {
    /// Last-write-wins (default).
    #[default]
    Newest,
    /// Prefer local changes.
    Ours,
    /// Prefer remote changes.
    Theirs,
    /// Require manual resolution.
    Manual,
}

/// Per-field merge strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldStrategy {
    /// Last-write-wins (default for scalar fields).
    Newest,
    /// Take the maximum value (for counters like `compaction_level`).
    Max,
    /// Perform set union (for arrays like `labels`, `waiters`).
    Union,
    /// Flag conflict for user resolution.
    Manual,
}

// ---------------------------------------------------------------------------
// Sovereignty
// ---------------------------------------------------------------------------

/// Federation sovereignty tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sovereignty {
    /// No sovereignty restriction (empty value).
    #[default]
    #[serde(rename = "")]
    None,
    /// Most open tier (public repos).
    T1,
    /// Organization-level.
    T2,
    /// Pseudonymous.
    T3,
    /// Anonymous.
    T4,
}

// ---------------------------------------------------------------------------
// Sub-configs
// ---------------------------------------------------------------------------

/// Sync configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// The sync mode. Currently only `dolt-native` is supported.
    #[serde(default)]
    pub mode: SyncMode,

    /// When to trigger export: `"push"` or `"change"`.
    #[serde(default = "default_sync_trigger_push")]
    pub export_on: String,

    /// When to trigger import: `"pull"` or `"change"`.
    #[serde(default = "default_sync_trigger_pull")]
    pub import_on: String,

    /// Whether to require confirmation on mass delete.
    #[serde(default)]
    pub require_confirmation_on_mass_delete: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mode: SyncMode::default(),
            export_on: default_sync_trigger_push(),
            import_on: default_sync_trigger_pull(),
            require_confirmation_on_mass_delete: false,
        }
    }
}

fn default_sync_trigger_push() -> String {
    "push".to_string()
}

fn default_sync_trigger_pull() -> String {
    "pull".to_string()
}

/// Conflict resolution configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConflictConfig {
    /// The global conflict resolution strategy.
    #[serde(default)]
    pub strategy: ConflictStrategy,

    /// Per-field strategy overrides keyed by field name.
    #[serde(default)]
    pub fields: HashMap<String, FieldStrategy>,
}

/// Federation configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FederationConfig {
    /// The Dolt remote URL (e.g., `dolthub://org/beads`).
    #[serde(default)]
    pub remote: String,

    /// The sovereignty tier.
    #[serde(default)]
    pub sovereignty: Sovereignty,
}

/// Git-related configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitConfig {
    /// Override commit author (e.g., `"beads-bot <beads@example.com>"`).
    #[serde(default)]
    pub author: String,

    /// Disable GPG signing for beads commits.
    #[serde(default, rename = "no-gpg-sign")]
    pub no_gpg_sign: bool,
}

/// Routing configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Routing mode.
    #[serde(default)]
    pub mode: String,

    /// Default route.
    #[serde(default = "default_route_dot")]
    pub default: String,

    /// Maintainer route.
    #[serde(default = "default_route_dot")]
    pub maintainer: String,

    /// Contributor route.
    #[serde(default = "default_contributor_route")]
    pub contributor: String,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            mode: String::new(),
            default: default_route_dot(),
            maintainer: default_route_dot(),
            contributor: default_contributor_route(),
        }
    }
}

fn default_route_dot() -> String {
    ".".to_string()
}

fn default_contributor_route() -> String {
    "~/.beads-planning".to_string()
}

/// Dolt-specific configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoltConfig {
    /// Whether to automatically create Dolt commits after write commands.
    /// Values: `"off"` | `"on"`.
    #[serde(default = "default_dolt_auto_commit", rename = "auto-commit")]
    pub auto_commit: String,
}

impl Default for DoltConfig {
    fn default() -> Self {
        Self {
            auto_commit: default_dolt_auto_commit(),
        }
    }
}

fn default_dolt_auto_commit() -> String {
    "on".to_string()
}

/// Validation configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Validation behavior on create. Values: `"none"` | `"warn"` | `"error"`.
    #[serde(default = "default_validation_none", rename = "on-create")]
    pub on_create: String,

    /// Validation behavior on sync. Values: `"none"` | `"warn"` | `"error"`.
    #[serde(default = "default_validation_none", rename = "on-sync")]
    pub on_sync: String,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            on_create: default_validation_none(),
            on_sync: default_validation_none(),
        }
    }
}

fn default_validation_none() -> String {
    "none".to_string()
}

/// Hierarchy configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyConfig {
    /// Maximum nesting depth for hierarchical IDs.
    #[serde(default = "default_max_depth", rename = "max-depth")]
    pub max_depth: u32,
}

impl Default for HierarchyConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
        }
    }
}

fn default_max_depth() -> u32 {
    3
}

/// Create command configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateConfig {
    /// Whether a description is required when creating issues.
    #[serde(default, rename = "require-description")]
    pub require_description: bool,
}

/// Multi-repo configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReposConfig {
    /// Primary repo path (where canonical issues live).
    #[serde(default)]
    pub primary: String,

    /// Additional repos to hydrate from.
    #[serde(default)]
    pub additional: Vec<String>,
}

/// AI configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// The AI model identifier.
    #[serde(default = "default_ai_model")]
    pub model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model: default_ai_model(),
        }
    }
}

fn default_ai_model() -> String {
    "claude-haiku-4-5-20251001".to_string()
}

/// Custom types configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypesConfig {
    /// Comma-separated list of custom issue types.
    #[serde(default)]
    pub custom: String,
}

/// Custom statuses configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusConfig {
    /// Comma-separated list of custom statuses.
    #[serde(default)]
    pub custom: String,
}

// ---------------------------------------------------------------------------
// Main config struct
// ---------------------------------------------------------------------------

/// The full beads configuration, corresponding to `.beads/config.yaml`.
///
/// All fields use `serde` defaults so that a partially-specified YAML file
/// will be deserialized correctly with sensible default values.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeadsConfig {
    /// Issue ID prefix (e.g., `"bd-"`).
    #[serde(default, rename = "issue-prefix")]
    pub prefix: Option<String>,

    /// Output JSON instead of human-readable text.
    #[serde(default)]
    pub json: bool,

    /// Disable database usage.
    #[serde(default, rename = "no-db")]
    pub no_db: bool,

    /// Database path override.
    #[serde(default)]
    pub db: Option<String>,

    /// Actor identity override.
    #[serde(default)]
    pub actor: Option<String>,

    /// User identity for messaging.
    #[serde(default)]
    pub identity: Option<String>,

    /// Disable git push operations.
    #[serde(default, rename = "no-push")]
    pub no_push: bool,

    /// Disable all git operations.
    #[serde(default, rename = "no-git-ops")]
    pub no_git_ops: bool,

    /// Custom issue types.
    #[serde(default)]
    pub types: TypesConfig,

    /// Custom statuses.
    #[serde(default)]
    pub status: StatusConfig,

    /// Sync configuration.
    #[serde(default)]
    pub sync: SyncConfig,

    /// Conflict resolution configuration.
    #[serde(default)]
    pub conflict: ConflictConfig,

    /// Federation configuration.
    #[serde(default)]
    pub federation: FederationConfig,

    /// Git-related configuration.
    #[serde(default)]
    pub git: GitConfig,

    /// Routing configuration.
    #[serde(default)]
    pub routing: RoutingConfig,

    /// Dolt-specific configuration.
    #[serde(default)]
    pub dolt: DoltConfig,

    /// Validation configuration.
    #[serde(default)]
    pub validation: ValidationConfig,

    /// Hierarchy configuration.
    #[serde(default)]
    pub hierarchy: HierarchyConfig,

    /// Create command configuration.
    #[serde(default)]
    pub create: CreateConfig,

    /// Multi-repo configuration.
    #[serde(default)]
    pub repos: ReposConfig,

    /// AI configuration.
    #[serde(default)]
    pub ai: AiConfig,

    /// Directory-to-label mapping for monorepo scoping.
    #[serde(default)]
    pub directory: DirectoryConfig,

    /// External projects for cross-project dependency resolution.
    #[serde(default)]
    pub external_projects: HashMap<String, String>,
}

/// Directory label configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectoryConfig {
    /// Maps directory patterns to labels.
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Helper methods on BeadsConfig
// ---------------------------------------------------------------------------

impl BeadsConfig {
    /// Return custom types as a vector of trimmed, non-empty strings.
    ///
    /// The `types.custom` field in the YAML is a comma-separated string.
    pub fn custom_types(&self) -> Vec<String> {
        parse_comma_list(&self.types.custom)
    }

    /// Return custom statuses as a vector of trimmed, non-empty strings.
    ///
    /// The `status.custom` field in the YAML is a comma-separated string.
    pub fn custom_statuses(&self) -> Vec<String> {
        parse_comma_list(&self.status.custom)
    }
}

/// Parse a comma-separated string into a vector of trimmed, non-empty strings.
fn parse_comma_list(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load configuration from `.beads/config.yaml` inside the given `.beads/` directory.
///
/// If the file does not exist, a default [`BeadsConfig`] is returned.
///
/// # Errors
///
/// Returns [`ConfigError::ReadError`] if the file exists but cannot be read,
/// or [`ConfigError::ParseError`] if it contains invalid YAML.
pub fn load_config(beads_dir: &Path) -> Result<BeadsConfig> {
    let config_path = beads_dir.join("config.yaml");

    if !config_path.exists() {
        return Ok(BeadsConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)?;

    // An empty file is valid and yields default config.
    if content.trim().is_empty() {
        return Ok(BeadsConfig::default());
    }

    let config: BeadsConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

/// Save configuration to `.beads/config.yaml` inside the given `.beads/` directory.
///
/// The directory is created if it does not exist.
///
/// # Errors
///
/// Returns [`ConfigError::ReadError`] on I/O failure or [`ConfigError::ParseError`]
/// if serialization fails.
pub fn save_config(beads_dir: &Path, config: &BeadsConfig) -> Result<()> {
    std::fs::create_dir_all(beads_dir)?;

    let config_path = beads_dir.join("config.yaml");
    let yaml = serde_yaml::to_string(config)?;
    std::fs::write(config_path, yaml)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_config() {
        let cfg = BeadsConfig::default();
        assert!(cfg.prefix.is_none());
        assert!(!cfg.json);
        assert!(!cfg.no_db);
        assert!(cfg.custom_types().is_empty());
        assert!(cfg.custom_statuses().is_empty());
    }

    #[test]
    fn test_load_missing_config_returns_default() {
        let dir = PathBuf::from("/nonexistent/path/.beads");
        let cfg = load_config(&dir).unwrap();
        assert!(cfg.prefix.is_none());
    }

    #[test]
    fn test_parse_comma_list() {
        assert_eq!(parse_comma_list(""), Vec::<String>::new());
        assert_eq!(parse_comma_list("a, b, c"), vec!["a", "b", "c"]);
        assert_eq!(parse_comma_list(" x "), vec!["x"]);
        assert_eq!(parse_comma_list(",,"), Vec::<String>::new());
    }

    #[test]
    fn test_roundtrip_config() {
        let dir = tempfile::tempdir().unwrap();
        let beads_dir = dir.path().join(".beads");

        let mut cfg = BeadsConfig::default();
        cfg.prefix = Some("test-".to_string());
        cfg.types.custom = "epic, spike".to_string();

        save_config(&beads_dir, &cfg).unwrap();
        let loaded = load_config(&beads_dir).unwrap();

        assert_eq!(loaded.prefix.as_deref(), Some("test-"));
        assert_eq!(loaded.custom_types(), vec!["epic", "spike"]);
    }

    #[test]
    fn test_deserialize_partial_yaml() {
        let yaml = "issue-prefix: proj-\njson: true\n";
        let cfg: BeadsConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.prefix.as_deref(), Some("proj-"));
        assert!(cfg.json);
        // Everything else should be default
        assert!(!cfg.no_db);
        assert_eq!(cfg.hierarchy.max_depth, 3);
    }

    #[test]
    fn test_conflict_config_defaults() {
        let cfg = BeadsConfig::default();
        assert_eq!(cfg.conflict.strategy, ConflictStrategy::Newest);
        assert!(cfg.conflict.fields.is_empty());
    }

    #[test]
    fn test_sync_config_defaults() {
        let cfg = BeadsConfig::default();
        assert_eq!(cfg.sync.export_on, "push");
        assert_eq!(cfg.sync.import_on, "pull");
    }
}
