//! Git directory detection and repository information.
//!
//! Provides functions for discovering the git repository root and
//! retrieving git user configuration. These mirror the Go `internal/git`
//! package's directory-detection logic.
//!
//! Ported from Go `internal/git/gitdir.go`.

use crate::commands::{git_command, GitError};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Walk up the directory tree from `start` looking for a `.git` directory
/// (or `.git` file, as used by git worktrees).
///
/// Returns the repository root directory (the parent of `.git`), or `None`
/// if the filesystem root is reached without finding one.
///
/// This function does **not** shell out to `git`; it performs a purely
/// filesystem-based search. For a more authoritative answer that respects
/// worktrees and submodules, use [`get_git_root_via_command`].
///
/// # Examples
///
/// ```no_run
/// use beads_git::gitdir::find_git_root;
/// use std::path::Path;
///
/// if let Some(root) = find_git_root(Path::new(".")) {
///     println!("Git root: {}", root.display());
/// }
/// ```
pub fn find_git_root(start: &Path) -> Option<PathBuf> {
    // Canonicalize so we work with absolute paths.
    let start = match start.canonicalize() {
        Ok(p) => p,
        Err(_) => return None,
    };

    let mut current = start.as_path();
    loop {
        let git_dir = current.join(".git");
        // .git can be a directory (regular repo) or a file (worktree/submodule).
        if git_dir.exists() {
            return Some(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent;
            }
            _ => break, // Reached filesystem root.
        }
    }

    None
}

/// Check whether `path` is inside a git repository.
///
/// Returns `true` if a `.git` directory or file is found at `path` or any
/// of its ancestors.
pub fn is_git_repo(path: &Path) -> bool {
    find_git_root(path).is_some()
}

/// Get the repository root using `git rev-parse --show-toplevel`.
///
/// This is more authoritative than [`find_git_root`] because it respects
/// worktrees, submodules, and other git internals. However, it requires
/// `git` to be installed and the path to be inside a valid repository.
///
/// Returns `None` if `git` is not available or the path is not in a repo.
pub fn get_git_root_via_command(cwd: &Path) -> Option<PathBuf> {
    match git_command(&["rev-parse", "--show-toplevel"], cwd) {
        Ok(output) => {
            let path_str = normalize_git_path(&output);
            Some(PathBuf::from(path_str))
        }
        Err(_) => None,
    }
}

/// Retrieve the `user.name` from git configuration.
///
/// Returns `None` if `git` is not installed or `user.name` is not set.
pub fn get_git_user_name() -> Option<String> {
    let output = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Retrieve the `user.email` from git configuration.
///
/// Returns `None` if `git` is not installed or `user.email` is not set.
pub fn get_git_user_email() -> Option<String> {
    let output = Command::new("git")
        .args(["config", "user.email"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if email.is_empty() {
        None
    } else {
        Some(email)
    }
}

/// Check whether the current directory is inside a git worktree (as opposed
/// to the main working tree).
///
/// Returns `Err` if not in a git repository at all.
pub fn is_worktree(cwd: &Path) -> std::result::Result<bool, GitError> {
    let git_dir = git_command(&["rev-parse", "--git-dir"], cwd)?;
    let common_dir = git_command(&["rev-parse", "--git-common-dir"], cwd)?;

    // Resolve both to absolute paths for comparison.
    let abs_git = Path::new(&git_dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&git_dir));
    let abs_common = Path::new(&common_dir)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&common_dir));

    Ok(abs_git != abs_common)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Normalize git paths for Windows compatibility.
///
/// Git on Windows may return MSYS-style paths like `/c/Users/...` or forward-
/// slash paths like `C:/Users/...`. This function converts them to native
/// format.
fn normalize_git_path(path: &str) -> String {
    // On non-Windows, return as-is.
    if std::path::MAIN_SEPARATOR != '\\' {
        return path.to_string();
    }

    let path = path.trim();

    // Convert /c/Users/... to C:\Users\...
    if path.len() >= 3
        && path.as_bytes()[0] == b'/'
        && path.as_bytes()[2] == b'/'
        && path.as_bytes()[1].is_ascii_alphabetic()
    {
        let drive = path.as_bytes()[1].to_ascii_uppercase() as char;
        let rest = &path[2..];
        return format!("{drive}:{}", rest.replace('/', "\\"));
    }

    // Convert C:/Users/... to C:\Users\...
    path.replace('/', "\\")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_git_root_in_repo() {
        // This test file is inside the beads-rs git repo, so we should find a root.
        let root = find_git_root(Path::new("."));
        assert!(root.is_some(), "expected to find git root from '.'");
        let root = root.unwrap();
        assert!(root.join(".git").exists(), ".git should exist at root");
    }

    #[test]
    fn test_is_git_repo() {
        assert!(is_git_repo(Path::new(".")));
    }

    #[test]
    fn test_find_git_root_temp_dir() {
        // A fresh temp directory should not be a git repo (in most cases).
        let dir = tempfile::tempdir().unwrap();
        let root = find_git_root(dir.path());
        // On some CI systems the temp dir might be inside a git repo,
        // so we just ensure this doesn't panic.
        let _ = root;
    }

    #[test]
    fn test_get_git_user_name() {
        // Just verify this doesn't panic. The value depends on the system config.
        let _ = get_git_user_name();
    }

    #[test]
    fn test_get_git_user_email() {
        // Just verify this doesn't panic.
        let _ = get_git_user_email();
    }

    #[test]
    fn test_normalize_git_path_unix() {
        // On Unix, this should be a no-op.
        if std::path::MAIN_SEPARATOR != '\\' {
            assert_eq!(normalize_git_path("/home/user/repo"), "/home/user/repo");
        }
    }

    #[test]
    fn test_normalize_git_path_windows_msys() {
        // Test MSYS-style path conversion (only meaningful on Windows).
        if std::path::MAIN_SEPARATOR == '\\' {
            assert_eq!(
                normalize_git_path("/c/Users/test/repo"),
                "C:\\Users\\test\\repo"
            );
        }
    }

    #[test]
    fn test_normalize_git_path_windows_forward_slash() {
        // Test forward-slash path conversion (only meaningful on Windows).
        if std::path::MAIN_SEPARATOR == '\\' {
            assert_eq!(
                normalize_git_path("C:/Users/test/repo"),
                "C:\\Users\\test\\repo"
            );
        }
    }
}
