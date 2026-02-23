//! `bd onboard` -- write onboarding content to agent instruction file.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::OnboardArgs;
use crate::context::RuntimeContext;

const ONBOARD_BEGIN_MARKER: &str = "<!-- BEGIN BD ONBOARD -->";
const ONBOARD_END_MARKER: &str = "<!-- END BD ONBOARD -->";

const ONBOARD_CONTENT: &str = r#"## Knowledge
This project uses mulch to build knowledge across different domains in this project.
At session start, use `mulch prime` or `mulch prime --files <path>` to regain the knowledge.
At session end, or when an important decision is made, use `mulch learn` to remember how to persist knowledge.
Use `mulch record <domain> --type <convention|pattern|decision|failure|reference|guide> --description <...>` and `mulch sync` to persist.

## Tasks
This project uses bd (beads) for task tracking. Use it for all tasks (`bd create`, `bd ready`, `bd close`).
Create beads BEFORE writing code, and always mark work you're tackling as in_progress.
Use `bd show <id>` for detailed issue view with dependencies. If a task is blocked
To create use `bd create --title="Summary" --description="Details" --type=task|bug|feature --priority=2` Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog). NOT "high"/"medium"/"low"
To close use `bd close <id1> <id2> ...` or `bd close <id> --reason="Explanation"`.
Run `bd prime` for a full overview."#;

/// Auto-discovery order for target files.
const TARGET_FILES: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    ".github/copilot-instructions.md",
    "CODEX.md",
    ".opencode/instructions.md",
];

fn build_onboard_section() -> String {
    format!(
        "{}\n\n{}\n\n{}\n",
        ONBOARD_BEGIN_MARKER, ONBOARD_CONTENT, ONBOARD_END_MARKER
    )
}

/// Resolve which file to target based on flags or auto-discovery.
fn resolve_target_file(args: &OnboardArgs) -> PathBuf {
    if args.agents {
        return PathBuf::from("AGENTS.md");
    }
    if args.claude {
        return PathBuf::from("CLAUDE.md");
    }
    if args.copilot {
        return PathBuf::from(".github/copilot-instructions.md");
    }
    if args.codex {
        return PathBuf::from("CODEX.md");
    }
    if args.opencode {
        return PathBuf::from(".opencode/instructions.md");
    }
    // Auto-discover: pick the first existing file
    for target in TARGET_FILES {
        if Path::new(target).exists() {
            return PathBuf::from(target);
        }
    }
    // Default
    PathBuf::from("AGENTS.md")
}

/// Replace the onboard section between markers in existing content.
fn replace_onboard_section(content: &str, section: &str) -> String {
    let start = match content.find(ONBOARD_BEGIN_MARKER) {
        Some(pos) => pos,
        None => return format!("{}\n\n{}", content, section),
    };
    let end = match content.find(ONBOARD_END_MARKER) {
        Some(pos) => pos,
        None => return format!("{}\n\n{}", content, section),
    };
    if start > end {
        return format!("{}\n\n{}", content, section);
    }

    let mut end_of_marker = end + ONBOARD_END_MARKER.len();
    // Find the next newline after end marker
    if let Some(nl) = content[end_of_marker..].find('\n') {
        end_of_marker += nl + 1;
    }

    format!(
        "{}{}{}",
        &content[..start],
        section,
        &content[end_of_marker..]
    )
}

/// Strip the onboard section from content (for removal).
fn strip_onboard_section(content: &str) -> String {
    let start = match content.find(ONBOARD_BEGIN_MARKER) {
        Some(pos) => pos,
        None => return content.to_string(),
    };
    let end = match content.find(ONBOARD_END_MARKER) {
        Some(pos) => pos,
        None => return content.to_string(),
    };
    if start > end {
        return content.to_string();
    }

    let mut end_of_marker = end + ONBOARD_END_MARKER.len();
    if let Some(nl) = content[end_of_marker..].find('\n') {
        end_of_marker += nl + 1;
    }

    // Remove leading blank lines before the section
    let mut trim_start = start;
    while trim_start > 0
        && (content.as_bytes()[trim_start - 1] == b'\n'
            || content.as_bytes()[trim_start - 1] == b'\r')
    {
        trim_start -= 1;
    }

    format!("{}{}", &content[..trim_start], &content[end_of_marker..])
}

