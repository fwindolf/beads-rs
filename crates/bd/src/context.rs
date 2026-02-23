//! Runtime context for command execution.
//!
//! The [`RuntimeContext`] holds all the state a command handler needs:
//! resolved database path, actor name, global flags, and (eventually)
//! the storage handle.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::GlobalArgs;

/// Runtime context passed to every command handler.
///
/// Constructed once in `main` after CLI parsing, before command dispatch.
#[derive(Debug)]
pub struct RuntimeContext {
    /// Resolved database directory path (e.g., `/repo/.beads`).
    pub db_path: Option<PathBuf>,

    /// Actor name for audit trail.
    pub actor: String,

    /// Whether to produce JSON output.
    pub json: bool,

    /// Sandbox mode: disables auto-sync.
    pub sandbox: bool,

    /// Allow operations on potentially stale data.
    pub allow_stale: bool,

    /// Read-only mode: block write operations.
    pub readonly: bool,

    /// Verbose output.
    pub verbose: bool,

    /// Quiet mode: suppress non-essential output.
    pub quiet: bool,
}

impl RuntimeContext {
    /// Build a `RuntimeContext` from parsed global arguments.
    ///
    /// Resolves the actor name using the same priority chain as the Go version:
    /// `--actor` flag > `BD_ACTOR` env > `BEADS_ACTOR` env > `git config user.name` > `$USER` > `"unknown"`.
    pub fn from_global_args(global: &GlobalArgs) -> Self {
        let actor = resolve_actor(global.actor.as_deref());

        let db_path = global.db.as_ref().map(PathBuf::from);

        Self {
            db_path,
            actor,
            json: global.json,
            sandbox: global.sandbox,
            allow_stale: global.allow_stale,
            readonly: global.readonly,
            verbose: global.verbose,
            quiet: global.quiet,
        }
    }

    /// Discover the `.beads` directory by walking up from the current directory.
    ///
    /// Returns `None` if no `.beads` directory is found.
    pub fn find_beads_dir() -> Option<PathBuf> {
        let mut dir = env::current_dir().ok()?;
        loop {
            let candidate = dir.join(".beads");
            if candidate.is_dir() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
        None
    }

    /// Returns the resolved database path, auto-discovering if needed.
    pub fn resolve_db_path(&self) -> Option<PathBuf> {
        if let Some(ref p) = self.db_path {
            return Some(p.clone());
        }
        // Auto-discover .beads directory
        Self::find_beads_dir()
    }

    /// Returns `true` if the `.beads` directory exists relative to the given path.
    pub fn beads_dir_exists(base: &Path) -> bool {
        base.join(".beads").is_dir()
    }
}

/// Resolves the actor name using the priority chain.
///
/// Priority: explicit flag > BD_ACTOR env > BEADS_ACTOR env > git config user.name > USER env > "unknown".
fn resolve_actor(flag_value: Option<&str>) -> String {
    // 1. Explicit flag value
    if let Some(actor) = flag_value {
        if !actor.is_empty() {
            return actor.to_string();
        }
    }

    // 2. BD_ACTOR env
    if let Ok(actor) = env::var("BD_ACTOR") {
        if !actor.is_empty() {
            return actor;
        }
    }

    // 3. BEADS_ACTOR env
    if let Ok(actor) = env::var("BEADS_ACTOR") {
        if !actor.is_empty() {
            return actor;
        }
    }

    // 4. git config user.name
    if let Ok(output) = Command::new("git").args(["config", "user.name"]).output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // 5. USER env (Unix) or USERNAME env (Windows)
    if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
        if !user.is_empty() {
            return user;
        }
    }

    // 6. Fallback
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_actor_with_flag() {
        assert_eq!(resolve_actor(Some("alice")), "alice");
    }

    #[test]
    fn resolve_actor_empty_flag_falls_through() {
        // With empty flag, it should fall through to env/git/default
        let result = resolve_actor(Some(""));
        assert!(!result.is_empty());
    }

    #[test]
    fn resolve_actor_none_falls_through() {
        let result = resolve_actor(None);
        // Should at least return something (git user, env, or "unknown")
        assert!(!result.is_empty());
    }
}
