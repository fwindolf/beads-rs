//! `bd swarm` -- swarm analysis and status for structured epics.
//!
//! Analyzes an epic's dependency structure using topological sort (Kahn's
//! algorithm) to compute parallelism waves, detect cycles, and show progress.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::cli::{SwarmArgs, SwarmCommands};
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd swarm` command.
pub fn run(ctx: &RuntimeContext, args: &SwarmArgs) -> Result<()> {
    match &args.command {
        SwarmCommands::Validate(a) => cmd_validate(ctx, &a.epic_id),
        SwarmCommands::Status(a) => cmd_status(ctx, &a.epic_id),
    }
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Result of analyzing an epic's dependency structure.
#[derive(Debug, Serialize)]
struct SwarmAnalysis {
    epic_id: String,
    epic_title: String,
    total_issues: usize,
    closed_issues: usize,
    waves: Vec<Wave>,
    max_parallelism: usize,
    estimated_sessions: usize,
    critical_path_length: usize,
    warnings: Vec<String>,
    errors: Vec<String>,
    swarmable: bool,
}

/// A wave of issues that can be worked on in parallel.
#[derive(Debug, Clone, Serialize)]
struct Wave {
    wave: usize,
    issues: Vec<WaveIssue>,
}

/// An issue within a wave.
#[derive(Debug, Clone, Serialize)]
struct WaveIssue {
    id: String,
    title: String,
    priority: i32,
    status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
}

/// Swarm status showing progress through waves.
#[derive(Debug, Serialize)]
struct SwarmStatus {
    epic_id: String,
    epic_title: String,
    total_issues: usize,
    completed: usize,
    progress_percent: f64,
    waves: Vec<StatusWave>,
}

/// A wave in status output with completion info.
#[derive(Debug, Serialize)]
struct StatusWave {
    wave: usize,
    total: usize,
    completed: usize,
    issues: Vec<StatusIssue>,
}

/// An issue in status output.
#[derive(Debug, Serialize)]
struct StatusIssue {
    id: String,
    title: String,
    status: String,
}

/// Internal representation of a child issue for graph analysis.
struct ChildIssue {
    id: String,
    title: String,
    priority: i32,
    status: String,
}

// ---------------------------------------------------------------------------
// Validate
// ---------------------------------------------------------------------------

fn cmd_validate(ctx: &RuntimeContext, epic_id: &str) -> Result<()> {
    let conn = open_db(ctx)?;

    // Load and verify epic
    let (eid, etitle, etype) = load_issue_basic(&conn, epic_id)?;
    if etype != "epic" && etype != "molecule" {
        bail!("'{}' is not an epic (type: {})", eid, etype);
    }

    // Load children and blocking deps
    let children = load_epic_children(&conn, &eid)?;
    let blocking_deps = load_blocking_deps(&conn, &eid, &children)?;

    // Analyze
    let analysis = analyze_epic(&eid, &etitle, &children, &blocking_deps);

    if ctx.json {
        output_json(&analysis);
        if !analysis.swarmable {
            std::process::exit(1);
        }
        return Ok(());
    }

    // Human-readable output
    println!();
    println!("Swarm analysis for {}: {:?}", eid, etitle);
    println!();

    for wave in &analysis.waves {
        let label = if wave.wave == 0 {
            format!("Wave {} (ready)", wave.wave)
        } else {
            format!("Wave {}", wave.wave)
        };
        println!("{}:  {} issues", label, wave.issues.len());
        for wi in &wave.issues {
            let needs = if wi.needs.is_empty() {
                String::new()
            } else {
                format!(" (needs: {})", wi.needs.join(", "))
            };
            println!("  o  {}  P{}  {}{}", wi.id, wi.priority, wi.title, needs);
        }
        println!();
    }

    println!("Summary:");
    println!("  Total issues: {}", analysis.total_issues);
    println!("  Waves: {}", analysis.waves.len());
    println!(
        "  Max parallelism: {}{}",
        analysis.max_parallelism,
        if !analysis.waves.is_empty() {
            format!(
                " (wave {})",
                analysis
                    .waves
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, w)| w.issues.len())
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            )
        } else {
            String::new()
        }
    );
    println!("  Estimated sessions: {}", analysis.estimated_sessions);
    println!("  Critical path length: {}", analysis.critical_path_length);

    if !analysis.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for w in &analysis.warnings {
            println!("  - {}", w);
        }
    }

    if !analysis.errors.is_empty() {
        println!();
        println!("Errors:");
        for e in &analysis.errors {
            println!("  - {}", e);
        }
    }

    println!();
    if analysis.swarmable {
        println!("Swarmable: YES");
    } else {
        println!("Swarmable: NO (fix errors first)");
        std::process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

fn cmd_status(ctx: &RuntimeContext, epic_id: &str) -> Result<()> {
    let conn = open_db(ctx)?;

    // Load and verify epic
    let (eid, etitle, etype) = load_issue_basic(&conn, epic_id)?;
    if etype != "epic" && etype != "molecule" {
        bail!("'{}' is not an epic (type: {})", eid, etype);
    }

    // Load children and blocking deps
    let children = load_epic_children(&conn, &eid)?;
    let blocking_deps = load_blocking_deps(&conn, &eid, &children)?;

    // Compute waves
    let child_ids: Vec<String> = children.iter().map(|c| c.id.clone()).collect();
    let waves_ids = compute_waves(&child_ids, &blocking_deps);

    // Build status
    let child_map: HashMap<&str, &ChildIssue> =
        children.iter().map(|c| (c.id.as_str(), c)).collect();

    let mut completed_total = 0usize;
    let mut status_waves = Vec::new();

    for (wave_idx, wave_ids) in waves_ids.iter().enumerate() {
        let mut wave_completed = 0usize;
        let mut issues = Vec::new();

        for id in wave_ids {
            if let Some(child) = child_map.get(id.as_str()) {
                if child.status == "closed" {
                    wave_completed += 1;
                    completed_total += 1;
                }
                issues.push(StatusIssue {
                    id: child.id.clone(),
                    title: child.title.clone(),
                    status: child.status.clone(),
                });
            }
        }

        status_waves.push(StatusWave {
            wave: wave_idx,
            total: wave_ids.len(),
            completed: wave_completed,
            issues,
        });
    }

    let progress = if children.is_empty() {
        0.0
    } else {
        (completed_total as f64 / children.len() as f64) * 100.0
    };

    let status = SwarmStatus {
        epic_id: eid.clone(),
        epic_title: etitle.clone(),
        total_issues: children.len(),
        completed: completed_total,
        progress_percent: progress,
        waves: status_waves,
    };

    if ctx.json {
        output_json(&status);
        return Ok(());
    }

    // Human-readable output
    println!();
    println!("Swarm status for {}:", eid);
    println!();

    for sw in &status.waves {
        let wave_label = if sw.completed == sw.total {
            format!("Wave {}: {}/{} complete", sw.wave, sw.completed, sw.total)
        } else if sw.wave > 0 && sw.completed == 0 {
            // Check if previous wave is complete
            let prev_complete = if sw.wave > 0 {
                status
                    .waves
                    .get(sw.wave - 1)
                    .map(|pw| pw.completed == pw.total)
                    .unwrap_or(true)
            } else {
                true
            };
            if prev_complete {
                format!("Wave {}: {}/{}", sw.wave, sw.completed, sw.total)
            } else {
                format!(
                    "Wave {}: {}/{} (blocked by wave {})",
                    sw.wave,
                    sw.completed,
                    sw.total,
                    sw.wave - 1
                )
            }
        } else {
            format!("Wave {}: {}/{} complete", sw.wave, sw.completed, sw.total)
        };

        println!("{}", wave_label);
        for si in &sw.issues {
            let marker = match si.status.as_str() {
                "closed" => "v",
                "in_progress" => "~",
                _ => "o",
            };
            let suffix = if si.status == "in_progress" {
                " (in_progress)"
            } else {
                ""
            };
            println!("  {}  {}  {}{}", marker, si.id, si.title, suffix);
        }
        println!();
    }

    println!(
        "Overall: {}/{} complete ({:.0}%)",
        status.completed, status.total_issues, status.progress_percent
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Analysis core (Kahn's algorithm)
// ---------------------------------------------------------------------------

/// Run full analysis on an epic's children and their blocking dependencies.
fn analyze_epic(
    epic_id: &str,
    epic_title: &str,
    children: &[ChildIssue],
    blocking_deps: &[(String, String)],
) -> SwarmAnalysis {
    if children.is_empty() {
        return SwarmAnalysis {
            epic_id: epic_id.to_string(),
            epic_title: epic_title.to_string(),
            total_issues: 0,
            closed_issues: 0,
            waves: Vec::new(),
            max_parallelism: 0,
            estimated_sessions: 0,
            critical_path_length: 0,
            warnings: vec!["Epic has no children".to_string()],
            errors: Vec::new(),
            swarmable: true,
        };
    }

    let child_ids: Vec<String> = children.iter().map(|c| c.id.clone()).collect();
    let child_map: HashMap<&str, &ChildIssue> =
        children.iter().map(|c| (c.id.as_str(), c)).collect();

    // Build dependency maps for structural checks
    let mut depends_on: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut depended_on_by: HashMap<&str, Vec<&str>> = HashMap::new();
    for c in children {
        depends_on.entry(c.id.as_str()).or_default();
        depended_on_by.entry(c.id.as_str()).or_default();
    }
    for (from, to) in blocking_deps {
        // from blocks to => to depends on from
        depends_on
            .entry(to.as_str())
            .or_default()
            .push(from.as_str());
        depended_on_by
            .entry(from.as_str())
            .or_default()
            .push(to.as_str());
    }

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Detect cycles via DFS
    let has_cycle = detect_cycle(&child_ids, &depends_on);
    if has_cycle {
        errors.push("Dependency cycle detected".to_string());
    }

    // Structural warnings
    for c in children {
        let lower = c.title.to_lowercase();
        let has_deps = !depends_on.get(c.id.as_str()).is_none_or(|d| d.is_empty());
        let has_dependents = !depended_on_by
            .get(c.id.as_str())
            .is_none_or(|d| d.is_empty());

        if !has_dependents
            && (lower.contains("foundation")
                || lower.contains("setup")
                || lower.contains("base")
                || lower.contains("core"))
        {
            warnings.push(format!(
                "{} ({}) has no dependents -- should other issues depend on it?",
                c.id, c.title
            ));
        }
        if !has_deps
            && (lower.contains("integration") || lower.contains("final") || lower.contains("test"))
        {
            warnings.push(format!(
                "{} ({}) has no dependencies -- should it depend on implementation?",
                c.id, c.title
            ));
        }
    }

    // Compute waves using Kahn's algorithm
    let waves_ids = compute_waves(&child_ids, blocking_deps);

    // Build waves with issue details
    let mut waves = Vec::new();
    let mut max_parallelism = 0usize;

    for (wave_idx, wave_ids) in waves_ids.iter().enumerate() {
        if wave_ids.len() > max_parallelism {
            max_parallelism = wave_ids.len();
        }

        let mut wave_issues = Vec::new();
        for id in wave_ids {
            if let Some(child) = child_map.get(id.as_str()) {
                let needs: Vec<String> = depends_on
                    .get(id.as_str())
                    .map(|deps| deps.iter().map(|d| d.to_string()).collect())
                    .unwrap_or_default();

                wave_issues.push(WaveIssue {
                    id: child.id.clone(),
                    title: child.title.clone(),
                    priority: child.priority,
                    status: child.status.clone(),
                    needs,
                });
            }
        }

        // Sort within wave by priority then ID for stable output
        wave_issues.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));

        waves.push(Wave {
            wave: wave_idx,
            issues: wave_issues,
        });
    }

    let closed_issues = children.iter().filter(|c| c.status == "closed").count();

    SwarmAnalysis {
        epic_id: epic_id.to_string(),
        epic_title: epic_title.to_string(),
        total_issues: children.len(),
        closed_issues,
        waves: waves.clone(),
        max_parallelism,
        estimated_sessions: children.len(),
        critical_path_length: waves.len(),
        warnings,
        errors: errors.clone(),
        swarmable: errors.is_empty(),
    }
}

/// Compute waves of parallel work using Kahn's algorithm.
///
/// Returns a `Vec<Vec<String>>` where each inner vec is one wave of issue IDs
/// that can be worked on in parallel.
fn compute_waves(child_ids: &[String], blocking_deps: &[(String, String)]) -> Vec<Vec<String>> {
    // Build in-degree map
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for id in child_ids {
        in_degree.entry(id.clone()).or_insert(0);
    }

    for (from, to) in blocking_deps {
        // from blocks to => to depends on from
        *in_degree.entry(to.clone()).or_insert(0) += 1;
        adj.entry(from.clone()).or_default().push(to.clone());
    }

    let mut waves = Vec::new();
    let mut remaining = in_degree.clone();

    loop {
        // Find all nodes with in-degree 0
        let mut wave: Vec<String> = remaining
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        if wave.is_empty() {
            break;
        }

        // Sort for deterministic output
        wave.sort();

        // Remove this wave and decrement neighbours
        for id in &wave {
            remaining.remove(id);
            if let Some(neighbors) = adj.get(id) {
                for n in neighbors {
                    if let Some(deg) = remaining.get_mut(n) {
                        *deg -= 1;
                    }
                }
            }
        }

        waves.push(wave);
    }

    // Anything remaining is in a cycle
    if !remaining.is_empty() {
        let mut cycle_ids: Vec<String> = remaining.keys().cloned().collect();
        cycle_ids.sort();
        waves.push(cycle_ids);
    }

    waves
}

/// Detect cycles using DFS with an explicit stack. Returns true if a cycle exists.
fn detect_cycle(child_ids: &[String], depends_on: &HashMap<&str, Vec<&str>>) -> bool {
    // Use color-based DFS: White=unvisited, Gray=in-progress, Black=done
    let mut color: HashMap<&str, u8> = HashMap::new(); // 0=white, 1=gray, 2=black

    for id in child_ids {
        color.insert(id.as_str(), 0);
    }

    for id in child_ids {
        if color.get(id.as_str()) == Some(&0) {
            // Iterative DFS with explicit stack
            let mut stack: Vec<(&str, bool)> = vec![(id.as_str(), false)];

            while let Some((node, returning)) = stack.pop() {
                if returning {
                    // All children processed, mark black
                    color.insert(node, 2);
                    continue;
                }

                match color.get(node) {
                    Some(&1) => return true, // Back edge = cycle
                    Some(&2) => continue,    // Already processed
                    _ => {}
                }

                // Mark gray (in progress)
                color.insert(node, 1);
                // Push a "return" marker so we mark black after children
                stack.push((node, true));

                if let Some(deps) = depends_on.get(node) {
                    for dep in deps {
                        match color.get(dep) {
                            Some(&1) => return true, // Back edge = cycle
                            Some(&2) => {}           // Already done
                            _ => stack.push((dep, false)),
                        }
                    }
                }
            }
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

/// Open the beads database (read-only).
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

/// Load basic issue info (id, title, issue_type) by ID.
fn load_issue_basic(conn: &rusqlite::Connection, id: &str) -> Result<(String, String, String)> {
    conn.query_row(
        "SELECT id, title, issue_type FROM issues WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .with_context(|| format!("issue '{}' not found", id))
}

/// Load all child issues of an epic (via parent-child dependencies).
fn load_epic_children(conn: &rusqlite::Connection, epic_id: &str) -> Result<Vec<ChildIssue>> {
    let mut stmt = conn.prepare(
        "SELECT i.id, i.title, i.priority, i.status \
         FROM issues i \
         JOIN dependencies d ON i.id = d.issue_id \
         WHERE d.depends_on_id = ?1 AND d.type = 'parent-child' \
         ORDER BY i.priority ASC, i.id ASC",
    )?;

    let children = stmt
        .query_map(rusqlite::params![epic_id], |row| {
            Ok(ChildIssue {
                id: row.get(0)?,
                title: row.get(1)?,
                priority: row.get(2)?,
                status: row.get::<_, String>(3).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(children)
}

/// Load blocking dependencies between children of an epic.
///
/// Returns pairs `(blocker_id, blocked_id)` where blocker blocks blocked.
fn load_blocking_deps(
    conn: &rusqlite::Connection,
    epic_id: &str,
    children: &[ChildIssue],
) -> Result<Vec<(String, String)>> {
    let child_set: std::collections::HashSet<&str> =
        children.iter().map(|c| c.id.as_str()).collect();

    let mut deps = Vec::new();

    for child in children {
        let mut stmt =
            conn.prepare("SELECT depends_on_id, type FROM dependencies WHERE issue_id = ?1")?;

        let rows: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![&child.id], |row| {
                Ok((row.get(0)?, row.get::<_, String>(1).unwrap_or_default()))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (dep_id, dep_type) in rows {
            // Skip parent-child to epic itself
            if dep_id == epic_id && dep_type == "parent-child" {
                continue;
            }
            // Only track blocking deps within children
            if affects_ready_work(&dep_type) && child_set.contains(dep_id.as_str()) {
                // dep_id blocks child.id
                deps.push((dep_id, child.id.clone()));
            }
        }
    }

    Ok(deps)
}

/// Check if a dependency type affects ready work (mirrors DependencyType::affects_ready_work).
fn affects_ready_work(dep_type: &str) -> bool {
    matches!(
        dep_type,
        "blocks" | "parent-child" | "conditional-blocks" | "waits-for"
    )
}
