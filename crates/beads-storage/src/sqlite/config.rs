//! Config and metadata key-value store operations for [`SqliteStore`].

use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::error::{Result, StorageError};
use crate::sqlite::store::SqliteStore;

// ---------------------------------------------------------------------------
// Connection-level helpers (shared with Transaction)
// ---------------------------------------------------------------------------

pub(crate) fn set_config_on_conn(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub(crate) fn get_config_on_conn(conn: &Connection, key: &str) -> Result<String> {
    conn.query_row(
        "SELECT value FROM config WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => StorageError::not_found("config", key),
        other => StorageError::Query(other),
    })
}

pub(crate) fn set_metadata_on_conn(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub(crate) fn get_metadata_on_conn(conn: &Connection, key: &str) -> Result<String> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => StorageError::not_found("metadata", key),
        other => StorageError::Query(other),
    })
}

// ---------------------------------------------------------------------------
// SqliteStore methods
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Sets a configuration key-value pair.
    pub fn set_config_impl(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.lock_conn()?;
        set_config_on_conn(&conn, key, value)
    }

    /// Gets a configuration value by key.
    pub fn get_config_impl(&self, key: &str) -> Result<String> {
        let conn = self.lock_conn()?;
        get_config_on_conn(&conn, key)
    }

    /// Returns all configuration key-value pairs.
    pub fn get_all_config_impl(&self) -> Result<HashMap<String, String>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SqliteStore {
        SqliteStore::open_in_memory().unwrap()
    }

    #[test]
    fn set_and_get_config() {
        let store = test_store();
        store.set_config_impl("test_key", "test_value").unwrap();
        let val = store.get_config_impl("test_key").unwrap();
        assert_eq!(val, "test_value");
    }

    #[test]
    fn get_config_not_found() {
        let store = test_store();
        let err = store.get_config_impl("nonexistent").unwrap_err();
        assert!(err.is_not_found());
    }

    #[test]
    fn upsert_config() {
        let store = test_store();
        store.set_config_impl("key1", "v1").unwrap();
        store.set_config_impl("key1", "v2").unwrap();
        let val = store.get_config_impl("key1").unwrap();
        assert_eq!(val, "v2");
    }

    #[test]
    fn get_all_config() {
        let store = test_store();
        let config = store.get_all_config_impl().unwrap();
        // Default config should be present.
        assert!(config.contains_key("compaction_enabled"));
    }
}
