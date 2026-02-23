//! `bd prime` -- output AI-optimized workflow context.
//!
//! Detects environment (MCP vs CLI) and adapts output accordingly.
//! Designed for Claude Code hooks to prevent agents from forgetting
//! bd workflow after context compaction.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;

use crate::cli::PrimeArgs;
use crate::context::RuntimeContext;

/// Execute the `bd prime` command.
pub fn run(_ctx: &RuntimeContext, args: &PrimeArgs) -> Result<()> {
    // Find .beads/ directory
    let beads_dir = RuntimeContext::find_beads_dir();
    if beads_dir.is_none() {
        // Not in a beads project - silent exit with success
        return Ok(());
    }
    let beads_dir = beads_dir.unwrap();

    // Detect MCP mode (unless overridden by flags)
    let mut mcp_mode = is_mcp_active();
    if args.full {
        mcp_mode = false;
    }
    if args.mcp {
        mcp_mode = true;
    }

    let stealth_mode = args.stealth;

    // Check for custom PRIME.md override (unless --export flag)
    if !args.export {
        let local_prime = PathBuf::from(".beads/PRIME.md");
        if let Ok(content) = std::fs::read_to_string(&local_prime) {
            print!("{content}");
            return Ok(());
        }
        let redirected_prime = beads_dir.join("PRIME.md");
        if redirected_prime != local_prime {
            if let Ok(content) = std::fs::read_to_string(&redirected_prime) {
                print!("{content}");
                return Ok(());
            }
        }
    }

    let stdout = io::stdout();
    let mut w = stdout.lock();
    output_prime_context(&mut w, mcp_mode, stealth_mode)?;
    Ok(())
}

/// Detect if MCP server is active by checking Claude settings.
fn is_mcp_active() -> bool {
    let home = match dirs_home() {
        Some(h) => h,
        None => return false,
    };

    let settings_path = home.join(".claude").join("settings.json");
    let data = match std::fs::read_to_string(&settings_path) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let settings: serde_json::Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(mcp_servers) = settings.get("mcpServers").and_then(|v| v.as_object()) {
        for key in mcp_servers.keys() {
            if key.to_lowercase().contains("beads") {
                return true;
            }
        }
    }
    false
}

/// Get user home directory.
fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Check if current branch has no upstream (ephemeral).
fn is_ephemeral_branch() -> bool {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .output();
    match output {
        Ok(o) => !o.status.success(),
        Err(_) => true,
    }
}

