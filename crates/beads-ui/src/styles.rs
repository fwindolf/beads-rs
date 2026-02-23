//! Ayu color theme and styling functions for beads CLI output.
//!
//! Uses the Ayu Dark color palette for consistent terminal styling.
//! Color source: <https://github.com/ayu-theme/ayu-colors>
//!
//! Design principles:
//! - Only actionable states get color (open/closed use standard text)
//! - P0/P1 get color (they need attention); P2 gets muted gold; P3/P4 are neutral
//! - Bugs and epics get color; other types use standard text
//! - Small Unicode symbols for icons, NOT emoji blobs

use beads_core::enums::{IssueType, Status};
use beads_core::issue::Issue;
use owo_colors::OwoColorize;

use crate::terminal::supports_color;

// ---------------------------------------------------------------------------
// Ayu Dark color palette (RGB values)
// ---------------------------------------------------------------------------

// Core semantic colors
const PASS: (u8, u8, u8) = (0xc2, 0xd9, 0x4c); // #c2d94c - bright green
const WARN: (u8, u8, u8) = (0xff, 0xb4, 0x54); // #ffb454 - bright yellow
const FAIL: (u8, u8, u8) = (0xf0, 0x71, 0x78); // #f07178 - bright red
const MUTED: (u8, u8, u8) = (0x6c, 0x76, 0x80); // #6c7680 - muted gray
const ACCENT: (u8, u8, u8) = (0x59, 0xc2, 0xff); // #59c2ff - bright blue

// Status colors
const STATUS_IN_PROGRESS: (u8, u8, u8) = (0xff, 0xb4, 0x54); // #ffb454 - yellow
const STATUS_CLOSED: (u8, u8, u8) = (0x80, 0x90, 0xa0); // #8090a0 - dimmed
const STATUS_BLOCKED: (u8, u8, u8) = (0xf2, 0x6d, 0x78); // #f26d78 - red
const STATUS_PINNED: (u8, u8, u8) = (0xd2, 0xa6, 0xff); // #d2a6ff - purple
const STATUS_HOOKED: (u8, u8, u8) = (0x59, 0xc2, 0xff); // #59c2ff - cyan

// Priority colors
const PRIORITY_P0: (u8, u8, u8) = (0xf0, 0x71, 0x78); // #f07178 - bright red
const PRIORITY_P1: (u8, u8, u8) = (0xff, 0x8f, 0x40); // #ff8f40 - orange
const PRIORITY_P2: (u8, u8, u8) = (0xe6, 0xb4, 0x50); // #e6b450 - muted gold

// Type colors
const TYPE_BUG: (u8, u8, u8) = (0xf2, 0x6d, 0x78); // #f26d78 - red
const TYPE_EPIC: (u8, u8, u8) = (0xd2, 0xa6, 0xff); // #d2a6ff - purple

// ---------------------------------------------------------------------------
// Status icons -- consistent semantic indicators
// ---------------------------------------------------------------------------

/// Open status icon (hollow circle -- available to work).
pub const STATUS_ICON_OPEN: &str = "\u{25CB}"; // ○
/// In-progress status icon (half-filled circle -- active work).
pub const STATUS_ICON_IN_PROGRESS: &str = "\u{25D0}"; // ◐
/// Blocked status icon (filled circle -- needs attention).
pub const STATUS_ICON_BLOCKED: &str = "\u{25CF}"; // ●
/// Closed status icon (checkmark -- completed).
pub const STATUS_ICON_CLOSED: &str = "\u{2713}"; // ✓
/// Deferred status icon (snowflake -- scheduled for later).
pub const STATUS_ICON_DEFERRED: &str = "\u{2744}"; // ❄
/// Pinned status icon.
pub const STATUS_ICON_PINNED: &str = "\u{1F4CC}"; // 📌

/// Priority icon -- small filled circle, colored by priority level.
pub const PRIORITY_ICON: &str = "\u{25CF}"; // ●

// General icons
pub const ICON_PASS: &str = "\u{2713}"; // ✓
pub const ICON_WARN: &str = "\u{26A0}"; // ⚠
pub const ICON_FAIL: &str = "\u{2716}"; // ✖
pub const ICON_SKIP: &str = "-";
pub const ICON_INFO: &str = "\u{2139}"; // ℹ

// Tree characters for hierarchical display
pub const TREE_CHILD: &str = "\u{23BF} "; // ⎿
pub const TREE_LAST: &str = "\u{2514}\u{2500} "; // └─
pub const TREE_INDENT: &str = "  ";

// Separators
pub const SEPARATOR_LIGHT: &str = "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}";
pub const SEPARATOR_HEAVY: &str = "\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}";

// ---------------------------------------------------------------------------
// Helper: apply truecolor only when color is supported
// ---------------------------------------------------------------------------

