//! `bd version` -- print version, build info, and platform.

use anyhow::Result;

use crate::context::RuntimeContext;
use crate::output::output_json;

/// Version string. Set at compile time via Cargo.toml (workspace version).
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build identifier. Can be overridden via environment variable at build time.
const BUILD: &str = {
    match option_env!("BD_BUILD") {
        Some(b) => b,
        None => "dev",
    }
};

/// Execute the `bd version` command.
pub fn run(ctx: &RuntimeContext) -> Result<()> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    if ctx.json {
        let info = serde_json::json!({
            "version": VERSION,
            "build": BUILD,
            "os": os,
            "arch": arch,
        });
        output_json(&info);
    } else {
        println!("bd version {} ({}) {}/{}", VERSION, BUILD, os, arch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constants_exist() {
        assert!(!VERSION.is_empty());
        assert!(!BUILD.is_empty());
    }
}
