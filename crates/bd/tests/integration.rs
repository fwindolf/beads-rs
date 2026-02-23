//! End-to-end CLI integration tests for the `bd` binary.
//!
//! Each test creates its own temporary directory, initializes a beads project,
//! and exercises the `bd` binary as a subprocess via `assert_cmd`.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `Command` targeting the cargo-built `bd` binary.
fn bd() -> Command {
    Command::cargo_bin("bd").unwrap()
}

/// Initialize a fresh beads project in a temp directory and return the handle.
fn init_project() -> TempDir {
    let tmp = TempDir::new().unwrap();
    bd().args(["init", "--prefix", "t", "--quiet"])
        .current_dir(tmp.path())
        .assert()
        .success();
    tmp
}

/// Create an issue and return its ID (parsed from `--json` output).
fn create_issue(tmp: &TempDir, title: &str, extra_args: &[&str]) -> String {
    let mut args = vec!["create", title, "--json"];
    args.extend_from_slice(extra_args);
    let output = bd().args(&args).current_dir(tmp.path()).output().unwrap();
    assert!(
        output.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    json["id"].as_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// Flow 1: Full lifecycle
// ---------------------------------------------------------------------------

#[test]
fn flow1_full_lifecycle() {
    let tmp = init_project();

    // Create three issues with different types and priorities
    let id1 = create_issue(
        &tmp,
        "Bug: login broken",
        &["-t", "bug", "-p", "0", "-d", "Users can't login"],
    );
    let id2 = create_issue(&tmp, "Feature: dark mode", &["-t", "feature", "-p", "2"]);
    let id3 = create_issue(&tmp, "Task: update docs", &["-t", "task", "-p", "3"]);

    // Verify all IDs start with the configured prefix
    assert!(id1.starts_with("t-"), "id1 should start with t-: {}", id1);
    assert!(id2.starts_with("t-"), "id2 should start with t-: {}", id2);
    assert!(id3.starts_with("t-"), "id3 should start with t-: {}", id3);

    // bd list --json => 3 issues
    let output = bd()
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = list.as_array().expect("list --json should return array");
    assert_eq!(arr.len(), 3, "should have 3 issues");

    // Check JSON structure on first issue.
    // Note: serde skip_serializing_if omits default values (status=open, issue_type=task),
    // so we check that key fields are present and non-null on the bug issue (P0, type=bug).
    let bug_issue = arr
        .iter()
        .find(|i| i["title"].as_str().map_or(false, |t| t.contains("login")))
        .expect("should find the login bug issue");
    assert!(bug_issue["id"].is_string());
    assert!(bug_issue["title"].is_string());
    assert!(
        bug_issue["issue_type"].is_string(),
        "bug issue should have issue_type serialized"
    );
    assert_eq!(bug_issue["issue_type"].as_str().unwrap(), "bug");
    assert!(bug_issue["priority"].is_number());
    assert!(bug_issue["created_at"].is_string());
    assert!(bug_issue["updated_at"].is_string());

    // bd show <id1> --json => single-element array
    let output = bd()
        .args(["show", &id1, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let show_arr = show.as_array().expect("show --json should return array");
    assert_eq!(show_arr.len(), 1);
    assert_eq!(show_arr[0]["id"].as_str().unwrap(), id1);

    // bd update <id1> --status in_progress
    bd().args(["update", &id1, "--status", "in_progress"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify status changed
    let output = bd()
        .args(["show", &id1, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let status_str = show[0]["status"].as_str().unwrap_or("");
    assert_eq!(status_str, "in_progress");

    // bd close <id1> -r "Fixed"
    bd().args(["close", &id1, "-r", "Fixed"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // bd list --json => only 2 non-closed issues
    let output = bd()
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);

    // bd list --all --json => all 3 including closed
    let output = bd()
        .args(["list", "--all", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 3);

    // bd reopen <id1>
    bd().args(["reopen", &id1])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify reopened -- status "open" is the serde default and may be omitted
    let output = bd()
        .args(["show", &id1, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let status_after_reopen = show[0]["status"].as_str().unwrap_or("open");
    assert_eq!(status_after_reopen, "open");
}

// ---------------------------------------------------------------------------
// Flow 2: Dependencies and ready work
// ---------------------------------------------------------------------------

#[test]
fn flow2_dependencies_and_ready() {
    let tmp = init_project();

    let parent = create_issue(&tmp, "Parent task", &["-t", "task", "-p", "1"]);
    let child = create_issue(&tmp, "Child task", &["-t", "task", "-p", "2"]);
    let unrelated = create_issue(&tmp, "Unrelated task", &["-t", "task", "-p", "3"]);

    // Add blocking dependency: child depends on (is blocked by) parent
    bd().args(["dep", "add", &child, &parent, "--type", "blocks"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // bd ready --json => child should NOT be ready (blocked), parent and unrelated should be
    let output = bd()
        .args(["ready", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let ready: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let ready_ids: Vec<&str> = ready
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert!(
        ready_ids.contains(&parent.as_str()),
        "parent should be ready"
    );
    assert!(
        ready_ids.contains(&unrelated.as_str()),
        "unrelated should be ready"
    );
    assert!(
        !ready_ids.contains(&child.as_str()),
        "child should NOT be ready (blocked)"
    );

    // Close parent => child becomes ready
    bd().args(["close", &parent])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = bd()
        .args(["ready", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let ready: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let ready_ids: Vec<&str> = ready
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert!(
        ready_ids.contains(&child.as_str()),
        "child should now be ready"
    );

    // bd dep list <child> => verify deps shown
    bd().args(["dep", "list", &child])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Depends on"));
}

// ---------------------------------------------------------------------------
// Flow 3: Search and filter
// ---------------------------------------------------------------------------

#[test]
fn flow3_search_and_filter() {
    let tmp = init_project();

    create_issue(&tmp, "Bug: login page broken", &["-t", "bug", "-p", "0"]);
    create_issue(
        &tmp,
        "Feature: dark mode toggle",
        &["-t", "feature", "-p", "2"],
    );
    create_issue(&tmp, "Bug: signup validation", &["-t", "bug", "-p", "1"]);

    // Search for "login"
    let output = bd()
        .args(["search", "login", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let results: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = results.as_array().unwrap();
    assert_eq!(arr.len(), 1, "search for 'login' should return 1 result");
    assert!(arr[0]["title"].as_str().unwrap().contains("login"));

    // Filter by type
    let output = bd()
        .args(["list", "--type", "bug", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2, "should have 2 bugs");

    // Filter by status
    let output = bd()
        .args(["list", "--status", "open", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 3, "all 3 should be open");

    // Ready with priority filter
    let output = bd()
        .args(["ready", "--priority", "0", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let ready: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(ready.as_array().unwrap().len(), 1, "only 1 P0 issue");
}

// ---------------------------------------------------------------------------
// Flow 4: Labels
// ---------------------------------------------------------------------------

#[test]
fn flow4_labels() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Label test issue", &[]);

    // Add labels
    bd().args(["label", &id, "add", "critical"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["label", &id, "add", "backend"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // List labels => both present
    bd().args(["label", &id, "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("backend"))
        .stdout(predicate::str::contains("critical"));

    // Remove one label
    bd().args(["label", &id, "remove", "critical"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify only backend remains
    bd().args(["label", &id, "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("backend"))
        .stdout(predicate::str::contains("critical").not());
}

// ---------------------------------------------------------------------------
// Flow 5: Comments and history
// ---------------------------------------------------------------------------

#[test]
fn flow5_comments_and_history() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Comment test issue", &[]);

    // Add two comments
    bd().args(["comment", &id, "First comment"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["comment", &id, "Second comment"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // List comments => both present
    let output = bd()
        .args(["comments", &id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let comments: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = comments.as_array().unwrap();
    assert_eq!(arr.len(), 2, "should have 2 comments");
    assert_eq!(arr[0]["text"].as_str().unwrap(), "First comment");
    assert_eq!(arr[1]["text"].as_str().unwrap(), "Second comment");

    // History => should have events (created, commented)
    let output = bd()
        .args(["history", &id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let history: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let events = history["events"].as_array().unwrap();
    assert!(
        events.len() >= 3,
        "should have at least 3 events (created + 2 commented)"
    );

    let event_types: Vec<&str> = events
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(
        event_types.contains(&"created"),
        "should have 'created' event"
    );
    assert!(
        event_types.contains(&"commented"),
        "should have 'commented' event"
    );
}

// ---------------------------------------------------------------------------
// Flow 6: Stats and views
// ---------------------------------------------------------------------------

#[test]
fn flow6_stats_and_views() {
    let tmp = init_project();

    create_issue(&tmp, "Bug one", &["-t", "bug", "-p", "0"]);
    create_issue(&tmp, "Bug two", &["-t", "bug", "-p", "1"]);
    create_issue(&tmp, "Feature one", &["-t", "feature", "-p", "2"]);
    create_issue(&tmp, "Task one", &["-t", "task", "-p", "3"]);
    create_issue(&tmp, "Task two", &["-t", "task", "-p", "4"]);

    // Count
    bd().args(["count"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("5"));

    // Stats
    let output = bd()
        .args(["stats", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stats: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stats["total"].as_i64().unwrap(), 5);
    assert_eq!(stats["open"].as_i64().unwrap(), 5);
    assert_eq!(stats["closed"].as_i64().unwrap(), 0);

    // Stale --days 0 => all issues should be "stale" with 0-day threshold
    bd().args(["stale", "--days", "0"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Orphans => all issues should be orphans (no dependencies)
    let output = bd()
        .args(["orphans", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let orphans: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        orphans["count"].as_i64().unwrap(),
        5,
        "all 5 should be orphans"
    );
}

// ---------------------------------------------------------------------------
// Flow 7: Config and KV
// ---------------------------------------------------------------------------

#[test]
fn flow7_config_and_kv() {
    let tmp = init_project();

    // Config set/get
    bd().args(["config", "set", "my.key", "my value"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["config", "get", "my.key"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("my value"));

    // Config list
    bd().args(["config", "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("my.key"));

    // Config unset
    bd().args(["config", "unset", "my.key"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // KV set/get
    bd().args(["kv", "set", "test-key", "test-value"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["kv", "get", "test-key"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test-value"));

    // KV list
    bd().args(["kv", "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test-key"));

    // KV delete
    bd().args(["kv", "delete", "test-key"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Flow 8: Templates
// ---------------------------------------------------------------------------

#[test]
fn flow8_templates() {
    let tmp = init_project();

    // Create a template with variables in title and description
    let output = bd()
        .args([
            "template",
            "create",
            "Setup {{project}}",
            "--description",
            "Init {{project}} with {{lang}}",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "template create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let tmpl_id = created["id"].as_str().unwrap().to_string();

    // Template list => should show the template
    bd().args(["template", "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup {{project}}"));

    // Template show => verify variables extracted
    let output = bd()
        .args(["template", "show", &tmpl_id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let vars = show["variables"].as_array().unwrap();
    let var_names: Vec<&str> = vars.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(
        var_names.contains(&"project"),
        "should extract 'project' variable"
    );
    assert!(
        var_names.contains(&"lang"),
        "should extract 'lang' variable"
    );

    // Instantiate the template
    bd().args([
        "template",
        "instantiate",
        &tmpl_id,
        "--var",
        "project=myapp",
        "--var",
        "lang=Rust",
    ])
    .current_dir(tmp.path())
    .assert()
    .success();

    // Verify cloned issue exists with substituted title
    let output = bd()
        .args(["list", "--all", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let titles: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["title"].as_str().unwrap())
        .collect();
    assert!(
        titles.contains(&"Setup myapp"),
        "should have substituted title 'Setup myapp', got: {:?}",
        titles
    );
}

// ---------------------------------------------------------------------------
// Flow 9: Gates
// ---------------------------------------------------------------------------

#[test]
fn flow9_gates() {
    let tmp = init_project();

    // Create a gate
    let output = bd()
        .args([
            "gate",
            "create",
            "Manual gate",
            "--await-type",
            "human",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "gate create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let gate_id = created["id"].as_str().unwrap().to_string();

    // Gate list => should show the gate
    let output = bd()
        .args(["gate", "list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let gates: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(gates.as_array().unwrap().len(), 1);

    // Gate check => should report pending (human gates are not auto-closable)
    bd().args(["gate", "check"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("pending").or(predicate::str::contains("none resolved")));

    // Gate close
    bd().args(["gate", "close", &gate_id, "--reason", "Approved"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Gate list => no open gates
    let output = bd()
        .args(["gate", "list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let gates: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        gates.as_array().unwrap().len(),
        0,
        "no open gates after close"
    );
}

// ---------------------------------------------------------------------------
// Flow 10: Graph
// ---------------------------------------------------------------------------

#[test]
fn flow10_graph() {
    let tmp = init_project();

    let a = create_issue(&tmp, "Foundation", &["-t", "task", "-p", "1"]);
    let b = create_issue(&tmp, "Build on foundation", &["-t", "task", "-p", "2"]);

    // b depends on a
    bd().args(["dep", "add", &b, &a, "--type", "blocks"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Graph for a single issue => should show layer output
    bd().args(["graph", &a])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("LAYER"));

    // Graph --all => should show output
    bd().args(["graph", "--all"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("LAYER"));

    // Graph --dot => should output Graphviz DOT format
    bd().args(["graph", &a, "--dot"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("digraph beads"))
        .stdout(predicate::str::contains("->"));
}

// ---------------------------------------------------------------------------
// Flow 11: JSON contract (VS Code adapter)
// ---------------------------------------------------------------------------

#[test]
fn flow11_json_contract() {
    let tmp = init_project();

    // bd create --json returns a single object (not array)
    let output = bd()
        .args([
            "create",
            "JSON contract test",
            "-t",
            "bug",
            "-p",
            "1",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        created.is_object(),
        "create --json should return a single object, got: {}",
        created
    );
    let id = created["id"].as_str().unwrap().to_string();

    // Verify field names on create output
    assert!(
        created.get("issue_type").is_some(),
        "should have 'issue_type' field"
    );
    assert!(
        created.get("created_at").is_some(),
        "should have 'created_at' field"
    );
    assert!(
        created.get("updated_at").is_some(),
        "should have 'updated_at' field"
    );

    // bd list --json returns array
    let output = bd()
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(list.is_array(), "list --json should return array");

    // Check field names on list output
    let first = &list.as_array().unwrap()[0];
    assert!(
        first.get("issue_type").is_some(),
        "list items should have 'issue_type'"
    );
    assert!(
        first.get("created_at").is_some(),
        "list items should have 'created_at'"
    );

    // bd show --json returns array
    let output = bd()
        .args(["show", &id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(show.is_array(), "show --json should return array");

    // bd close --json returns array
    let output = bd()
        .args(["close", &id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let closed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(closed.is_array(), "close --json should return array");

    // Verify JSON does NOT use wrong field names
    let item = &closed.as_array().unwrap()[0];
    assert!(
        item.get("type").is_none(),
        "should NOT have bare 'type' field (use 'issue_type')"
    );
    assert!(
        item.get("created").is_none(),
        "should NOT have bare 'created' field (use 'created_at')"
    );
}

// ---------------------------------------------------------------------------
// Flow 12: Swarm analysis
// ---------------------------------------------------------------------------

#[test]
fn flow12_swarm_analysis() {
    let tmp = init_project();

    // Create an epic
    let epic = create_issue(&tmp, "My epic", &["-t", "epic", "-p", "1"]);

    // Create 3 child tasks with parent-child deps to the epic
    let child1 = create_issue(&tmp, "Child 1: foundation", &["-t", "task", "-p", "1"]);
    let child2 = create_issue(&tmp, "Child 2: build", &["-t", "task", "-p", "2"]);
    let child3 = create_issue(&tmp, "Child 3: integrate", &["-t", "task", "-p", "2"]);

    // Add parent-child deps (child depends on epic)
    for child in [&child1, &child2, &child3] {
        bd().args(["dep", "add", child, &epic, "--type", "parent-child"])
            .current_dir(tmp.path())
            .assert()
            .success();
    }

    // Add blocking deps between children: child2 blocks child3, child1 blocks child2
    bd().args(["dep", "add", &child2, &child1, "--type", "blocks"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["dep", "add", &child3, &child2, "--type", "blocks"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Swarm validate => should produce wave output
    let output = bd()
        .args(["swarm", "validate", &epic, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "swarm validate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let analysis: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(analysis["total_issues"].as_i64().unwrap(), 3);
    assert!(
        analysis["swarmable"].as_bool().unwrap(),
        "should be swarmable (no cycles)"
    );

    let waves = analysis["waves"].as_array().unwrap();
    assert!(
        waves.len() >= 2,
        "should have at least 2 waves with the dependency chain"
    );

    // Wave 0 should contain child1 (no blocking deps within children)
    let wave0_ids: Vec<&str> = waves[0]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert!(
        wave0_ids.contains(&child1.as_str()),
        "wave 0 should contain child1 (foundation)"
    );
    assert!(
        !wave0_ids.contains(&child3.as_str()),
        "wave 0 should NOT contain child3 (blocked)"
    );
}

// ---------------------------------------------------------------------------
// Additional edge-case tests
// ---------------------------------------------------------------------------

#[test]
fn init_creates_beads_dir() {
    let tmp = TempDir::new().unwrap();
    bd().args(["init", "--prefix", "test", "--quiet"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify .beads directory and database exist
    assert!(tmp.path().join(".beads").is_dir());
    assert!(tmp.path().join(".beads").join("beads.db").is_file());
    assert!(tmp.path().join(".beads").join("metadata.json").is_file());
}

#[test]
fn init_refuses_double_init() {
    let tmp = init_project();

    // Second init without --force should fail
    bd().args(["init", "--prefix", "t", "--quiet"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already initialized"));
}

#[test]
fn create_without_title_fails() {
    let tmp = init_project();

    bd().args(["create"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn show_nonexistent_issue_fails() {
    let tmp = init_project();

    bd().args(["show", "t-nonexistent"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn close_already_closed_issue_warns() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Close me", &[]);

    bd().args(["close", &id])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Second close should print warning
    bd().args(["close", &id])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("already closed"));
}

#[test]
fn reopen_non_closed_fails() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Open issue", &[]);

    bd().args(["reopen", &id])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not closed"));
}

#[test]
fn delete_issue() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Delete me", &[]);

    bd().args(["delete", &id, "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Should be gone
    bd().args(["show", &id])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn version_command() {
    bd().args(["version"]).assert().success();
}

#[test]
fn count_by_status() {
    let tmp = init_project();

    create_issue(&tmp, "Issue 1", &[]);
    create_issue(&tmp, "Issue 2", &[]);
    let id3 = create_issue(&tmp, "Issue 3", &[]);

    bd().args(["close", &id3])
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = bd()
        .args(["count", "--by-status", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let counts: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(counts["open"].as_i64().unwrap(), 2);
    assert_eq!(counts["closed"].as_i64().unwrap(), 1);
}

#[test]
fn dep_cycles_detection() {
    let tmp = init_project();

    let a = create_issue(&tmp, "Issue A", &[]);
    let b = create_issue(&tmp, "Issue B", &[]);

    // Create a cycle: a -> b -> a
    bd().args(["dep", "add", &a, &b, "--type", "blocks"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["dep", "add", &b, &a, "--type", "blocks"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Detect cycles
    bd().args(["dep", "cycles"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("cycle"));
}

#[test]
fn create_with_labels() {
    let tmp = init_project();

    let output = bd()
        .args([
            "create",
            "Labeled issue",
            "-l",
            "frontend",
            "-l",
            "urgent",
            "--json",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let id = created["id"].as_str().unwrap();

    // Verify labels via show
    let output = bd()
        .args(["show", id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let labels: Vec<&str> = show[0]["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert!(labels.contains(&"frontend"));
    assert!(labels.contains(&"urgent"));
}

#[test]
fn rename_issue() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Old title", &[]);

    bd().args(["rename", &id, "New title"])
        .current_dir(tmp.path())
        .assert()
        .success();

    bd().args(["show", &id])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("New title"));
}

#[test]
fn update_multiple_fields() {
    let tmp = init_project();
    let id = create_issue(&tmp, "Multi update test", &["-t", "task", "-p", "3"]);

    bd().args([
        "update",
        &id,
        "--title",
        "Updated title",
        "--priority",
        "1",
        "--type",
        "bug",
    ])
    .current_dir(tmp.path())
    .assert()
    .success();

    let output = bd()
        .args(["show", &id, "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let issue = &show[0];
    assert_eq!(issue["title"].as_str().unwrap(), "Updated title");
    assert_eq!(issue["priority"].as_i64().unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Phase 8: Setup & maintenance commands
// ---------------------------------------------------------------------------

#[test]
fn quickstart_displays_guide() {
    bd().args(["quickstart"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependency-Aware Issue Tracker"))
        .stdout(predicate::str::contains("GETTING STARTED"))
        .stdout(predicate::str::contains("CREATING ISSUES"))
        .stdout(predicate::str::contains("READY WORK"))
        .stdout(predicate::str::contains("bd init"))
        .stdout(predicate::str::contains("Ready to start!"));
}

#[test]
fn onboard_creates_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    bd().args(["onboard", "--claude"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote onboarding to CLAUDE.md"));

    let content = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("<!-- BEGIN BD ONBOARD -->"));
    assert!(content.contains("<!-- END BD ONBOARD -->"));
    assert!(content.contains("mulch prime"));
    assert!(content.contains("bd create"));
}

#[test]
fn onboard_check_reports_installed() {
    let tmp = tempfile::TempDir::new().unwrap();
    // First install
    bd().args(["onboard", "--claude"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // Then check
    bd().args(["onboard", "--claude", "--check"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Onboard section installed"));
}

#[test]
fn onboard_check_reports_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("CLAUDE.md"), "# No onboard\n").unwrap();
    bd().args(["onboard", "--claude", "--check"])
        .current_dir(tmp.path())
        .assert()
        .failure();
}

#[test]
fn onboard_remove_deletes_section() {
    let tmp = tempfile::TempDir::new().unwrap();
    // Install first
    bd().args(["onboard", "--agents"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // Remove
    bd().args(["onboard", "--agents", "--remove"])
        .current_dir(tmp.path())
        .assert()
        .success();
    // File should be deleted (was only the onboard section)
    assert!(!tmp.path().join("AGENTS.md").exists());
}

#[test]
fn bootstrap_explains_sqlite_workflow() {
    bd().args(["bootstrap"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bootstrap"))
        .stdout(predicate::str::contains("bd init"))
        .stdout(predicate::str::contains("SQLite"));
}

#[test]
fn preflight_shows_checklist() {
    bd().args(["preflight"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PR Readiness Checklist"))
        .stdout(predicate::str::contains("cargo test"))
        .stdout(predicate::str::contains("cargo clippy"));
}

#[test]
fn prime_outputs_workflow_context() {
    let tmp = init_project();
    bd().args(["prime", "--full"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Beads Workflow Context"))
        .stdout(predicate::str::contains("SESSION CLOSE PROTOCOL"))
        .stdout(predicate::str::contains("bd ready"))
        .stdout(predicate::str::contains("bd create"));
}

#[test]
fn prime_mcp_mode_minimal() {
    let tmp = init_project();
    bd().args(["prime", "--mcp"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Beads Issue Tracker Active"))
        .stdout(predicate::str::contains("Core Rules"));
}

#[test]
fn prime_stealth_mode_no_git() {
    let tmp = init_project();
    bd().args(["prime", "--full", "--stealth"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("flush-only"));
}

#[test]
fn prime_silent_outside_beads_project() {
    let tmp = TempDir::new().unwrap();
    bd().args(["prime"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn upgrade_status_no_beads() {
    // Outside a beads project, should still work
    bd().args(["upgrade", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bd version"));
}

#[test]
fn upgrade_status_json() {
    bd().args(["upgrade", "status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("current_version"));
}

#[test]
fn upgrade_ack_in_project() {
    let tmp = init_project();
    bd().args(["upgrade", "ack"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn worktree_info_shows_current() {
    let tmp = init_project();

    // Initialize as a git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    bd().args(["worktree", "info"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Branch"))
        .stdout(predicate::str::contains("Beads"));
}

#[test]
fn worktree_list_in_git_repo() {
    let tmp = init_project();

    // Initialize as a git repo first
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "init", "--allow-empty"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    bd().args(["worktree", "list"])
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"));
}
