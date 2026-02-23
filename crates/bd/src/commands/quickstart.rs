//! `bd quickstart` -- display a quick-start guide for new users.

use anyhow::Result;
use beads_ui::styles::{render_accent, render_bold, render_pass, render_warn};

use crate::context::RuntimeContext;

/// Execute the `bd quickstart` command.
pub fn run(_ctx: &RuntimeContext) -> Result<()> {
    println!();
    println!("{}", render_bold("bd - Dependency-Aware Issue Tracker"));
    println!();
    println!("Issues chained together like beads.");
    println!();

    println!("{}", render_bold("GETTING STARTED"));
    println!("  {}   Initialize bd in your project", render_accent("bd init"));
    println!("            Creates .beads/ directory with project-specific database");
    println!(
        "            Auto-detects prefix from directory name (e.g., myapp-1, myapp-2)"
    );
    println!();
    println!(
        "  {}   Initialize with custom prefix",
        render_accent("bd init --prefix api")
    );
    println!("            Issues will be named: api-<hash> (e.g., api-a3f2dd)");
    println!();

    println!("{}", render_bold("CREATING ISSUES"));
    println!("  {}", render_accent("bd create \"Fix login bug\""));
    println!(
        "  {}",
        render_accent("bd create \"Add auth\" -p 0 -t feature")
    );
    println!(
        "  {}",
        render_accent("bd create \"Write tests\" -d \"Unit tests for auth\" --assignee alice")
    );
    println!();

    println!("{}", render_bold("VIEWING ISSUES"));
    println!("  {}       List all issues", render_accent("bd list"));
    println!(
        "  {}  List by status",
        render_accent("bd list --status open")
    );
    println!(
        "  {}  List by priority (0-4, 0=highest)",
        render_accent("bd list --priority 0")
    );
    println!("  {}       Show issue details", render_accent("bd show bd-1"));
    println!();

    println!("{}", render_bold("MANAGING DEPENDENCIES"));
    println!(
        "  {}     Add dependency (bd-2 blocks bd-1)",
        render_accent("bd dep add bd-1 bd-2")
    );
    println!(
        "  {}  Visualize dependency tree",
        render_accent("bd dep tree bd-1")
    );
    println!(
        "  {}      Detect circular dependencies",
        render_accent("bd dep cycles")
    );
    println!();

    println!("{}", render_bold("DEPENDENCY TYPES"));
    println!(
        "  {}  Task B must complete before task A",
        render_warn("blocks")
    );
    println!(
        "  {}  Soft connection, doesn't block progress",
        render_warn("related")
    );
    println!(
        "  {}  Epic/subtask hierarchical relationship",
        render_warn("parent-child")
    );
    println!(
        "  {}  Auto-created when AI discovers related work",
        render_warn("discovered-from")
    );
    println!();

    println!("{}", render_bold("READY WORK"));
    println!(
        "  {}       Show issues ready to work on",
        render_accent("bd ready")
    );
    println!("            Ready = status is 'open' AND no blocking dependencies");
    println!("            Perfect for agents to claim next work!");
    println!();

    println!("{}", render_bold("UPDATING ISSUES"));
    println!(
        "  {}",
        render_accent("bd update bd-1 --status in_progress")
    );
    println!("  {}", render_accent("bd update bd-1 --priority 0"));
    println!("  {}", render_accent("bd update bd-1 --assignee bob"));
    println!();

    println!("{}", render_bold("CLOSING ISSUES"));
    println!("  {}", render_accent("bd close bd-1"));
    println!(
        "  {}",
        render_accent("bd close bd-2 bd-3 --reason \"Fixed in PR #42\"")
    );
    println!();

    println!("{}", render_bold("DATABASE LOCATION"));
    println!("  bd automatically discovers your database:");
    println!("    1. {} flag", render_accent("--db /path/to/db.db"));
    println!("    2. {} environment variable", render_accent("$BEADS_DB"));
    println!(
        "    3. {} in current directory or ancestors",
        render_accent(".beads/*.db")
    );
    println!("    4. {} as fallback", render_accent("~/.beads/default.db"));
    println!();

    println!("{}", render_bold("AGENT INTEGRATION"));
    println!("  bd is designed for AI-supervised workflows:");
    println!("    - Agents create issues when discovering new work");
    println!(
        "    - {} shows unblocked work ready to claim",
        render_accent("bd ready")
    );
    println!(
        "    - Use {} flags for programmatic parsing",
        render_accent("--json")
    );
    println!("    - Dependencies prevent agents from duplicating effort");
    println!();

    println!("{}", render_bold("GIT WORKFLOW (AUTO-SYNC)"));
    println!("  bd automatically keeps git in sync:");
    println!(
        "    {} Export to JSONL after CRUD operations",
        render_pass("✓")
    );
    println!(
        "    {} Import from JSONL when newer than DB",
        render_pass("✓")
    );
    println!(
        "    {} Works seamlessly across machines and team members",
        render_pass("✓")
    );
    println!();

    println!("{}", render_pass("Ready to start!"));
    println!(
        "Run {} to create your first issue.",
        render_accent("bd create \"My first issue\"")
    );
    println!();

    Ok(())
}
