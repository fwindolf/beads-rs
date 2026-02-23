//! Clap CLI definitions for the `bd` command.
//!
//! This module defines the complete CLI structure using clap 4 derive macros.
//! It mirrors the Go Cobra command tree from the original beads project.

use clap::{Args, Parser, Subcommand};

/// bd -- Dependency-aware issue tracker.
///
/// Issues chained together like beads. A lightweight issue tracker
/// with first-class dependency support.
#[derive(Parser, Debug)]
#[command(
    name = "bd",
    about = "Dependency-aware issue tracker",
    long_about = "Issues chained together like beads. A lightweight issue tracker with first-class dependency support.",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Global flags available to all subcommands.
#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Database path (default: auto-discover .beads/*.db).
    #[arg(long, global = true)]
    pub db: Option<String>,

    /// Actor name for audit trail (default: $BD_ACTOR, git user.name, $USER).
    #[arg(long, global = true, env = "BD_ACTOR")]
    pub actor: Option<String>,

    /// Output in JSON format.
    #[arg(long, global = true)]
    pub json: bool,

    /// Sandbox mode: disable auto-sync.
    #[arg(long, global = true)]
    pub sandbox: bool,

    /// Allow operations on potentially stale data (skip staleness check).
    #[arg(long, global = true)]
    pub allow_stale: bool,

    /// Read-only mode: block write operations (for worker sandboxes).
    #[arg(long, global = true)]
    pub readonly: bool,

    /// Enable verbose/debug output.
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output (errors only).
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,
}

/// All available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ===== Working With Issues =====
    /// Create a new issue (or multiple issues from markdown file).
    #[command(alias = "new")]
    Create(CreateArgs),

    /// Show issue details.
    #[command(alias = "view")]
    Show(ShowArgs),

    /// List issues.
    List(ListArgs),

    /// Close one or more issues.
    Close(CloseArgs),

    /// Update issue fields.
    Update(UpdateArgs),

    /// Delete issues.
    Delete(DeleteArgs),

    /// Add a comment to an issue.
    Comment(CommentArgs),

    /// List comments on an issue.
    Comments(CommentsArgs),

    // ===== Views & Reports =====
    /// Show ready work (open, no active blockers).
    Ready(ReadyArgs),

    /// Full-text search across issues.
    Search(SearchArgs),

    // ===== Dependencies & Structure =====
    /// Manage dependencies between issues.
    Dep(DepArgs),

    /// Show child issues (shortcut for `dep children`).
    Children(ChildrenArgs),

    /// Add a "related" dependency between two issues.
    Relate(RelateArgs),

    /// Remove a "related" dependency between two issues.
    Unrelate(UnrelateArgs),

    // ===== Workflow Operations (Phase 3) =====
    /// Interactively edit an issue (stub).
    Edit(EditArgs),

    /// Rename an issue's title.
    Rename(RenameArgs),

    /// Rename issue ID prefix (stub).
    RenamePrefix(RenamePrefixArgs),

    /// Reopen a closed issue.
    Reopen(ReopenArgs),

    /// Get or set issue status.
    #[command(name = "status")]
    StatusCmd(StatusCmdArgs),

    /// Manage labels on an issue.
    Label(LabelArgs),

    /// Move an issue to a new prefix (stub).
    #[command(name = "move")]
    MoveCmd(MoveCmdArgs),

    /// Refile an issue (stub).
    Refile(RefileArgs),

    /// Defer an issue for later.
    Defer(DeferArgs),

    /// Undefer a deferred issue.
    Undefer(UndeferArgs),

    /// Mark an issue as a duplicate of another.
    #[command(name = "duplicate")]
    DuplicateCmd(DuplicateCmdArgs),

    /// Mark an issue as superseded by another.
    Supersede(SupersedeArgs),

    /// Show where an issue lives (stub).
    #[command(name = "where")]
    WhereCmd(WhereCmdArgs),

    /// Show last N modified issues.
    #[command(name = "last-touched")]
    LastTouched(LastTouchedArgs),

    /// Show open issues sorted by priority.
    Todo(TodoArgs),

    // ===== Views & Reports (Phase 2) =====
    /// Count issues by status.
    Count(CountArgs),

    /// Show project statistics.
    Stats(StatsArgs),

    /// Show stale issues (not updated in N days).
    Stale(StaleArgs),

    /// Show orphan issues (no dependencies at all).
    Orphans(OrphansArgs),

    /// Show event history for an issue.
    History(HistoryArgs),

    /// Show dependency diff between two points in time (not yet implemented).
    Diff,

    /// Display issue dependency graph.
    Graph(GraphArgs),

    /// Find duplicate issues (not yet implemented).
    #[command(alias = "find-duplicates")]
    Duplicates,

    /// Promote a child issue to top-level (not yet implemented).
    Promote,

    /// Create a git branch from an issue (not yet implemented).
    Branch,

    // ===== Setup & Configuration =====
    /// Initialize bd in the current directory.
    Init(InitArgs),

    /// Manage configuration.
    Config(ConfigArgs),

    /// Sync (deprecated: use 'bd dolt push' and 'bd dolt pull' instead).
    Sync,

    /// Print version information.
    Version,

    // ===== Molecules, Formulas & Templates (Phase 4 stubs) =====
    /// Molecule operations (group/workflow management).
    Mol(MolArgs),

    /// Formula operations (list, show, create, delete).
    Formula(FormulaArgs),

    /// Execute a formula (cook).
    Cook(CookArgs),

    /// Ephemeral formula execution.
    Wisp(WispArgs),

    /// Template operations (list, show, create, delete).
    Template(TemplateArgs),

    // ===== Phase 5: Sync, Import/Export & Integrations =====
    /// Import issues from external sources.
    Import(ImportArgs),

    /// Export issues to external formats.
    Export(ExportArgs),

    /// Jira integration.
    Jira(JiraArgs),

    /// Linear integration.
    Linear(LinearArgs),

    /// GitHub integration.
    #[command(name = "github")]
    Github(GithubArgs),

    /// GitLab integration.
    #[command(name = "gitlab")]
    Gitlab(GitlabArgs),

    /// Mail integration (delegates to external command).
    Mail(MailArgs),

    // ===== Phase 6: Database & Maintenance =====
    /// Check and repair database health.
    Doctor(DoctorArgs),

    /// Dolt-compatible database operations (stubs -- we use SQLite).
    Dolt(DoltArgs),

    /// Clean up temporary data and orphaned records.
    Cleanup,

    /// Compact the database (vacuum and optimize).
    Compact,

    /// Reset the database (WARNING: deletes all data).
    Reset,

    /// Run database migrations.
    Migrate,

    /// Administrative operations.
    Admin(AdminArgs),

    /// Detect pollution in issue data.
    #[command(name = "detect-pollution")]
    DetectPollution,

    /// Lint issues for common problems.
    Lint(LintArgs),

    /// Restore a deleted or archived issue.
    Restore(RestoreArgs),

    // ===== Phase 7: Advanced Features =====
    /// Agent operations (AI/automation agents).
    Agent(AgentArgs),

    /// Hook management (install, uninstall, list, test).
    Hook(HookArgs),

    /// Manage beads hooks.
    Hooks,

    /// Federation between beads instances.
    Federation,

    /// Version-control operations for beads data.
    Vc(VcArgs),

    /// Repository management.
    Repo(RepoArgs),

    /// Context management (set/get/clear working context).
    #[command(name = "context")]
    ContextCmd(ContextCmdArgs),

    /// Audit trail and compliance reporting.
    Audit,

    /// Swarm operations (distributed coordination).
    Swarm(SwarmArgs),

    /// Gate management (quality gates on issues).
    Gate(GateArgs),

    /// Slot management (time-boxed work slots).
    Slot,

    /// Merge a work slot back into the main timeline.
    #[command(name = "merge-slot")]
    MergeSlot,

    /// Pour issues into a container/molecule.
    Pour,

    /// Quick-create an issue with minimal input.
    Quick,

    /// Thank a contributor for their work on an issue.
    Thanks(ThanksArgs),

    /// List all known issue types (built-in + custom).
    Types(TypesArgs),

    /// Human-readable export/display.
    Human,

    /// Show issue details (alias for `show`).
    Info(InfoArgs),

    /// Route an issue to a team or person.
    Route,

    /// Show routed issues.
    Routed,

    /// Epic management.
    Epic,

    // ===== Phase 8: Utilities, Completion & Polish =====
    /// Execute a raw SQL query against the beads database.
    Query(QueryArgs),

    /// Interactive SQL shell (stub).
    Sql,

    /// Key-value metadata operations.
    Kv(KvArgs),

    /// Generate shell completions.
    Completion(CompletionArgs),

    /// Quick-start guide for new users.
    Quickstart,

    /// Write onboarding content to agent instruction file.
    Onboard(OnboardArgs),

    /// Bootstrap a beads project.
    Bootstrap,

    /// Run preflight checks.
    Preflight(PreflightArgs),

    /// Output AI-optimized workflow context.
    Prime(PrimeArgs),

    /// Check and manage bd version upgrades.
    Upgrade(UpgradeArgs),

    /// Manage git worktrees with shared beads database.
    Worktree(WorktreeArgs),
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

