//! `bd worktree` -- manage git worktrees with beads database sharing.
//!
//! Creates worktrees that share the parent repo's `.beads/` database via
//! a redirect file, so all worktrees see the same issues.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::cli::{WorktreeArgs, WorktreeCommands, WorktreeCreateArgs, WorktreeRemoveArgs};
use crate::context::RuntimeContext;
use crate::output::output_json;

/// Execute the `bd worktree` command.
pub fn run(ctx: &RuntimeContext, args: &WorktreeArgs) -> Result<()> {
    match &args.command {
        WorktreeCommands::Create(a) => run_create(ctx, a),
        WorktreeCommands::Remove(a) => run_remove(ctx, a),
        WorktreeCommands::List => run_list(ctx),
        WorktreeCommands::Info => run_info(ctx),
    }
}

fn run_create(ctx: &RuntimeContext, args: &WorktreeCreateArgs) -> Result<()> {
    let name = args
        .name
        .as_deref()
        .context("worktree name is required")?;

    let branch = args.branch.as_deref().unwrap_or(name);

    // Get repo root
    let repo_root = git_repo_root()?;

    // Worktree path is relative to repo root
    let wt_path = repo_root.join(name);
    if wt_path.exists() {
        bail!("Path already exists: {}", wt_path.display());
    }

    // Find the main .beads/ directory
    let main_beads = repo_root.join(".beads");
    if !main_beads.is_dir() {
        bail!(
            "No .beads directory found at {}. Run 'bd init' first.",
            repo_root.display()
        );
    }

    // Create the worktree
    let output = Command::new("git")
        .args(["worktree", "add", "-b", branch])
        .arg(&wt_path)
        .output()
        .context("failed to run git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree add failed: {stderr}");
    }

    // Create .beads/redirect in the new worktree
    let wt_beads = wt_path.join(".beads");
    fs::create_dir_all(&wt_beads).context("failed to create .beads in worktree")?;

    let redirect_path = wt_beads.join("redirect");
    let main_beads_abs = fs::canonicalize(&main_beads)
        .unwrap_or_else(|_| main_beads.clone());
    fs::write(&redirect_path, main_beads_abs.to_string_lossy().as_bytes())
        .context("failed to write redirect file")?;

    // Add worktree path to .gitignore
    add_to_gitignore(&repo_root, name);

    if ctx.json {
        output_json(&serde_json::json!({
            "path": wt_path.display().to_string(),
            "branch": branch,
            "redirect_to": main_beads_abs.display().to_string(),
        }));
    } else {
        println!("Created worktree: {}", wt_path.display());
        println!("  Branch: {branch}");
        println!(
            "  Beads: redirects to {}",
            main_beads_abs.display()
        );
    }

    Ok(())
}

fn run_remove(ctx: &RuntimeContext, args: &WorktreeRemoveArgs) -> Result<()> {
    let repo_root = git_repo_root()?;
    let wt_path = resolve_worktree_path(&repo_root, &args.name)?;

    // Safety checks (unless --force)
    if !args.force {
        check_worktree_safety(&wt_path)?;
    }

    // Remove the worktree
    let mut cmd_args = vec!["worktree", "remove"];
    if args.force {
        cmd_args.push("--force");
    }
    let wt_str = wt_path.to_string_lossy().to_string();
    cmd_args.push(&wt_str);

    let output = Command::new("git")
        .args(&cmd_args)
        .output()
        .context("failed to run git worktree remove")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree remove failed: {stderr}");
    }

    // Clean up .gitignore entry
    remove_from_gitignore(&repo_root, &args.name);

    if ctx.json {
        output_json(&serde_json::json!({
            "removed": wt_path.display().to_string(),
        }));
    } else {
        println!("Removed worktree: {}", wt_path.display());
    }

    Ok(())
}

