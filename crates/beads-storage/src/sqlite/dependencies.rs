//! Dependency CRUD operations and cycle detection for [`SqliteStore`].

use std::collections::{HashSet, VecDeque};

use chrono::Utc;
use rusqlite::{params, Connection};

use beads_core::dependency::Dependency;
use beads_core::enums::{DependencyType, EventType};
use beads_core::issue::Issue;

use crate::error::{Result, StorageError};
use crate::sqlite::issues::{emit_event, format_datetime, scan_issue, ISSUE_COLUMNS, ISSUE_COLUMNS_PREFIXED};
use crate::sqlite::store::SqliteStore;
use crate::traits::IssueWithDependencyMetadata;
use crate::traits::TreeNode;

// ---------------------------------------------------------------------------
// Connection-level helpers
// ---------------------------------------------------------------------------

/// Inserts a dependency on the given connection, with cycle detection for
/// blocking types.
pub(crate) fn add_dependency_on_conn(
    conn: &Connection,
    dep: &Dependency,
    actor: &str,
) -> Result<()> {
    // Cycle detection for blocking dependency types.
    if dep.dep_type.affects_ready_work() {
        detect_cycle(conn, &dep.issue_id, &dep.depends_on_id)?;
    }

    let now = Utc::now();
    let now_str = format_datetime(&now);
    let created_at_str = format_datetime(&dep.created_at);

    conn.execute(
        "INSERT OR REPLACE INTO dependencies
         (issue_id, depends_on_id, type, created_at, created_by, metadata, thread_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            dep.issue_id,
            dep.depends_on_id,
            dep.dep_type.as_str(),
            created_at_str,
            dep.created_by,
            dep.metadata,
            dep.thread_id,
        ],
    )?;

    // Emit event on the source issue.
    emit_event(
        conn,
        &dep.issue_id,
        EventType::DependencyAdded,
        actor,
        None,
        Some(&dep.depends_on_id),
        Some(dep.dep_type.as_str()),
        &now_str,
    )?;

    Ok(())
}

/// Removes a dependency on the given connection.
pub(crate) fn remove_dependency_on_conn(
    conn: &Connection,
    issue_id: &str,
    depends_on_id: &str,
    actor: &str,
) -> Result<()> {
    let now = Utc::now();
    let now_str = format_datetime(&now);

    let affected = conn.execute(
        "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2",
        params![issue_id, depends_on_id],
    )?;

    if affected == 0 {
        return Err(StorageError::not_found(
            "dependency",
            format!("{issue_id} -> {depends_on_id}"),
        ));
    }

    emit_event(
        conn,
        issue_id,
        EventType::DependencyRemoved,
        actor,
        Some(depends_on_id),
        None,
        None,
        &now_str,
    )?;

    Ok(())
}

/// Returns raw dependency records for an issue on the given connection.
pub(crate) fn get_dependency_records_on_conn(
    conn: &Connection,
    issue_id: &str,
) -> Result<Vec<Dependency>> {
    let mut stmt = conn.prepare(
        "SELECT issue_id, depends_on_id, type, created_at, created_by, metadata, thread_id
         FROM dependencies WHERE issue_id = ?1",
    )?;
    let rows = stmt.query_map(params![issue_id], |row| {
        Ok(Dependency {
            issue_id: row.get("issue_id")?,
            depends_on_id: row.get("depends_on_id")?,
            dep_type: DependencyType::from(row.get::<_, String>("type")?.as_str()),
            created_at: crate::sqlite::issues::parse_datetime(&row.get::<_, String>("created_at")?),
            created_by: row.get("created_by")?,
            metadata: row.get("metadata")?,
            thread_id: row.get("thread_id")?,
        })
    })?;

    let mut deps = Vec::new();
    for row in rows {
        deps.push(row?);
    }
    Ok(deps)
}

// ---------------------------------------------------------------------------
// Cycle detection
// ---------------------------------------------------------------------------