/// Arguments for `bd create`.
#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Issue title (positional argument).
    pub title: Option<String>,

    /// Issue title (alternative to positional argument).
    #[arg(long)]
    pub title_flag: Option<String>,

    /// Issue description.
    #[arg(short = 'd', long)]
    pub description: Option<String>,

    /// Issue type (bug|feature|task|epic|chore|decision).
    #[arg(short = 't', long = "type", default_value = "task")]
    pub issue_type: String,

    /// Priority (0-4 or P0-P4).
    #[arg(short = 'p', long, default_value = "2")]
    pub priority: String,

    /// Assignee.
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// Labels (comma-separated, repeatable).
    #[arg(short = 'l', long = "label", num_args = 1..)]
    pub labels: Vec<String>,

    /// Explicit issue ID (e.g., 'bd-42' for partitioning).
    #[arg(long)]
    pub id: Option<String>,

    /// Parent issue ID for hierarchical child.
    #[arg(long)]
    pub parent: Option<String>,

    /// Preview what would be created without actually creating.
    #[arg(long)]
    pub dry_run: bool,

    /// Output only the issue ID (for scripting).
    #[arg(long)]
    pub silent: bool,

    /// Force creation even if prefix doesn't match.
    #[arg(long)]
    pub force: bool,
}

// ---------------------------------------------------------------------------
// Show
// ---------------------------------------------------------------------------

/// Arguments for `bd show`.
#[derive(Args, Debug)]
pub struct ShowArgs {
    /// Issue IDs to display.
    #[arg(required = true)]
    pub ids: Vec<String>,

    /// Show compact one-line output per issue.
    #[arg(long)]
    pub short: bool,
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// Arguments for `bd list`.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Filter by status (open, in_progress, blocked, deferred, closed).
    #[arg(short = 's', long)]
    pub status: Option<String>,

    /// Filter by issue type.
    #[arg(short = 't', long = "type")]
    pub issue_type: Option<String>,

    /// Filter by assignee.
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// Filter by labels (AND: must have ALL).
    #[arg(short = 'l', long = "label", num_args = 1..)]
    pub labels: Vec<String>,

    /// Filter by labels (OR: must have ANY). Comma-separated.
    #[arg(long = "label-any", num_args = 1..)]
    pub label_any: Vec<String>,

    /// Filter by priority (0-4 or P0-P4).
    #[arg(short = 'p', long)]
    pub priority: Option<String>,

    /// Sort by field: priority, created, updated, closed, status, id, title, type, assignee.
    #[arg(long)]
    pub sort: Option<String>,

    /// Reverse sort order.
    #[arg(short = 'r', long)]
    pub reverse: bool,

    /// Limit results (default 50, use 0 for unlimited).
    #[arg(short = 'n', long, default_value = "50")]
    pub limit: i32,

    /// Show all issues including closed.
    #[arg(long)]
    pub all: bool,

    /// Show detailed multi-line output for each issue.
    #[arg(long)]
    pub long: bool,

    /// Display issues in a tree format with status/priority symbols.
    #[arg(long)]
    pub tree: bool,
}

// ---------------------------------------------------------------------------
// Close
// ---------------------------------------------------------------------------

/// Arguments for `bd close`.
#[derive(Args, Debug)]
pub struct CloseArgs {
    /// Issue IDs to close.
    pub ids: Vec<String>,

    /// Reason for closing.
    #[arg(short = 'r', long)]
    pub reason: Option<String>,

    /// Force close pinned issues or unsatisfied gates.
    #[arg(short = 'f', long)]
    pub force: bool,
}

// ---------------------------------------------------------------------------
// Ready
// ---------------------------------------------------------------------------

/// Arguments for `bd ready`.
#[derive(Args, Debug)]
pub struct ReadyArgs {
    /// Sort policy: priority (default), hybrid, oldest.
    #[arg(short = 's', long, default_value = "priority")]
    pub sort: String,

