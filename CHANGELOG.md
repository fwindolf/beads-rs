# Changelog

All notable changes to beads-rs will be documented in this file.

## [0.2.1] - 2026-02-23

### Changed
- `bd onboard` rewritten: writes onboarding content (mulch knowledge + bd task tracking)
  directly into agent instruction files using HTML markers for safe insert/update
- Target flags: `--auto`, `--agents`, `--claude`, `--copilot`, `--codex`, `--opencode`
  (mutually exclusive) with auto-discovery fallback
- `bd onboard --check` to verify if onboard section is installed
- `bd onboard --remove` to remove the onboard section (deletes file if empty)
- 18 unit tests + 4 integration tests for onboard command

## [0.2.0] - 2026-02-23

### Added
- `bd quickstart` - Interactive quick-start guide with colored output showing
  all major workflows (creating issues, dependencies, ready work, agent integration)
- `bd onboard` - Display minimal AGENTS.md snippet for AI agent integration,
  with instructions for Claude Code and GitHub Copilot
- `bd bootstrap` - Explain SQLite bootstrap workflow (automatic via `bd init`)
- `bd preflight` - PR readiness checklist with `--check` mode that runs
  `cargo test` and `cargo clippy` automatically
- `bd prime` - AI-optimized workflow context output with MCP detection,
  stealth mode, ephemeral branch support, and custom PRIME.md overrides
- `bd upgrade status/review/ack` - Version tracking with embedded changelog,
  upgrade detection, and acknowledgement
- `bd worktree create/remove/list/info` - Full git worktree management with
  beads database sharing via redirect files, safety checks, and .gitignore management
- Integration tests for all new commands (14 new tests)

### Changed
- `bd preflight` now accepts `--check` and `--fix` flags (previously no-op stub)
- `bd prime` now accepts `--full`, `--mcp`, `--stealth`, `--export` flags
- `bd upgrade` is now a subcommand group with `status`, `review`, `ack`
- `bd worktree` now has `info` subcommand and `--branch`/`--force` flags

## [0.1.0] - 2025-12-01

### Added
- Initial release: Rust rewrite of beads issue tracker
- 104+ CLI commands matching Go `bd` interface
- SQLite storage backend (embedded, zero-configuration)
- Full dependency tracking with cycle detection
- Ready work detection (`bd ready`)
- Hash-based IDs (SHA256 + base36)
- JSON output on all commands (`--json`)
- Labels, comments, events - full issue lifecycle
- Search & filtering by status, type, priority, assignee, labels
- Statistics & views (count, stats, stale, orphans, history)
- Dependency graph visualization (ASCII, Graphviz DOT, JSON)
- Templates with `{{variable}}` substitution
- Gates (async workflow primitives)
- Formula engine (TOML-based workflow recipes)
- Swarm analysis (topological sort for parallel work)
- Shell completions (bash, zsh, fish, PowerShell)
- 189 tests (unit + integration)
