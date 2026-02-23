//! Complex queries: ready work, blocked issues, epic status.

use chrono::Utc;

use beads_core::filter::WorkFilter;
use beads_core::issue::Issue;

use crate::error::Result;
use crate::sqlite::issues::{format_datetime, scan_issue, ISSUE_COLUMNS};
use crate::sqlite::store::SqliteStore;
use crate::traits::{BlockedIssue, EpicStatus, Statistics};

impl SqliteStore {
    /// Returns issues that are ready to work on.
    ///
    /// An issue is ready if:
    /// - status is "open"
    /// - it has no open blocking dependencies (type="blocks")
    /// - it is not ephemeral (unless `include_ephemeral` is set)
    /// - it is not deferred past now (unless `include_deferred` is set)
    /// - it is not a template
    pub fn get_ready_work_impl(&self, filter: &WorkFilter) -> Result<Vec<Issue>> {
        let conn = self.lock_conn()?;
        let now = Utc::now();
        let now_str = format_datetime(&now);

        let mut where_clauses: Vec<String> = vec![
            "i.status = 'open'".to_string(),
            "i.is_template = 0".to_string(),
        ];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        // Exclude issues with open blocking dependencies.
        where_clauses.push(
            "NOT EXISTS (
                SELECT 1 FROM dependencies d
                INNER JOIN issues blocker ON blocker.id = d.depends_on_id
                WHERE d.issue_id = i.id
                  AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                  AND blocker.status IN ('open', 'in_progress', 'blocked', 'deferred', 'hooked')
            )"
            .to_string(),
        );

        if !filter.include_ephemeral {
            where_clauses.push("(i.ephemeral = 0 OR i.ephemeral IS NULL)".to_string());
        }

        if !filter.include_deferred {
            where_clauses.push(format!(
                "(i.defer_until IS NULL OR i.defer_until <= ?{param_idx})"
            ));
            param_values.push(Box::new(now_str.clone()));
            param_idx += 1;
        }

        // Optional filters.
        if let Some(ref issue_type) = filter.issue_type {
            where_clauses.push(format!("i.issue_type = ?{param_idx}"));
            param_values.push(Box::new(issue_type.clone()));
            param_idx += 1;
        }
        if let Some(priority) = filter.priority {
            where_clauses.push(format!("i.priority = ?{param_idx}"));
            param_values.push(Box::new(priority));
            param_idx += 1;
        }
        if let Some(ref assignee) = filter.assignee {
            where_clauses.push(format!("i.assignee = ?{param_idx}"));
            param_values.push(Box::new(assignee.clone()));
            param_idx += 1;
        }
        if filter.unassigned {
            where_clauses.push("(i.assignee IS NULL OR i.assignee = '')".to_string());
        }
        if let Some(ref mol_type) = filter.mol_type {
            where_clauses.push(format!("i.mol_type = ?{param_idx}"));
            param_values.push(Box::new(mol_type.as_str().to_string()));
            param_idx += 1;
        }
        if let Some(ref wisp_type) = filter.wisp_type {
            where_clauses.push(format!("i.wisp_type = ?{param_idx}"));
            param_values.push(Box::new(wisp_type.as_str().to_string()));
            param_idx += 1;
        }

        // Label filters (AND).
        for label in &filter.labels {
            where_clauses.push(format!(
                "EXISTS (SELECT 1 FROM labels WHERE labels.issue_id = i.id AND labels.label = ?{param_idx})"
            ));
            param_values.push(Box::new(label.clone()));
            param_idx += 1;
        }

        // Label filters (OR).
        if !filter.labels_any.is_empty() {
            let placeholders: Vec<String> = filter
                .labels_any
                .iter()
                .enumerate()
                .map(|(j, _)| format!("?{}", param_idx + j))
                .collect();
            where_clauses.push(format!(
                "EXISTS (SELECT 1 FROM labels WHERE labels.issue_id = i.id AND labels.label IN ({}))",
                placeholders.join(",")
            ));
            for label in &filter.labels_any {
                param_values.push(Box::new(label.clone()));
            }
            param_idx += filter.labels_any.len();
        }

        let where_sql = where_clauses.join(" AND ");