fn run_list(ctx: &RuntimeContext) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("failed to run git worktree list")?;

    if !output.status.success() {
        bail!("git worktree list failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let worktrees = parse_worktree_list(&stdout);

    let repo_root = git_repo_root().ok();
    let main_beads = repo_root.as_ref().map(|r| r.join(".beads"));

    if ctx.json {
        let entries: Vec<_> = worktrees
            .iter()
            .map(|wt| {
                let beads_state = main_beads
                    .as_ref()
                    .map(|mb| get_beads_state(&wt.path, mb))
                    .unwrap_or("none");
                serde_json::json!({
                    "path": wt.path,
                    "branch": wt.branch,
                    "bare": wt.bare,
                    "beads": beads_state,
                })
            })
            .collect();
        output_json(&serde_json::json!(entries));
    } else {
        println!(
            "{:<20} {:<50} {:<20} {}",
            "NAME", "PATH", "BRANCH", "BEADS"
        );
        for wt in &worktrees {
            let beads_state = main_beads
                .as_ref()
                .map(|mb| get_beads_state(&wt.path, mb))
                .unwrap_or("none");

            let name = Path::new(&wt.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| wt.path.clone());

            let display_name = if wt.bare {
                "(bare)".to_string()
            } else if Some(&wt.path)
                == repo_root.as_ref().map(|r| r.display().to_string()).as_ref()
            {
                "(main)".to_string()
            } else {
                name
            };

            println!(
                "{:<20} {:<50} {:<20} {}",
                display_name, wt.path, wt.branch, beads_state
            );
        }
    }

    Ok(())
}

fn run_info(ctx: &RuntimeContext) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Check if we're in a worktree
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .context("not in a git repository")?;

    let common_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()?;

    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let is_worktree = git_dir != common_dir && !common_dir.is_empty();

    // Get current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Get main repo root
    let main_root = if is_worktree {
        let common = PathBuf::from(&common_dir);
        common.parent().map(|p| p.to_path_buf())
    } else {
        Some(cwd.clone())
    };

    // Check for redirect
    let redirect_target = get_redirect_target(&cwd);

    if ctx.json {
        let mut info = serde_json::json!({
            "path": cwd.display().to_string(),
            "branch": branch,
            "is_worktree": is_worktree,
        });
        if let Some(ref root) = main_root {
            info["main_repo"] = serde_json::json!(root.display().to_string());
        }
        if let Some(ref target) = redirect_target {
            info["redirect_to"] = serde_json::json!(target);
        }
        output_json(&info);
    } else {
        println!("Worktree: {}", cwd.display());
        println!("  Branch: {branch}");
        if is_worktree {
            if let Some(ref root) = main_root {
                println!("  Main repo: {}", root.display());
            }
        } else {
            println!("  (this is the main working tree)");
        }
        if let Some(ref target) = redirect_target {
            println!("  Beads: redirects to {target}");
        } else if cwd.join(".beads").is_dir() {
            println!("  Beads: local .beads/ directory");
        } else {
            println!("  Beads: not configured");
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct WorktreeEntry {
    path: String,
    branch: String,
    bare: bool,
}

fn parse_worktree_list(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut path = String::new();
    let mut branch = String::new();
    let mut bare = false;

    for line in output.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            if !path.is_empty() {
                entries.push(WorktreeEntry {
                    path: std::mem::take(&mut path),
                    branch: std::mem::take(&mut branch),
                    bare,
                });
                bare = false;
            }
            path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            branch = b.to_string();
        } else if line == "bare" {
            bare = true;
        } else if line.is_empty() && !path.is_empty() {
            entries.push(WorktreeEntry {
                path: std::mem::take(&mut path),
                branch: std::mem::take(&mut branch),
                bare,
            });
            bare = false;
        }
    }

    if !path.is_empty() {
        entries.push(WorktreeEntry { path, branch, bare });
    }

    entries
}

fn git_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("not in a git repository")?;

    if !output.status.success() {
        bail!("not in a git repository");
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

fn resolve_worktree_path(repo_root: &Path, name: &str) -> Result<PathBuf> {
    // Try as absolute path
    let abs = PathBuf::from(name);
    if abs.is_absolute() && abs.exists() {
        return Ok(abs);
    }

    // Try relative to repo root
    let rel = repo_root.join(name);
    if rel.exists() {
        return Ok(rel);
    }

    bail!("Worktree not found: {name}");
}

fn check_worktree_safety(wt_path: &Path) -> Result<()> {
    // Check for uncommitted changes
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(wt_path)
        .output()
        .context("failed to check git status")?;

    let status = String::from_utf8_lossy(&output.stdout);
    if !status.trim().is_empty() {
        bail!(
            "Worktree has uncommitted changes. Use --force to remove anyway.\n{}",
            status.trim()
        );
    }

    Ok(())
}

fn get_beads_state(wt_path: &str, main_beads: &Path) -> &'static str {
    let path = Path::new(wt_path);
    let beads_dir = path.join(".beads");

    if !beads_dir.is_dir() {
        return "none";
    }

    let redirect = beads_dir.join("redirect");
    if redirect.exists() {
        return "redirect";
    }

    // Check if this IS the main beads dir
    if let Ok(canonical) = fs::canonicalize(&beads_dir) {
        if let Ok(main_canonical) = fs::canonicalize(main_beads) {
            if canonical == main_canonical {
                return "shared";
            }
        }
    }

    "local"
}

fn get_redirect_target(base: &Path) -> Option<String> {
    let redirect = base.join(".beads").join("redirect");
    fs::read_to_string(redirect).ok().map(|s| s.trim().to_string())
}

fn add_to_gitignore(repo_root: &Path, entry: &str) {
    let gitignore = repo_root.join(".gitignore");
    let content = fs::read_to_string(&gitignore).unwrap_or_default();

    // Check if already present
    let line = format!("/{entry}");
    if content.lines().any(|l| l.trim() == line) {
        return;
    }

    let mut new_content = content;
    if !new_content.ends_with('\n') && !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str(&format!("{line}\n"));
    let _ = fs::write(&gitignore, new_content);
}

fn remove_from_gitignore(repo_root: &Path, entry: &str) {
    let gitignore = repo_root.join(".gitignore");
    let content = match fs::read_to_string(&gitignore) {
        Ok(c) => c,
        Err(_) => return,
    };

    let line = format!("/{entry}");
    let new_content: Vec<&str> = content
        .lines()
        .filter(|l| l.trim() != line)
        .collect();

    let _ = fs::write(&gitignore, new_content.join("\n") + "\n");
}
