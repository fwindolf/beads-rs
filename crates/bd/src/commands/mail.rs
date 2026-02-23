//! `bd mail` -- mail integration (delegates to external command).
//!
//! Mail is pure delegation: it finds a configured external command and
//! passes through all arguments. The delegate is resolved in this order:
//!
//! 1. `BEADS_MAIL_DELEGATE` environment variable
//! 2. `BD_MAIL_DELEGATE` environment variable
//! 3. `mail.delegate` key in the beads config table
//!
//! If no delegate is found, a helpful error message is printed.

use anyhow::{bail, Context, Result};

use crate::cli::MailArgs;
use crate::context::RuntimeContext;

/// Execute the `bd mail` command.
pub fn run(ctx: &RuntimeContext, args: &MailArgs) -> Result<()> {
    // 1. Try environment variables
    let delegate = std::env::var("BEADS_MAIL_DELEGATE")
        .or_else(|_| std::env::var("BD_MAIL_DELEGATE"))
        .unwrap_or_default();

    // 2. If no env delegate, try loading from config database
    let delegate = if delegate.is_empty() {
        load_delegate_from_config(ctx).unwrap_or_default()
    } else {
        delegate
    };

    // 3. If still no delegate, print helpful error
    if delegate.is_empty() {
        bail!(
            "No mail delegate configured.\n\n\
             Set one of the following:\n\
             - Environment variable: BEADS_MAIL_DELEGATE='<command>'\n\
             - Environment variable: BD_MAIL_DELEGATE='<command>'\n\
             - Config:               bd config set mail.delegate '<command>'\n\n\
             Example delegates:\n\
             - 'sendmail -t'  (Unix sendmail)\n\
             - 'msmtp -t'    (msmtp relay)\n\
             - 'mailx'       (BSD mail)"
        );
    }

    // 4. Split delegate command into program and arguments
    let parts: Vec<&str> = delegate.split_whitespace().collect();
    if parts.is_empty() {
        bail!("mail delegate is empty after parsing: '{}'", delegate);
    }

    let program = parts[0];
    let base_args = &parts[1..];

    // 5. Execute delegate with pass-through
    let status = std::process::Command::new(program)
        .args(base_args)
        .args(&args.args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("failed to execute mail delegate: {}", program))?;

    std::process::exit(status.code().unwrap_or(1));
}

/// Try to load the mail delegate from the beads config table.
fn load_delegate_from_config(ctx: &RuntimeContext) -> Option<String> {
    let beads_dir = ctx.resolve_db_path()?;
    let db_path = beads_dir.join("beads.db");

    if !db_path.exists() {
        return None;
    }

    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;

    conn.query_row(
        "SELECT value FROM config WHERE key = 'mail.delegate'",
        [],
        |row| row.get(0),
    )
    .ok()
}