        // Sort order.
        let order_sql = match filter.sort_policy {
            beads_core::enums::SortPolicy::Priority => "i.priority ASC, i.created_at ASC",
            beads_core::enums::SortPolicy::Oldest => "i.created_at ASC",
            _ => "i.priority ASC, i.created_at ASC", // Hybrid default
        };

        let limit_sql = filter
            .limit
            .map(|l| format!(" LIMIT {l}"))
            .unwrap_or_default();

        // We need to alias the issue table as `i` but select with the ISSUE_COLUMNS
        // which reference bare column names. We'll use a subquery approach.
        let sql = format!(
            "SELECT {ISSUE_COLUMNS} FROM issues i WHERE {where_sql} ORDER BY {order_sql}{limit_sql}"
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), scan_issue)?;

        let mut issues = Vec::new();
        for row in rows {
            issues.push(row?);
        }

        let _ = param_idx;
        Ok(issues)
    }

    /// Returns issues that have at least one open blocking dependency.
    pub fn get_blocked_issues_impl(&self, filter: &WorkFilter) -> Result<Vec<BlockedIssue>> {
        let conn = self.lock_conn()?;

        let mut where_clauses: Vec<String> = vec![
            "i.status IN ('open', 'in_progress', 'blocked', 'deferred', 'hooked')".to_string(),
        ];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        // Must have at least one open blocker.
        where_clauses.push(
            "EXISTS (
                SELECT 1 FROM dependencies d
                INNER JOIN issues blocker ON blocker.id = d.depends_on_id
                WHERE d.issue_id = i.id
                  AND d.type = 'blocks'
                  AND blocker.status IN ('open', 'in_progress', 'blocked', 'deferred', 'hooked')
            )"
            .to_string(),
        );

        if let Some(ref assignee) = filter.assignee {
            where_clauses.push(format!("i.assignee = ?{param_idx}"));
            param_values.push(Box::new(assignee.clone()));
            param_idx += 1;
        }
        if let Some(priority) = filter.priority {
            where_clauses.push(format!("i.priority = ?{param_idx}"));
            param_values.push(Box::new(priority));
            param_idx += 1;
        }

        let where_sql = where_clauses.join(" AND ");
        let limit_sql = filter
            .limit
            .map(|l| format!(" LIMIT {l}"))
            .unwrap_or_default();

        let sql = format!(
            "SELECT {ISSUE_COLUMNS},
                    (SELECT COUNT(*)
                     FROM dependencies d
                     INNER JOIN issues blocker ON blocker.id = d.depends_on_id
                     WHERE d.issue_id = i.id
                       AND d.type = 'blocks'
                       AND blocker.status IN ('open', 'in_progress', 'blocked', 'deferred', 'hooked')
                    ) AS blocked_by_count
             FROM issues i
             WHERE {where_sql}
             ORDER BY i.priority ASC, i.created_at ASC{limit_sql}"
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let issue = scan_issue(row)?;
            let blocked_by_count: i32 = row.get("blocked_by_count")?;
            Ok(BlockedIssue {
                issue,
                blocked_by_count,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        let _ = param_idx;
        Ok(result)
    }

    /// Returns epics where all children are closed.
    pub fn get_epics_eligible_for_closure_impl(&self) -> Result<Vec<EpicStatus>> {
        let conn = self.lock_conn()?;

        let sql = format!(
            "SELECT {ISSUE_COLUMNS},
                    (SELECT COUNT(*)
                     FROM dependencies d
                     INNER JOIN issues child ON child.id = d.issue_id
                     WHERE d.depends_on_id = i.id AND d.type = 'parent-child'
                    ) AS total_children,
                    (SELECT COUNT(*)
                     FROM dependencies d
                     INNER JOIN issues child ON child.id = d.issue_id
                     WHERE d.depends_on_id = i.id
                       AND d.type = 'parent-child'
                       AND child.status = 'closed'
                    ) AS closed_children
             FROM issues i
             WHERE i.issue_type = 'epic'
               AND i.status != 'closed'
               AND (SELECT COUNT(*)
                    FROM dependencies d
                    INNER JOIN issues child ON child.id = d.issue_id
                    WHERE d.depends_on_id = i.id AND d.type = 'parent-child'
                   ) > 0
               AND (SELECT COUNT(*)
                    FROM dependencies d
                    INNER JOIN issues child ON child.id = d.issue_id
                    WHERE d.depends_on_id = i.id AND d.type = 'parent-child'
                   ) = (SELECT COUNT(*)
                        FROM dependencies d
                        INNER JOIN issues child ON child.id = d.issue_id
                        WHERE d.depends_on_id = i.id
                          AND d.type = 'parent-child'
                          AND child.status = 'closed'
                       )
             ORDER BY i.created_at ASC"
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let issue = scan_issue(row)?;
            let total_children: i32 = row.get("total_children")?;
            let closed_children: i32 = row.get("closed_children")?;
            Ok(EpicStatus {
                epic: issue,
                total_children,
                closed_children,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Returns aggregate statistics.
    pub fn get_statistics_impl(&self) -> Result<Statistics> {
        let conn = self.lock_conn()?;
        let mut stats = Statistics::default();

        stats.total_issues = conn.query_row(
            "SELECT COUNT(*) FROM issues",
            [],
            |row| row.get(0),
        )?;
        stats.open_issues = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'open'",
            [],
            |row| row.get(0),
        )?;
        stats.closed_issues = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'closed'",
            [],
            |row| row.get(0),
        )?;
        stats.in_progress_issues = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'in_progress'",
            [],
            |row| row.get(0),
        )?;
        stats.blocked_issues = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'blocked'",
            [],
            |row| row.get(0),
        )?;
        stats.deferred_issues = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'deferred'",
            [],
            |row| row.get(0),
        )?;

        // By type.
        {
            let mut stmt = conn.prepare(
                "SELECT issue_type, COUNT(*) FROM issues GROUP BY issue_type ORDER BY COUNT(*) DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                stats.by_type.push(row?);
            }
        }

        // By priority.
        {
            let mut stmt = conn.prepare(
                "SELECT priority, COUNT(*) FROM issues GROUP BY priority ORDER BY priority ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                stats.by_priority.push(row?);
            }
        }

        // By assignee.
        {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(assignee, '(unassigned)'), COUNT(*)
                 FROM issues
                 WHERE status != 'closed'
                 GROUP BY assignee
                 ORDER BY COUNT(*) DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                stats.by_assignee.push(row?);
            }
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beads_core::dependency::Dependency;
    use beads_core::enums::{DependencyType, Status};
    use beads_core::issue::IssueBuilder;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn get_ready_work_excludes_blocked() {
        let store = test_store();
        let blocker = IssueBuilder::new("Blocker")
            .id("bd-blk1")
            .status(Status::Open)
            .build();
        let blocked = IssueBuilder::new("Blocked")
            .id("bd-blk2")
            .status(Status::Open)
            .build();
        let ready = IssueBuilder::new("Ready")
            .id("bd-rdy1")
            .status(Status::Open)
            .build();

        store.create_issue_impl(&blocker, "alice").unwrap();
        store.create_issue_impl(&blocked, "alice").unwrap();
        store.create_issue_impl(&ready, "alice").unwrap();

        let dep = Dependency {
            issue_id: "bd-blk2".into(),
            depends_on_id: "bd-blk1".into(),
            dep_type: DependencyType::Blocks,
            created_at: Utc::now(),
            created_by: "alice".into(),
            metadata: String::new(),
            thread_id: String::new(),
        };
        store.add_dependency_impl(&dep, "alice").unwrap();

        let work = store
            .get_ready_work_impl(&WorkFilter::default())
            .unwrap();
        let ids: Vec<&str> = work.iter().map(|i| i.id.as_str()).collect();
        // blocker is ready (it blocks others but is not itself blocked).
        assert!(ids.contains(&"bd-blk1"));
        assert!(ids.contains(&"bd-rdy1"));
        assert!(!ids.contains(&"bd-blk2"));
    }

    #[test]
    fn get_statistics() {
        let store = test_store();
        let issue1 = IssueBuilder::new("Open")
            .id("bd-st1")
            .status(Status::Open)
            .build();
        let issue2 = IssueBuilder::new("Closed")
            .id("bd-st2")
            .status(Status::Closed)
            .closed_at(Utc::now())
            .build();
        store.create_issue_impl(&issue1, "alice").unwrap();
        store.create_issue_impl(&issue2, "alice").unwrap();

        let stats = store.get_statistics_impl().unwrap();
        assert_eq!(stats.total_issues, 2);
        assert_eq!(stats.open_issues, 1);
        assert_eq!(stats.closed_issues, 1);
    }
}
