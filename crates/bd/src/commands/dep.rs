//! `bd dep` -- dependency management (add/remove/list/cycles/parents/children).

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use chrono::Utc;

use beads_core::enums::DependencyType;

use crate::cli::{DepArgs, DepCommands};
use crate::context::RuntimeContext;
use crate::output::{output_json, output_table};

/// Execute the `bd dep` command.
pub fn run(ctx: &RuntimeContext, args: &DepArgs) -> Result<()> {
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

    match &args.command {
        DepCommands::Add(add_args) => {
            if ctx.readonly {
                bail!("cannot add dependencies in read-only mode");
            }

            let conn = rusqlite::Connection::open(&db_path)
                .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            // Validate dependency type
            let dep_type = DependencyType::from(add_args.dep_type.as_str());
            if !dep_type.is_valid() {
                bail!(
                    "invalid dependency type '{}' (valid: blocks, related, parent-child, discovered-from)",
                    add_args.dep_type
                );
            }

            // Validate both issues exist
            for id in [&add_args.from, &add_args.to] {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
                        rusqlite::params![id],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);
                if !exists {
                    bail!("issue '{}' not found", id);
                }
            }

            let now_str = Utc::now().to_rfc3339();

            conn.execute(
                "INSERT OR IGNORE INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    &add_args.from,
                    &add_args.to,
                    dep_type.as_str(),
                    &now_str,
                    &ctx.actor,
                ],
            )
            .with_context(|| {
                format!(
                    "failed to add dependency {} -> {}",
                    add_args.from, add_args.to
                )
            })?;

            // Record event
            conn.execute(
                "INSERT INTO events (issue_id, event_type, actor, new_value, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    &add_args.from,
                    "dependency_added",
                    &ctx.actor,
                    format!("{}:{}", dep_type.as_str(), add_args.to),
                    &now_str,
                ],
            )?;

            if ctx.json {
                output_json(&serde_json::json!({
                    "from": add_args.from,
                    "to": add_args.to,
                    "type": dep_type.as_str(),
                }));
            } else if !ctx.quiet {
                println!(
                    "Added dependency: {} --[{}]--> {}",
                    add_args.from, dep_type, add_args.to
                );
            }
        }

        DepCommands::Remove(remove_args) => {
            if ctx.readonly {
                bail!("cannot remove dependencies in read-only mode");
            }

            let conn = rusqlite::Connection::open(&db_path)
                .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            let now_str = Utc::now().to_rfc3339();

            let changes = conn.execute(
                "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2",
                rusqlite::params![&remove_args.from, &remove_args.to],
            )?;

            if changes > 0 {
                // Record event
                conn.execute(
                    "INSERT INTO events (issue_id, event_type, actor, old_value, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        &remove_args.from,
                        "dependency_removed",
                        &ctx.actor,
                        &remove_args.to,
                        &now_str,
                    ],
                )?;
            }

            if ctx.json {
                output_json(&serde_json::json!({
                    "from": remove_args.from,
                    "to": remove_args.to,
                    "removed": changes > 0,
                }));
            } else if changes > 0 {
                if !ctx.quiet {
                    println!(
                        "Removed dependency: {} -> {}",
                        remove_args.from, remove_args.to
                    );
                }
            } else {
                eprintln!(
                    "No dependency found: {} -> {}",
                    remove_args.from, remove_args.to
                );
            }
        }

        DepCommands::List(list_args) => {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            // Dependencies (this issue depends on)
            let mut dep_stmt = conn.prepare(
                "SELECT d.depends_on_id, d.type, i.title, i.status \
                 FROM dependencies d \
                 LEFT JOIN issues i ON d.depends_on_id = i.id \
                 WHERE d.issue_id = ?1 \
                 ORDER BY d.type, d.depends_on_id",
            )?;
            let deps: Vec<(String, String, String, String)> = dep_stmt
                .query_map(rusqlite::params![&list_args.id], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get::<_, String>(2).unwrap_or_default(),
                        row.get::<_, String>(3).unwrap_or_default(),
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();

            // Dependents (issues that depend on this one)
            let mut dependent_stmt = conn.prepare(
                "SELECT d.issue_id, d.type, i.title, i.status \
                 FROM dependencies d \
                 LEFT JOIN issues i ON d.issue_id = i.id \
                 WHERE d.depends_on_id = ?1 \
                 ORDER BY d.type, d.issue_id",
            )?;
            let dependents: Vec<(String, String, String, String)> = dependent_stmt
                .query_map(rusqlite::params![&list_args.id], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get::<_, String>(2).unwrap_or_default(),
                        row.get::<_, String>(3).unwrap_or_default(),
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();

            if ctx.json {
                let deps_json: Vec<serde_json::Value> = deps
                    .iter()
                    .map(|(id, dep_type, title, status)| {
                        serde_json::json!({
                            "id": id,
                            "type": dep_type,
                            "title": title,
                            "status": status,
                            "direction": "depends_on",
                        })
                    })
                    .collect();
                let dependents_json: Vec<serde_json::Value> = dependents
                    .iter()
                    .map(|(id, dep_type, title, status)| {
                        serde_json::json!({
                            "id": id,
                            "type": dep_type,
                            "title": title,
                            "status": status,
                            "direction": "depended_on_by",
                        })
                    })
                    .collect();
                output_json(&serde_json::json!({
                    "issue_id": list_args.id,
                    "depends_on": deps_json,
                    "depended_on_by": dependents_json,
                }));
            } else {
                if deps.is_empty() && dependents.is_empty() {
                    println!("No dependencies for {}", list_args.id);
                    return Ok(());
                }

                if !deps.is_empty() {
                    println!("Depends on:");
                    for (id, dep_type, title, status) in &deps {
                        println!("  [{}] {} {} ({})", dep_type, id, title, status);
                    }
                }

                if !dependents.is_empty() {
                    if !deps.is_empty() {
                        println!();
                    }
                    println!("Depended on by:");
                    for (id, dep_type, title, status) in &dependents {
                        println!("  [{}] {} {} ({})", dep_type, id, title, status);
                    }
                }
            }
        }

        DepCommands::Cycles => {
            let conn = rusqlite::Connection::open_with_flags(
                &db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .with_context(|| format!("failed to open database: {}", db_path.display()))?;

            let cycles = detect_cycles(&conn)?;

            if ctx.json {
                output_json(&cycles);
            } else if cycles.is_empty() {
                println!("No dependency cycles detected");
            } else {
                println!("Found {} dependency cycle(s):\n", cycles.len());
                for (i, cycle) in cycles.iter().enumerate() {
                    print!("{}. ", i + 1);
                    for (j, id) in cycle.iter().enumerate() {
                        if j > 0 {
                            print!(" -> ");
                        }
                        print!("{}", id);
                    }
                    // Close the cycle by printing the first element again
                    if !cycle.is_empty() {
                        print!(" -> {}", cycle[0]);
                    }
                    println!();
                }
            }
        }

        DepCommands::Parents(parent_args) => {
            run_parents(ctx, &db_path, &parent_args.id)?;
        }

        DepCommands::Children(children_args) => {
            run_children(ctx, &db_path, &children_args.id)?;
        }
    }

    Ok(())
}

/// Run `dep parents <id>` -- show parent issues.
pub fn run_parents(ctx: &RuntimeContext, db_path: &std::path::Path, issue_id: &str) -> Result<()> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    // In parent-child deps: issue_id=child depends on depends_on_id=parent.
    // Parents of X: SELECT depends_on_id WHERE issue_id = X AND type = 'parent-child'
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.status, i.priority \
         FROM dependencies d \
         JOIN issues i ON d.depends_on_id = i.id \
         WHERE d.issue_id = ?1 AND d.type = 'parent-child' \
         ORDER BY i.id",
    )?;
    let parents: Vec<(String, String, String, i32)> = stmt
        .query_map(rusqlite::params![issue_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_parents: Vec<serde_json::Value> = parents
            .iter()
            .map(|(id, title, status, priority)| {
                serde_json::json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "issue_id": issue_id,
            "parents": json_parents,
        }));
    } else if parents.is_empty() {
        println!("No parents for {}", issue_id);
    } else {
        let headers = &["ID", "PRI", "STATUS", "TITLE"];
        let rows: Vec<Vec<String>> = parents
            .iter()
            .map(|(id, title, status, priority)| {
                vec![
                    id.clone(),
                    format!("P{}", priority),
                    status.clone(),
                    title.clone(),
                ]
            })
            .collect();
        println!("Parents of {}:", issue_id);
        output_table(headers, &rows);
    }

    Ok(())
}