/// Applies truecolor foreground to a string, falling back to plain text
/// when color is not supported.
fn color_str(s: &str, rgb: (u8, u8, u8)) -> String {
    if supports_color() {
        s.truecolor(rgb.0, rgb.1, rgb.2).to_string()
    } else {
        s.to_string()
    }
}

/// Applies truecolor foreground + bold to a string.
fn color_bold_str(s: &str, rgb: (u8, u8, u8)) -> String {
    if supports_color() {
        s.truecolor(rgb.0, rgb.1, rgb.2).bold().to_string()
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// Core semantic render helpers
// ---------------------------------------------------------------------------

/// Renders text with pass (green) styling.
pub fn render_pass(s: &str) -> String {
    color_str(s, PASS)
}

/// Renders text with warning (yellow) styling.
pub fn render_warn(s: &str) -> String {
    color_str(s, WARN)
}

/// Renders text with fail (red) styling.
pub fn render_fail(s: &str) -> String {
    color_str(s, FAIL)
}

/// Renders text with muted (gray) styling.
pub fn render_muted(s: &str) -> String {
    color_str(s, MUTED)
}

/// Renders text with accent (blue) styling.
pub fn render_accent(s: &str) -> String {
    color_str(s, ACCENT)
}

/// Renders text in bold.
pub fn render_bold(s: &str) -> String {
    if supports_color() {
        s.bold().to_string()
    } else {
        s.to_string()
    }
}

/// Renders a category header in uppercase with accent color and bold.
pub fn render_category(s: &str) -> String {
    let upper = s.to_uppercase();
    color_bold_str(&upper, ACCENT)
}

/// Renders the light separator line in muted color.
pub fn render_separator() -> String {
    render_muted(SEPARATOR_LIGHT)
}

// ---------------------------------------------------------------------------
// Icon renderers
// ---------------------------------------------------------------------------

pub fn render_pass_icon() -> String {
    color_str(ICON_PASS, PASS)
}

pub fn render_warn_icon() -> String {
    color_str(ICON_WARN, WARN)
}

pub fn render_fail_icon() -> String {
    color_str(ICON_FAIL, FAIL)
}

pub fn render_skip_icon() -> String {
    color_str(ICON_SKIP, MUTED)
}

pub fn render_info_icon() -> String {
    color_str(ICON_INFO, ACCENT)
}

// ---------------------------------------------------------------------------
// Status rendering
// ---------------------------------------------------------------------------

/// Returns the appropriate icon for a status with semantic coloring.
/// This is the canonical source for status icon rendering.
pub fn render_status_icon(status: &Status) -> &'static str {
    match status {
        Status::Open => STATUS_ICON_OPEN,
        Status::InProgress => STATUS_ICON_IN_PROGRESS,
        Status::Blocked => STATUS_ICON_BLOCKED,
        Status::Closed => STATUS_ICON_CLOSED,
        Status::Deferred => STATUS_ICON_DEFERRED,
        Status::Pinned => STATUS_ICON_PINNED,
        _ => "?",
    }
}

/// Returns the colored status icon string.
pub fn render_status_icon_colored(status: &Status) -> String {
    let icon = render_status_icon(status);
    match status {
        Status::Open => icon.to_string(), // no color
        Status::InProgress => color_str(icon, STATUS_IN_PROGRESS),
        Status::Blocked => color_str(icon, STATUS_BLOCKED),
        Status::Closed => color_str(icon, STATUS_CLOSED),
        Status::Deferred => color_str(icon, MUTED),
        Status::Pinned => color_str(icon, STATUS_PINNED),
        Status::Hooked => color_str(STATUS_ICON_IN_PROGRESS, STATUS_HOOKED),
        _ => "?".to_string(),
    }
}

/// Renders a status string with semantic coloring.
/// in_progress/blocked/pinned/hooked get color; open/closed use standard text.
pub fn render_status(status: &Status) -> String {
    let s = status.as_str();
    match status {
        Status::InProgress => color_str(s, STATUS_IN_PROGRESS),
        Status::Blocked => color_str(s, STATUS_BLOCKED),
        Status::Pinned => color_str(s, STATUS_PINNED),
        Status::Hooked => color_str(s, STATUS_HOOKED),
        Status::Closed => color_str(s, STATUS_CLOSED),
        _ => s.to_string(), // open and others -- standard text
    }
}

// ---------------------------------------------------------------------------
// Priority rendering
// ---------------------------------------------------------------------------

/// Renders a priority level with semantic styling.
/// Format: `● P{n}` (icon + label).
/// P0 is bold red, P1 is orange, P2 is muted gold, P3/P4 are neutral.
pub fn render_priority(priority: i32) -> String {
    let label = format!("{} P{}", PRIORITY_ICON, priority);
    match priority {
        0 => color_bold_str(&label, PRIORITY_P0),
        1 => color_str(&label, PRIORITY_P1),
        2 => color_str(&label, PRIORITY_P2),
        _ => label, // P3, P4, others -- no color
    }
}

/// Renders just the priority label without icon (e.g. `P2`).
/// Use when space is constrained.
pub fn render_priority_compact(priority: i32) -> String {
    let label = format!("P{}", priority);
    match priority {
        0 => color_bold_str(&label, PRIORITY_P0),
        1 => color_str(&label, PRIORITY_P1),
        2 => color_str(&label, PRIORITY_P2),
        _ => label,
    }
}

// ---------------------------------------------------------------------------
// Type rendering
// ---------------------------------------------------------------------------

/// Renders an issue type with semantic styling.
/// Bugs and epics get color; all other types use standard text.
pub fn render_type(issue_type: &IssueType) -> String {
    let s = issue_type.as_str();
    match issue_type {
        IssueType::Bug => color_str(s, TYPE_BUG),
        IssueType::Epic => color_str(s, TYPE_EPIC),
        _ => s.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Compact issue rendering
// ---------------------------------------------------------------------------

/// Renders a compact one-line issue summary with colors.
/// Format: `ID [Priority] [Type] Status - Title`
///
/// When status is "closed", the entire line is dimmed.
pub fn render_issue_compact(issue: &Issue) -> String {
    if issue.status == Status::Closed {
        // Entire line is dimmed -- visually shows "done"
        let line = format!(
            "{} [P{}] [{}] {} - {}",
            issue.id,
            issue.priority,
            issue.issue_type.as_str(),
            issue.status.as_str(),
            issue.title,
        );
        color_str(&line, STATUS_CLOSED)
    } else {
        format!(
            "{} [{}] [{}] {} - {}",
            &issue.id,
            render_priority(issue.priority),
            render_type(&issue.issue_type),
            render_status(&issue.status),
            issue.title,
        )
    }
}

/// Renders an entire line in the closed/dimmed style.
pub fn render_closed_line(line: &str) -> String {
    color_str(line, STATUS_CLOSED)
}

/// Renders priority with color only if not closed.
pub fn render_priority_for_status(priority: i32, status: &Status) -> String {
    if *status == Status::Closed {
        format!("P{}", priority)
    } else {
        render_priority(priority)
    }
}

/// Renders type with color only if not closed.
pub fn render_type_for_status(issue_type: &IssueType, status: &Status) -> String {
    if *status == Status::Closed {
        issue_type.as_str().to_string()
    } else {
        render_type(issue_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::enums::{IssueType, Status};
    use beads_core::issue::IssueBuilder;

    #[test]
    fn status_icon_returns_correct_icons() {
        assert_eq!(render_status_icon(&Status::Open), STATUS_ICON_OPEN);
        assert_eq!(
            render_status_icon(&Status::InProgress),
            STATUS_ICON_IN_PROGRESS
        );
        assert_eq!(render_status_icon(&Status::Blocked), STATUS_ICON_BLOCKED);
        assert_eq!(render_status_icon(&Status::Closed), STATUS_ICON_CLOSED);
        assert_eq!(render_status_icon(&Status::Deferred), STATUS_ICON_DEFERRED);
        assert_eq!(render_status_icon(&Status::Pinned), STATUS_ICON_PINNED);
        assert_eq!(
            render_status_icon(&Status::Custom("unknown".into())),
            "?"
        );
    }

    #[test]
    fn render_priority_formats_correctly() {
        // In tests, NO_COLOR may or may not be set; just verify the string contains the label.
        let p0 = render_priority(0);
        assert!(p0.contains("P0"));
        let p3 = render_priority(3);
        assert!(p3.contains("P3"));
    }

    #[test]
    fn render_type_contains_type_name() {
        let bug = render_type(&IssueType::Bug);
        assert!(bug.contains("bug"));
        let task = render_type(&IssueType::Task);
        assert!(task.contains("task"));
    }

    #[test]
    fn render_issue_compact_contains_fields() {
        let issue = IssueBuilder::new("Fix login crash")
            .id("bd-abc123")
            .priority(1)
            .issue_type(IssueType::Bug)
            .status(Status::InProgress)
            .build();

        let rendered = render_issue_compact(&issue);
        assert!(rendered.contains("bd-abc123"));
        assert!(rendered.contains("Fix login crash"));
    }

    #[test]
    fn render_issue_compact_closed_dims_line() {
        let issue = IssueBuilder::new("Old task")
            .id("bd-xyz")
            .status(Status::Closed)
            .build();

        let rendered = render_issue_compact(&issue);
        assert!(rendered.contains("Old task"));
        assert!(rendered.contains("bd-xyz"));
    }
}
