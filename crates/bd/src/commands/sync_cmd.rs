//! `bd sync` -- deprecated command stub.
//!
//! Prints a deprecation message directing users to `bd dolt push` and `bd dolt pull`.

use anyhow::Result;

use crate::context::RuntimeContext;

/// Execute the `bd sync` command (deprecated).
pub fn run(_ctx: &RuntimeContext) -> Result<()> {
    eprintln!("bd sync is deprecated. Use 'bd dolt push' and 'bd dolt pull' instead.");
    Ok(())
}