    /// Maximum issues to show.
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: i32,

    /// Filter by assignee.
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// Filter by labels (AND: must have ALL).
    #[arg(short = 'l', long = "label", num_args = 1..)]
    pub labels: Vec<String>,

    /// Filter by issue type.
    #[arg(short = 't', long = "type")]
    pub issue_type: Option<String>,

    /// Filter by priority.
    #[arg(short = 'p', long)]
    pub priority: Option<i32>,

    /// Show only unassigned issues.
    #[arg(short = 'u', long)]
    pub unassigned: bool,
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Arguments for `bd search`.
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query.
    pub query: String,

    /// Filter by status.
    #[arg(short = 's', long)]
    pub status: Option<String>,

    /// Filter by issue type.
    #[arg(short = 't', long = "type")]
    pub issue_type: Option<String>,

    /// Filter by assignee.
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// Filter by labels.
    #[arg(short = 'l', long = "label", num_args = 1..)]
    pub labels: Vec<String>,

    /// Limit results.
    #[arg(short = 'n', long, default_value = "50")]
    pub limit: i32,
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

/// Arguments for `bd delete`.
#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Issue IDs to delete.
    #[arg(required = true)]
    pub ids: Vec<String>,

    /// Force deletion without confirmation.
    #[arg(short = 'f', long)]
    pub force: bool,
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

/// Arguments for `bd init`.
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Issue prefix (default: current directory name).
    #[arg(short = 'p', long)]
    pub prefix: Option<String>,

    /// Suppress output.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Force re-initialization even if data already exists.
    #[arg(long)]
    pub force: bool,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Arguments for `bd config`.
#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

/// Config subcommands.
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Set a configuration value.
    Set(ConfigSetArgs),
    /// Get a configuration value.
    Get(ConfigGetArgs),
    /// List all configuration values.
    List,
    /// Unset a configuration value.
    Unset(ConfigUnsetArgs),
}

/// Arguments for `bd config set`.
#[derive(Args, Debug)]
pub struct ConfigSetArgs {
    /// Configuration key.
    pub key: String,
    /// Configuration value.
    pub value: String,
}

/// Arguments for `bd config get`.
#[derive(Args, Debug)]
pub struct ConfigGetArgs {
    /// Configuration key.
    pub key: String,
}

/// Arguments for `bd config unset`.
#[derive(Args, Debug)]
pub struct ConfigUnsetArgs {
    /// Configuration key.
    pub key: String,
}

// ---------------------------------------------------------------------------
// Dep
// ---------------------------------------------------------------------------

/// Arguments for `bd dep`.
#[derive(Args, Debug)]
pub struct DepArgs {
    #[command(subcommand)]
    pub command: DepCommands,
}

/// Dependency subcommands.
#[derive(Subcommand, Debug)]
pub enum DepCommands {
    /// Add a dependency between issues.
    Add(DepAddArgs),
    /// Remove a dependency between issues.
    Remove(DepRemoveArgs),
    /// List dependencies for an issue.
    List(DepListArgs),
    /// Detect dependency cycles.
    Cycles,
    /// Show parent issues (issues with parent-child dependency where given issue is the child).
    Parents(DepParentsArgs),
    /// Show child issues (issues that depend on given issue via parent-child).
    Children(DepChildrenArgs),
}

/// Arguments for `bd dep add`.
#[derive(Args, Debug)]
pub struct DepAddArgs {
    /// Source issue ID.
    pub from: String,
    /// Target issue ID.
    pub to: String,
    /// Dependency type (blocks, related, parent-child, discovered-from).
    #[arg(short = 't', long = "type", default_value = "blocks")]
    pub dep_type: String,
}

/// Arguments for `bd dep remove`.
#[derive(Args, Debug)]
pub struct DepRemoveArgs {
    /// Source issue ID.
    pub from: String,
    /// Target issue ID.
    pub to: String,
}

/// Arguments for `bd dep list`.
#[derive(Args, Debug)]
pub struct DepListArgs {
    /// Issue ID to list dependencies for.
    pub id: String,
}

/// Arguments for `bd dep parents`.
#[derive(Args, Debug)]
pub struct DepParentsArgs {
    /// Issue ID to find parents of.
    pub id: String,
}

/// Arguments for `bd dep children`.
#[derive(Args, Debug)]
pub struct DepChildrenArgs {
    /// Issue ID to find children of.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Comment
// ---------------------------------------------------------------------------

/// Arguments for `bd comment` (add a comment).
#[derive(Args, Debug)]
pub struct CommentArgs {
    /// Issue ID.
    pub id: String,
    /// Comment text (if not provided, opens editor).
    pub text: Option<String>,
}

/// Arguments for `bd comments` (list comments).
#[derive(Args, Debug)]
pub struct CommentsArgs {
    /// Issue ID.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

/// Arguments for `bd update`.
#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Issue ID to update.
    pub id: String,

    /// New title.
    #[arg(long)]
    pub title: Option<String>,

    /// New description.
    #[arg(short = 'd', long)]
    pub description: Option<String>,

    /// New issue type.
    #[arg(short = 't', long = "type")]
    pub issue_type: Option<String>,

    /// New priority (0-4 or P0-P4).
    #[arg(short = 'p', long)]
    pub priority: Option<String>,

    /// New assignee.
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// New status.
    #[arg(short = 's', long)]
    pub status: Option<String>,

    /// Add labels.
    #[arg(long = "add-label", num_args = 1..)]
    pub add_labels: Vec<String>,

    /// Remove labels.
    #[arg(long = "remove-label", num_args = 1..)]
    pub remove_labels: Vec<String>,
}

// ---------------------------------------------------------------------------
// Children (top-level alias)
// ---------------------------------------------------------------------------

/// Arguments for `bd children` (top-level alias for `bd dep children`).
#[derive(Args, Debug)]
pub struct ChildrenArgs {
    /// Issue ID to find children of.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Relate / Unrelate
// ---------------------------------------------------------------------------

/// Arguments for `bd relate`.
#[derive(Args, Debug)]
pub struct RelateArgs {
    /// Source issue ID.
    pub from: String,
    /// Target issue ID.
    pub to: String,
}

/// Arguments for `bd unrelate`.
#[derive(Args, Debug)]
pub struct UnrelateArgs {
    /// Source issue ID.
    pub from: String,
    /// Target issue ID.
    pub to: String,
}

// ---------------------------------------------------------------------------
// Count
// ---------------------------------------------------------------------------

/// Arguments for `bd count`.
#[derive(Args, Debug)]
pub struct CountArgs {
    /// Filter by status.
    #[arg(short = 's', long)]
    pub status: Option<String>,

