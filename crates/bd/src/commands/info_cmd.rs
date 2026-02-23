//! `bd info` -- show issue details (alias for `bd show`).

use anyhow::Result;

use crate::cli::{InfoArgs, ShowArgs};
use crate::context::RuntimeContext;

/// Execute the `bd info` command by delegating to `bd show`.
pub fn run(ctx: &RuntimeContext, args: &InfoArgs) -> Result<()> {
    let show_args = ShowArgs {
        ids: args.ids.clone(),
        short: false,
    };
    super::show::run(ctx, &show_args)
}
