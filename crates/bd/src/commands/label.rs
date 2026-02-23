//! `bd label` -- manage labels on an issue.

use anyhow::{Context, Result, bail};

use crate::cli::{LabelArgs, LabelCommands};
use crate::context::RuntimeContext;
use crate::output::{load_labels, output_json};

/// Execute the `bd label` command.
pub fn run(ctx: &RuntimeContext, args: &LabelArgs) -> Result<()> {
    let beads_dir = ctx
        .resolve_db_path()
        .context("no beads database found. Run 'bd init' to create one.")?;
    let db_path = beads_dir.join("beads.db");

    if !db_path.exists() {
        bail!(
            "no beads database found at {}\nHint: run 'bd init' to create a database",
            db_path.display()
        );
    }

    // Check if issue exists
    let conn = rusqlite::Connection::open(&db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            rusqlite::params![&args.id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !exists {
        bail!("issue '{}' not found", args.id);
    }

    match &args.command {
        LabelCommands::Add(add_args) => {
            if ctx.readonly {
                bail!("cannot add labels in read-only mode");
            }

            conn.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                rusqlite::params![&args.id, &add_args.label],
            )
            .with_context(|| format!("failed to add label '{}' to {}", add_args.label, args.id))?;

            if ctx.json {
                output_json(&serde_json::json!({
                    "status": "added",
                    "issue_id": args.id,
                    "label": add_args.label,
                }));
            } else if !ctx.quiet {
                println!("Added label '{}' to {}", add_args.label, args.id);
            }
        }

        LabelCommands::Remove(remove_args) => {
            if ctx.readonly {
                bail!("cannot remove labels in read-only mode");
            }

            let changes = conn.execute(
                "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
                rusqlite::params![&args.id, &remove_args.label],
            )?;

            if ctx.json {
                output_json(&serde_json::json!({
                    "status": "removed",
                    "issue_id": args.id,
                    "label": remove_args.label,
                    "removed": changes > 0,
                }));
            } else if changes > 0 {
                if !ctx.quiet {
                    println!("Removed label '{}' from {}", remove_args.label, args.id);
                }
            } else {
                eprintln!("Label '{}' not found on {}", remove_args.label, args.id);
            }
        }

        LabelCommands::List => {
            let labels = load_labels(&conn, &args.id);

            if ctx.json {
                output_json(&labels);
            } else if labels.is_empty() {
                println!("{} has no labels", args.id);
            } else {
                println!("Labels for {}:", args.id);
                for label in &labels {
                    println!("  - {}", label);
                }
            }
        }
    }

    Ok(())
}
