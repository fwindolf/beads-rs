# beads-rs

A Rust rewrite of [beads](https://github.com/steveyegge/beads) — a dependency-aware issue tracker designed for AI agents.

## What is beads?

Beads is a lightweight, git-backed issue tracker with first-class dependency support. Its killer feature is `bd ready`: agents ask "what can I work on next?" and get back only unblocked tasks. No human needed to sequence work.

Originally written in Go by [Steve Yegge](https://github.com/steveyegge), this Rust port provides the same CLI interface (`bd`) with an embedded SQLite backend instead of Dolt.

## Install

### From GitHub Releases (recommended)

```bash
# Linux / macOS
curl -fsSL https://github.com/fwindolf/beads-rs/releases/latest/download/bd-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m).tar.gz | tar xz -C ~/.local/bin

# Windows (PowerShell)
Invoke-WebRequest -Uri "https://github.com/fwindolf/beads-rs/releases/latest/download/bd-windows-x86_64.zip" -OutFile bd.zip; Expand-Archive bd.zip -DestinationPath "$env:LOCALAPPDATA\bd"; $env:PATH += ";$env:LOCALAPPDATA\bd"
```

### From source

```bash
cargo install --git https://github.com/fwindolf/beads-rs --bin bd
```

## Quick start

```bash
# Initialize in any project directory
bd init

# Create issues
bd create "Fix login bug" -t bug -p 0
bd create "Add dark mode" -t feature -p 2
bd create "Write tests" -t task -p 1

# Add dependencies (tests blocked by dark mode)
bd dep add <tests-id> <dark-mode-id> --type blocks

# See what's ready to work on
bd ready

# Work on an issue
bd update <id> --status in_progress
bd close <id> -r "Done"

# Visualize the dependency graph
bd graph --all
```

## Features

### Core (fully implemented)
- **104+ CLI commands** matching the Go `bd` interface
- **Dependency tracking** with cycle detection and 18 relationship types
- **Ready work detection** — automatically computes unblocked tasks
- **Hash-based IDs** (SHA256 + base36) — collision-free distributed creation
- **SQLite storage** — embedded, zero-configuration
- **JSON output** on all commands (`--json`) for programmatic use
- **Labels, comments, events** — full issue lifecycle
- **Search & filtering** — by status, type, priority, assignee, labels
- **Statistics & views** — count, stats, stale, orphans, history

### Advanced (implemented)
- **Dependency graph visualization** — ASCII, Graphviz DOT, JSON
- **Templates** — reusable issue templates with `{{variable}}` substitution
- **Gates** — async workflow primitives (timer, human, GitHub CI/PR)
- **Formula engine** — TOML-based workflow recipes with conditions
- **Swarm analysis** — topological sort for parallel work planning
- **Agent state tracking** — lifecycle management for AI agents
- **Shell completions** — bash, zsh, fish, PowerShell

### Stubs (CLI accepts, not yet implemented)
- External integrations (Jira, Linear, GitLab, GitHub sync)
- Import/export (Obsidian, markdown)
- Molecules (advanced workflow orchestration)
- AI compaction

## Architecture

```
beads-rs/
  crates/
    bd/              # CLI binary (clap 4, 104+ commands)
    beads-core/      # Domain model, types, ID generation
    beads-storage/   # Storage trait + SQLite implementation
    beads-config/    # Configuration management
    beads-formula/   # Formula parsing and cooking engine
    beads-query/     # Query language (stub)
    beads-ui/        # Terminal styling (Ayu theme)
    beads-git/       # Git operations
    beads-timeparsing/
    beads-lockfile/
```

## Tests

```bash
cargo test --workspace    # 189 tests (unit + integration)
```

## Attribution

This project is a Rust rewrite of [beads](https://github.com/steveyegge/beads) by [Steve Yegge](https://github.com/steveyegge) and contributors. The original project is licensed under the MIT License.

The CLI interface, command names, flags, JSON output format, and core concepts (dependency types, ready work computation, hash-based IDs) are derived from the original Go implementation.

## License

MIT — see [LICENSE](LICENSE)