/// Detects whether adding an edge `issue_id -> depends_on_id` would create a
/// cycle in the blocking dependency graph. Uses BFS from `depends_on_id` to
/// see if `issue_id` is reachable.
fn detect_cycle(conn: &Connection, issue_id: &str, depends_on_id: &str) -> Result<()> {
    // If adding A depends-on B, check that B does not already (transitively)
    // depend on A. We BFS from B through the "blocks" graph.
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(depends_on_id.to_string());

    while let Some(current) = queue.pop_front() {
        if current == issue_id {
            return Err(StorageError::CycleDetected);
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        // Follow outgoing blocking edges from `current`.
        let mut stmt = conn.prepare_cached(
            "SELECT depends_on_id FROM dependencies
             WHERE issue_id = ?1 AND type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')",
        )?;
        let rows = stmt.query_map(params![current], |row| {
            row.get::<_, String>(0)
        })?;
        for row in rows {
            let next = row?;
            if !visited.contains(&next) {
                queue.push_back(next);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SqliteStore methods
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Adds a dependency edge.
    pub fn add_dependency_impl(&self, dep: &Dependency, actor: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        add_dependency_on_conn(&conn, dep, actor)
    }

    /// Removes a dependency edge.
    pub fn remove_dependency_impl(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        actor: &str,
    ) -> Result<()> {
        let conn = self.lock_conn()?;
        remove_dependency_on_conn(&conn, issue_id, depends_on_id, actor)
    }

    /// Returns issues that the given issue depends on.
    pub fn get_dependencies_impl(&self, issue_id: &str) -> Result<Vec<Issue>> {
        let conn = self.lock_conn()?;
        let sql = format!(
            "SELECT {ISSUE_COLUMNS_PREFIXED} FROM issues
             INNER JOIN dependencies d ON issues.id = d.depends_on_id
             WHERE d.issue_id = ?1"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![issue_id], scan_issue)?;
        let mut issues = Vec::new();
        for row in rows {
            issues.push(row?);
        }
        Ok(issues)
    }

    /// Returns issues that depend on the given issue.
    pub fn get_dependents_impl(&self, issue_id: &str) -> Result<Vec<Issue>> {
        let conn = self.lock_conn()?;
        let sql = format!(
            "SELECT {ISSUE_COLUMNS_PREFIXED} FROM issues
             INNER JOIN dependencies d ON issues.id = d.issue_id
             WHERE d.depends_on_id = ?1"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![issue_id], scan_issue)?;
        let mut issues = Vec::new();
        for row in rows {
            issues.push(row?);
        }
        Ok(issues)
    }

    /// Returns dependencies with their edge metadata.
    pub fn get_dependencies_with_metadata_impl(
        &self,
        issue_id: &str,
    ) -> Result<Vec<IssueWithDependencyMetadata>> {
        let conn = self.lock_conn()?;
        get_deps_with_metadata(&conn, issue_id, true)
    }

    /// Returns dependents with their edge metadata.
    pub fn get_dependents_with_metadata_impl(
        &self,
        issue_id: &str,
    ) -> Result<Vec<IssueWithDependencyMetadata>> {
        let conn = self.lock_conn()?;
        get_deps_with_metadata(&conn, issue_id, false)
    }

    /// Traverses the dependency tree from a root issue.
    pub fn get_dependency_tree_impl(
        &self,
        issue_id: &str,
        max_depth: i32,
        _show_all_paths: bool,
        reverse: bool,
    ) -> Result<Vec<TreeNode>> {
        let conn = self.lock_conn()?;

        let mut result = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, i32, DependencyType)> = VecDeque::new();

        // Start with the root.
        let root = crate::sqlite::issues::get_issue_on_conn(&conn, issue_id)?;
        result.push(TreeNode {
            issue: root,
            depth: 0,
            dep_type: DependencyType::Blocks,
            reverse,
        });
        visited.insert(issue_id.to_string());
        queue.push_back((issue_id.to_string(), 0, DependencyType::Blocks));

        while let Some((current_id, depth, _)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Get adjacent edges.
            let (sql, param) = if reverse {
                (
                    "SELECT d.issue_id, d.type FROM dependencies d WHERE d.depends_on_id = ?1",
                    current_id.clone(),
                )
            } else {
                (
                    "SELECT d.depends_on_id, d.type FROM dependencies d WHERE d.issue_id = ?1",
                    current_id.clone(),
                )
            };

            let mut stmt = conn.prepare(sql)?;
            let edges: Vec<(String, String)> = stmt
                .query_map(params![param], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .filter_map(|r| r.ok())
                .collect();

            for (next_id, dep_type_str) in edges {
                if visited.contains(&next_id) {
                    continue;
                }
                visited.insert(next_id.clone());

                if let Ok(issue) = crate::sqlite::issues::get_issue_on_conn(&conn, &next_id) {
                    let dep_type = DependencyType::from(dep_type_str.as_str());
                    result.push(TreeNode {
                        issue,
                        depth: depth + 1,
                        dep_type: dep_type.clone(),
                        reverse,
                    });
                    queue.push_back((next_id, depth + 1, dep_type));
                }
            }
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns issues with their dependency edge metadata.
///
/// `forward=true` means "get dependencies of `issue_id`" (issue_id is the source).
/// `forward=false` means "get dependents of `issue_id`" (issue_id is the target).
fn get_deps_with_metadata(
    conn: &Connection,
    issue_id: &str,
    forward: bool,
) -> Result<Vec<IssueWithDependencyMetadata>> {
    let (join_col, filter_col) = if forward {
        ("depends_on_id", "issue_id")
    } else {
        ("issue_id", "depends_on_id")
    };

    let sql = format!(
        "SELECT {ISSUE_COLUMNS_PREFIXED},
                d.issue_id AS dep_issue_id,
                d.depends_on_id AS dep_depends_on_id,
                d.type AS dep_type,
                d.created_at AS dep_created_at,
                d.created_by AS dep_created_by,
                d.metadata AS dep_metadata,
                d.thread_id AS dep_thread_id
         FROM issues
         INNER JOIN dependencies d ON issues.id = d.{join_col}
         WHERE d.{filter_col} = ?1"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![issue_id], |row| {
        let issue = scan_issue(row)?;
        let dep = Dependency {
            issue_id: row.get("dep_issue_id")?,
            depends_on_id: row.get("dep_depends_on_id")?,
            dep_type: DependencyType::from(
                row.get::<_, String>("dep_type")?.as_str(),
            ),
            created_at: crate::sqlite::issues::parse_datetime(
                &row.get::<_, String>("dep_created_at")?,
            ),
            created_by: row.get("dep_created_by")?,
            metadata: row.get("dep_metadata")?,
            thread_id: row.get("dep_thread_id")?,
        };
        Ok(IssueWithDependencyMetadata {
            issue,
            dependency: dep,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::issue::IssueBuilder;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    fn make_dep(issue_id: &str, depends_on_id: &str) -> Dependency {
        Dependency {
            issue_id: issue_id.into(),
            depends_on_id: depends_on_id.into(),
            dep_type: DependencyType::Blocks,
            created_at: Utc::now(),
            created_by: "test".into(),
            metadata: String::new(),
            thread_id: String::new(),
        }
    }

    #[test]
    fn add_and_get_dependency() {
        let store = test_store();
        let issue1 = IssueBuilder::new("Parent").id("bd-p1").build();
        let issue2 = IssueBuilder::new("Child").id("bd-c1").build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();

        let dep = make_dep("bd-c1", "bd-p1");
        store.add_dependency_impl(&dep, "alice").unwrap();

        let deps = store.get_dependencies_impl("bd-c1").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].id, "bd-p1");

        let dependents = store.get_dependents_impl("bd-p1").unwrap();
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].id, "bd-c1");
    }

    #[test]
    fn remove_dependency() {
        let store = test_store();
        let issue1 = IssueBuilder::new("A").id("bd-a1").build();
        let issue2 = IssueBuilder::new("B").id("bd-b1").build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();

        let dep = make_dep("bd-b1", "bd-a1");
        store.add_dependency_impl(&dep, "alice").unwrap();
        store
            .remove_dependency_impl("bd-b1", "bd-a1", "alice")
            .unwrap();

        let deps = store.get_dependencies_impl("bd-b1").unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn cycle_detection() {
        let store = test_store();
        let issue1 = IssueBuilder::new("A").id("bd-cy1").build();
        let issue2 = IssueBuilder::new("B").id("bd-cy2").build();
        let issue3 = IssueBuilder::new("C").id("bd-cy3").build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();
        store.create_issue_impl(&issue3, "alice").unwrap();

        // A -> B -> C
        store
            .add_dependency_impl(&make_dep("bd-cy1", "bd-cy2"), "alice")
            .unwrap();
        store
            .add_dependency_impl(&make_dep("bd-cy2", "bd-cy3"), "alice")
            .unwrap();

        // C -> A would create a cycle.
        let err = store
            .add_dependency_impl(&make_dep("bd-cy3", "bd-cy1"), "alice")
            .unwrap_err();
        assert!(matches!(err, StorageError::CycleDetected));
    }
}
