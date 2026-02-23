//! Output formatting helpers for the `bd` CLI.
//!
//! Provides JSON output, table formatting, and human-readable issue display
//! in both compact (one-liner) and detailed (multi-line) formats.

use beads_core::enums::Status;
use beads_core::issue::Issue;
use serde::Serialize;
use std::io::{self, Write};

/// A view model for JSON output that matches the VS Code extension adapter contract.
///
/// Field names are mapped from the internal `Issue` type:
/// - `issue_type` -> serialized as `type`
/// - `created_at` -> serialized as `created` (ISO 8601 string)
/// - `updated_at` -> serialized as `updated` (ISO 8601 string)
/// - `labels` as `Vec<String>`
/// - `status` as lowercase string
#[allow(dead_code)]
#[derive(Serialize)]
pub struct BeadView {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,
}

#[allow(dead_code)]
impl BeadView {
    /// Build a `BeadView` from an `Issue` and its labels.
    ///
    /// If `labels` is empty the field will serialize as `[]`.
    /// `description` is omitted when empty.
    /// `assignee` / `close_reason` are omitted when empty.
    pub fn from_issue(issue: &Issue, labels: Vec<String>) -> Self {
        Self {
            id: issue.id.clone(),
            title: issue.title.clone(),
            status: issue.status.as_str().to_string(),
            priority: issue.priority,
            issue_type: issue.issue_type.as_str().to_string(),
            labels,
            description: if issue.description.is_empty() {
                None
            } else {
                Some(issue.description.clone())
            },
            created: Some(issue.created_at.to_rfc3339()),
            updated: Some(issue.updated_at.to_rfc3339()),
            assignee: if issue.assignee.is_empty() {
                None
            } else {
                Some(issue.assignee.clone())
            },
            close_reason: if issue.close_reason.is_empty() {
                None
            } else {
                Some(issue.close_reason.clone())
            },
        }
    }

    /// Build a `BeadView` from an `Issue`, using the labels already on the issue.
    pub fn from_issue_with_own_labels(issue: &Issue) -> Self {
        Self::from_issue(issue, issue.labels.clone())
    }
}

/// Load labels for an issue from the database.
///
/// Returns an empty `Vec` if the query fails.
pub fn load_labels(conn: &rusqlite::Connection, issue_id: &str) -> Vec<String> {
    conn.prepare("SELECT label FROM labels WHERE issue_id = ?1 ORDER BY label")
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![issue_id], |row| row.get(0))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default()
}

/// Populate the `labels` field on an issue by loading from the database.
#[allow(dead_code)]
pub fn populate_labels(conn: &rusqlite::Connection, issue: &mut Issue) {
    issue.labels = load_labels(conn, &issue.id);
}

/// Populate labels on a slice of issues from the database.
pub fn populate_labels_bulk(conn: &rusqlite::Connection, issues: &mut [Issue]) {
    for issue in issues.iter_mut() {
        issue.labels = load_labels(conn, &issue.id);
    }
}

/// Print a value as pretty-printed JSON to stdout.
///
/// Terminates the process with exit code 1 if serialization fails.
pub fn output_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            // Ignore broken pipe errors (e.g., piped to `head`)
            let _ = writeln!(handle, "{}", json);
        }
        Err(e) => {
            eprintln!("Error: failed to serialize JSON: {}", e);
            std::process::exit(1);
        }
    }
}

/// Print a simple table with headers and rows.
///
/// Each row is a `Vec<String>` with columns matching the headers.
/// Column widths are computed from the data for alignment.
pub fn output_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        return;
    }

    // Compute column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Print header
    for (i, header) in headers.iter().enumerate() {
        if i > 0 {
            let _ = write!(handle, "  ");
        }
        let _ = write!(handle, "{:<width$}", header, width = widths[i]);
    }
    let _ = writeln!(handle);

    // Print separator
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            let _ = write!(handle, "  ");
        }
        let _ = write!(handle, "{}", "-".repeat(*width));
    }
    let _ = writeln!(handle);

    // Print rows
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                let _ = write!(handle, "  ");
            }
            if i < widths.len() {
                let _ = write!(handle, "{:<width$}", cell, width = widths[i]);
            } else {
                let _ = write!(handle, "{}", cell);
            }
        }
        let _ = writeln!(handle);
    }
}

/// Format an issue as a compact one-line string.
///
/// Format: `[P{priority}] [{type}] {id}: {title} ({status})`
pub fn format_issue_compact(issue: &Issue) -> String {
    let status_indicator = match &issue.status {
        Status::Open => "open",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Deferred => "deferred",
        Status::Closed => "closed",
        Status::Hooked => "hooked",
        _ => issue.status.as_str(),
    };

    let assignee_part = if issue.assignee.is_empty() {
        String::new()
    } else {
        format!(" @{}", issue.assignee)
    };

    format!(
        "[P{}] [{}] {}: {} ({}{})",
        issue.priority, issue.issue_type, issue.id, issue.title, status_indicator, assignee_part,
    )
}

