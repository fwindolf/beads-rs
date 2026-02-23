//! Cook/expand formulas: variable substitution, condition evaluation, step filtering.

use std::collections::{HashMap, HashSet};

use crate::types::{CookedStep, Formula, FormulaError};

/// Substitute `{{variable}}` patterns in a string with provided values.
/// Unresolved variables are left as-is.
pub fn substitute_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if i + 4 <= len && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start < len && is_var_start(bytes[start]) {
                let mut end = start + 1;
                while end < len && is_var_cont(bytes[end]) {
                    end += 1;
                }
                if end + 1 < len && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                    let name = &text[start..end];
                    if let Some(val) = vars.get(name) {
                        result.push_str(val);
                    } else {
                        result.push_str(&text[i..end + 2]);
                    }
                    i = end + 2;
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Extract all `{{variable}}` names referenced in a formula's steps
/// (titles, descriptions, assignees, and conditions).
pub fn extract_variables(formula: &Formula) -> Vec<String> {
    let mut vars: HashSet<String> = HashSet::new();
    for step in &formula.steps {
        scan_vars(&step.title, &mut vars);
        scan_vars(&step.description, &mut vars);
        if let Some(ref a) = step.assignee {
            scan_vars(a, &mut vars);
        }
        if let Some(ref c) = step.condition {
            scan_vars(c, &mut vars);
        }
    }
    let mut result: Vec<String> = vars.into_iter().collect();
    result.sort();
    result
}

/// Validate that all required variables are provided.
/// Missing required vars with no default trigger an error.
pub fn validate_vars(
    formula: &Formula,
    provided: &HashMap<String, String>,
) -> Result<(), FormulaError> {
    for (name, def) in &formula.vars {
        if def.required && !provided.contains_key(name) {
            return Err(FormulaError::MissingVariable(name.clone()));
        }
    }
    Ok(())
}

/// Build the full variable map: provided values override defaults.
pub fn resolve_vars(
    formula: &Formula,
    provided: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for (name, def) in &formula.vars {
        if let Some(ref default) = def.default {
            vars.insert(name.clone(), default.clone());
        }
    }
    // Provided values override defaults
    for (k, v) in provided {
        vars.insert(k.clone(), v.clone());
    }
    vars
}

/// Evaluate a simple condition string against variables.
///
/// Supported formats:
/// - `"{{var}}"` -- truthy (non-empty and not "false"/"0")
/// - `"!{{var}}"` -- negated truthy
/// - `"{{var}} == value"` -- equality
/// - `"{{var}} != value"` -- inequality
///
/// Returns `true` if the condition passes (step should be included).
pub fn evaluate_condition(condition: &str, vars: &HashMap<String, String>) -> bool {
    let cond = condition.trim();
    if cond.is_empty() {
        return true;
    }

    // Check for == or !=
    if let Some(pos) = cond.find("!=") {
        let lhs = substitute_vars(cond[..pos].trim(), vars);
        let rhs = cond[pos + 2..].trim();
        return lhs != rhs;
    }
    if let Some(pos) = cond.find("==") {
        let lhs = substitute_vars(cond[..pos].trim(), vars);
        let rhs = cond[pos + 2..].trim();
        return lhs == rhs;
    }

    // Truthy / negated truthy
    let (negated, expr) = if let Some(stripped) = cond.strip_prefix('!') {
        (true, stripped.trim())
    } else {
        (false, cond)
    };

    let resolved = substitute_vars(expr, vars);
    let truthy = !resolved.is_empty() && resolved != "false" && resolved != "0";
    if negated { !truthy } else { truthy }
}

/// Cook a formula: validate variables, evaluate conditions, substitute, and filter.
///
/// Returns the list of steps that should be created as issues.
pub fn cook(
    formula: &Formula,
    provided: &HashMap<String, String>,
) -> Result<Vec<CookedStep>, FormulaError> {
    // 1. Validate required variables
    validate_vars(formula, provided)?;

    // 2. Build full variable map (defaults + provided)
    let vars = resolve_vars(formula, provided);

    // 3. Evaluate conditions and collect surviving step IDs
    let mut included_ids: HashSet<String> = HashSet::new();
    for step in &formula.steps {
        if let Some(ref cond) = step.condition {
            if !evaluate_condition(cond, &vars) {
                continue;
            }
        }
        included_ids.insert(step.id.clone());
    }

    // 4. Build cooked steps, filtering out deps that reference removed steps
    let mut cooked = Vec::new();
    for step in &formula.steps {
        if !included_ids.contains(&step.id) {
            continue;
        }

        let needs: Vec<String> = step
            .needs
            .iter()
            .filter(|dep| included_ids.contains(dep.as_str()))
            .cloned()
            .collect();

        cooked.push(CookedStep {
            id: step.id.clone(),
            title: substitute_vars(&step.title, &vars),
            description: substitute_vars(&step.description, &vars),
            issue_type: step.r#type.clone(),
            priority: step.priority,
            needs,
            gate: step.gate.clone(),
            assignee: step.assignee.as_ref().map(|a| substitute_vars(a, &vars)),
            labels: step.labels.clone(),
        });
    }

    Ok(cooked)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn is_var_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_var_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Scan a string for `{{name}}` patterns and insert variable names into the set.
fn scan_vars(text: &str, vars: &mut HashSet<String>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + 4 < len {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start < len && is_var_start(bytes[start]) {
                let mut end = start + 1;
                while end < len && is_var_cont(bytes[end]) {
                    end += 1;
                }
                if end + 1 < len && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                    let name = &text[start..end];
                    vars.insert(name.to_string());
                    i = end + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Formula, Step, StepGate, VarDef};

    fn make_vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // -- substitute_vars ---------------------------------------------------

    #[test]
    fn substitute_simple() {
        let vars = make_vars(&[("name", "auth")]);
        assert_eq!(substitute_vars("Design {{name}}", &vars), "Design auth");
    }

    #[test]
    fn substitute_multiple() {
        let vars = make_vars(&[("a", "X"), ("b", "Y")]);
        assert_eq!(substitute_vars("{{a}}-{{b}}", &vars), "X-Y");
    }

    #[test]
    fn substitute_missing_left_alone() {
        let vars = make_vars(&[("a", "X")]);
        assert_eq!(substitute_vars("{{a}} {{missing}}", &vars), "X {{missing}}");
    }

    #[test]
    fn substitute_no_vars() {
        let vars = HashMap::new();
        assert_eq!(substitute_vars("plain text", &vars), "plain text");
    }

    // -- evaluate_condition ------------------------------------------------

    #[test]
    fn condition_equality() {
        let vars = make_vars(&[("type", "feature")]);
        assert!(evaluate_condition("{{type}} == feature", &vars));
        assert!(!evaluate_condition("{{type}} == bug", &vars));
    }

    #[test]
    fn condition_inequality() {
        let vars = make_vars(&[("type", "feature")]);
        assert!(evaluate_condition("{{type}} != bug", &vars));
        assert!(!evaluate_condition("{{type}} != feature", &vars));
    }

    #[test]
    fn condition_truthy() {
        let vars = make_vars(&[("docs", "yes")]);
        assert!(evaluate_condition("{{docs}}", &vars));
    }

    #[test]
    fn condition_falsy_zero() {
        let vars = make_vars(&[("docs", "0")]);
        assert!(!evaluate_condition("{{docs}}", &vars));
    }

    #[test]
    fn condition_negated() {
        let vars = make_vars(&[("skip", "false")]);
        assert!(evaluate_condition("!{{skip}}", &vars));
    }

    #[test]
    fn condition_empty_passes() {
        let vars = HashMap::new();
        assert!(evaluate_condition("", &vars));
    }

    // -- extract_variables -------------------------------------------------

    #[test]
    fn extract_vars_from_formula() {
        let f = Formula {
            formula: "test".into(),
            description: String::new(),
            r#type: "workflow".into(),
            version: 1,
            vars: HashMap::new(),
            steps: vec![
                Step {
                    id: "a".into(),
                    title: "Design {{component}}".into(),
                    description: "For {{owner}}".into(),
                    r#type: "task".into(),
                    priority: 2,
                    needs: vec![],
                    condition: None,
                    gate: None,
                    assignee: Some("{{owner}}".into()),
                    labels: vec![],
                },
            ],
            source: String::new(),
        };
        let vars = extract_variables(&f);
        assert_eq!(vars, vec!["component", "owner"]);
    }

    // -- cook --------------------------------------------------------------

    #[test]
    fn cook_basic() {
        let f = Formula {
            formula: "test".into(),
            description: String::new(),
            r#type: "workflow".into(),
            version: 1,
            vars: HashMap::from([
                ("name".into(), VarDef {
                    description: String::new(),
                    required: true,
                    default: None,
                }),
            ]),
            steps: vec![
                Step {
                    id: "design".into(),
                    title: "Design {{name}}".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 2,
                    needs: vec![],
                    condition: None,
                    gate: None,
                    assignee: None,
                    labels: vec![],
                },
                Step {
                    id: "impl".into(),
                    title: "Implement {{name}}".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 2,
                    needs: vec!["design".into()],
                    condition: None,
                    gate: None,
                    assignee: None,
                    labels: vec![],
                },
            ],
            source: String::new(),
        };

        let vars = make_vars(&[("name", "auth")]);
        let cooked = cook(&f, &vars).unwrap();
        assert_eq!(cooked.len(), 2);
        assert_eq!(cooked[0].title, "Design auth");
        assert_eq!(cooked[1].title, "Implement auth");
        assert_eq!(cooked[1].needs, vec!["design"]);
    }

    #[test]
    fn cook_filters_by_condition() {
        let f = Formula {
            formula: "test".into(),
            description: String::new(),
            r#type: "workflow".into(),
            version: 1,
            vars: HashMap::from([
                ("type".into(), VarDef {
                    description: String::new(),
                    required: true,
                    default: None,
                }),
            ]),
            steps: vec![
                Step {
                    id: "tests".into(),
                    title: "Run tests".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 2,
                    needs: vec![],
                    condition: None,
                    gate: None,
                    assignee: None,
                    labels: vec![],
                },
                Step {
                    id: "docs".into(),
                    title: "Update docs".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 3,
                    needs: vec!["tests".into()],
                    condition: Some("{{type}} == major".into()),
                    gate: None,
                    assignee: None,
                    labels: vec![],
                },
                Step {
                    id: "release".into(),
                    title: "Release".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 1,
                    needs: vec!["tests".into(), "docs".into()],
                    condition: None,
                    gate: None,
                    assignee: None,
                    labels: vec![],
                },
            ],
            source: String::new(),
        };

        // With type=patch, "docs" is filtered out
        let vars = make_vars(&[("type", "patch")]);
        let cooked = cook(&f, &vars).unwrap();
        assert_eq!(cooked.len(), 2);
        assert_eq!(cooked[0].id, "tests");
        assert_eq!(cooked[1].id, "release");
        // "docs" was filtered out, so release's needs should not include it
        assert_eq!(cooked[1].needs, vec!["tests"]);

        // With type=major, all steps included
        let vars = make_vars(&[("type", "major")]);
        let cooked = cook(&f, &vars).unwrap();
        assert_eq!(cooked.len(), 3);
        assert_eq!(cooked[2].needs, vec!["tests", "docs"]);
    }

    #[test]
    fn cook_missing_required_var() {
        let f = Formula {
            formula: "test".into(),
            description: String::new(),
            r#type: "workflow".into(),
            version: 1,
            vars: HashMap::from([
                ("name".into(), VarDef {
                    description: String::new(),
                    required: true,
                    default: None,
                }),
            ]),
            steps: vec![],
            source: String::new(),
        };
        let result = cook(&f, &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn cook_uses_defaults() {
        let f = Formula {
            formula: "test".into(),
            description: String::new(),
            r#type: "workflow".into(),
            version: 1,
            vars: HashMap::from([
                ("env".into(), VarDef {
                    description: String::new(),
                    required: false,
                    default: Some("staging".into()),
                }),
            ]),
            steps: vec![
                Step {
                    id: "deploy".into(),
                    title: "Deploy to {{env}}".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 2,
                    needs: vec![],
                    condition: None,
                    gate: None,
                    assignee: None,
                    labels: vec![],
                },
            ],
            source: String::new(),
        };

        // No vars provided -- should use default
        let cooked = cook(&f, &HashMap::new()).unwrap();
        assert_eq!(cooked[0].title, "Deploy to staging");

        // Override default
        let vars = make_vars(&[("env", "prod")]);
        let cooked = cook(&f, &vars).unwrap();
        assert_eq!(cooked[0].title, "Deploy to prod");
    }

    #[test]
    fn cook_with_gate() {
        let f = Formula {
            formula: "test".into(),
            description: String::new(),
            r#type: "workflow".into(),
            version: 1,
            vars: HashMap::new(),
            steps: vec![
                Step {
                    id: "ci".into(),
                    title: "Wait for CI".into(),
                    description: String::new(),
                    r#type: "task".into(),
                    priority: 2,
                    needs: vec![],
                    condition: None,
                    gate: Some(StepGate {
                        r#type: "gh:run".into(),
                        id: "ci.yml".into(),
                        timeout: "30m".into(),
                    }),
                    assignee: None,
                    labels: vec![],
                },
            ],
            source: String::new(),
        };
        let cooked = cook(&f, &HashMap::new()).unwrap();
        assert_eq!(cooked[0].gate.as_ref().unwrap().r#type, "gh:run");
    }
}
