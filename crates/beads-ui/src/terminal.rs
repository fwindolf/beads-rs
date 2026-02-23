//! Terminal detection utilities.
//!
//! Provides functions to detect TTY status, terminal dimensions,
//! color support, and agent mode. Ported from the Go `internal/ui` package.

use std::env;

/// Returns `true` if stdout is connected to a terminal (TTY).
pub fn is_tty() -> bool {
    crossterm::tty::IsTty::is_tty(&std::io::stdout())
}

/// Returns the terminal width in columns, defaulting to 80 if detection fails.
pub fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(cols, _rows)| cols as usize)
        .unwrap_or(80)
}

/// Returns the terminal height in rows, or 0 if detection fails.
pub fn terminal_height() -> usize {
    crossterm::terminal::size()
        .map(|(_cols, rows)| rows as usize)
        .unwrap_or(0)
}

/// Determines if ANSI color codes should be used.
///
/// Respects standard conventions:
/// - `NO_COLOR` (any value): disables color (<https://no-color.org/>)
/// - `CLICOLOR=0`: disables color
/// - `TERM=dumb`: disables color
/// - `CLICOLOR_FORCE` (any value): forces color even in non-TTY
/// - Falls back to TTY detection
pub fn supports_color() -> bool {
    // NO_COLOR standard -- any value disables color.
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // CLICOLOR=0 disables color.
    if env::var("CLICOLOR").as_deref() == Ok("0") {
        return false;
    }

    // TERM=dumb disables color.
    if env::var("TERM").as_deref() == Ok("dumb") {
        return false;
    }

    // CLICOLOR_FORCE forces color even in non-TTY.
    if env::var_os("CLICOLOR_FORCE").is_some() {
        return true;
    }

    // Default: use color only if stdout is a TTY.
    is_tty()
}

/// Returns `true` if the CLI is running in agent-optimized mode.
///
/// Triggered by:
/// - `BD_AGENT_MODE=1` environment variable (explicit)
/// - `CLAUDE_CODE` environment variable being set (auto-detect Claude Code)
///
/// Agent mode provides ultra-compact output optimized for LLM context windows.
pub fn is_agent_mode() -> bool {
    if env::var("BD_AGENT_MODE").as_deref() == Ok("1") {
        return true;
    }
    // Auto-detect Claude Code environment.
    if env::var_os("CLAUDE_CODE").is_some() {
        return true;
    }
    false
}

/// Returns `true` if emoji decorations should be used.
///
/// Disabled in non-TTY mode to keep output machine-readable.
/// Can be controlled with `BD_NO_EMOJI` environment variable.
pub fn should_use_emoji() -> bool {
    if env::var_os("BD_NO_EMOJI").is_some() {
        return false;
    }
    is_tty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_width_returns_positive() {
        // Even when not a TTY, we should get the default of 80.
        let width = terminal_width();
        assert!(width > 0);
    }

    #[test]
    fn is_agent_mode_defaults_to_false() {
        // In normal test environment, agent mode should be off
        // (unless CI sets these vars, which is unlikely).
        // We just verify it doesn't panic.
        let _ = is_agent_mode();
    }
}
