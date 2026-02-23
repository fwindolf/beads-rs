//! `bd mol` -- molecule operations.
//!
//! Implements:
//! - `pour`: create persistent issues from a formula
//! - `wisp`: create ephemeral issues from a formula
//! - `show`: display a molecule (formula-created issue set) and its children
//! - `progress`: show completion progress for a molecule
//!
//! Other subcommands remain stubs.

use anyhow::{Context, Result, bail};

use beads_formula::engine;
use beads_formula::parser;

use crate::cli::{MolArgs, MolCommands};
use crate::commands::cook::{create_issues, parse_var_flags};
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd mol` command.
pub fn run(ctx: &RuntimeContext, args: &MolArgs) -> Result<()> {
    match &args.command {
        MolCommands::Pour(a) => cmd_pour(ctx, a),
        MolCommands::Wisp(a) => cmd_wisp(ctx, a),
        MolCommands::Show(a) => cmd_show(ctx, a),
        MolCommands::Progress(a) => cmd_progress(ctx, a),
        MolCommands::Bond(_) => stub("bond"),
        MolCommands::Squash(_) => stub("squash"),
        MolCommands::Burn(_) => stub("burn"),
        MolCommands::Distill(_) => stub("distill"),
        MolCommands::Seed(_) => stub("seed"),
        MolCommands::Stale(_) => stub("stale"),
        MolCommands::ReadyGated(_) => stub("ready-gated"),
        MolCommands::Current(_) => stub("current"),
    }
}

fn stub(name: &str) -> Result<()> {
    println!("bd mol {}: not yet implemented", name);
    Ok(())
}

// ---------------------------------------------------------------------------
// Pour
// ---------------------------------------------------------------------------

fn cmd_pour(ctx: &RuntimeContext, args: &crate::cli::MolPourArgs) -> Result<()> {
    let formula_name = args
        .id
        .as_deref()
        .context("formula name or path is required")?;

    let cwd = std::env::current_dir()?;
    let path = parser::find_formula(formula_name, &cwd).map_err(|e| anyhow::anyhow!("{}", e))?;
    let formula = parser::load_formula(&path).map_err(|e| anyhow::anyhow!("{}", e))?;

    let vars = parse_var_flags(&args.vars)?;
    let cooked = engine::cook(&formula, &vars).map_err(|e| anyhow::anyhow!("{}", e))?;

    if cooked.is_empty() {
        println!("No steps to create (all filtered by conditions).");
        return Ok(());
    }

    if args.dry_run {
        return print_pour_preview(&formula.formula, &cooked, false);
    }

    create_issues(ctx, &formula.formula, &cooked, false)
}

// ---------------------------------------------------------------------------
// Wisp
// ---------------------------------------------------------------------------

fn cmd_wisp(ctx: &RuntimeContext, args: &crate::cli::MolWispArgs) -> Result<()> {
    let formula_name = args
        .id
        .as_deref()
        .context("formula name or path is required")?;

    let cwd = std::env::current_dir()?;
    let path = parser::find_formula(formula_name, &cwd).map_err(|e| anyhow::anyhow!("{}", e))?;
    let formula = parser::load_formula(&path).map_err(|e| anyhow::anyhow!("{}", e))?;

    let vars = parse_var_flags(&args.vars)?;
    let cooked = engine::cook(&formula, &vars).map_err(|e| anyhow::anyhow!("{}", e))?;

    if cooked.is_empty() {
        println!("No steps to create (all filtered by conditions).");
        return Ok(());
    }

    if args.dry_run {
        return print_pour_preview(&formula.formula, &cooked, true);
    }

    create_issues(ctx, &formula.formula, &cooked, true)
}

fn print_pour_preview(
    formula_name: &str,
    steps: &[beads_formula::types::CookedStep],
    ephemeral: bool,
) -> Result<()> {
    let mode = if ephemeral { "wisp" } else { "pour" };
    println!("Formula: {} ({})", formula_name, mode);
    println!("Steps ({}):", steps.len());
    for step in steps {
        let deps = if step.needs.is_empty() {
            String::new()
        } else {
            format!(" (needs: {})", step.needs.join(", "))
        };
        println!(
            "  {} [P{}] [{}] {}{}",
            step.id, step.priority, step.issue_type, step.title, deps,
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Show
// ---------------------------------------------------------------------------

fn cmd_show(ctx: &RuntimeContext, args: &crate::cli::MolShowArgs) -> Result<()> {
    let id = args.id.as_deref().context("molecule ID is required")?;

    let conn = open_db(ctx)?;

    // Find all issues with the formula:<id> label
    let label = format!("formula:{}", id);
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.status, i.priority, i.issue_type \
         FROM issues i \
         JOIN labels l ON i.id = l.issue_id \
         WHERE l.label = ?1 \
         ORDER BY i.created_at ASC",
    )?;

    let issues: Vec<(String, String, String, i32, String)> = stmt
        .query_map(rusqlite::params![&label], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, String>(2).unwrap_or_default(),
                row.get(3)?,
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if issues.is_empty() {
        bail!("no issues found for molecule '{}'", id);
    }

    if ctx.json {
        let items: Vec<serde_json::Value> = issues
            .iter()
            .map(|(id, title, status, pri, itype)| {
                serde_json::json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": pri,
                    "type": itype,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "molecule": id,
            "issues": items,
        }));
    } else {
        println!("Molecule: {}", id);
        println!("Issues ({}):", issues.len());
        let headers = &["ID", "PRI", "STATUS", "TYPE", "TITLE"];
        let rows: Vec<Vec<String>> = issues
            .iter()
            .map(|(id, title, status, pri, itype)| {
                vec![
                    id.clone(),
                    format!("P{}", pri),
                    status.clone(),
                    itype.clone(),
                    title.clone(),
                ]
            })
            .collect();
        output_table(headers, &rows);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

fn cmd_progress(ctx: &RuntimeContext, args: &crate::cli::MolProgressArgs) -> Result<()> {
    let id = args.id.as_deref().context("molecule ID is required")?;

    let conn = open_db(ctx)?;

    let label = format!("formula:{}", id);

    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM labels WHERE label = ?1",
        rusqlite::params![&label],
        |row| row.get(0),
    )?;

    if total == 0 {
        bail!("no issues found for molecule '{}'", id);
    }

    let closed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM issues i \
         JOIN labels l ON i.id = l.issue_id \
         WHERE l.label = ?1 AND i.status = 'closed'",
        rusqlite::params![&label],
        |row| row.get(0),
    )?;

    let pct = if total > 0 {
        (closed as f64 / total as f64 * 100.0) as i32
    } else {
        0
    };

    if ctx.json {
        output_json(&serde_json::json!({
            "molecule": id,
            "total": total,
            "closed": closed,
            "open": total - closed,
            "percent": pct,
        }));
    } else {
        println!(
            "Molecule '{}': {}/{} steps complete ({}%)",
            id, closed, total, pct,
        );
        // Simple progress bar
        let bar_width = 30;
        let filled = (pct as usize * bar_width) / 100;
        let empty = bar_width - filled;
        println!("  [{}{}]", "#".repeat(filled), "-".repeat(empty),);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_db(ctx: &RuntimeContext) -> Result<rusqlite::Connection> {
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

    rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))
}