    /// Filter by issue type.
    #[arg(short = 't', long = "type")]
    pub issue_type: Option<String>,

    /// Filter by assignee.
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// Group by status.
    #[arg(long)]
    pub by_status: bool,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Arguments for `bd stats`.
#[derive(Args, Debug)]
pub struct StatsArgs {
    // No additional arguments beyond global --json.
}

// ---------------------------------------------------------------------------
// Stale
// ---------------------------------------------------------------------------

/// Arguments for `bd stale`.
#[derive(Args, Debug)]
pub struct StaleArgs {
    /// Number of days without updates to consider stale (default 30).
    #[arg(short = 'd', long, default_value = "30")]
    pub days: i32,
}

// ---------------------------------------------------------------------------
// Orphans
// ---------------------------------------------------------------------------

/// Arguments for `bd orphans`.
#[derive(Args, Debug)]
pub struct OrphansArgs {
    // No additional arguments beyond global --json.
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

/// Arguments for `bd history`.
#[derive(Args, Debug)]
pub struct HistoryArgs {
    /// Issue ID to show history for.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

/// Arguments for `bd graph`.
#[derive(Args, Debug)]
pub struct GraphArgs {
    /// Issue ID to graph (show its dependency subgraph).
    pub id: Option<String>,

    /// Graph all open issues (finds connected components).
    #[arg(long)]
    pub all: bool,

    /// Output Graphviz DOT format.
    #[arg(long)]
    pub dot: bool,

    /// Compact tree output (default when not --dot or --json).
    #[arg(long)]
    pub compact: bool,
}

// ---------------------------------------------------------------------------
// Edit (Phase 3 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd edit`.
#[derive(Args, Debug)]
pub struct EditArgs {
    /// Issue ID to edit.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Rename
// ---------------------------------------------------------------------------

/// Arguments for `bd rename`.
#[derive(Args, Debug)]
pub struct RenameArgs {
    /// Issue ID to rename.
    pub id: String,
    /// New title for the issue.
    pub new_title: String,
}

// ---------------------------------------------------------------------------
// RenamePrefix (stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd rename-prefix`.
#[derive(Args, Debug)]
pub struct RenamePrefixArgs {
    /// Old prefix.
    pub old: String,
    /// New prefix.
    pub new: String,
}

// ---------------------------------------------------------------------------
// Reopen
// ---------------------------------------------------------------------------

/// Arguments for `bd reopen`.
#[derive(Args, Debug)]
pub struct ReopenArgs {
    /// Issue ID to reopen.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Status (get/set)
// ---------------------------------------------------------------------------

/// Arguments for `bd status`.
#[derive(Args, Debug)]
pub struct StatusCmdArgs {
    /// Issue ID.
    pub id: String,
    /// New status (if provided, sets the status; otherwise prints current status).
    pub new_status: Option<String>,
}

// ---------------------------------------------------------------------------
// Label (subcommands)
// ---------------------------------------------------------------------------

/// Arguments for `bd label`.
#[derive(Args, Debug)]
pub struct LabelArgs {
    /// Issue ID.
    pub id: String,
    #[command(subcommand)]
    pub command: LabelCommands,
}

/// Label subcommands.
#[derive(Subcommand, Debug)]
pub enum LabelCommands {
    /// Add a label to an issue.
    Add(LabelAddArgs),
    /// Remove a label from an issue.
    Remove(LabelRemoveArgs),
    /// List labels on an issue.
    List,
}

/// Arguments for `bd label <id> add`.
#[derive(Args, Debug)]
pub struct LabelAddArgs {
    /// Label to add.
    pub label: String,
}

/// Arguments for `bd label <id> remove`.
#[derive(Args, Debug)]
pub struct LabelRemoveArgs {
    /// Label to remove.
    pub label: String,
}

// ---------------------------------------------------------------------------
// Move (stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd move`.
#[derive(Args, Debug)]
pub struct MoveCmdArgs {
    /// Issue ID.
    pub id: String,
    /// New prefix.
    pub new_prefix: String,
}

// ---------------------------------------------------------------------------
// Refile (stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd refile`.
#[derive(Args, Debug)]
pub struct RefileArgs {
    /// Issue ID to refile.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Defer
// ---------------------------------------------------------------------------

/// Arguments for `bd defer`.
#[derive(Args, Debug)]
pub struct DeferArgs {
    /// Issue ID to defer.
    pub id: String,

