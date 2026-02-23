//! `bd cook` -- formula execution.
//!
//! Loads a formula file, substitutes variables, evaluates conditions,
//! and either previews the cooked steps (--dry-run) or creates issues
//! in the database.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use beads_core::enums::IssueType;
use beads_core::idgen;
use beads_formula::engine;
use beads_formula::parser;
use beads_formula::types::CookedStep;

use crate::cli::CookArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd cook` command.
pub fn run(ctx: &RuntimeContext, args: &CookArgs) -> Result<()> {
    let formula_name = args
        .formula
        .as_deref()
        .context("formula name or path is required")?;

    // 1. Find and load the formula
    let cwd = std::env::current_dir()?;
    let path = parser::find_formula(formula_name, &cwd)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let formula = parser::load_formula(&path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // 2. Parse --var flags into HashMap
    let vars = parse_var_flags(&args.vars)?;

    // 3. Cook the formula
    let cooked = engine::cook(&formula, &vars)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if cooked.is_empty() {
        println!("No steps to create (all filtered by conditions).");
        return Ok(());
    }

    // 4. Dry-run: print the cooked steps
    if args.dry_run || ctx.json {
        return print_cooked(ctx, &formula.formula, &cooked);
    }

    // 5. Create issues in the database
    create_issues(ctx, &formula.formula, &cooked, false)
}

/// Parse `--var key=value` flags into a HashMap.
pub(crate) fn parse_var_flags(vars: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for v in vars {
        let parts: Vec<&str> = v.splitn(2, '=').collect();
        if parts.len() != 2 {
            bail!("invalid variable format '{}': expected key=value", v);
        }
        map.insert(parts[0].to_string(), parts[1].to_string());
    }
    Ok(map)
}

/// Print cooked steps as a tree or JSON.
fn print_cooked(ctx: &RuntimeContext, formula_name: &str, steps: &[CookedStep]) -> Result<()> {
    if ctx.json {
        output_json(&serde_json::json!({
            "formula": formula_name,
            "steps": steps,
        }));
        return Ok(());
    }

    println!("Formula: {}", formula_name);
    println!("Steps ({}):", steps.len());
    for step in steps {
        let deps = if step.needs.is_empty() {
            String::new()
        } else {
            format!(" (needs: {})", step.needs.join(", "))
        };
        let gate_info = if let Some(ref g) = step.gate {
            format!(" [gate:{}]", g.r#type)
        } else {
            String::new()
        };
        let assignee_info = match &step.assignee {
            Some(a) if !a.is_empty() => format!(" @{}", a),
            _ => String::new(),
        };
        println!(
            "  {} [P{}] [{}] {}{}{}{}",
            step.id, step.priority, step.issue_type, step.title,
            deps, gate_info, assignee_info,
        );
    }
    Ok(())
}

/// Create issues in the database for each cooked step.
pub(crate) fn create_issues(
    ctx: &RuntimeContext,
    formula_name: &str,
    steps: &[CookedStep],
    ephemeral: bool,
) -> Result<()> {
    if ctx.readonly {
        bail!("cannot create issues in read-only mode");
    }

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

    let conn = rusqlite::Connection::open(&db_path)
        .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    let prefix: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'issue_prefix'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "bd".to_string());

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))
        .unwrap_or(0);

    let hash_length = idgen::compute_adaptive_length(
        count as usize,
        idgen::adaptive_defaults::MIN_LENGTH,
        idgen::adaptive_defaults::MAX_LENGTH,
        idgen::adaptive_defaults::MAX_COLLISION_PROB,
    );

    let now = Utc::now();
    let now_str = now.to_rfc3339();

    // Map step IDs to issue IDs
    let mut id_map: HashMap<String, String> = HashMap::new();
    let mut created: Vec<serde_json::Value> = Vec::new();

    for step in steps {
        let issue_type = IssueType::from(step.issue_type.as_str()).normalize();

        // Generate unique issue ID
        let mut issue_id = String::new();
        for nonce in 0..10 {
            let candidate = idgen::generate_hash_id(
                &prefix,
                &step.title,
                &step.description,
                &ctx.actor,
                now,
                hash_length,
                nonce,
            );
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                    rusqlite::params![&candidate],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if !exists && !id_map.values().any(|v| v == &candidate) {
                issue_id = candidate;
                break;
            }
        }
        if issue_id.is_empty() {
            bail!("failed to generate unique ID for step '{}'", step.id);
        }

        let ephemeral_flag: i32 = if ephemeral { 1 } else { 0 };

        conn.execute(
            "INSERT INTO issues (id, title, description, status, priority, issue_type, assignee, \
             is_template, ephemeral, created_at, created_by, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                &issue_id,
                &step.title,
                &step.description,
                "open",
                step.priority,
                issue_type.as_str(),
                step.assignee.as_deref().unwrap_or(""),
                ephemeral_flag,
                &now_str,
                &ctx.actor,
                &now_str,
            ],
        )
        .with_context(|| format!("failed to create issue for step '{}'", step.id))?;

        // Record event
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, new_value, comment, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &issue_id,
                "created",
                &ctx.actor,
                &step.title,
                &format!("Cooked from formula {} step {}", formula_name, step.id),
                &now_str,
            ],
        )?;

        // Add labels
        for label in &step.labels {
            conn.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                rusqlite::params![&issue_id, label],
            )?;
        }

        // Add formula source label
        conn.execute(
            "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
            rusqlite::params![&issue_id, &format!("formula:{}", formula_name)],
        )?;

        if ephemeral {
            conn.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                rusqlite::params![&issue_id, "ephemeral"],
            )?;
        }

        id_map.insert(step.id.clone(), issue_id.clone());
        created.push(serde_json::json!({
            "id": issue_id,
            "step": step.id,
            "title": step.title,
        }));
    }

    // Create dependencies (needs -> blocks)
    for step in steps {
        if let Some(issue_id) = id_map.get(&step.id) {
            for need in &step.needs {
                if let Some(dep_id) = id_map.get(need) {
                    conn.execute(
                        "INSERT OR IGNORE INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) \
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![issue_id, dep_id, "blocks", &now_str, &ctx.actor],
                    )?;
                }
            }
        }
    }

    // Output
    if ctx.json {
        output_json(&serde_json::json!({
            "formula": formula_name,
            "ephemeral": ephemeral,
            "created": created,
        }));
    } else {
        let mode = if ephemeral { "wisp" } else { "pour" };
        println!(
            "Cooked formula '{}' ({} mode) -> {} issues:",
            formula_name, mode, created.len()
        );
        for entry in &created {
            println!(
                "  {} (step {}): {}",
                entry["id"].as_str().unwrap_or(""),
                entry["step"].as_str().unwrap_or(""),
                entry["title"].as_str().unwrap_or(""),
            );
        }
    }

    Ok(())
}
