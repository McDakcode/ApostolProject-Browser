// Made by MrDuck && Ox-Alpha
//! apb-history
//!
//! Visit history for a single profile. Recording is gated by
//! `RecordingPolicy`, which the caller derives from the owning profile's
//! `StorageMode` (Ephemeral profiles / private windows must pass
//! `RecordingPolicy::Disabled` — see design doc §10A.3).

use apb_storage::{Migration, Store, StorageError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

pub type Result<T> = std::result::Result<T, HistoryError>;

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "create history table",
    sql: "
        CREATE TABLE IF NOT EXISTS history (
            id         TEXT PRIMARY KEY,
            url        TEXT NOT NULL,
            title      TEXT NOT NULL,
            visited_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
        CREATE INDEX IF NOT EXISTS idx_history_visited_at ON history(visited_at);
    ",
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingPolicy {
    Enabled,
    /// No-op writes. Used for Anonymous/ephemeral profiles and private
    /// windows — history simply never touches disk.
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Visit {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub visited_at: chrono::DateTime<chrono::Utc>,
}

pub struct HistoryStore {
    store: Store,
    policy: RecordingPolicy,
}

impl HistoryStore {
    pub fn new(store: Store, policy: RecordingPolicy) -> Self {
        Self { store, policy }
    }

    pub fn record(&self, url: &str, title: &str) -> Result<Option<Visit>> {
        if self.policy == RecordingPolicy::Disabled {
            return Ok(None);
        }
        let visit = Visit {
            id: Uuid::new_v4(),
            url: url.to_string(),
            title: title.to_string(),
            visited_at: chrono::Utc::now(),
        };
        self.store.with_conn(|c| {
            c.execute(
                "INSERT INTO history (id, url, title, visited_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    visit.id.to_string(),
                    visit.url,
                    visit.title,
                    visit.visited_at.to_rfc3339()
                ],
            )
        })?;
        Ok(Some(visit))
    }

    pub fn recent(&self, limit: u32) -> Result<Vec<Visit>> {
        self.store
            .with_conn(|c| {
                let mut stmt = c.prepare(
                    "SELECT id, url, title, visited_at FROM history ORDER BY visited_at DESC LIMIT ?1",
                )?;
                let rows = stmt.query_map([limit], |r| {
                    Ok(Visit {
                        id: Uuid::parse_str(&r.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
                        url: r.get(1)?,
                        title: r.get(2)?,
                        visited_at: chrono::DateTime::parse_from_rfc3339(&r.get::<_, String>(3)?)
                            .map(|d| d.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                    })
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(Into::into)
    }

    /// Clear browsing data — backs "Clear Site Data" / Panic Button actions
    /// (§10A.10, §10A.26).
    pub fn clear_all(&self) -> Result<()> {
        self.store.with_conn(|c| c.execute("DELETE FROM history", []))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_policy_records_visits() {
        let store = Store::open_in_memory(MIGRATIONS).unwrap();
        let hs = HistoryStore::new(store, RecordingPolicy::Enabled);
        hs.record("https://a.com", "A").unwrap();
        hs.record("https://b.com", "B").unwrap();
        assert_eq!(hs.recent(10).unwrap().len(), 2);
    }

    #[test]
    fn disabled_policy_never_persists() {
        let store = Store::open_in_memory(MIGRATIONS).unwrap();
        let hs = HistoryStore::new(store, RecordingPolicy::Disabled);
        let result = hs.record("https://a.com", "A").unwrap();
        assert!(result.is_none());
        assert_eq!(hs.recent(10).unwrap().len(), 0);
    }

    #[test]
    fn clear_all_wipes_history() {
        let store = Store::open_in_memory(MIGRATIONS).unwrap();
        let hs = HistoryStore::new(store, RecordingPolicy::Enabled);
        hs.record("https://a.com", "A").unwrap();
        hs.clear_all().unwrap();
        assert_eq!(hs.recent(10).unwrap().len(), 0);
    }
}

// Made by MrDuck && Ox-Alpha