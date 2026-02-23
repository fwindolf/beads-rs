//! `bd` -- dependency-aware issue tracker CLI.
//!
//! This is the entry point for the beads Rust port. It parses CLI arguments
//! with clap, resolves the runtime context, and dispatches to command handlers.

mod cli;
mod commands;
mod context;
mod output;

use std::sync::atomic::{AtomicBool, Ordering};

use clap::Parser;

use cli::{Cli, Commands};
use context::RuntimeContext;

/// Tracks whether a Ctrl+C has already been received.
static CTRLC_RECEIVED: AtomicBool = AtomicBool::new(false);

fn main() {
    // Install signal handlers for graceful shutdown.
    // First Ctrl+C: exit cleanly. Second: force exit.
    let _ = ctrlc::set_handler(|| {
        if CTRLC_RECEIVED.swap(true, Ordering::SeqCst) {
            // Second signal: force exit
            std::process::exit(1);
        }
        // First signal: exit cleanly
        std::process::exit(0);
    });

    // Parse CLI arguments
    let cli = Cli::parse();

    // Build runtime context from global args
    let ctx = RuntimeContext::from_global_args(&cli.global);

    // Set up logging based on verbosity
    if ctx.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("bd=debug")
            .with_writer(std::io::stderr)
            .init();
    }

    // Dispatch to command handler
    let result = match cli.command {
        Some(Commands::Version) => commands::version::run(&ctx),
        Some(Commands::Init(args)) => commands::init::run(&ctx, &args),
        Some(Commands::Create(args)) => commands::create::run(&ctx, &args),
        Some(Commands::Show(args)) => commands::show::run(&ctx, &args),
        Some(Commands::List(args)) => commands::list::run(&ctx, &args),
        Some(Commands::Close(args)) => commands::close::run(&ctx, &args),
        Some(Commands::Ready(args)) => commands::ready::run(&ctx, &args),
        Some(Commands::Search(args)) => commands::search::run(&ctx, &args),
        Some(Commands::Delete(args)) => commands::delete::run(&ctx, &args),
        Some(Commands::Config(args)) => commands::config_cmd::run(&ctx, &args),
        Some(Commands::Dep(args)) => commands::dep::run(&ctx, &args),
        Some(Commands::Comment(args)) => commands::comment::run_add(&ctx, &args),
        Some(Commands::Comments(args)) => commands::comment::run_list(&ctx, &args),
        Some(Commands::Update(args)) => commands::update::run(&ctx, &args),
        Some(Commands::Sync) => commands::sync_cmd::run(&ctx),
        // Phase 2: Dependencies & Structure
        Some(Commands::Children(args)) => commands::children_cmd::run(&ctx, &args),
        Some(Commands::Relate(args)) => commands::relate::run_relate(&ctx, &args),
        Some(Commands::Unrelate(args)) => commands::relate::run_unrelate(&ctx, &args),
        // Phase 2: Views & Reports
        Some(Commands::Count(args)) => commands::count::run(&ctx, &args),
        Some(Commands::Stats(args)) => commands::stats::run(&ctx, &args),
        Some(Commands::Stale(args)) => commands::stale::run(&ctx, &args),
        Some(Commands::Orphans(args)) => commands::orphans::run(&ctx, &args),
        Some(Commands::History(args)) => commands::history::run(&ctx, &args),
        Some(Commands::Diff) => commands::diff_cmd::run(&ctx),
        Some(Commands::Graph(args)) => commands::graph::run(&ctx, &args),
        Some(Commands::Duplicates) => commands::duplicates::run(&ctx),
        Some(Commands::Promote) => commands::promote::run(&ctx),
        Some(Commands::Branch) => commands::branch::run(&ctx),
        // Phase 3: Workflow Operations
        Some(Commands::Edit(args)) => commands::edit::run(&ctx, &args),
        Some(Commands::Rename(args)) => commands::rename::run(&ctx, &args),
        Some(Commands::RenamePrefix(args)) => commands::rename_prefix::run(&ctx, &args),
        Some(Commands::Reopen(args)) => commands::reopen::run(&ctx, &args),
        Some(Commands::StatusCmd(args)) => commands::status_cmd::run(&ctx, &args),
        Some(Commands::Label(args)) => commands::label::run(&ctx, &args),
        Some(Commands::MoveCmd(args)) => commands::move_cmd::run(&ctx, &args),
        Some(Commands::Refile(args)) => commands::refile::run(&ctx, &args),
        Some(Commands::Defer(args)) => commands::defer_cmd::run(&ctx, &args),
        Some(Commands::Undefer(args)) => commands::undefer::run(&ctx, &args),
        Some(Commands::DuplicateCmd(args)) => commands::duplicate_cmd::run(&ctx, &args),
        Some(Commands::Supersede(args)) => commands::supersede::run(&ctx, &args),
        Some(Commands::WhereCmd(args)) => commands::where_cmd::run(&ctx, &args),
        Some(Commands::LastTouched(args)) => commands::last_touched::run(&ctx, &args),
        Some(Commands::Todo(args)) => commands::todo::run(&ctx, &args),
        // Phase 4: Molecules, Formulas & Templates
        Some(Commands::Mol(args)) => commands::mol::run(&ctx, &args),
        Some(Commands::Formula(args)) => commands::formula::run(&ctx, &args),
        Some(Commands::Cook(args)) => commands::cook::run(&ctx, &args),
        Some(Commands::Wisp(args)) => commands::wisp::run(&ctx, &args),
        Some(Commands::Template(args)) => commands::template::run(&ctx, &args),
        // Phase 5: Sync, Import/Export & Integrations
        Some(Commands::Import(args)) => commands::import::run(&ctx, &args),
        Some(Commands::Export(args)) => commands::export::run(&ctx, &args),
        Some(Commands::Jira(args)) => commands::jira::run(&ctx, &args),
        Some(Commands::Linear(args)) => commands::linear::run(&ctx, &args),
        Some(Commands::Github(args)) => commands::github::run(&ctx, &args),
        Some(Commands::Gitlab(args)) => commands::gitlab::run(&ctx, &args),
        Some(Commands::Mail(args)) => commands::mail::run(&ctx, &args),
        // Phase 6: Database & Maintenance
        Some(Commands::Doctor(args)) => commands::doctor::run(&ctx, &args),
        Some(Commands::Dolt(args)) => commands::dolt::run(&ctx, &args),
        Some(Commands::Cleanup) => commands::cleanup::run(&ctx),
        Some(Commands::Compact) => commands::compact::run(&ctx),
        Some(Commands::Reset) => commands::reset::run(&ctx),
        Some(Commands::Migrate) => commands::migrate::run(&ctx),
        Some(Commands::Admin(args)) => commands::admin::run(&ctx, &args),
        Some(Commands::DetectPollution) => commands::detect_pollution::run(&ctx),
        Some(Commands::Lint(args)) => commands::lint::run(&ctx, &args),
        Some(Commands::Restore(args)) => commands::restore::run(&ctx, &args),
        // Phase 7: Advanced Features
        Some(Commands::Agent(args)) => commands::agent::run(&ctx, &args),
        Some(Commands::Hook(args)) => commands::hook::run(&ctx, &args),
        Some(Commands::Hooks) => commands::phase7_stubs::run_hooks(&ctx),
        Some(Commands::Federation) => commands::phase7_stubs::run_federation(&ctx),
        Some(Commands::Vc(args)) => commands::vc::run(&ctx, &args),
        Some(Commands::Repo(args)) => commands::repo_cmd::run(&ctx, &args),
        Some(Commands::ContextCmd(args)) => commands::context_cmd::run(&ctx, &args),
        Some(Commands::Audit) => commands::phase7_stubs::run_audit(&ctx),
        Some(Commands::Swarm(args)) => commands::swarm::run(&ctx, &args),
        Some(Commands::Gate(args)) => commands::gate::run(&ctx, &args),
        Some(Commands::Slot) => commands::phase7_stubs::run_slot(&ctx),
        Some(Commands::MergeSlot) => commands::phase7_stubs::run_merge_slot(&ctx),
        Some(Commands::Pour) => commands::phase7_stubs::run_pour(&ctx),
        Some(Commands::Quick) => commands::phase7_stubs::run_quick(&ctx),
        Some(Commands::Thanks(args)) => commands::thanks::run(&ctx, &args),
        Some(Commands::Types(args)) => commands::types_cmd::run(&ctx, &args),
        Some(Commands::Human) => commands::phase7_stubs::run_human(&ctx),
        Some(Commands::Info(args)) => commands::info_cmd::run(&ctx, &args),
        Some(Commands::Route) => commands::phase7_stubs::run_route(&ctx),
        Some(Commands::Routed) => commands::phase7_stubs::run_routed(&ctx),
        Some(Commands::Epic) => commands::phase7_stubs::run_epic(&ctx),
        // Phase 8: Utilities, Completion & Polish
        Some(Commands::Query(args)) => commands::query::run(&ctx, &args),
        Some(Commands::Sql) => commands::misc::run_sql(&ctx),
        Some(Commands::Kv(args)) => commands::kv::run(&ctx, &args),
        Some(Commands::Completion(args)) => commands::completion::run(&ctx, &args),
        Some(Commands::Quickstart) => commands::quickstart::run(&ctx),
        Some(Commands::Onboard(args)) => commands::onboard::run(&ctx, &args),
        Some(Commands::Bootstrap) => commands::misc::run_bootstrap(&ctx),
        Some(Commands::Preflight(args)) => commands::preflight::run(&ctx, &args),
        Some(Commands::Prime(args)) => commands::prime::run(&ctx, &args),
        Some(Commands::Upgrade(args)) => commands::upgrade::run(&ctx, &args.command),
        Some(Commands::Worktree(args)) => commands::worktree::run(&ctx, &args),
        None => {
            // No subcommand -- print help
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!();
            Ok(())
        }
    };

    // Handle errors: print message and exit with code 1
    if let Err(e) = result {
        // For JSON mode, output error as JSON
        if cli.global.json {
            let err_json = serde_json::json!({
                "error": format!("{:#}", e),
            });
            if let Ok(s) = serde_json::to_string_pretty(&err_json) {
                eprintln!("{}", s);
            }
        } else {
            eprintln!("Error: {:#}", e);
        }
        std::process::exit(1);
    }
}