    /// Defer until date (ISO 8601 date string, e.g. 2025-06-01).
    #[arg(long)]
    pub until: Option<String>,
}

// ---------------------------------------------------------------------------
// Undefer
// ---------------------------------------------------------------------------

/// Arguments for `bd undefer`.
#[derive(Args, Debug)]
pub struct UndeferArgs {
    /// Issue ID to undefer.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Duplicate
// ---------------------------------------------------------------------------

/// Arguments for `bd duplicate`.
#[derive(Args, Debug)]
pub struct DuplicateCmdArgs {
    /// Issue ID to mark as duplicate.
    pub id: String,
    /// Issue ID this is a duplicate of.
    pub duplicate_of: String,
}

// ---------------------------------------------------------------------------
// Supersede
// ---------------------------------------------------------------------------

/// Arguments for `bd supersede`.
#[derive(Args, Debug)]
pub struct SupersedeArgs {
    /// Issue ID to mark as superseded.
    pub id: String,
    /// Issue ID that supersedes this one.
    pub superseded_by: String,
}

// ---------------------------------------------------------------------------
// Where (stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd where`.
#[derive(Args, Debug)]
pub struct WhereCmdArgs {
    /// Issue ID to locate.
    pub id: String,
}

// ---------------------------------------------------------------------------
// LastTouched
// ---------------------------------------------------------------------------

/// Arguments for `bd last-touched`.
#[derive(Args, Debug)]
pub struct LastTouchedArgs {
    /// Maximum number of issues to show.
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: i32,
}

// ---------------------------------------------------------------------------
// Todo
// ---------------------------------------------------------------------------

/// Arguments for `bd todo`.
#[derive(Args, Debug)]
pub struct TodoArgs {
    /// Maximum number of issues to show.
    #[arg(short = 'n', long, default_value = "50")]
    pub limit: i32,
}

// ---------------------------------------------------------------------------
// Mol (Phase 4 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd mol`.
#[derive(Args, Debug)]
pub struct MolArgs {
    #[command(subcommand)]
    pub command: MolCommands,
}

/// Molecule subcommands.
#[derive(Subcommand, Debug)]
pub enum MolCommands {
    /// Show molecule details.
    Show(MolShowArgs),
    /// Pour issues into a molecule.
    Pour(MolPourArgs),
    /// Wisp operations within a molecule.
    Wisp(MolWispArgs),
    /// Bond issues together in a molecule.
    Bond(MolBondArgs),
    /// Squash molecule contents.
    Squash(MolSquashArgs),
    /// Burn (remove) a molecule.
    Burn(MolBurnArgs),
    /// Distill a molecule.
    Distill(MolDistillArgs),
    /// Seed a molecule.
    Seed(MolSeedArgs),
    /// Show stale molecules.
    Stale(MolStaleArgs),
    /// Show ready-gated molecules.
    ReadyGated(MolReadyGatedArgs),
    /// Show current molecule context.
    Current(MolCurrentArgs),
    /// Show molecule progress.
    Progress(MolProgressArgs),
}

/// Arguments for `bd mol show`.
#[derive(Args, Debug)]
pub struct MolShowArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol pour`.
#[derive(Args, Debug)]
pub struct MolPourArgs {
    /// Formula name or file path.
    pub id: Option<String>,

    /// Variable substitution (key=value), repeatable.
    #[arg(long = "var", num_args = 1..)]
    pub vars: Vec<String>,

    /// Preview cooked steps without creating issues.
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `bd mol wisp`.
#[derive(Args, Debug)]
pub struct MolWispArgs {
    /// Formula name or file path.
    pub id: Option<String>,

    /// Variable substitution (key=value), repeatable.
    #[arg(long = "var", num_args = 1..)]
    pub vars: Vec<String>,

    /// Preview cooked steps without creating issues.
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `bd mol bond`.
#[derive(Args, Debug)]
pub struct MolBondArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol squash`.
#[derive(Args, Debug)]
pub struct MolSquashArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol burn`.
#[derive(Args, Debug)]
pub struct MolBurnArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol distill`.
#[derive(Args, Debug)]
pub struct MolDistillArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol seed`.
#[derive(Args, Debug)]
pub struct MolSeedArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol stale`.
#[derive(Args, Debug)]
pub struct MolStaleArgs {
    /// Number of days to consider stale.
    #[arg(short = 'd', long, default_value = "30")]
    pub days: i32,
}

/// Arguments for `bd mol ready-gated`.
#[derive(Args, Debug)]
pub struct MolReadyGatedArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

/// Arguments for `bd mol current`.
#[derive(Args, Debug)]
pub struct MolCurrentArgs {}

/// Arguments for `bd mol progress`.
#[derive(Args, Debug)]
pub struct MolProgressArgs {
    /// Molecule identifier.
    pub id: Option<String>,
}

// ---------------------------------------------------------------------------
// Formula (Phase 4 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd formula`.
#[derive(Args, Debug)]
pub struct FormulaArgs {
    #[command(subcommand)]
    pub command: FormulaCommands,
}

/// Formula subcommands.
#[derive(Subcommand, Debug)]
pub enum FormulaCommands {
    /// List available formulas.
    List,
    /// Show formula details.
    Show(FormulaShowArgs),
    /// Create a new formula.
    Create(FormulaCreateArgs),
    /// Delete a formula.
    Delete(FormulaDeleteArgs),
}

/// Arguments for `bd formula show`.
#[derive(Args, Debug)]
pub struct FormulaShowArgs {
    /// Formula name or identifier.
    pub name: String,
}

/// Arguments for `bd formula create`.
#[derive(Args, Debug)]
pub struct FormulaCreateArgs {
    /// Formula name.
    pub name: String,
}

/// Arguments for `bd formula delete`.
#[derive(Args, Debug)]
pub struct FormulaDeleteArgs {
    /// Formula name or identifier.
    pub name: String,
}

// ---------------------------------------------------------------------------
// Cook (Phase 4 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd cook`.
#[derive(Args, Debug)]
pub struct CookArgs {
    /// Formula name or file path.
    pub formula: Option<String>,

    /// Variable substitution (key=value), repeatable.
    #[arg(long = "var", num_args = 1..)]
    pub vars: Vec<String>,

    /// Preview cooked steps without creating issues.
    #[arg(long)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Wisp (Phase 4 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd wisp`.
#[derive(Args, Debug)]
pub struct WispArgs {
    /// Wisp expression or identifier.
    pub expr: Option<String>,
}

// ---------------------------------------------------------------------------
// Template (Phase 4 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd template`.
#[derive(Args, Debug)]
pub struct TemplateArgs {
    #[command(subcommand)]
    pub command: TemplateCommands,
}

/// Template subcommands.
#[derive(Subcommand, Debug)]
pub enum TemplateCommands {
    /// List available templates.
    List,
    /// Show template details and extract variables.
    Show(TemplateShowArgs),
    /// Create a new template issue.
    Create(TemplateCreateArgs),
    /// Delete a template issue.
    Delete(TemplateDeleteArgs),
    /// Instantiate a template (clone with variable substitution).
    Instantiate(TemplateInstantiateArgs),
}

/// Arguments for `bd template show`.
#[derive(Args, Debug)]
pub struct TemplateShowArgs {
    /// Template issue ID.
    pub id: String,
}

/// Arguments for `bd template create`.
#[derive(Args, Debug)]
pub struct TemplateCreateArgs {
    /// Template title.
    pub title: String,

    /// Template description.
    #[arg(short = 'd', long)]
    pub description: Option<String>,

    /// Issue type (bug|feature|task|epic|chore|decision).
    #[arg(short = 't', long = "type", default_value = "task")]
    pub issue_type: String,

    /// Priority (0-4 or P0-P4).
    #[arg(short = 'p', long, default_value = "2")]
    pub priority: String,
}

/// Arguments for `bd template delete`.
#[derive(Args, Debug)]
pub struct TemplateDeleteArgs {
    /// Template issue ID.
    pub id: String,
}

/// Arguments for `bd template instantiate`.
#[derive(Args, Debug)]
pub struct TemplateInstantiateArgs {
    /// Template issue ID to instantiate.
    pub id: String,

    /// Variable substitution (key=value), repeatable.
    #[arg(long = "var", num_args = 1..)]
    pub vars: Vec<String>,

    /// ID prefix for new issues (default: use configured prefix).
    #[arg(long)]
    pub prefix: Option<String>,
}

// ---------------------------------------------------------------------------
// Import (Phase 5 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd import`.
#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Source file or URL to import from.
    pub source: Option<String>,