/// Format an issue in detailed multi-line view.
///
/// Shows all populated fields with section headers.
pub fn format_issue_detail(issue: &Issue) -> String {
    let mut lines = Vec::new();

    // Header line
    lines.push(format!(
        "{} [P{}] [{}] {}",
        issue.id, issue.priority, issue.issue_type, issue.title
    ));

    // Status and assignment
    lines.push(format!("Status: {}", issue.status));
    if !issue.assignee.is_empty() {
        lines.push(format!("Assignee: {}", issue.assignee));
    }
    if !issue.owner.is_empty() {
        lines.push(format!("Owner: {}", issue.owner));
    }

    // Timestamps
    lines.push(format!(
        "Created: {} by {}",
        issue.created_at.format("%Y-%m-%d %H:%M"),
        if issue.created_by.is_empty() {
            "unknown"
        } else {
            &issue.created_by
        }
    ));
    lines.push(format!(
        "Updated: {}",
        issue.updated_at.format("%Y-%m-%d %H:%M")
    ));
    if let Some(ref closed_at) = issue.closed_at {
        lines.push(format!("Closed: {}", closed_at.format("%Y-%m-%d %H:%M")));
        if !issue.close_reason.is_empty() {
            lines.push(format!("Reason: {}", issue.close_reason));
        }
    }

    // Time-based scheduling
    if let Some(ref due_at) = issue.due_at {
        lines.push(format!("Due: {}", due_at.format("%Y-%m-%d %H:%M")));
    }
    if let Some(ref defer_until) = issue.defer_until {
        lines.push(format!(
            "Deferred until: {}",
            defer_until.format("%Y-%m-%d %H:%M")
        ));
    }

    // Content sections
    if !issue.description.is_empty() {
        lines.push(String::new());
        lines.push("DESCRIPTION".to_string());
        lines.push(issue.description.clone());
    }
    if !issue.design.is_empty() {
        lines.push(String::new());
        lines.push("DESIGN".to_string());
        lines.push(issue.design.clone());
    }
    if !issue.notes.is_empty() {
        lines.push(String::new());
        lines.push("NOTES".to_string());
        lines.push(issue.notes.clone());
    }
    if !issue.acceptance_criteria.is_empty() {
        lines.push(String::new());
        lines.push("ACCEPTANCE CRITERIA".to_string());
        lines.push(issue.acceptance_criteria.clone());
    }

    // Labels
    if !issue.labels.is_empty() {
        lines.push(String::new());
        lines.push(format!("Labels: {}", issue.labels.join(", ")));
    }

    // External ref
    if let Some(ref ext) = issue.external_ref {
        lines.push(format!("External ref: {}", ext));
    }

    lines.join("\n")
}

/// Format an issue as a compact row for list output.
///
/// Returns a vector of column values suitable for [`output_table`].
pub fn format_issue_row(issue: &Issue) -> Vec<String> {
    vec![
        issue.id.clone(),
        format!("P{}", issue.priority),
        issue.issue_type.to_string(),
        issue.status.to_string(),
        issue.title.clone(),
        issue.assignee.clone(),
    ]
}

/// Status symbol for pretty/tree output (matches Go version).
pub fn status_symbol(status: &Status) -> &'static str {
    match status {
        Status::Open => "o",
        Status::InProgress => "~",
        Status::Blocked => "!",
        Status::Closed => "x",
        Status::Deferred => "*",
        Status::Pinned => "p",
        Status::Hooked => "^",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::issue::IssueBuilder;

    #[test]
    fn compact_format_basic() {
        let issue = IssueBuilder::new("Fix the bug")
            .id("bd-abc123")
            .priority(1)
            .build();
        let formatted = format_issue_compact(&issue);
        assert!(formatted.contains("bd-abc123"));
        assert!(formatted.contains("Fix the bug"));
        assert!(formatted.contains("[P1]"));
    }

    #[test]
    fn detail_format_includes_sections() {
        let issue = IssueBuilder::new("Fix the bug")
            .id("bd-abc123")
            .description("A detailed description")
            .priority(1)
            .assignee("alice")
            .build();
        let formatted = format_issue_detail(&issue);
        assert!(formatted.contains("DESCRIPTION"));
        assert!(formatted.contains("A detailed description"));
        assert!(formatted.contains("Assignee: alice"));
    }

    #[test]
    fn row_format_columns() {
        let issue = IssueBuilder::new("Test")
            .id("bd-xyz")
            .priority(2)
            .assignee("bob")
            .build();
        let row = format_issue_row(&issue);
        assert_eq!(row[0], "bd-xyz");
        assert_eq!(row[1], "P2");
        assert_eq!(row[5], "bob");
    }

    #[test]
    fn table_output_smoke() {
        // Just ensure it doesn't panic
        let headers = &["ID", "Priority", "Title"];
        let rows = vec![
            vec!["bd-1".into(), "P0".into(), "Critical bug".into()],
            vec!["bd-2".into(), "P2".into(), "Nice to have".into()],
        ];
        output_table(headers, &rows);
    }

    #[test]
    fn status_symbols() {
        assert_eq!(status_symbol(&Status::Open), "o");
        assert_eq!(status_symbol(&Status::Closed), "x");
        assert_eq!(status_symbol(&Status::InProgress), "~");
    }
}
