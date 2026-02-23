//! [`SqliteStore`] -- SQLite-backed storage implementation.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use tracing::{debug, info};

use crate::error::{Result, StorageError};
use crate::sqlite::schema;

/// SQLite-backed implementation of the [`Storage`](crate::traits::Storage) trait.
///
/// Wraps a [`rusqlite::Connection`] in a `Mutex` for thread safety.  All
/// public methods acquire the lock, execute SQL, and release it.
pub struct SqliteStore {
    /// The mutex-protected SQLite connection.
    pub(crate) conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Opens (or creates) a SQLite database at the given path.
    ///
    /// Enables WAL mode and foreign keys, then initialises the schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        info!(?path, "opening SQLite database");

        let conn = Connection::open(path).map_err(|e| {
            StorageError::Connection(format!("failed to open {}: {e}", path.display()))
        })?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.configure_connection()?;
        store.init_schema()?;

        Ok(store)
    }

    /// Opens an in-memory SQLite database (useful for tests).
    pub fn open_in_memory() -> Result<Self> {
        debug!("opening in-memory SQLite database");
        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::Connection(format!("failed to open in-memory db: {e}")))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.configure_connection()?;
        store.init_schema()?;

        Ok(store)
    }

    /// Sets connection pragmas (WAL mode, foreign keys, busy timeout).
    fn configure_connection(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::Connection(format!("mutex poisoned: {e}"))
        })?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|e| StorageError::Connection(format!("failed to set pragmas: {e}")))?;

        Ok(())
    }

    /// Creates all tables and indexes if they do not exist, then runs
    /// migrations.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::Connection(format!("mutex poisoned: {e}"))
        })?;

        // Check if schema is already at current version.
        let version: std::result::Result<i32, _> = conn.query_row(
            "SELECT value FROM config WHERE key = 'schema_version'",
            [],
            |row| {
                let v: String = row.get(0)?;
                Ok(v.parse::<i32>().unwrap_or(0))
            },
        );
        if let Ok(v) = version {
            if v >= schema::CURRENT_SCHEMA_VERSION {
                debug!(version = v, "schema already at current version, skipping init");
                return Ok(());
            }
        }

        // Execute DDL statements.
        for stmt in schema::SCHEMA_STATEMENTS {
            conn.execute_batch(stmt).map_err(|e| StorageError::Migration {
                name: "init_schema".into(),
                reason: format!("{e}\nStatement: {}", truncate(stmt, 120)),
            })?;
        }

        // Insert default config (INSERT OR IGNORE to be idempotent).
        for &(key, value) in schema::DEFAULT_CONFIG {
            conn.execute(
                "INSERT OR IGNORE INTO config (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|e| StorageError::Migration {
                name: "default_config".into(),
                reason: format!("failed to insert {key}: {e}"),
            })?;
        }

        // Run migrations.
        Self::run_migrations_on_conn(&conn)?;

        // Mark schema version.
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('schema_version', ?1)",
            rusqlite::params![schema::CURRENT_SCHEMA_VERSION.to_string()],
        )
        .map_err(|e| StorageError::Migration {
            name: "schema_version".into(),
            reason: e.to_string(),
        })?;

        info!("schema initialized (version {})", schema::CURRENT_SCHEMA_VERSION);
        Ok(())
    }

    /// Applies pending migrations tracked via the `metadata` table.
    fn run_migrations_on_conn(conn: &Connection) -> Result<()> {
        for &(name, sql) in schema::MIGRATIONS {
            let key = format!("migration:{name}");
            let already_applied: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM metadata WHERE key = ?1",
                    rusqlite::params![key],
                    |row| row.get::<_, i32>(0),
                )
                .unwrap_or(0)
                > 0;

            if already_applied {
                debug!(name, "migration already applied, skipping");
                continue;
            }

            debug!(name, "applying migration");
            conn.execute_batch(sql).map_err(|e| StorageError::Migration {
                name: name.to_string(),
                reason: e.to_string(),
            })?;

            conn.execute(
                "INSERT INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, "applied"],
            )
            .map_err(|e| StorageError::Migration {
                name: name.to_string(),
                reason: format!("failed to mark migration: {e}"),
            })?;
        }
        Ok(())
    }

    /// Acquires the connection lock. Helper used by all operation modules.
    pub(crate) fn lock_conn(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|e| {
            StorageError::Connection(format!("mutex poisoned: {e}"))
        })
    }
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStore").finish_non_exhaustive()
    }
}

/// Truncates a string for error messages.
fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory() {
        let store = SqliteStore::open_in_memory().unwrap();
        // Verify tables exist by querying config.
        let conn = store.lock_conn().unwrap();
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM config", [], |row| row.get(0))
            .unwrap();
        assert!(count > 0, "default config should be inserted");
    }

    #[test]
    fn schema_version_set() {
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.lock_conn().unwrap();
        let version: String = conn
            .query_row(
                "SELECT value FROM config WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            version,
            schema::CURRENT_SCHEMA_VERSION.to_string()
        );
    }

    #[test]
    fn idempotent_init() {
        let store = SqliteStore::open_in_memory().unwrap();
        // Re-init should succeed without error.
        store.init_schema().unwrap();
    }
}
