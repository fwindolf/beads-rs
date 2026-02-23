//! `bd upgrade` -- check and manage bd version upgrades.

use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::cli::UpgradeCommands;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Current version from Cargo.toml.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A single version changelog entry.
struct VersionChange {
    version: &'static str,
    date: &'static str,
    changes: &'static [&'static str],
}

/// Embedded changelog for version tracking.
static VERSION_CHANGES: &[VersionChange] = &[
    VersionChange {
        version: "0.2.0",
        date: "2026-02-23",
        changes: &[
            "Implement bd quickstart - interactive quick-start guide",
            "Implement bd onboard - AGENTS.md integration snippet",
            "Implement bd preflight - PR readiness checklist with --check mode",
            "Implement bd prime - AI-optimized workflow context output",
            "Implement bd upgrade - version tracking with status/review/ack",
            "Implement bd worktree - git worktree management with beads redirect",
            "Implement bd bootstrap - database bootstrap guidance",
        ],
    },
    VersionChange {
        version: "0.1.0",
        date: "2025-12-01",
        changes: &[
            "Initial release: Rust rewrite of beads issue tracker",
            "104+ CLI commands matching Go bd interface",
            "SQLite storage backend (embedded, zero-config)",
            "Full dependency tracking with cycle detection",
            "Ready work detection, hash-based IDs, JSON output",
        ],
    },
];

/// Execute the `bd upgrade` command.
pub fn run(ctx: &RuntimeContext, command: &UpgradeCommands) -> Result<()> {
    match command {
        UpgradeCommands::Status => run_status(ctx),
        UpgradeCommands::Review => run_review(ctx),
        UpgradeCommands::Ack => run_ack(ctx),
    }
}

fn run_status(ctx: &RuntimeContext) -> Result<()> {
    let (upgraded, previous) = detect_upgrade();

    if ctx.json {
        let mut result = serde_json::json!({
            "upgraded": upgraded,
            "current_version": VERSION,
        });
        if upgraded {
            if let Some(prev) = &previous {
                result["previous_version"] = serde_json::json!(prev);
            }
        }
        output_json(&result);
        return Ok(());
    }

    if upgraded {
        if let Some(prev) = previous {
            println!("bd upgraded from v{prev} to v{VERSION}");
            let new_versions = get_versions_since(&prev);
            if !new_versions.is_empty() {
                println!(
                    "   {} version{} with changes available",
                    new_versions.len(),
                    if new_versions.len() == 1 { "" } else { "s" }
                );
                println!();
                println!("Run 'bd upgrade review' to see what changed");
            }
        }
    } else {
        println!("bd version: v{VERSION} (no upgrade detected)");
    }

    Ok(())
}

fn run_review(ctx: &RuntimeContext) -> Result<()> {
    let (upgraded, previous) = detect_upgrade();

    if !upgraded || previous.is_none() {
        println!("You're already on v{VERSION} (no upgrade detected)");
        println!("Run 'bd upgrade status' to check version info");
        return Ok(());
    }
    let previous = previous.unwrap();
    let new_versions = get_versions_since(&previous);

    if ctx.json {
        let entries: Vec<_> = new_versions
            .iter()
            .map(|vc| {
                serde_json::json!({
                    "version": vc.version,
                    "date": vc.date,
                    "changes": vc.changes,
                })
            })
            .collect();
        output_json(&serde_json::json!({
            "current_version": VERSION,
            "previous_version": previous,
            "new_versions": entries,
        }));
        return Ok(());
    }

    println!();
    println!("Upgraded from v{previous} to v{VERSION}");
    println!("{}", "=".repeat(60));
    println!();

    if new_versions.is_empty() {
        println!("v{VERSION} is newer than v{previous} but not in changelog");
        return Ok(());
    }

    for vc in &new_versions {
        let marker = if vc.version == VERSION {
            " <-- current"
        } else {
            ""
        };
        println!("## v{} ({}){}", vc.version, vc.date, marker);
        println!();
        for change in vc.changes {
            println!("  - {change}");
        }
        println!();
    }

    println!("Run 'bd upgrade ack' to mark this version as seen");
    println!();

    Ok(())
}

fn run_ack(ctx: &RuntimeContext) -> Result<()> {
    let beads_dir = RuntimeContext::find_beads_dir();
    if beads_dir.is_none() {
        println!("Error: No .beads directory found");
        return Ok(());
    }
    let beads_dir = beads_dir.unwrap();

    let version_path = beads_dir.join(".local_version");
    let previous = read_local_version(&version_path);
    write_local_version(&version_path, VERSION);

    if ctx.json {
        output_json(&serde_json::json!({
            "acknowledged": true,
            "current_version": VERSION,
            "previous_version": previous,
        }));
        return Ok(());
    }

    match previous.as_deref() {
        Some(prev) if prev == VERSION => println!("Already on v{VERSION}"),
        Some(prev) => println!("Acknowledged upgrade from v{prev} to v{VERSION}"),
        None => println!("Acknowledged bd v{VERSION}"),
    }

    Ok(())
}

/// Detect if an upgrade occurred by comparing stored version with current.
fn detect_upgrade() -> (bool, Option<String>) {
    let beads_dir = match RuntimeContext::find_beads_dir() {
        Some(d) => d,
        None => return (false, None),
    };

    let version_path = beads_dir.join(".local_version");
    let last = read_local_version(&version_path);

    match last {
        Some(ref prev) if prev != VERSION => (true, Some(prev.clone())),
        Some(_) => (false, Some(VERSION.to_string())),
        None => (false, None),
    }
}

fn read_local_version(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn write_local_version(path: &Path, version: &str) {
    let _ = fs::write(path, format!("{version}\n"));
}

/// Get all version entries newer than `since_version`.
fn get_versions_since(since_version: &str) -> Vec<&'static VersionChange> {
    // VERSION_CHANGES is newest-first
    let mut idx = None;
    for (i, vc) in VERSION_CHANGES.iter().enumerate() {
        if vc.version == since_version {
            idx = Some(i);
            break;
        }
    }

    match idx {
        Some(0) => vec![], // already on newest
        Some(i) => {
            let mut result: Vec<_> = VERSION_CHANGES[..i].iter().collect();
            result.reverse(); // chronological order
            result
        }
        None => VERSION_CHANGES.iter().collect(), // unknown version, show all
    }
}