/// Run `dep children <id>` -- show child issues.
pub fn run_children(ctx: &RuntimeContext, db_path: &std::path::Path, issue_id: &str) -> Result<()> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    // Children: issues where issue_id=child depends on depends_on_id=parent
    // Children of X: SELECT issue_id FROM dependencies WHERE depends_on_id = X AND type = 'parent-child'
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.status, i.priority \
         FROM dependencies d \
         JOIN issues i ON d.issue_id = i.id \
         WHERE d.depends_on_id = ?1 AND d.type = 'parent-child' \
         ORDER BY i.id",
    )?;
    let children: Vec<(String, String, String, i32)> = stmt
        .query_map(rusqlite::params![issue_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if ctx.json {
        let json_children: Vec<serde_json::Value> = children
            .iter()
            .map(|(id, title, status, priority)| {
                serde_json::json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "issue_id": issue_id,
            "children": json_children,
        }));
    } else if children.is_empty() {
        println!("No children for {}", issue_id);
    } else {
        let headers = &["ID", "PRI", "STATUS", "TITLE"];
        let rows: Vec<Vec<String>> = children
            .iter()
            .map(|(id, title, status, priority)| {
                vec![
                    id.clone(),
                    format!("P{}", priority),
                    status.clone(),
                    title.clone(),
                ]
            })
            .collect();
        println!("Children of {}:", issue_id);
        output_table(headers, &rows);
    }

    Ok(())
}

/// Detect dependency cycles using DFS on blocking dependency types.
///
/// Returns a list of cycles, where each cycle is a list of issue IDs.
fn detect_cycles(conn: &rusqlite::Connection) -> Result<Vec<Vec<String>>> {
    // Load all blocking dependencies (blocks, parent-child, conditional-blocks, waits-for)
    let mut stmt = conn.prepare(
        "SELECT issue_id, depends_on_id FROM dependencies \
         WHERE type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')",
    )?;

    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_nodes: HashSet<String> = HashSet::new();

    let edges: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (from, to) in &edges {
        graph.entry(from.clone()).or_default().push(to.clone());
        all_nodes.insert(from.clone());
        all_nodes.insert(to.clone());
    }

    let mut cycles: Vec<Vec<String>> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut rec_stack: HashSet<String> = HashSet::new();
    let mut path: Vec<String> = Vec::new();

    for node in &all_nodes {
        if !visited.contains(node) {
            dfs_cycles(
                node,
                &graph,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    Ok(cycles)
}

/// DFS helper for cycle detection.
fn dfs_cycles(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor.as_str()) {
                dfs_cycles(neighbor, graph, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(neighbor.as_str()) {
                // Found a cycle -- extract it from the path
                if let Some(start) = path.iter().position(|n| n == neighbor) {
                    let cycle: Vec<String> = path[start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
}