/// Check if any git remote is configured.
fn has_git_remote() -> bool {
    let output = std::process::Command::new("git").args(["remote"]).output();
    match output {
        Ok(o) => !String::from_utf8_lossy(&o.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

/// Output workflow context adapted to the environment.
fn output_prime_context(w: &mut impl Write, mcp_mode: bool, stealth_mode: bool) -> Result<()> {
    if mcp_mode {
        output_mcp_context(w, stealth_mode)
    } else {
        output_cli_context(w, stealth_mode)
    }
}

/// Minimal context for MCP users.
fn output_mcp_context(w: &mut impl Write, stealth_mode: bool) -> Result<()> {
    let ephemeral = is_ephemeral_branch();
    let local_only = !has_git_remote();

    let close_protocol = if stealth_mode || local_only {
        "Before saying \"done\": bd sync --flush-only"
    } else if ephemeral {
        "Before saying \"done\": git status -> git add -> bd sync -> git commit (no push - ephemeral branch)"
    } else {
        "Before saying \"done\": git status -> git add -> bd sync -> git commit -> bd sync -> git push"
    };

    write!(
        w,
        r#"# Beads Issue Tracker Active

# SESSION CLOSE PROTOCOL

{close_protocol}

## Core Rules
- **Default**: Use beads for ALL task tracking (`bd create`, `bd ready`, `bd close`)
- **Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking
- **Workflow**: Create beads issue BEFORE writing code, mark in_progress when starting
- Persistence you don't need beats lost context

Start: Check `bd ready` for available work.
"#
    )?;
    Ok(())
}

/// Full CLI reference for non-MCP users.
fn output_cli_context(w: &mut impl Write, stealth_mode: bool) -> Result<()> {
    let ephemeral = is_ephemeral_branch();
    let local_only = !has_git_remote();

    let (close_protocol, close_note, sync_section, completing_workflow, git_workflow_rule) =
        if stealth_mode || local_only {
            (
                "[ ] bd sync --flush-only    (export beads to JSONL only)",
                if local_only && !stealth_mode {
                    "**Note:** No git remote configured. Issues are saved locally only."
                } else {
                    ""
                },
                "### Sync & Collaboration\n- `bd sync --flush-only` - Export to JSONL",
                "```bash\nbd close <id1> <id2> ...    # Close all completed issues at once\nbd sync --flush-only        # Export to JSONL\n```",
                if local_only && !stealth_mode {
                    "Git workflow: local-only (no git remote)"
                } else {
                    "Git workflow: stealth mode (no git ops)"
                },
            )
        } else if ephemeral {
            (
                "[ ] 1. git status              (check what changed)\n[ ] 2. git add <files>         (stage code changes)\n[ ] 3. bd sync     (pull beads updates from main)\n[ ] 4. git commit -m \"...\"     (commit code changes)",
                "**Note:** This is an ephemeral branch (no upstream). Code is merged to main locally, not pushed.",
                "### Sync & Collaboration\n- `bd sync` - Pull beads updates from main (for ephemeral branches)\n- `bd sync --status` - Check sync status without syncing",
                "```bash\nbd close <id1> <id2> ...    # Close all completed issues at once\nbd sync         # Pull latest beads from main\ngit add . && git commit -m \"...\"  # Commit your changes\n# Merge to main when ready (local merge, not push)\n```",
                "Git workflow: run `bd sync` at session end",
            )
        } else {
            (
                "[ ] 1. git status              (check what changed)\n[ ] 2. git add <files>         (stage code changes)\n[ ] 3. bd sync                 (commit beads changes)\n[ ] 4. git commit -m \"...\"     (commit code)\n[ ] 5. bd sync                 (commit any new beads changes)\n[ ] 6. git push                (push to remote)",
                "**NEVER skip this.** Work is not done until pushed.",
                "### Sync & Collaboration\n- `bd sync` - Sync with git remote (run at session end)\n- `bd sync --status` - Check sync status without syncing",
                "```bash\nbd close <id1> <id2> ...    # Close all completed issues at once\nbd sync                     # Push to remote\n```",
                "Git workflow: hooks auto-sync, run `bd sync` at session end",
            )
        };

    write!(
        w,
        r#"# Beads Workflow Context

> **Context Recovery**: Run `bd prime` after compaction, clear, or new session
> Hooks auto-call this in Claude Code when .beads/ detected

# SESSION CLOSE PROTOCOL

**CRITICAL**: Before saying "done" or "complete", you MUST run this checklist:

```
{close_protocol}
```

{close_note}

## Core Rules
- **Default**: Use beads for ALL task tracking (`bd create`, `bd ready`, `bd close`)
- **Prohibited**: Do NOT use TodoWrite, TaskCreate, or markdown files for task tracking
- **Workflow**: Create beads issue BEFORE writing code, mark in_progress when starting
- Persistence you don't need beats lost context
- {git_workflow_rule}
- Session management: check `bd ready` for available work

## Essential Commands

### Finding Work
- `bd ready` - Show issues ready to work (no blockers)
- `bd list --status=open` - All open issues
- `bd list --status=in_progress` - Your active work
- `bd show <id>` - Detailed issue view with dependencies

### Creating & Updating
- `bd create --title="Summary" --description="Details" --type=task|bug|feature --priority=2` - New issue
  - Priority: 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog). NOT "high"/"medium"/"low"
- `bd update <id> --status=in_progress` - Claim work
- `bd update <id> --assignee=username` - Assign to someone
- `bd update <id> --title/--description/--notes/--design` - Update fields inline
- `bd close <id>` - Mark complete
- `bd close <id1> <id2> ...` - Close multiple issues at once (more efficient)
- `bd close <id> --reason="explanation"` - Close with reason
- **WARNING**: Do NOT use `bd edit` - it opens $EDITOR which blocks agents

### Dependencies & Blocking
- `bd dep add <issue> <depends-on>` - Add dependency
- `bd blocked` - Show all blocked issues
- `bd show <id>` - See what's blocking/blocked by this issue

{sync_section}

### Project Health
- `bd stats` - Project statistics (open/closed/blocked counts)
- `bd doctor` - Check for issues (sync problems, missing hooks)

## Common Workflows

**Starting work:**
```bash
bd ready           # Find available work
bd show <id>       # Review issue details
bd update <id> --status=in_progress  # Claim it
```

**Completing work:**
{completing_workflow}

**Creating dependent work:**
```bash
bd create --title="Implement feature X" --type=feature
bd create --title="Write tests for X" --type=task
bd dep add <tests-id> <feature-id>  # Tests depend on Feature
```
"#
    )?;
    Ok(())
}
