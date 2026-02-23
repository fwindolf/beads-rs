//! `bd template` -- template operations (list, show, create, delete, instantiate).
//!
//! Templates are issues with `is_template=1` that can be cloned with
//! `{{variable}}` substitution. Instantiation recursively clones the
//! template and its children (via parent-child dependencies), remapping
//! all dependencies between cloned issues.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use chrono::Utc;

use beads_core::enums::IssueType;
use beads_core::idgen;

use crate::cli::{TemplateArgs, TemplateCommands};
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd template` command.
pub fn run(ctx: &RuntimeContext, args: &TemplateArgs) -> Result<()> {
    match &args.command {
        TemplateCommands::List => cmd_list(ctx),
        TemplateCommands::Show(a) => cmd_show(ctx, &a.id),
        TemplateCommands::Create(a) => cmd_create(ctx, a),
        TemplateCommands::Delete(a) => cmd_delete(ctx, &a.id),
        TemplateCommands::Instantiate(a) => cmd_instantiate(ctx, a),
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

fn cmd_list(ctx: &RuntimeContext) -> Result<()> {
    let conn = open_db(ctx, false)?;

    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, issue_type, assignee, created_at, created_by, updated_at \
         FROM issues WHERE COALESCE(is_template, 0) = 1 \
         ORDER BY created_at DESC",
    )?;

    let templates: Vec<TemplateRow> = stmt
        .query_map([], |row| {
            Ok(TemplateRow {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get::<_, String>(2).unwrap_or_default(),
                status: row.get::<_, String>(3).unwrap_or_default(),
                priority: row.get(4)?,
                issue_type: row.get::<_, String>(5).unwrap_or_default(),
                assignee: row.get::<_, String>(6).unwrap_or_default(),
                created_at: row.get::<_, String>(7).unwrap_or_default(),
                created_by: row.get::<_, String>(8).unwrap_or_default(),
                updated_at: row.get::<_, String>(9).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        output_json(&templates);
    } else if templates.is_empty() {
        println!("No templates found.");
    } else {
        let headers = &["ID", "PRI", "TYPE", "TITLE"];
        let rows: Vec<Vec<String>> = templates
            .iter()
            .map(|t| {
                vec![
                    t.id.clone(),
                    format!("P{}", t.priority),
                    t.issue_type.clone(),
                    t.title.clone(),
                ]
            })
            .collect();
        output_table(headers, &rows);
    }

    Ok(())
}

/// Lightweight row for template listing/JSON.
#[derive(serde::Serialize)]
struct TemplateRow {
    id: String,
    title: String,
    description: String,
    status: String,
    priority: i32,
    issue_type: String,
    assignee: String,
    created_at: String,
    created_by: String,
    updated_at: String,
}

// ---------------------------------------------------------------------------
// Show
// ---------------------------------------------------------------------------

fn cmd_show(ctx: &RuntimeContext, id: &str) -> Result<()> {
    let conn = open_db(ctx, false)?;

    let row = load_template_row(&conn, id)?;

    // Extract variables from title and description
    let variables = extract_variables(&row.title, &row.description);

    if ctx.json {
        let mut val = serde_json::to_value(&row)?;
        if let Some(obj) = val.as_object_mut() {
            obj.insert("variables".to_string(), serde_json::to_value(&variables)?);
        }
        output_json(&val);
    } else {
        println!(
            "{} [P{}] [{}] {}",
            row.id, row.priority, row.issue_type, row.title
        );
        println!("Status: {}", row.status);
        if !row.assignee.is_empty() {
            println!("Assignee: {}", row.assignee);
        }
        println!("Created: {} by {}", row.created_at, row.created_by);
        if !row.description.is_empty() {
            println!("\nDESCRIPTION");
            println!("{}", row.description);
        }
        if variables.is_empty() {
            println!("\nNo template variables found.");
        } else {
            println!("\nVARIABLES");
            for var in &variables {
                println!("  {{{{{}}}}}", var);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

fn cmd_create(ctx: &RuntimeContext, args: &crate::cli::TemplateCreateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot create templates in read-only mode");
    }

    let conn = open_db(ctx, true)?;

    // Parse priority
    let priority = parse_priority(&args.priority)?;
    let issue_type = IssueType::from(args.issue_type.as_str()).normalize();

    // Get prefix
    let prefix: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'issue_prefix'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "bd".to_string());

    // Generate ID
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))
        .unwrap_or(0);

    let hash_length = idgen::compute_adaptive_length(
        count as usize,
        idgen::adaptive_defaults::MIN_LENGTH,
        idgen::adaptive_defaults::MAX_LENGTH,
        idgen::adaptive_defaults::MAX_COLLISION_PROB,
    );

    let description = args.description.as_deref().unwrap_or("");
    let now = Utc::now();

    let mut issue_id = String::new();
    for nonce in 0..10 {
        let candidate = idgen::generate_hash_id(
            &prefix,
            &args.title,
            description,
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
        if !exists {
            issue_id = candidate;
            break;
        }
    }

    if issue_id.is_empty() {
        bail!("failed to generate unique ID after 10 attempts");
    }

    let now_str = now.to_rfc3339();

    conn.execute(
        "INSERT INTO issues (id, title, description, status, priority, issue_type, is_template, created_at, created_by, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9)",
        rusqlite::params![
            &issue_id,
            &args.title,
            description,
            "open",
            priority,
            issue_type.as_str(),
            &now_str,
            &ctx.actor,
            &now_str,
        ],
    )
    .with_context(|| format!("failed to create template {}", issue_id))?;

    // Record event
    conn.execute(
        "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&issue_id, "created", &ctx.actor, &args.title, &now_str],
    )?;

    if ctx.json {
        output_json(&serde_json::json!({
            "id": issue_id,
            "title": args.title,
            "is_template": true,
        }));
    } else {
        println!("Created template: {}", issue_id);
        println!("  Title: {}", args.title);
        println!("  Priority: P{}", priority);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

fn cmd_delete(ctx: &RuntimeContext, id: &str) -> Result<()> {
    if ctx.readonly {
        bail!("cannot delete templates in read-only mode");
    }

    let conn = open_db(ctx, true)?;

    // Verify it's actually a template
    let is_template: bool = conn
        .query_row(
            "SELECT COALESCE(is_template, 0) FROM issues WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .with_context(|| format!("template '{}' not found", id))?;

    if !is_template {
        bail!("issue '{}' is not a template", id);
    }

    // Delete dependencies first (cascade should handle this, but be explicit)
    conn.execute(
        "DELETE FROM dependencies WHERE issue_id = ?1 OR depends_on_id = ?1",
        rusqlite::params![id],
    )?;
    conn.execute(
        "DELETE FROM labels WHERE issue_id = ?1",
        rusqlite::params![id],
    )?;
    conn.execute(
        "DELETE FROM comments WHERE issue_id = ?1",
        rusqlite::params![id],
    )?;
    conn.execute(
        "DELETE FROM events WHERE issue_id = ?1",
        rusqlite::params![id],
    )?;
    conn.execute("DELETE FROM issues WHERE id = ?1", rusqlite::params![id])?;

    if ctx.json {
        output_json(&serde_json::json!({ "deleted": id }));
    } else {
        println!("Deleted template: {}", id);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

fn cmd_instantiate(ctx: &RuntimeContext, args: &crate::cli::TemplateInstantiateArgs) -> Result<()> {
    if ctx.readonly {
        bail!("cannot instantiate templates in read-only mode");
    }

    let conn = open_db(ctx, true)?;

    // 1. Load the root template issue
    let root = load_template_row(&conn, &args.id)?;

    // 2. Recursively load all children via parent-child deps
    let mut all_ids: Vec<String> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    collect_children_recursive(&conn, &args.id, &mut all_ids, &mut visited)?;
    // Prepend root
    all_ids.insert(0, args.id.clone());

    // 3. Load all issues
    let mut templates: HashMap<String, TemplateRow> = HashMap::new();
    templates.insert(args.id.clone(), root);
    for child_id in &all_ids[1..] {
        let row = load_issue_row(&conn, child_id)?;
        templates.insert(child_id.clone(), row);
    }

    // 4. Extract all variables from all issues
    let mut all_vars: HashSet<String> = HashSet::new();
    for tmpl in templates.values() {
        for var in extract_variables(&tmpl.title, &tmpl.description) {
            all_vars.insert(var);
        }
    }

    // 5. Parse provided variables
    let mut var_map: HashMap<String, String> = HashMap::new();
    for v in &args.vars {
        let parts: Vec<&str> = v.splitn(2, '=').collect();
        if parts.len() != 2 {
            bail!("invalid variable format '{}': expected key=value", v);
        }
        var_map.insert(parts[0].to_string(), parts[1].to_string());
    }

    // 6. Validate all variables are provided
    let mut missing: Vec<String> = Vec::new();
    for var in &all_vars {
        if !var_map.contains_key(var) {
            missing.push(var.clone());
        }
    }
    if !missing.is_empty() {
        missing.sort();
        bail!(
            "missing template variables: {}\nProvide them with --var key=value",
            missing.join(", ")
        );
    }

    // 7. Determine prefix for new IDs
    let prefix: String = if let Some(ref p) = args.prefix {
        p.clone()
    } else {
        conn.query_row(
            "SELECT value FROM config WHERE key = 'issue_prefix'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "bd".to_string())
    };

    // 8. Clone each issue with substituted text and new IDs
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
    let mut id_map: HashMap<String, String> = HashMap::new(); // old_id -> new_id
    let mut created_issues: Vec<serde_json::Value> = Vec::new();

    for old_id in &all_ids {
        let tmpl = &templates[old_id];
        let new_title = substitute_variables(&tmpl.title, &var_map);
        let new_description = substitute_variables(&tmpl.description, &var_map);

        // Generate new ID
        let mut new_id = String::new();
        for nonce in 0..10 {
            let candidate = idgen::generate_hash_id(
                &prefix,
                &new_title,
                &new_description,
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
                new_id = candidate;
                break;
            }
        }

        if new_id.is_empty() {
            bail!(
                "failed to generate unique ID for cloned issue (template: {})",
                old_id
            );
        }

        id_map.insert(old_id.clone(), new_id.clone());

        conn.execute(
            "INSERT INTO issues (id, title, description, status, priority, issue_type, assignee, \
             is_template, created_at, created_by, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10)",
            rusqlite::params![
                &new_id,
                &new_title,
                &new_description,
                "open",
                tmpl.priority,
                &tmpl.issue_type,
                &tmpl.assignee,
                &now_str,
                &ctx.actor,
                &now_str,
            ],
        )
        .with_context(|| format!("failed to create cloned issue from template {}", old_id))?;

        // Record event
        conn.execute(
            "INSERT INTO events (issue_id, event_type, actor, new_value, comment, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![&new_id, "created", &ctx.actor, &new_title, &format!("Instantiated from template {}", old_id), &now_str],
        )?;

        created_issues.push(serde_json::json!({
            "id": new_id,
            "title": new_title,
            "from_template": old_id,
        }));
    }

    // 9. Recreate all dependencies between cloned issues (map old IDs to new IDs)
    let mut dep_stmt = conn.prepare(
        "SELECT issue_id, depends_on_id, type FROM dependencies \
         WHERE issue_id IN (SELECT value FROM json_each(?1)) \
            OR depends_on_id IN (SELECT value FROM json_each(?1))",
    )?;

    let all_ids_json = serde_json::to_string(&all_ids)?;
    let deps: Vec<(String, String, String)> = dep_stmt
        .query_map(rusqlite::params![&all_ids_json], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (from_id, to_id, dep_type) in &deps {
        // Only recreate deps where both sides are in our template set
        if let (Some(new_from), Some(new_to)) = (id_map.get(from_id), id_map.get(to_id)) {
            conn.execute(
                "INSERT OR IGNORE INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![new_from, new_to, dep_type, &now_str, &ctx.actor],
            )?;
        }
    }

    // 10. Copy labels for each cloned issue
    for old_id in &all_ids {
        if let Some(new_id) = id_map.get(old_id) {
            let mut label_stmt = conn.prepare("SELECT label FROM labels WHERE issue_id = ?1")?;
            let labels: Vec<String> = label_stmt
                .query_map(rusqlite::params![old_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            for label in labels {
                conn.execute(
                    "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                    rusqlite::params![new_id, &label],
                )?;
            }
        }
    }

    // Output
    if ctx.json {
        output_json(&serde_json::json!({
            "template": args.id,
            "created": created_issues,
            "variables": var_map,
        }));
    } else {
        println!(
            "Instantiated template {} -> {} issues:",
            args.id,
            created_issues.len()
        );
        for entry in &created_issues {
            println!(
                "  {} (from {}): {}",
                entry["id"].as_str().unwrap_or(""),
                entry["from_template"].as_str().unwrap_or(""),
                entry["title"].as_str().unwrap_or(""),
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract `{{variable_name}}` patterns from title and description.
///
/// Variable names must match `[a-zA-Z_][a-zA-Z0-9_]*`.
fn extract_variables(title: &str, description: &str) -> Vec<String> {
    let mut vars: HashSet<String> = HashSet::new();
    extract_vars_from_str(title, &mut vars);
    extract_vars_from_str(description, &mut vars);
    let mut result: Vec<String> = vars.into_iter().collect();
    result.sort();
    result
}

/// Scan a string for `{{name}}` patterns and insert variable names into the set.
fn extract_vars_from_str(text: &str, vars: &mut HashSet<String>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + 4 < len {
        // Look for "{{"
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i + 2;
            // First char must be letter or underscore
            if start < len && is_var_start(bytes[start]) {
                let mut end = start + 1;
                while end < len && is_var_cont(bytes[end]) {
                    end += 1;
                }
                // Must end with "}}"
                if end + 1 < len && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                    let name = &text[start..end];
                    vars.insert(name.to_string());
                    i = end + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn is_var_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_var_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Replace all `{{key}}` occurrences with the provided values.
fn substitute_variables(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if i + 4 <= len && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start < len && is_var_start(bytes[start]) {
                let mut end = start + 1;
                while end < len && is_var_cont(bytes[end]) {
                    end += 1;
                }
                if end + 1 < len && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                    let name = &text[start..end];
                    if let Some(val) = vars.get(name) {
                        result.push_str(val);
                    } else {
                        result.push_str(&text[i..end + 2]);
                    }
                    i = end + 2;
                    continue;
                }
            }
        }
        result.push(text.as_bytes()[i] as char);
        i += 1;
    }
    result
}

/// Recursively collect all children (via parent-child deps) of a given issue ID.
fn collect_children_recursive(
    conn: &rusqlite::Connection,
    parent_id: &str,
    result: &mut Vec<String>,
    visited: &mut HashSet<String>,
) -> Result<()> {
    if visited.contains(parent_id) {
        return Ok(());
    }
    visited.insert(parent_id.to_string());

    // Children are issues where: dep(child -> parent, type='parent-child')
    // i.e., child.issue_id has depends_on_id = parent_id with type='parent-child'
    let mut stmt = conn.prepare(
        "SELECT issue_id FROM dependencies WHERE depends_on_id = ?1 AND type = 'parent-child'",
    )?;

    let children: Vec<String> = stmt
        .query_map(rusqlite::params![parent_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for child_id in children {
        if !visited.contains(&child_id) {
            result.push(child_id.clone());
            collect_children_recursive(conn, &child_id, result, visited)?;
        }
    }

    Ok(())
}

/// Load a template issue row (verifies it's a template).
fn load_template_row(conn: &rusqlite::Connection, id: &str) -> Result<TemplateRow> {
    let row = load_issue_row(conn, id)?;

    // Verify it's actually a template
    let is_template: bool = conn
        .query_row(
            "SELECT COALESCE(is_template, 0) FROM issues WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !is_template {
        bail!("issue '{}' is not a template (is_template != 1)", id);
    }

    Ok(row)
}

/// Load an issue row by ID.
fn load_issue_row(conn: &rusqlite::Connection, id: &str) -> Result<TemplateRow> {
    conn.query_row(
        "SELECT id, title, description, status, priority, issue_type, assignee, created_at, created_by, updated_at \
         FROM issues WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(TemplateRow {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get::<_, String>(2).unwrap_or_default(),
                status: row.get::<_, String>(3).unwrap_or_default(),
                priority: row.get(4)?,
                issue_type: row.get::<_, String>(5).unwrap_or_default(),
                assignee: row.get::<_, String>(6).unwrap_or_default(),
                created_at: row.get::<_, String>(7).unwrap_or_default(),
                created_by: row.get::<_, String>(8).unwrap_or_default(),
                updated_at: row.get::<_, String>(9).unwrap_or_default(),
            })
        },
    )
    .with_context(|| format!("issue '{}' not found", id))
}

/// Parse a priority string (bare number or P-prefixed).
fn parse_priority(s: &str) -> Result<i32> {
    let s = s.trim();
    let num_str = if s.starts_with('P') || s.starts_with('p') {
        &s[1..]
    } else {
        s
    };
    let p: i32 = num_str
        .parse()
        .with_context(|| format!("invalid priority '{}': expected 0-4 or P0-P4", s))?;
    if !(0..=4).contains(&p) {
        bail!("priority must be between 0 and 4 (got {})", p);
    }
    Ok(p)
}

/// Open the beads database (read-only or read-write).
fn open_db(ctx: &RuntimeContext, writable: bool) -> Result<rusqlite::Connection> {
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

    if writable {
        rusqlite::Connection::open(&db_path)
            .with_context(|| format!("failed to open database: {}", db_path.display()))
    } else {
        rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("failed to open database: {}", db_path.display()))
    }
}
