//! `bd onboard` -- display minimal snippet for AGENTS.md integration.

use anyhow::Result;
use beads_ui::styles::{render_accent, render_bold, render_pass};

use crate::context::RuntimeContext;

/// Content to add to AGENTS.md.
const AGENTS_CONTENT: &str = r#"## Issue Tracking

This project uses **bd (beads)** for issue tracking.
Run `bd prime` for workflow context, or install hooks (`bd hooks install`) for auto-injection.

**Quick reference:**
- `bd ready` - Find unblocked work
- `bd create "Title" --type task --priority 2` - Create issue
- `bd close <id>` - Complete work
- `bd sync` - Sync with git (run at session end)

For full workflow details: `bd prime`"#;

/// Execute the `bd onboard` command.
pub fn run(_ctx: &RuntimeContext) -> Result<()> {
    println!();
    println!("{}", render_bold("bd Onboarding"));
    println!();
    println!("Add this minimal snippet to AGENTS.md (or create it):");
    println!();
    println!("{}", render_accent("--- BEGIN AGENTS.MD CONTENT ---"));
    println!("{AGENTS_CONTENT}");
    println!("{}", render_accent("--- END AGENTS.MD CONTENT ---"));
    println!();
    println!("{}", render_bold("For GitHub Copilot users:"));
    println!("Add the same content to .github/copilot-instructions.md");
    println!();
    println!("{}", render_bold("How it works:"));
    println!(
        "   - {} provides dynamic workflow context (~80 lines)",
        render_accent("bd prime")
    );
    println!(
        "   - {} auto-injects bd prime at session start",
        render_accent("bd hooks install")
    );
    println!("   - AGENTS.md only needs this minimal pointer, not full instructions");
    println!();
    println!(
        "{}",
        render_pass("This keeps AGENTS.md lean while bd prime provides up-to-date workflow details.")
    );
    println!();

    Ok(())
}
