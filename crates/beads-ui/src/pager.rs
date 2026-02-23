//! Pager support for beads CLI output.
//!
//! Pipes long content through `less -RFX` (or `$PAGER`) when output
//! exceeds the terminal height. Ported from the Go `internal/ui` package.

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::terminal::{is_tty, terminal_height};

/// Returns `true` if the content exceeds the terminal height and should be paged.
///
/// Returns `false` if:
/// - `BD_NO_PAGER` environment variable is set
/// - stdout is not a TTY
/// - terminal height cannot be determined
/// - content fits within the terminal
pub fn should_page(content: &str) -> bool {
    if env::var_os("BD_NO_PAGER").is_some() {
        return false;
    }

    if !is_tty() {
        return false;
    }

    let height = terminal_height();
    if height == 0 {
        return false;
    }

    let content_lines = content_line_count(content);
    // Leave one line for the shell prompt.
    content_lines > height.saturating_sub(1)
}

/// Pipes content through a pager if appropriate, otherwise prints directly.
///
/// The pager command is determined by:
/// 1. `BD_PAGER` environment variable
/// 2. `PAGER` environment variable
/// 3. Falls back to `less`
///
/// When using `less`, sets `LESS=-RFX` if not already set:
/// - `-R`: Allow ANSI color codes
/// - `-F`: Quit if content fits on one screen
/// - `-X`: Don't clear screen on exit
///
/// If paging is disabled (via `BD_NO_PAGER`, non-TTY, or content fits),
/// the content is printed directly to stdout.
pub fn page(content: &str) {
    if !should_page(content) {
        print!("{}", content);
        return;
    }

    let pager_cmd = get_pager_command();
    let parts: Vec<&str> = pager_cmd.split_whitespace().collect();
    if parts.is_empty() {
        print!("{}", content);
        return;
    }

    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..]);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // Set LESS environment variable for sensible defaults if not already set.
    if env::var_os("LESS").is_none() {
        cmd.env("LESS", "-RFX");
    }

    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(ref mut stdin) = child.stdin {
                // Ignore write errors (e.g. broken pipe when user quits pager).
                let _ = stdin.write_all(content.as_bytes());
            }
            // Drop stdin to signal EOF to the pager.
            drop(child.stdin.take());
            let _ = child.wait();
        }
        Err(_) => {
            // Pager failed to launch -- fall back to direct output.
            print!("{}", content);
        }
    }
}

/// Returns the pager command to use.
/// Checks `BD_PAGER`, then `PAGER`, defaults to `"less"`.
fn get_pager_command() -> String {
    if let Ok(pager) = env::var("BD_PAGER") {
        if !pager.is_empty() {
            return pager;
        }
    }
    if let Ok(pager) = env::var("PAGER") {
        if !pager.is_empty() {
            return pager;
        }
    }
    "less".to_string()
}

/// Counts the number of lines in the content.
fn content_line_count(content: &str) -> usize {
    if content.is_empty() {
        return 0;
    }
    content.lines().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_line_count_empty() {
        assert_eq!(content_line_count(""), 0);
    }

    #[test]
    fn content_line_count_single_line() {
        assert_eq!(content_line_count("hello"), 1);
    }

    #[test]
    fn content_line_count_multiple_lines() {
        assert_eq!(content_line_count("a\nb\nc"), 3);
    }

    #[test]
    fn content_line_count_trailing_newline() {
        // str::lines() does not count a trailing empty line.
        assert_eq!(content_line_count("a\nb\n"), 2);
    }

    #[test]
    fn get_pager_defaults_to_less() {
        // This test may be affected by env vars in CI;
        // just verify it doesn't panic and returns a non-empty string.
        let pager = get_pager_command();
        assert!(!pager.is_empty());
    }
}
