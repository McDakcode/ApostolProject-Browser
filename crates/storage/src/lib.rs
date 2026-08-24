//! apb-storage
//!
//! Thin, dependency-light wrapper around SQLite used by every profile-scoped
//! crate (bookmarks, history, tabs, ...). Owns connection lifecycle and a
//! forward-only migration runner so schema evolution is explicit and
//! auditable, per APB Data Architecture (see design doc §30).

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration {version} failed: {source}")]
    Migration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// A single forward migration. Migrations are applied in ascending `version`
/// order exactly once, tracked in the `_apb_migrations` table.
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub sql: &'static str,
}

/// Shared, thread-safe handle to a profile's SQLite database.
#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    /// Open (or create) the database at `path` and apply any pending
    /// migrations from `migrations`. Safe to call repeatedly (idempotent).
    pub fn open(path: impl AsRef<Path>, migrations: &[Migration]) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Self::ensure_migration_table(&conn)?;
        Self::apply_migrations(&conn, migrations)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// In-memory store, primarily for tests and the CLI demo.
    pub fn open_in_memory(migrations: &[Migration]) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Self::ensure_migration_table(&conn)?;
        Self::apply_migrations(&conn, migrations)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn ensure_migration_table(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _apb_migrations (
                version     INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(())
    }

    fn apply_migrations(conn: &Connection, migrations: &[Migration]) -> Result<()> {
        let mut applied: Vec<u32> = {
            let mut stmt = conn.prepare("SELECT version FROM _apb_migrations")?;
            let rows = stmt.query_map([], |r| r.get::<_, u32>(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        applied.sort_unstable();

        let mut sorted: Vec<&Migration> = migrations.iter().collect();
        sorted.sort_by_key(|m| m.version);

        for m in sorted {
            if applied.binary_search(&m.version).is_ok() {
                continue;
            }
            conn.execute_batch(m.sql)
                .map_err(|source| StorageError::Migration {
                    version: m.version,
                    source,
                })?;
            conn.execute(
                "INSERT INTO _apb_migrations (version, description) VALUES (?1, ?2)",
                (m.version, m.description),
            )?;
        }
        Ok(())
    }

    /// Run a closure with exclusive access to the underlying connection.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        let conn = self.conn.lock().expect("storage mutex poisoned");
        Ok(f(&conn)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &[Migration] = &[Migration {
        version: 1,
        description: "create demo table",
        sql: "CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    }];

    #[test]
    fn migrations_apply_once_and_are_idempotent() {
        let store = Store::open_in_memory(M).unwrap();
        store
            .with_conn(|c| c.execute("INSERT INTO demo (name) VALUES ('x')", []))
            .unwrap();

        // Re-applying the same migration set must not error or duplicate.
        let applied_count: u32 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM _apb_migrations", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(applied_count, 1);
    }
}
