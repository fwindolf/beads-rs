//! `bd graph` -- dependency graph visualization.
//!
//! Renders a layered DAG of issue dependencies in three formats:
//! - Default compact tree (layers with status symbols)
//! - Graphviz DOT (`--dot`)
//! - JSON (`--json` global flag)

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Write};

use anyhow::{Context, Result, bail};

use crate::cli::GraphArgs;
use crate::context::RuntimeContext;
use crate::output::{output_json, status_symbol};

// ---------------------------------------------------------------------------
// Graph data structures
// ---------------------------------------------------------------------------

/// A node in the dependency graph.
struct Node {
    id: String,
    title: String,
    status: String,
    priority: i32,
    layer: Option<usize>,
}

/// An edge in the dependency graph.
struct Edge {
    from: String,
    to: String,
    dep_type: String,
}

/// The full graph ready for rendering.
struct DepGraph {
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
    /// Forward adjacency: from -> [to]  (dependency direction: `from` depends on `to`).
    forward: HashMap<String, Vec<String>>,
    /// Reverse adjacency: to -> [from].
    #[allow(dead_code)]
    reverse: HashMap<String, Vec<String>>,
}

// Blocking dependency types that form the DAG structure.
const BLOCKING_TYPES: &[&str] = &["blocks", "parent-child", "conditional-blocks", "waits-for"];

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Execute the `bd graph` command.
pub fn run(ctx: &RuntimeContext, args: &GraphArgs) -> Result<()> {
    if args.id.is_none() && !args.all {
        bail!("specify an issue ID or use --all to graph all open issues");
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

    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open database: {}", db_path.display()))?;

    if args.all {
        run_all(ctx, args, &conn)
    } else {
        let id = args.id.as_ref().unwrap();
        let graph = build_subgraph(&conn, id)?;
        render(ctx, args, &graph)
    }
}

// ---------------------------------------------------------------------------
// --all: discover connected components and render each
// ---------------------------------------------------------------------------

fn run_all(ctx: &RuntimeContext, args: &GraphArgs, conn: &rusqlite::Connection) -> Result<()> {
    // Load all non-closed, non-template, non-gate issues.
    let mut stmt = conn.prepare(
        "SELECT id, title, status, priority FROM issues \
         WHERE status != 'closed' AND is_template = 0 AND issue_type != 'gate'",
    )?;
    let issues: Vec<(String, String, String, i32)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if issues.is_empty() {
        if ctx.json {
            output_json(&serde_json::json!({ "components": [] }));
        } else {
            println!("No open issues found.");
        }
        return Ok(());
    }

    // Load all blocking edges.
    let blocking_filter = BLOCKING_TYPES
        .iter()
        .map(|t| format!("'{}'", t))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT issue_id, depends_on_id, type FROM dependencies WHERE type IN ({})",
        blocking_filter
    );
    let mut edge_stmt = conn.prepare(&query)?;
    let all_edges: Vec<(String, String, String)> = edge_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    // Build full node map (include nodes mentioned in edges too).
    let mut node_map: HashMap<String, Node> = HashMap::new();
    for (id, title, status, priority) in &issues {
        node_map.insert(
            id.clone(),
            Node {
                id: id.clone(),
                title: title.clone(),
                status: status.clone(),
                priority: *priority,
                layer: None,
            },
        );
    }

    // For nodes referenced by edges but not in our issue set, load them.
    let mut missing: HashSet<String> = HashSet::new();
    for (from, to, _) in &all_edges {
        if !node_map.contains_key(from) {
            missing.insert(from.clone());
        }
        if !node_map.contains_key(to) {
            missing.insert(to.clone());
        }
    }
    for mid in &missing {
        if let Ok((title, status, priority)) = conn.query_row(
            "SELECT title, status, priority FROM issues WHERE id = ?1",
            rusqlite::params![mid],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            },
        ) {
            node_map.insert(
                mid.clone(),
                Node {
                    id: mid.clone(),
                    title,
                    status,
                    priority,
                    layer: None,
                },
            );
        }
    }

    // Build adjacency for connected component detection (undirected).
    let mut adj: HashMap<String, HashSet<String>> = HashMap::new();
    for (from, to, _) in &all_edges {
        adj.entry(from.clone()).or_default().insert(to.clone());
        adj.entry(to.clone()).or_default().insert(from.clone());
    }

    // Find connected components using BFS.
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();

    for node_id in node_map.keys() {
        if visited.contains(node_id) {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(node_id.clone());
        visited.insert(node_id.clone());

        while let Some(current) = queue.pop_front() {
            component.push(current.clone());
            if let Some(neighbors) = adj.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) && node_map.contains_key(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        components.push(component);
    }

    // Sort components by size descending so largest graphs appear first.
    components.sort_by_key(|b| std::cmp::Reverse(b.len()));

    // Build and render each component.
    if ctx.json {
        let mut json_components = Vec::new();
        for comp_nodes in &components {
            let comp_set: HashSet<&String> = comp_nodes.iter().collect();
            let graph = build_graph_from_sets(&node_map, &all_edges, &comp_set);
            json_components.push(graph_to_json(&graph));
        }
        output_json(&serde_json::json!({ "components": json_components }));
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for (i, comp_nodes) in components.iter().enumerate() {
            if comp_nodes.len() == 1 {
                // Singletons with no edges -- skip if there are many.
                continue;
            }
            let comp_set: HashSet<&String> = comp_nodes.iter().collect();
            let graph = build_graph_from_sets(&node_map, &all_edges, &comp_set);

            if i > 0 {
                let _ = writeln!(handle);
                let _ = writeln!(handle, "{}", "=".repeat(60));
                let _ = writeln!(handle);
            }
            if components.len() > 1 {
                let _ = writeln!(handle, "Component {} ({} issues)", i + 1, comp_nodes.len());
                let _ = writeln!(handle);
            }
            render_to(ctx, args, &graph, &mut handle)?;
        }

        // Summarize singletons.
        let singletons: Vec<&Vec<String>> = components.iter().filter(|c| c.len() == 1).collect();
        if !singletons.is_empty() {
            let _ = writeln!(handle);
            let _ = writeln!(
                handle,
                "({} isolated issues with no blocking dependencies)",
                singletons.len()
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Build subgraph for a single issue (BFS both directions)
// ---------------------------------------------------------------------------

fn build_subgraph(conn: &rusqlite::Connection, root_id: &str) -> Result<DepGraph> {
    // Verify the root issue exists.
    let (title, status, priority): (String, String, i32) = conn
        .query_row(
            "SELECT title, status, priority FROM issues WHERE id = ?1",
            rusqlite::params![root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .with_context(|| format!("issue '{}' not found", root_id))?;

    let mut nodes: HashMap<String, Node> = HashMap::new();
    nodes.insert(
        root_id.to_string(),
        Node {
            id: root_id.to_string(),
            title,
            status,
            priority,
            layer: None,
        },
    );

    let mut all_edges: Vec<Edge> = Vec::new();
    let mut forward: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();

    let blocking_filter = BLOCKING_TYPES
        .iter()
        .map(|t| format!("'{}'", t))
        .collect::<Vec<_>>()
        .join(", ");

    // BFS: explore in both directions to find full connected component.
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();
    queue.push_back(root_id.to_string());
    visited.insert(root_id.to_string());

    while let Some(current) = queue.pop_front() {
        // Forward: current depends on X.
        let fwd_query = format!(
            "SELECT depends_on_id, type FROM dependencies WHERE issue_id = ?1 AND type IN ({})",
            blocking_filter
        );
        let mut fwd_stmt = conn.prepare(&fwd_query)?;
        let fwd_deps: Vec<(String, String)> = fwd_stmt
            .query_map(rusqlite::params![&current], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (dep_id, dep_type) in fwd_deps {
            forward
                .entry(current.clone())
                .or_default()
                .push(dep_id.clone());
            reverse
                .entry(dep_id.clone())
                .or_default()
                .push(current.clone());
            all_edges.push(Edge {
                from: current.clone(),
                to: dep_id.clone(),
                dep_type,
            });

            if !visited.contains(&dep_id) {
                visited.insert(dep_id.clone());
                queue.push_back(dep_id.clone());
                // Load node info.
                if let Ok((t, s, p)) = conn.query_row(
                    "SELECT title, status, priority FROM issues WHERE id = ?1",
                    rusqlite::params![&dep_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i32>(2)?,
                        ))
                    },
                ) {
                    let node_id = dep_id.clone();
                    nodes.insert(
                        dep_id,
                        Node {
                            id: node_id,
                            title: t,
                            status: s,
                            priority: p,
                            layer: None,
                        },
                    );
                }
            }
        }

        // Reverse: X depends on current.
        let rev_query = format!(
            "SELECT issue_id, type FROM dependencies WHERE depends_on_id = ?1 AND type IN ({})",
            blocking_filter
        );
        let mut rev_stmt = conn.prepare(&rev_query)?;
        let rev_deps: Vec<(String, String)> = rev_stmt
            .query_map(rusqlite::params![&current], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (dep_id, dep_type) in rev_deps {
            // dep_id depends on current -- add edge dep_id -> current.
            forward
                .entry(dep_id.clone())
                .or_default()
                .push(current.clone());
            reverse
                .entry(current.clone())
                .or_default()
                .push(dep_id.clone());
            all_edges.push(Edge {
                from: dep_id.clone(),
                to: current.clone(),
                dep_type,
            });

            if !visited.contains(&dep_id) {
                visited.insert(dep_id.clone());
                queue.push_back(dep_id.clone());
                if let Ok((t, s, p)) = conn.query_row(
                    "SELECT title, status, priority FROM issues WHERE id = ?1",
                    rusqlite::params![&dep_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i32>(2)?,
                        ))
                    },
                ) {
                    let node_id = dep_id.clone();
                    nodes.insert(
                        dep_id,
                        Node {
                            id: node_id,
                            title: t,
                            status: s,
                            priority: p,
                            layer: None,
                        },
                    );
                }
            }
        }
    }

    // Deduplicate edges.
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();
    let mut unique_edges: Vec<Edge> = Vec::new();
    for edge in all_edges {
        let key = (edge.from.clone(), edge.to.clone());
        if seen_edges.insert(key) {
            unique_edges.push(edge);
        }
    }

    // Deduplicate adjacency lists.
    for v in forward.values_mut() {
        let set: HashSet<String> = v.drain(..).collect();
        *v = set.into_iter().collect();
    }
    for v in reverse.values_mut() {
        let set: HashSet<String> = v.drain(..).collect();
        *v = set.into_iter().collect();
    }

    let mut graph = DepGraph {
        nodes,
        edges: unique_edges,
        forward,
        reverse,
    };

    assign_layers(&mut graph);
    Ok(graph)
}

// ---------------------------------------------------------------------------
// Build graph from pre-loaded data (for --all mode)
// ---------------------------------------------------------------------------

fn build_graph_from_sets(
    all_nodes: &HashMap<String, Node>,
    all_edges: &[(String, String, String)],
    node_set: &HashSet<&String>,
) -> DepGraph {
    let mut nodes: HashMap<String, Node> = HashMap::new();
    for id in node_set {
        if let Some(n) = all_nodes.get(*id) {
            nodes.insert(
                n.id.clone(),
                Node {
                    id: n.id.clone(),
                    title: n.title.clone(),
                    status: n.status.clone(),
                    priority: n.priority,
                    layer: None,
                },
            );
        }
    }

    let mut edges = Vec::new();
    let mut forward: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();

    for (from, to, dep_type) in all_edges {
        if node_set.contains(from) && node_set.contains(to) {
            edges.push(Edge {
                from: from.clone(),
                to: to.clone(),
                dep_type: dep_type.clone(),
            });
            forward.entry(from.clone()).or_default().push(to.clone());
            reverse.entry(to.clone()).or_default().push(from.clone());
        }
    }

    let mut graph = DepGraph {
        nodes,
        edges,
        forward,
        reverse,
    };
    assign_layers(&mut graph);
    graph
}

// ---------------------------------------------------------------------------
// Layer assignment: longest path from sources
// ---------------------------------------------------------------------------

fn assign_layers(graph: &mut DepGraph) {
    // In our edge model: issue_id (from) depends_on depends_on_id (to).
    // So `to` must come BEFORE `from` -- `to` is the dependency.
    // Layer 0 = nodes with no dependencies (nothing in `forward` for them).
    // Layer of a node = max(layer of dependencies) + 1.

    let node_ids: Vec<String> = graph.nodes.keys().cloned().collect();

    // Detect if there are cycles -- if so, cap iterations to avoid infinite loop.
    let max_iterations = node_ids.len() + 1;
    let mut iteration = 0;

    loop {
        let mut changed = false;
        iteration += 1;

        for id in &node_ids {
            let deps = graph.forward.get(id).cloned().unwrap_or_default();

            if deps.is_empty() {
                // No dependencies -- layer 0.
                let node = graph.nodes.get_mut(id).unwrap();
                if node.layer.is_none() {
                    node.layer = Some(0);
                    changed = true;
                }
                continue;
            }

            // Check if all dependencies have been assigned a layer.
            let mut all_assigned = true;
            let mut max_dep_layer: usize = 0;
            for dep_id in &deps {
                if let Some(dep_node) = graph.nodes.get(dep_id) {
                    if let Some(l) = dep_node.layer {
                        max_dep_layer = max_dep_layer.max(l);
                    } else {
                        all_assigned = false;
                    }
                }
                // If dep_id not in nodes, treat as resolved (layer 0).
            }

            if all_assigned {
                let new_layer = max_dep_layer + 1;
                let node = graph.nodes.get_mut(id).unwrap();
                if node.layer != Some(new_layer) {
                    node.layer = Some(new_layer);
                    changed = true;
                }
            }
        }

        if !changed || iteration >= max_iterations {
            break;
        }
    }

    // Assign any remaining unassigned nodes (in cycles) to a special layer.
    let max_layer = graph
        .nodes
        .values()
        .filter_map(|n| n.layer)
        .max()
        .unwrap_or(0);

    for node in graph.nodes.values_mut() {
        if node.layer.is_none() {
            node.layer = Some(max_layer + 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering dispatch
// ---------------------------------------------------------------------------

fn render(ctx: &RuntimeContext, args: &GraphArgs, graph: &DepGraph) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    render_to(ctx, args, graph, &mut handle)
}

fn render_to<W: Write>(
    ctx: &RuntimeContext,
    args: &GraphArgs,
    graph: &DepGraph,
    w: &mut W,
) -> Result<()> {
    if ctx.json {
        let json = graph_to_json(graph);
        let s = serde_json::to_string_pretty(&json).context("failed to serialize graph JSON")?;
        let _ = writeln!(w, "{}", s);
    } else if args.dot {
        render_dot(graph, w);
    } else {
        render_compact(graph, w);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Compact tree rendering
// ---------------------------------------------------------------------------

fn render_compact<W: Write>(graph: &DepGraph, w: &mut W) {
    if graph.nodes.is_empty() {
        let _ = writeln!(w, "No issues in graph.");
        return;
    }

    // Group nodes by layer.
    let mut layers: HashMap<usize, Vec<&Node>> = HashMap::new();
    for node in graph.nodes.values() {
        let layer = node.layer.unwrap_or(0);
        layers.entry(layer).or_default().push(node);
    }

    // Sort layers by number, nodes within each layer by priority then ID.
    let mut layer_nums: Vec<usize> = layers.keys().cloned().collect();
    layer_nums.sort();

    for (i, layer_num) in layer_nums.iter().enumerate() {
        if i > 0 {
            let _ = writeln!(w);
        }

        let label = if *layer_num == 0 {
            "LAYER 0 (ready)".to_string()
        } else {
            format!("LAYER {}", layer_num)
        };
        let _ = writeln!(w, "{}", label);

        let nodes = layers.get_mut(layer_num).unwrap();
        nodes.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));

        for node in nodes.iter() {
            let sym = status_sym(&node.status);

            // Find what this node depends on (its "needs").
            let needs: Vec<&str> = graph
                .forward
                .get(&node.id)
                .map(|deps| deps.iter().map(|s| s.as_str()).collect())
                .unwrap_or_default();

            let needs_str = if needs.is_empty() {
                String::new()
            } else {
                format!(" (needs: {})", needs.join(", "))
            };

            let _ = writeln!(
                w,
                "  {}  {}  P{}  {}{}",
                sym, node.id, node.priority, node.title, needs_str
            );
        }
    }
}

// ---------------------------------------------------------------------------
// DOT rendering
// ---------------------------------------------------------------------------

fn render_dot<W: Write>(graph: &DepGraph, w: &mut W) {
    let _ = writeln!(w, "digraph beads {{");
    let _ = writeln!(w, "  rankdir=TB;");
    let _ = writeln!(w, "  node [shape=box];");

    // Emit nodes.
    let mut node_ids: Vec<&String> = graph.nodes.keys().collect();
    node_ids.sort();
    for id in &node_ids {
        let node = &graph.nodes[*id];
        // Escape quotes in title.
        let escaped_title = node.title.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = writeln!(
            w,
            "  \"{}\" [label=\"{}\\n{}\\nP{} {}\"];",
            id, id, escaped_title, node.priority, node.status
        );
    }

    // Emit edges: from depends on to, so arrow goes to -> from
    // (showing "blocks" direction: to blocks from).
    for edge in &graph.edges {
        let _ = writeln!(w, "  \"{}\" -> \"{}\";", edge.to, edge.from);
    }

    let _ = writeln!(w, "}}");
}

// ---------------------------------------------------------------------------
// JSON rendering
// ---------------------------------------------------------------------------

fn graph_to_json(graph: &DepGraph) -> serde_json::Value {
    let mut node_list: Vec<&Node> = graph.nodes.values().collect();
    node_list.sort_by(|a, b| {
        a.layer
            .unwrap_or(0)
            .cmp(&b.layer.unwrap_or(0))
            .then_with(|| a.id.cmp(&b.id))
    });

    let nodes_json: Vec<serde_json::Value> = node_list
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "title": n.title,
                "layer": n.layer.unwrap_or(0),
                "status": n.status,
                "priority": n.priority,
            })
        })
        .collect();

    let edges_json: Vec<serde_json::Value> = graph
        .edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "from": e.from,
                "to": e.to,
                "type": e.dep_type,
            })
        })
        .collect();

    // Build layers array.
    let max_layer = graph
        .nodes
        .values()
        .filter_map(|n| n.layer)
        .max()
        .unwrap_or(0);

    let mut layers_arr: Vec<Vec<&str>> = vec![Vec::new(); max_layer + 1];
    for node in graph.nodes.values() {
        let l = node.layer.unwrap_or(0);
        if l < layers_arr.len() {
            layers_arr[l].push(&node.id);
        }
    }
    // Sort each layer.
    for layer in &mut layers_arr {
        layer.sort();
    }

    serde_json::json!({
        "nodes": nodes_json,
        "edges": edges_json,
        "layers": layers_arr,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a status string to its display symbol.
fn status_sym(status: &str) -> &'static str {
    use beads_core::enums::Status;
    let s = Status::from(status);
    status_symbol(&s)
}
