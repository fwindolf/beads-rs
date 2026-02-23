//! Parse formula files (TOML and JSON) and resolve formula paths.

use std::path::{Path, PathBuf};

use crate::types::{Formula, FormulaError};

/// Parse a formula from a TOML string.
pub fn parse_toml(content: &str) -> Result<Formula, FormulaError> {
    toml::from_str(content).map_err(|e| FormulaError::Parse(e.to_string()))
}

/// Parse a formula from a JSON string.
pub fn parse_json(content: &str) -> Result<Formula, FormulaError> {
    serde_json::from_str(content).map_err(|e| FormulaError::Parse(e.to_string()))
}

/// Load a formula from a file path (auto-detect TOML vs JSON by extension).
pub fn load_formula(path: &Path) -> Result<Formula, FormulaError> {
    let content = std::fs::read_to_string(path)?;
    let mut formula = match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => parse_toml(&content)?,
        Some("json") => parse_json(&content)?,
        _ => {
            // Try JSON first, then TOML
            parse_json(&content).or_else(|_| parse_toml(&content))?
        }
    };
    formula.source = path.display().to_string();
    Ok(formula)
}

/// Search for a formula by name in standard locations.
///
/// Search order:
/// 1. Exact path (if it exists as-is)
/// 2. Current directory with standard extensions
/// 3. `.beads/formulas/` under cwd
/// 4. `~/.beads/formulas/`
pub fn find_formula(name: &str, cwd: &Path) -> Result<PathBuf, FormulaError> {
    // 1. Exact path
    let exact = Path::new(name);
    if exact.is_absolute() && exact.exists() {
        return Ok(exact.to_path_buf());
    }
    let relative = cwd.join(name);
    if relative.exists() {
        return Ok(relative);
    }

    // Standard suffixes to try
    let suffixes = [
        ".formula.toml",
        ".formula.json",
        ".toml",
        ".json",
    ];

    // 2. Current directory
    for suffix in &suffixes {
        let candidate = cwd.join(format!("{}{}", name, suffix));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 3. .beads/formulas/ under cwd
    let beads_formulas = cwd.join(".beads").join("formulas");
    if beads_formulas.is_dir() {
        for suffix in &suffixes {
            let candidate = beads_formulas.join(format!("{}{}", name, suffix));
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // 4. ~/.beads/formulas/
    if let Some(home) = home_dir() {
        let home_formulas = home.join(".beads").join("formulas");
        if home_formulas.is_dir() {
            for suffix in &suffixes {
                let candidate = home_formulas.join(format!("{}{}", name, suffix));
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }
    }

    Err(FormulaError::Parse(format!(
        "formula '{}' not found (searched cwd, .beads/formulas/, ~/.beads/formulas/)",
        name
    )))
}

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_minimal() {
        let json = r#"{"formula": "test", "steps": [{"id": "a", "title": "Do A"}]}"#;
        let f = parse_json(json).unwrap();
        assert_eq!(f.formula, "test");
        assert_eq!(f.steps.len(), 1);
        assert_eq!(f.steps[0].id, "a");
        assert_eq!(f.steps[0].r#type, "task"); // default
        assert_eq!(f.r#type, "workflow"); // default
    }

    #[test]
    fn parse_toml_with_vars() {
        let toml_str = r#"
formula = "mol-feature"
description = "Feature workflow"
version = 1

[vars.component]
description = "Component name"
required = true

[vars.owner]
description = "Who owns this"
default = "unassigned"

[[steps]]
id = "design"
title = "Design {{component}}"
type = "task"

[[steps]]
id = "implement"
title = "Implement {{component}}"
needs = ["design"]
"#;
        let f = parse_toml(toml_str).unwrap();
        assert_eq!(f.formula, "mol-feature");
        assert_eq!(f.vars.len(), 2);
        assert!(f.vars["component"].required);
        assert_eq!(f.vars["owner"].default.as_deref(), Some("unassigned"));
        assert_eq!(f.steps.len(), 2);
        assert_eq!(f.steps[1].needs, vec!["design"]);
    }

    #[test]
    fn parse_json_with_condition_and_gate() {
        let json = r#"{
            "formula": "release",
            "version": 1,
            "vars": {
                "type": {"description": "Release type", "required": true}
            },
            "steps": [
                {
                    "id": "tests",
                    "title": "Run tests",
                    "gate": {"type": "gh:run", "id": "ci.yml", "timeout": "30m"}
                },
                {
                    "id": "docs",
                    "title": "Update docs",
                    "condition": "{{type}} == major",
                    "needs": ["tests"]
                }
            ]
        }"#;
        let f = parse_json(json).unwrap();
        assert_eq!(f.steps[0].gate.as_ref().unwrap().r#type, "gh:run");
        assert_eq!(f.steps[1].condition.as_deref(), Some("{{type}} == major"));
    }
}
