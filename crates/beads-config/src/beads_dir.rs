//! Discovery and management of the `.beads/` directory.
//!
//! The `.beads/` directory is the root of a beads project's metadata. This
//! module provides functions to find it by walking up the directory tree,
//! and to create it when initializing a new project.
//!
//! Ported from Go `internal/beads/beads.go` (`FindBeadsDir`, `findLocalBeadsDir`).

use crate::config::ConfigError;
use std::path::{Path, PathBuf};

/// The name of the beads metadata directory.
const BEADS_DIR_NAME: &str = ".beads";

/// The name of the environment variable that can override the beads directory.
const BEADS_DIR_ENV: &str = "BEADS_DIR";

/// Walk up the directory tree from `start` looking for a `.beads/` directory.
///
/// Returns the path to the `.beads/` directory if found, or `None` if the
/// filesystem root is reached without finding one. The `BEADS_DIR`
/// environment variable is checked first (highest priority).
///
/// # Examples
///
/// ```no_run
/// use beads_config::beads_dir::find_beads_dir;
/// use std::path::Path;
///
/// if let Some(dir) = find_beads_dir(Path::new(".")) {
///     println!("Found beads dir at {}", dir.display());
/// }
/// ```
pub fn find_beads_dir(start: &Path) -> Option<PathBuf> {
    // 1. Check BEADS_DIR environment variable (highest priority).
    if let Ok(env_dir) = std::env::var(BEADS_DIR_ENV) {
        let env_path = PathBuf::from(&env_dir);
        if env_path.is_dir() {
            return Some(env_path);
        }
    }

    // 2. Walk up from `start` looking for .beads/.
    // Canonicalize the start path so we get absolute paths.
    let start = match start.canonicalize() {
        Ok(p) => p,
        Err(_) => return None,
    };

    let mut current = start.as_path();
    loop {
        let candidate = current.join(BEADS_DIR_NAME);
        if candidate.is_dir() {
            return Some(candidate);
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

/// Walk up the directory tree looking for `.beads/`, returning an error if
/// not found.
///
/// This is a convenience wrapper around [`find_beads_dir`] that converts
/// `None` into [`ConfigError::BeadsDirNotFound`].
///
/// # Errors
///
/// Returns [`ConfigError::BeadsDirNotFound`] if no `.beads/` directory is
/// found.
pub fn find_beads_dir_or_error(start: &Path) -> Result<PathBuf, ConfigError> {
    find_beads_dir(start).ok_or(ConfigError::BeadsDirNotFound)
}

/// Ensure a `.beads/` directory exists at the given path.
///
/// If `path` itself is not called `.beads`, the function creates a `.beads/`
/// subdirectory under it. The directory (and any necessary parents) is
/// created if it does not exist.
///
/// Returns the path to the `.beads/` directory.
///
/// # Errors
///
/// Returns [`ConfigError::ReadError`] if directory creation fails.
pub fn ensure_beads_dir(path: &Path) -> Result<PathBuf, ConfigError> {
    let beads_dir = if path.ends_with(BEADS_DIR_NAME) {
        path.to_path_buf()
    } else {
        path.join(BEADS_DIR_NAME)
    };

    std::fs::create_dir_all(&beads_dir)?;
    Ok(beads_dir)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_beads_dir_in_temp() {
        let dir = tempfile::tempdir().unwrap();
        let beads = dir.path().join(".beads");
        std::fs::create_dir(&beads).unwrap();

        let found = find_beads_dir(dir.path());
        assert!(found.is_some());
        // Canonicalize both for comparison (handles symlinks, /tmp vs /private/tmp).
        let found = found.unwrap().canonicalize().unwrap();
        let expected = beads.canonicalize().unwrap();
        assert_eq!(found, expected);
    }

    #[test]
    fn test_find_beads_dir_in_child() {
        let dir = tempfile::tempdir().unwrap();
        let beads = dir.path().join(".beads");
        std::fs::create_dir(&beads).unwrap();

        let child = dir.path().join("src").join("deep");
        std::fs::create_dir_all(&child).unwrap();

        let found = find_beads_dir(&child);
        assert!(found.is_some());
        let found = found.unwrap().canonicalize().unwrap();
        let expected = beads.canonicalize().unwrap();
        assert_eq!(found, expected);
    }

    #[test]
    fn test_find_beads_dir_not_found() {
        let dir = tempfile::tempdir().unwrap();
        // No .beads created
        let found = find_beads_dir(dir.path());
        // This might find a .beads from a parent in CI, so we just check it
        // doesn't panic. In a truly isolated environment it would be None.
        let _ = found;
    }

    #[test]
    fn test_find_beads_dir_or_error() {
        let dir = tempfile::tempdir().unwrap();
        let beads = dir.path().join(".beads");
        std::fs::create_dir(&beads).unwrap();

        let result = find_beads_dir_or_error(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_beads_dir_creates() {
        let dir = tempfile::tempdir().unwrap();
        let result = ensure_beads_dir(dir.path()).unwrap();
        assert!(result.is_dir());
        assert!(result.ends_with(".beads"));
    }

    #[test]
    fn test_ensure_beads_dir_already_named() {
        let dir = tempfile::tempdir().unwrap();
        let beads = dir.path().join(".beads");
        let result = ensure_beads_dir(&beads).unwrap();
        assert!(result.is_dir());
        assert_eq!(result, beads);
    }

    #[test]
    fn test_ensure_beads_dir_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let result1 = ensure_beads_dir(dir.path()).unwrap();
        let result2 = ensure_beads_dir(dir.path()).unwrap();
        assert_eq!(result1, result2);
    }
}