    /// Import format (json, csv, markdown).
    #[arg(short = 'f', long, default_value = "json")]
    pub format: String,
}

// ---------------------------------------------------------------------------
// Export (Phase 5 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd export`.
#[derive(Args, Debug)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: Option<ExportCommands>,
}

/// Export subcommands.
#[derive(Subcommand, Debug)]
pub enum ExportCommands {
    /// Export to Obsidian vault format.
    Obsidian(ExportObsidianArgs),
}

/// Arguments for `bd export obsidian`.
#[derive(Args, Debug)]
pub struct ExportObsidianArgs {
    /// Output directory for the Obsidian vault.
    pub output: Option<String>,
}

// ---------------------------------------------------------------------------
// Jira (Phase 5 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd jira`.
#[derive(Args, Debug)]
pub struct JiraArgs {
    #[command(subcommand)]
    pub command: JiraCommands,
}

/// Jira subcommands.
#[derive(Subcommand, Debug)]
pub enum JiraCommands {
    /// Configure Jira integration.
    Config,
    /// Sync issues with Jira.
    Sync,
    /// Import issues from Jira.
    Import,
}

// ---------------------------------------------------------------------------
// Linear (Phase 5 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd linear`.
#[derive(Args, Debug)]
pub struct LinearArgs {
    #[command(subcommand)]
    pub command: LinearCommands,
}

/// Linear subcommands.
#[derive(Subcommand, Debug)]
pub enum LinearCommands {
    /// Configure Linear integration.
    Config,
    /// Sync issues with Linear.
    Sync,
    /// Import issues from Linear.
    Import,
}

// ---------------------------------------------------------------------------
// Github (Phase 5 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd github`.
#[derive(Args, Debug)]
pub struct GithubArgs {
    #[command(subcommand)]
    pub command: GithubCommands,
}

/// GitHub subcommands.
#[derive(Subcommand, Debug)]
pub enum GithubCommands {
    /// Configure GitHub integration.
    Config,
    /// Sync issues with GitHub.
    Sync,
    /// Import issues from GitHub.
    Import,
}

// ---------------------------------------------------------------------------
// Gitlab (Phase 5 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd gitlab`.
#[derive(Args, Debug)]
pub struct GitlabArgs {
    #[command(subcommand)]
    pub command: GitlabCommands,
}

/// GitLab subcommands.
#[derive(Subcommand, Debug)]
pub enum GitlabCommands {
    /// Configure GitLab integration.
    Config,
    /// Sync issues with GitLab.
    Sync,
    /// Import issues from GitLab.
    Import,
}

// ---------------------------------------------------------------------------
// Mail (Phase 5 -- delegate to external command)
// ---------------------------------------------------------------------------

/// Arguments for `bd mail`.
#[derive(Args, Debug)]
pub struct MailArgs {
    /// Arguments passed through to the mail delegate command.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Doctor (Phase 6)
// ---------------------------------------------------------------------------

/// Arguments for `bd doctor`.
#[derive(Args, Debug)]
pub struct DoctorArgs {
    #[command(subcommand)]
    pub command: Option<DoctorCommands>,
}

/// Doctor subcommands.
#[derive(Subcommand, Debug)]
pub enum DoctorCommands {
    /// Attempt to fix detected issues.
    Fix,
    /// Check database health (default if no subcommand given).
    Health,
    /// Validate database schema and data integrity.
    Validate,
    /// Detect data pollution.
    Pollution,
    /// Check for orphaned artifacts.
    Artifacts,
}

// ---------------------------------------------------------------------------
// Dolt (Phase 6 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd dolt`.
#[derive(Args, Debug)]
pub struct DoltArgs {
    #[command(subcommand)]
    pub command: DoltCommands,
}

/// Dolt subcommands (stubs -- we use SQLite not Dolt).
#[derive(Subcommand, Debug)]
pub enum DoltCommands {
    /// Run a SQL query against the database.
    Sql(DoltSqlArgs),
    /// Show database status.
    Status,
    /// Show commit log.
    Log,
    /// Commit current state.
    Commit(DoltCommitArgs),
    /// Push changes to remote.
    Push,
    /// Pull changes from remote.
    Pull,
}

/// Arguments for `bd dolt sql`.
#[derive(Args, Debug)]
pub struct DoltSqlArgs {
    /// SQL query to execute.
    #[arg(short = 'q', long)]
    pub query: Option<String>,
}

/// Arguments for `bd dolt commit`.
#[derive(Args, Debug)]
pub struct DoltCommitArgs {
    /// Commit message.
    #[arg(short = 'm', long)]
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Admin (Phase 6 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd admin`.
#[derive(Args, Debug)]
pub struct AdminArgs {
    #[command(subcommand)]
    pub command: AdminCommands,
}

/// Admin subcommands.
#[derive(Subcommand, Debug)]
pub enum AdminCommands {
    /// Manage command aliases.
    Aliases,
    /// Run administrative cleanup.
    Cleanup,
    /// Administrative database compaction.
    Compact,
    /// Administrative database reset.
    Reset,
}

// ---------------------------------------------------------------------------
// Lint (Phase 6)
// ---------------------------------------------------------------------------

/// Arguments for `bd lint`.
#[derive(Args, Debug)]
pub struct LintArgs {
    /// Fix detected issues automatically where possible.
    #[arg(long)]
    pub fix: bool,
}

// ---------------------------------------------------------------------------
// Restore (Phase 6 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd restore`.
#[derive(Args, Debug)]
pub struct RestoreArgs {
    /// Issue ID to restore.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Agent (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd agent`.
#[derive(Args, Debug)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommands,
}

/// Agent subcommands.
#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// List registered agents.
    List,
    /// Show agent details.
    Show(AgentShowArgs),
    /// Set agent state (idle, spawning, running, working, stuck, done, stopped, dead).
    State(AgentStateArgs),
    /// Set agent to "running" and assign a hook bead.
    Run(AgentRunArgs),
    /// Route work to an agent.
    Route(AgentRouteArgs),
}

/// Arguments for `bd agent show`.
#[derive(Args, Debug)]
pub struct AgentShowArgs {
    /// Agent name or identifier.
    pub name: String,
}

/// Arguments for `bd agent state`.
#[derive(Args, Debug)]
pub struct AgentStateArgs {
    /// Agent name or identifier.
    pub name: String,
    /// New state (idle, spawning, running, working, stuck, done, stopped, dead).
    pub new_state: String,
}

/// Arguments for `bd agent run`.
#[derive(Args, Debug)]
pub struct AgentRunArgs {
    /// Agent name or identifier.
    pub name: String,
    /// Hook bead ID (the issue the agent is working on).
    pub hook_bead: String,
}

/// Arguments for `bd agent route`.
#[derive(Args, Debug)]
pub struct AgentRouteArgs {
    /// Agent name or identifier.
    pub name: String,
}

// ---------------------------------------------------------------------------
// Swarm (Phase 7)
// ---------------------------------------------------------------------------

/// Arguments for `bd swarm`.
#[derive(Args, Debug)]
pub struct SwarmArgs {
    #[command(subcommand)]
    pub command: SwarmCommands,
}

/// Swarm subcommands.
#[derive(Subcommand, Debug)]
pub enum SwarmCommands {
    /// Validate epic structure for swarming (dependency graph analysis).
    Validate(SwarmValidateArgs),
    /// Show current swarm status (progress through waves).
    Status(SwarmStatusArgs),
}

/// Arguments for `bd swarm validate`.
#[derive(Args, Debug)]
pub struct SwarmValidateArgs {
    /// Epic issue ID to analyze.
    pub epic_id: String,
}

/// Arguments for `bd swarm status`.
#[derive(Args, Debug)]
pub struct SwarmStatusArgs {
    /// Epic issue ID to show status for.
    pub epic_id: String,
}

// ---------------------------------------------------------------------------
// Hook (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd hook`.
#[derive(Args, Debug)]
pub struct HookArgs {
    #[command(subcommand)]
    pub command: HookCommands,
}

/// Hook subcommands.
#[derive(Subcommand, Debug)]
pub enum HookCommands {
    /// Install a hook.
    Install(HookInstallArgs),
    /// Uninstall a hook.
    Uninstall(HookUninstallArgs),
    /// List installed hooks.
    List,
    /// Test a hook.
    Test(HookTestArgs),
}

/// Arguments for `bd hook install`.
#[derive(Args, Debug)]
pub struct HookInstallArgs {
    /// Hook name.
    pub name: String,
}

/// Arguments for `bd hook uninstall`.
#[derive(Args, Debug)]
pub struct HookUninstallArgs {
    /// Hook name.
    pub name: String,
}

/// Arguments for `bd hook test`.
#[derive(Args, Debug)]
pub struct HookTestArgs {
    /// Hook name.
    pub name: String,
}

// ---------------------------------------------------------------------------
// Vc (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd vc`.
#[derive(Args, Debug)]
pub struct VcArgs {
    #[command(subcommand)]
    pub command: VcCommands,
}

/// Version-control subcommands.
#[derive(Subcommand, Debug)]
pub enum VcCommands {
    /// Commit beads data.
    Commit(VcCommitArgs),
    /// Push beads data to remote.
    Push,
    /// Pull beads data from remote.
    Pull,
    /// Show version-control status.
    Status,
}

/// Arguments for `bd vc commit`.
#[derive(Args, Debug)]
pub struct VcCommitArgs {
    /// Commit message.
    #[arg(short = 'm', long)]
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// Repo (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd repo`.
#[derive(Args, Debug)]
pub struct RepoArgs {
    #[command(subcommand)]
    pub command: RepoCommands,
}

/// Repo subcommands.
#[derive(Subcommand, Debug)]
pub enum RepoCommands {
    /// List known repositories.
    List,
    /// Show repository info.
    Info(RepoInfoArgs),
}

/// Arguments for `bd repo info`.
#[derive(Args, Debug)]
pub struct RepoInfoArgs {
    /// Repository name or path.
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Context (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd context`.
#[derive(Args, Debug)]
pub struct ContextCmdArgs {
    #[command(subcommand)]
    pub command: ContextCmdCommands,
}

/// Context subcommands.
#[derive(Subcommand, Debug)]
pub enum ContextCmdCommands {
    /// Set the working context.
    Set(ContextSetArgs),
    /// Get the current working context.
    Get,
    /// Clear the working context.
    Clear,
}

/// Arguments for `bd context set`.
#[derive(Args, Debug)]
pub struct ContextSetArgs {
    /// Context value (e.g., molecule ID, prefix, label).
    pub value: String,
}

// ---------------------------------------------------------------------------
// Gate (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd gate`.
#[derive(Args, Debug)]
pub struct GateArgs {
    #[command(subcommand)]
    pub command: GateCommands,
}

/// Gate subcommands.
#[derive(Subcommand, Debug)]
pub enum GateCommands {
    /// List all open gate issues.
    List,
    /// Show gate details (await_type, await_id, timeout, waiters, status).
    Show(GateShowArgs),
    /// Create a new gate issue.
    Create(GateCreateArgs),
    /// Close (satisfy) a gate manually.
    Close(GateCloseArgs),
    /// Check all open gates and auto-close resolved ones.
    Check,
}

/// Arguments for `bd gate show`.
#[derive(Args, Debug)]
pub struct GateShowArgs {
    /// Gate issue ID.
    pub id: String,
}

/// Arguments for `bd gate create`.
#[derive(Args, Debug)]
pub struct GateCreateArgs {
    /// Gate title.
    pub title: String,

    /// Await type (human|timer|gh:run|gh:pr).
    #[arg(long)]
    pub await_type: Option<String>,

    /// Await identifier (run ID, PR number, or duration like "30m").
    #[arg(long)]
    pub await_id: Option<String>,

    /// Timeout duration string (e.g. "30m", "2h", "1d"), stored as nanoseconds.
    #[arg(long)]
    pub timeout: Option<String>,

    /// Waiter to notify (repeatable).
    #[arg(long = "waiter", num_args = 1..)]
    pub waiters: Vec<String>,
}

/// Arguments for `bd gate close`.
#[derive(Args, Debug)]
pub struct GateCloseArgs {
    /// Gate issue ID.
    pub id: String,

    /// Reason for closing the gate.
    #[arg(short = 'r', long)]
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Thanks (Phase 7 stub)
// ---------------------------------------------------------------------------

/// Arguments for `bd thanks`.
#[derive(Args, Debug)]
pub struct ThanksArgs {
    /// Issue ID to thank the contributor for.
    pub id: String,
}

// ---------------------------------------------------------------------------
// Types (Phase 7 -- real implementation)
// ---------------------------------------------------------------------------

/// Arguments for `bd types`.
#[derive(Args, Debug)]
pub struct TypesArgs {
    // No additional arguments beyond global --json.
}

// ---------------------------------------------------------------------------
// Info (Phase 7 -- alias for show)
// ---------------------------------------------------------------------------

/// Arguments for `bd info` (alias for `bd show`).
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Issue IDs to display.
    #[arg(required = true)]
    pub ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Query (Phase 8 -- real implementation)
// ---------------------------------------------------------------------------

/// Arguments for `bd query`.
#[derive(Args, Debug)]
pub struct QueryArgs {
    /// SQL query to execute.
    pub sql: String,
}

// ---------------------------------------------------------------------------
// Kv (Phase 8 -- real implementation)
// ---------------------------------------------------------------------------

/// Arguments for `bd kv`.
#[derive(Args, Debug)]
pub struct KvArgs {
    #[command(subcommand)]
    pub command: KvCommands,
}

/// KV subcommands.
#[derive(Subcommand, Debug)]
pub enum KvCommands {
    /// Get a metadata value.
    Get(KvGetArgs),
    /// Set a metadata value.
    Set(KvSetArgs),
    /// List all metadata entries.
    List,
    /// Delete a metadata entry.
    Delete(KvDeleteArgs),
}

/// Arguments for `bd kv get`.
#[derive(Args, Debug)]
pub struct KvGetArgs {
    /// Metadata key.
    pub key: String,
}

/// Arguments for `bd kv set`.
#[derive(Args, Debug)]
pub struct KvSetArgs {
    /// Metadata key.
    pub key: String,
    /// Metadata value.
    pub value: String,
}

/// Arguments for `bd kv delete`.
#[derive(Args, Debug)]
pub struct KvDeleteArgs {
    /// Metadata key.
    pub key: String,
}

// ---------------------------------------------------------------------------
// Completion (Phase 8 -- real implementation)
// ---------------------------------------------------------------------------

/// Arguments for `bd completion`.
#[derive(Args, Debug)]
pub struct CompletionArgs {
    #[command(subcommand)]
    pub command: CompletionCommands,
}

/// Completion subcommands.
#[derive(Subcommand, Debug)]
pub enum CompletionCommands {
    /// Generate Bash completions.
    Bash,
    /// Generate Zsh completions.
    Zsh,
    /// Generate Fish completions.
    Fish,
    /// Generate PowerShell completions.
    Powershell,
}

// ---------------------------------------------------------------------------
// Preflight
// ---------------------------------------------------------------------------

/// Arguments for `bd onboard`.
#[derive(Args, Debug)]
pub struct OnboardArgs {
    /// Auto-discover target file (default).
    #[arg(long, group = "target")]
    pub auto: bool,

    /// Write to AGENTS.md.
    #[arg(long, group = "target")]
    pub agents: bool,

    /// Write to CLAUDE.md.
    #[arg(long, group = "target")]
    pub claude: bool,

    /// Write to .github/copilot-instructions.md.
    #[arg(long, group = "target")]
    pub copilot: bool,

    /// Write to CODEX.md.
    #[arg(long, group = "target")]
    pub codex: bool,

    /// Write to .opencode/instructions.md.
    #[arg(long, group = "target")]
    pub opencode: bool,

    /// Check if onboard section is installed.
    #[arg(long, conflicts_with = "remove")]
    pub check: bool,

    /// Remove the onboard section instead of writing it.
    #[arg(long, conflicts_with = "check")]
    pub remove: bool,
}

/// Arguments for `bd preflight`.
#[derive(Args, Debug)]
pub struct PreflightArgs {
    /// Run checks automatically instead of showing static checklist.
    #[arg(long)]
    pub check: bool,

    /// Auto-fix issues where possible (not yet implemented).
    #[arg(long)]
    pub fix: bool,
}

// ---------------------------------------------------------------------------
// Prime
// ---------------------------------------------------------------------------

/// Arguments for `bd prime`.
#[derive(Args, Debug)]
pub struct PrimeArgs {
    /// Force full CLI output (ignore MCP detection).
    #[arg(long)]
    pub full: bool,

    /// Force MCP mode (minimal output).
    #[arg(long)]
    pub mcp: bool,

    /// Stealth mode (no git operations, flush only).
    #[arg(long)]
    pub stealth: bool,

    /// Output default content (ignores PRIME.md override).
    #[arg(long)]
    pub export: bool,
}

// ---------------------------------------------------------------------------
// Upgrade
// ---------------------------------------------------------------------------

/// Arguments for `bd upgrade`.
#[derive(Args, Debug)]
pub struct UpgradeArgs {
    #[command(subcommand)]
    pub command: UpgradeCommands,
}

/// Upgrade subcommands.
#[derive(Subcommand, Debug)]
pub enum UpgradeCommands {
    /// Check if bd has been upgraded since last use.
    Status,
    /// Review changes since last bd version.
    Review,
    /// Acknowledge the current bd version.
    Ack,
}

// ---------------------------------------------------------------------------
// Worktree
// ---------------------------------------------------------------------------

/// Arguments for `bd worktree`.
#[derive(Args, Debug)]
pub struct WorktreeArgs {
    #[command(subcommand)]
    pub command: WorktreeCommands,
}

/// Worktree subcommands.
#[derive(Subcommand, Debug)]
pub enum WorktreeCommands {
    /// Create a new worktree with shared beads database.
    Create(WorktreeCreateArgs),
    /// Remove a worktree.
    Remove(WorktreeRemoveArgs),
    /// List all worktrees with beads state.
    List,
    /// Show info about the current worktree.
    Info,
}

/// Arguments for `bd worktree create`.
#[derive(Args, Debug)]
pub struct WorktreeCreateArgs {
    /// Name for the new worktree.
    pub name: Option<String>,

    /// Branch name (defaults to worktree name).
    #[arg(long)]
    pub branch: Option<String>,
}

/// Arguments for `bd worktree remove`.
#[derive(Args, Debug)]
pub struct WorktreeRemoveArgs {
    /// Name of the worktree to remove.
    pub name: String,

    /// Skip safety checks (uncommitted changes, unpushed commits).
    #[arg(long)]
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_version() {
        // Verify the parser doesn't panic for basic invocations
        let cli = Cli::try_parse_from(["bd", "version"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_create() {
        let cli = Cli::try_parse_from(["bd", "create", "Test issue"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Create(args)) => {
                assert_eq!(args.title, Some("Test issue".to_string()));
            }
            _ => panic!("Expected Create command"),
        }
    }

    #[test]
    fn cli_global_flags() {
        let cli = Cli::try_parse_from(["bd", "--json", "--verbose", "list"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(cli.global.json);
        assert!(cli.global.verbose);
    }

    #[test]
    fn cli_parses_config_set() {
        let cli = Cli::try_parse_from(["bd", "config", "set", "key", "value"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_dep_add() {
        let cli = Cli::try_parse_from(["bd", "dep", "add", "bd-abc", "bd-def"]);
        assert!(cli.is_ok());
    }
}
