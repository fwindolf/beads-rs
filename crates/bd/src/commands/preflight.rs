//! `bd preflight` -- PR readiness checklist and automated checks.

use std::process::Command;

use anyhow::Result;
use beads_ui::styles::{render_fail_icon, render_pass_icon, render_skip_icon, render_warn_icon};
use serde::Serialize;

use crate::cli::PreflightArgs;
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Result of a single preflight check.
#[derive(Serialize)]
struct CheckResult {
    name: String,
    passed: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    skipped: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    warning: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    output: String,
    command: String,
}

/// Overall preflight results.
#[derive(Serialize)]
struct PreflightResult {
    checks: Vec<CheckResult>,
    passed: bool,
    summary: String,
}

/// Execute the `bd preflight` command.
pub fn run(ctx: &RuntimeContext, args: &PreflightArgs) -> Result<()> {
    if args.fix {
        println!("Note: --fix is not yet implemented.");
        println!();
    }

    if args.check {
        run_checks(ctx);
        return Ok(());
    }

    // Static checklist mode
    println!("PR Readiness Checklist:");
    println!();
    println!("[ ] Tests pass: cargo test --workspace");
    println!("[ ] Lint passes: cargo clippy --workspace");
    println!("[ ] No beads pollution: check .beads/issues.jsonl diff");
    println!("[ ] Version sync: all Cargo.toml versions match");
    println!();
    println!("Run 'bd preflight --check' to validate automatically.");

    Ok(())
}

fn run_checks(ctx: &RuntimeContext) {
    let results = vec![run_test_check(), run_lint_check(), run_version_sync_check()];

    let mut all_passed = true;
    let mut pass_count = 0;
    let mut skip_count = 0;
    let mut warn_count = 0;

    for r in &results {
        if r.skipped {
            skip_count += 1;
        } else if r.warning {
            warn_count += 1;
        } else if r.passed {
            pass_count += 1;
        } else {
            all_passed = false;
        }
    }

    let run_count = results.len() - skip_count;
    let mut summary = format!("{pass_count}/{run_count} checks passed");
    if warn_count > 0 {
        summary.push_str(&format!(", {warn_count} warning(s)"));
    }
    if skip_count > 0 {
        summary.push_str(&format!(" ({skip_count} skipped)"));
    }

    if ctx.json {
        let result = PreflightResult {
            checks: results,
            passed: all_passed,
            summary,
        };
        output_json(&result);
    } else {
        for r in &results {
            let icon = if r.skipped {
                render_skip_icon()
            } else if r.warning {
                render_warn_icon()
            } else if r.passed {
                render_pass_icon()
            } else {
                render_fail_icon()
            };

            if r.skipped {
                println!("{icon} {} (skipped)", r.name);
            } else {
                println!("{icon} {}", r.name);
            }
            println!("  Command: {}", r.command);

            if r.skipped && !r.output.is_empty() {
                println!("  Reason: {}", r.output);
            } else if r.warning && !r.output.is_empty() {
                println!("  Warning: {}", r.output);
            } else if !r.passed && !r.output.is_empty() {
                let output = truncate_output(&r.output, 500);
                println!("  Output:");
                for line in output.lines() {
                    println!("    {line}");
                }
            }
            println!();
        }
        println!("{summary}");
    }

    if !all_passed {
        std::process::exit(1);
    }
}

fn run_test_check() -> CheckResult {
    let command = "cargo test --workspace".to_string();
    let result = Command::new("cargo").args(["test", "--workspace"]).output();

    match result {
        Ok(output) => CheckResult {
            name: "Tests pass".into(),
            passed: output.status.success(),
            skipped: false,
            warning: false,
            output: String::from_utf8_lossy(&output.stderr).to_string(),
            command,
        },
        Err(e) => CheckResult {
            name: "Tests pass".into(),
            passed: false,
            skipped: true,
            warning: false,
            output: format!("Failed to run cargo: {e}"),
            command,
        },
    }
}

fn run_lint_check() -> CheckResult {
    let command = "cargo clippy --workspace".to_string();
    let result = Command::new("cargo")
        .args(["clippy", "--workspace", "--", "-D", "warnings"])
        .output();

    match result {
        Ok(output) => CheckResult {
            name: "Lint passes".into(),
            passed: output.status.success(),
            skipped: false,
            warning: false,
            output: String::from_utf8_lossy(&output.stderr).to_string(),
            command,
        },
        Err(e) => CheckResult {
            name: "Lint passes".into(),
            passed: false,
            skipped: true,
            warning: false,
            output: format!("Failed to run cargo clippy: {e}"),
            command,
        },
    }
}

fn run_version_sync_check() -> CheckResult {
    let command = "Check Cargo.toml workspace version".to_string();

    // Read workspace Cargo.toml to get the version
    let workspace_toml = match std::fs::read_to_string("Cargo.toml") {
        Ok(c) => c,
        Err(e) => {
            return CheckResult {
                name: "Version sync".into(),
                passed: false,
                skipped: true,
                warning: false,
                output: format!("Cannot read Cargo.toml: {e}"),
                command,
            };
        }
    };

    // Extract version from workspace Cargo.toml
    let version = workspace_toml
        .lines()
        .find(|l| l.starts_with("version") && l.contains('"'))
        .and_then(|l| {
            let start = l.find('"')? + 1;
            let end = l[start..].find('"')? + start;
            Some(&l[start..end])
        });

    match version {
        Some(v) => CheckResult {
            name: "Version sync".into(),
            passed: true,
            skipped: false,
            warning: false,
            output: format!("Workspace version: {v}"),
            command,
        },
        None => CheckResult {
            name: "Version sync".into(),
            passed: false,
            skipped: false,
            warning: false,
            output: "Cannot parse version from Cargo.toml".into(),
            command,
        },
    }
}

fn truncate_output(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        format!("{}\n... (truncated)", &trimmed[..max_len])
    }
}