/// Write or update the onboard section in the given file.
fn write_onboard_section(path: &Path) -> Result<()> {
    let section = build_onboard_section();

    let new_content = if path.exists() {
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        if content.contains(ONBOARD_BEGIN_MARKER) {
            replace_onboard_section(&content, &section)
        } else {
            format!("{}\n\n{}", content, section)
        }
    } else {
        // Create parent dir if needed
        if let Some(parent) = path.parent() {
            if parent != Path::new("") && parent != Path::new(".") {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create directory {}", parent.display()))?;
            }
        }
        section
    };

    fs::write(path, &new_content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Remove the onboard section from the given file.
fn remove_onboard_section(path: &Path) -> Result<()> {
    if !path.exists() {
        println!("No {} found", path.display());
        return Ok(());
    }

    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

    if !content.contains(ONBOARD_BEGIN_MARKER) {
        println!("No onboard section found in {}", path.display());
        return Ok(());
    }

    let new_content = strip_onboard_section(&content);
    if new_content.trim().is_empty() {
        fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
        println!(
            "Removed {} (file was empty after removing onboard section)",
            path.display()
        );
        return Ok(());
    }

    fs::write(path, &new_content).with_context(|| format!("write {}", path.display()))?;
    println!("Removed onboard section from {}", path.display());
    Ok(())
}

/// Check if the onboard section is installed in the given file.
fn check_onboard_section(path: &Path) -> Result<()> {
    if !path.exists() {
        println!("\u{2717} {} not found", path.display());
        anyhow::bail!("{} not found", path.display());
    }

    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

    if content.contains(ONBOARD_BEGIN_MARKER) {
        println!("\u{2713} Onboard section installed in {}", path.display());
        Ok(())
    } else {
        println!("\u{2717} No onboard section in {}", path.display());
        anyhow::bail!("no onboard section in {}", path.display());
    }
}

/// Execute the `bd onboard` command.
pub fn run(_ctx: &RuntimeContext, args: &OnboardArgs) -> Result<()> {
    let target = resolve_target_file(args);

    if args.check {
        return check_onboard_section(&target);
    }

    if args.remove {
        remove_onboard_section(&target)?;
        return Ok(());
    }

    write_onboard_section(&target)?;
    println!("Wrote onboarding to {}", target.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn default_args() -> OnboardArgs {
        OnboardArgs {
            auto: false,
            agents: false,
            claude: false,
            copilot: false,
            codex: false,
            opencode: false,
            check: false,
            remove: false,
        }
    }

    #[test]
    fn build_section_has_markers() {
        let section = build_onboard_section();
        assert!(section.starts_with(ONBOARD_BEGIN_MARKER));
        assert!(section.contains(ONBOARD_END_MARKER));
        assert!(section.contains("mulch prime"));
        assert!(section.contains("bd create"));
        assert!(section.contains("bd prime"));
    }

    #[test]
    fn resolve_explicit_flags() {
        let mut args = default_args();
        args.agents = true;
        assert_eq!(resolve_target_file(&args), PathBuf::from("AGENTS.md"));

        let mut args = default_args();
        args.claude = true;
        assert_eq!(resolve_target_file(&args), PathBuf::from("CLAUDE.md"));

        let mut args = default_args();
        args.copilot = true;
        assert_eq!(
            resolve_target_file(&args),
            PathBuf::from(".github/copilot-instructions.md")
        );

        let mut args = default_args();
        args.codex = true;
        assert_eq!(resolve_target_file(&args), PathBuf::from("CODEX.md"));
    }

    #[test]
    fn write_creates_new_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");

        write_onboard_section(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(ONBOARD_BEGIN_MARKER));
        assert!(content.contains(ONBOARD_END_MARKER));
        assert!(content.contains("mulch prime"));
    }

    #[test]
    fn write_appends_to_existing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("AGENTS.md");
        fs::write(&path, "# My Project\n\nExisting content.").unwrap();

        write_onboard_section(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# My Project"));
        assert!(content.contains(ONBOARD_BEGIN_MARKER));
    }

    #[test]
    fn write_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");

        write_onboard_section(&path).unwrap();
        let first = fs::read_to_string(&path).unwrap();

        write_onboard_section(&path).unwrap();
        let second = fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn write_creates_parent_dir() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".github").join("copilot-instructions.md");

        write_onboard_section(&path).unwrap();

        assert!(tmp.path().join(".github").exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(ONBOARD_BEGIN_MARKER));
    }

    #[test]
    fn write_replaces_in_surrounding_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("AGENTS.md");
        let section = build_onboard_section();
        let initial = format!("# Header\n\n{}# Footer\n", section);
        fs::write(&path, &initial).unwrap();

        write_onboard_section(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Header"));
        assert!(content.contains("# Footer"));
        assert_eq!(
            content.matches(ONBOARD_BEGIN_MARKER).count(),
            1,
            "should have exactly one begin marker"
        );
    }

    #[test]
    fn remove_strips_section() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        let section = build_onboard_section();
        let content = format!("# Header\n\n{}\n# Footer\n", section);
        fs::write(&path, &content).unwrap();

        remove_onboard_section(&path).unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(!result.contains(ONBOARD_BEGIN_MARKER));
        assert!(result.contains("# Header"));
        assert!(result.contains("# Footer"));
    }

    #[test]
    fn remove_deletes_empty_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("AGENTS.md");
        fs::write(&path, build_onboard_section()).unwrap();

        remove_onboard_section(&path).unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn remove_noop_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("NONEXISTENT.md");

        // Should not error
        remove_onboard_section(&path).unwrap();
    }

    #[test]
    fn remove_noop_no_markers() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "# Just some content\n").unwrap();

        remove_onboard_section(&path).unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert_eq!(result, "# Just some content\n");
    }

    #[test]
    fn strip_section_middle() {
        let section = build_onboard_section();
        let content = format!("before\n\n{}after\n", section);
        let result = strip_onboard_section(&content);

        assert!(!result.contains(ONBOARD_BEGIN_MARKER));
        assert!(result.contains("before"));
        assert!(result.contains("after"));
    }

    #[test]
    fn strip_no_markers() {
        let content = "no markers here\n";
        assert_eq!(strip_onboard_section(content), content);
    }

    #[test]
    fn replace_section_existing() {
        let section = build_onboard_section();
        let old = format!(
            "header\n{}\nold content\n{}\nfooter\n",
            ONBOARD_BEGIN_MARKER, ONBOARD_END_MARKER
        );
        let result = replace_onboard_section(&old, &section);

        assert_eq!(result.matches(ONBOARD_BEGIN_MARKER).count(), 1);
        assert!(result.contains("header"));
        assert!(result.contains("footer"));
        assert!(!result.contains("old content"));
    }

    #[test]
    fn check_installed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, build_onboard_section()).unwrap();

        assert!(check_onboard_section(&path).is_ok());
    }

    #[test]
    fn check_not_installed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "# No onboard\n").unwrap();

        assert!(check_onboard_section(&path).is_err());
    }

    #[test]
    fn check_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("NONEXISTENT.md");

        assert!(check_onboard_section(&path).is_err());
    }

    #[test]
    fn replace_appends_when_no_markers() {
        let section = build_onboard_section();
        let content = "no markers\n";
        let result = replace_onboard_section(content, &section);

        assert!(result.contains("no markers"));
        assert!(result.contains(ONBOARD_BEGIN_MARKER));
    }
}
