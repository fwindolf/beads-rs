//! Git command execution wrappers.
//!
//! Provides a thin wrapper around `git` subprocess invocation so that the
//! rest of the codebase does not need to deal with `std::process::Command`
//! directly.
//!
//! Ported from Go `internal/git/gitdir.go` (command execution parts).

use std::path::Path;
use std::process::Command;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur when running git commands.
#[derive(Debug, Error)]
pub enum GitError {
    /// The git binary could not be found or spawned.
    #[error("failed to execute git: {0}")]
    SpawnError(#[from] std::io::Error),

    /// The git command exited with a non-zero status.
    #[error("git command failed (exit code {code:?}): {stderr}")]
    CommandFailed {
        /// The exit code, or `None` if the process was killed by a signal.
        code: Option<i32>,
        /// The content of stderr.
        stderr: String,
    },

    /// Not inside a git repository.
    #[error("not a git repository")]
    NotARepo,
}

/// A specialized `Result` type for git operations.
pub type Result<T> = std::result::Result<T, GitError>;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Execute a `git` command with the given arguments and working directory.
///
/// Returns the trimmed contents of stdout on success.
///
/// # Errors
///
/// Returns [`GitError::SpawnError`] if `git` cannot be found, or
/// [`GitError::CommandFailed`] if the command exits with a non-zero status.
///
/// # Examples
///
/// ```no_run
/// use beads_git::commands::git_command;
/// use std::path::Path;
///
/// let branch = git_command(&["rev-parse", "--abbrev-ref", "HEAD"], Path::new(".")).unwrap();
/// println!("Current branch: {branch}");
/// ```
pub fn git_command(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::CommandFailed {
            code: output.status.code(),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_command_version() {
        // `git --version` should succeed on any system with git installed.
        let result = git_command(&["--version"], Path::new("."));
        assert!(result.is_ok(), "git --version failed: {result:?}");
        let output = result.unwrap();
        assert!(
            output.starts_with("git version"),
            "unexpected output: {output}"
        );
    }

    #[test]
    fn test_git_command_failure() {
        // An invalid git subcommand should fail.
        let result = git_command(&["not-a-real-subcommand"], Path::new("."));
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::CommandFailed { code, stderr } => {
                assert!(code.is_some());
                assert!(!stderr.is_empty());
            }
            other => panic!("expected CommandFailed, got: {other:?}"),
        }
    }

    #[test]
    fn test_git_command_bad_cwd() {
        // Running git in a nonexistent directory should fail.
        let result = git_command(&["status"], Path::new("/nonexistent/directory/xyz"));
        assert!(result.is_err());
    }
}
