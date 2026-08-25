// Made by MrDuck && Ox-Alpha
//! apb-profiles
//!
//! Profile isolation is the foundation of APB's privacy model (design doc §5,
//! §10A.2). Each profile owns its own SQLite database, its own notes/canvas
//! directories on disk, and a `PrivacyLevel` + `StorageMode` that downstream
//! crates (network, ai, extensions) must respect. This crate does not touch
//! cookies/webview storage directly — that partitioning is delegated to the
//! browser engine adapter, keyed by `Profile::storage_root`.

use apb_storage::{Migration, Store, StorageError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("profile not found: {0}")]
    NotFound(Uuid),
    #[error("a profile named '{0}' already exists")]
    DuplicateName(String),
}

pub type Result<T> = std::result::Result<T, ProfileError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Standard,
    Balanced,
    Strict,
    Maximum,
    Custom,
}

/// Whether a profile's browsing data survives past the current session.
/// `Ephemeral` profiles (Anonymous Profile / Temporary Sessions, §10A.2-3)
/// are wiped by the engine on window close; enforcement lives at the
/// engine/storage-partition layer, this flag is the source of truth for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMode {
    Persistent,
    Ephemeral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
    pub accent_color: String,
    pub privacy_level: PrivacyLevel,
    pub storage_mode: StorageMode,
    pub search_engine: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Profile {
// Made by MrDuck && Ox-Alpha
    fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            icon: "circle".into(),
            accent_color: "#6C8CFF".into(),
            privacy_level: PrivacyLevel::Balanced,
            storage_mode: StorageMode::Persistent,
            search_engine: "duckduckgo".into(),
            created_at: chrono::Utc::now(),
        }
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "create profiles table",
    sql: "CREATE TABLE IF NOT EXISTS profiles (
        id            TEXT PRIMARY KEY,
        name          TEXT NOT NULL UNIQUE,
        icon          TEXT NOT NULL,
        accent_color  TEXT NOT NULL,
        privacy_level TEXT NOT NULL,
        storage_mode  TEXT NOT NULL,
        search_engine TEXT NOT NULL,
        created_at    TEXT NOT NULL
    );",
}];

/// Root-level registry of all profiles on this device. Lives in a single
/// top-level `registry.sqlite` (outside any individual profile directory),
/// while each profile's *content* (bookmarks/history/notes) lives in its own
/// isolated subtree under `data_root/profiles/<id>/`.
pub struct ProfileManager {
    registry: Store,
    data_root: PathBuf,
}

impl ProfileManager {
    pub fn open(data_root: impl AsRef<Path>) -> Result<Self> {
        let data_root = data_root.as_ref().to_path_buf();
        std::fs::create_dir_all(data_root.join("profiles"))?;
        let registry = Store::open(data_root.join("registry.sqlite"), MIGRATIONS)?;
        Ok(Self { registry, data_root })
    }

    #[cfg(test)]
    pub fn open_in_memory(data_root: impl AsRef<Path>) -> Result<Self> {
        let data_root = data_root.as_ref().to_path_buf();
        std::fs::create_dir_all(data_root.join("profiles"))?;
        let registry = Store::open_in_memory(MIGRATIONS)?;
        Ok(Self { registry, data_root })
    }

    /// Filesystem root for a given profile: `<data_root>/profiles/<id>/`.
    /// `notes/` and `canvas/` beneath this must be plain files (§30) — never
    /// packed into SQLite.
    pub fn storage_root(&self, id: Uuid) -> PathBuf {
        self.data_root.join("profiles").join(id.to_string())
    }

// Made by MrDuck && Ox-Alpha
    pub fn create(&self, name: &str) -> Result<Profile> {
        let profile = Profile::new(name);
        let root = self.storage_root(profile.id);
        std::fs::create_dir_all(root.join("notes"))?;
        std::fs::create_dir_all(root.join("canvas"))?;

        let inserted = self.registry.with_conn(|c| {
            c.execute(
                "INSERT INTO profiles (id, name, icon, accent_color, privacy_level, storage_mode, search_engine, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    profile.id.to_string(),
                    profile.name,
                    profile.icon,
                    profile.accent_color,
                    format!("{:?}", profile.privacy_level),
                    format!("{:?}", profile.storage_mode),
                    profile.search_engine,
                    profile.created_at.to_rfc3339(),
                ],
            )
        });

        match inserted {
            Ok(_) => Ok(profile),
            Err(StorageError::Sqlite(rusqlite::Error::SqliteFailure(e, _)))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(ProfileError::DuplicateName(name.to_string()))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Convenience constructor for an Anonymous Profile (§10A.2): ephemeral
    /// storage mode + Maximum privacy level by default.
    pub fn create_anonymous(&self) -> Result<Profile> {
        let mut profile = Profile::new(format!("Anonymous {}", &Uuid::new_v4().to_string()[..4]));
        profile.privacy_level = PrivacyLevel::Maximum;
        profile.storage_mode = StorageMode::Ephemeral;

        let root = self.storage_root(profile.id);
        std::fs::create_dir_all(root.join("notes"))?;
        std::fs::create_dir_all(root.join("canvas"))?;

        self.registry.with_conn(|c| {
            c.execute(
                "INSERT INTO profiles (id, name, icon, accent_color, privacy_level, storage_mode, search_engine, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    profile.id.to_string(),
                    profile.name,
                    profile.icon,
                    profile.accent_color,
                    format!("{:?}", profile.privacy_level),
                    format!("{:?}", profile.storage_mode),
                    profile.search_engine,
                    profile.created_at.to_rfc3339(),
                ],
            )
        })?;
        Ok(profile)
    }

    pub fn list(&self) -> Result<Vec<Profile>> {
        self.registry
            .with_conn(|c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, icon, accent_color, privacy_level, storage_mode, search_engine, created_at
                     FROM profiles ORDER BY created_at ASC",
                )?;
                let rows = stmt.query_map([], row_to_profile)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(Into::into)
    }

    /// Rename a profile. Rejects duplicates (UNIQUE constraint) and unknown ids.
    pub fn rename(&self, id: Uuid, name: &str) -> Result<()> {
        let result = self.registry.with_conn(|c| {
            c.execute(
                "UPDATE profiles SET name = ?1 WHERE id = ?2",
                rusqlite::params![name, id.to_string()],
            )
        });
        match result {
            Ok(0) => Err(ProfileError::NotFound(id)),
            Ok(_) => Ok(()),
            Err(StorageError::Sqlite(rusqlite::Error::SqliteFailure(e, _)))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(ProfileError::DuplicateName(name.to_string()))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete(&self, id: Uuid) -> Result<()> {
        let affected = self
            .registry
            .with_conn(|c| c.execute("DELETE FROM profiles WHERE id = ?1", [id.to_string()]))?;
        if affected == 0 {
            return Err(ProfileError::NotFound(id));
        }
        let root = self.storage_root(id);
        if root.exists() {
            std::fs::remove_dir_all(root)?;
        }
        Ok(())
    }
}

// Made by MrDuck && Ox-Alpha
fn row_to_profile(row: &rusqlite::Row) -> rusqlite::Result<Profile> {
    let privacy: String = row.get(4)?;
    let storage: String = row.get(5)?;
    let created_at: String = row.get(7)?;
    Ok(Profile {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        name: row.get(1)?,
        icon: row.get(2)?,
        accent_color: row.get(3)?,
        privacy_level: match privacy.as_str() {
            "Standard" => PrivacyLevel::Standard,
            "Strict" => PrivacyLevel::Strict,
            "Maximum" => PrivacyLevel::Maximum,
            "Custom" => PrivacyLevel::Custom,
            _ => PrivacyLevel::Balanced,
        },
        storage_mode: match storage.as_str() {
            "Ephemeral" => StorageMode::Ephemeral,
            _ => StorageMode::Persistent,
        },
        search_engine: row.get(6)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_list_delete_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("apb-test-{}", Uuid::new_v4()));
        let mgr = ProfileManager::open(&tmp).unwrap();

        let p1 = mgr.create("Personal").unwrap();
        let _p2 = mgr.create("Work").unwrap();
        assert_eq!(mgr.list().unwrap().len(), 2);

        // Duplicate name must be rejected.
        assert!(matches!(mgr.create("Personal"), Err(ProfileError::DuplicateName(_))));

        let root = mgr.storage_root(p1.id);
        assert!(root.join("notes").exists());

        mgr.delete(p1.id).unwrap();
        assert_eq!(mgr.list().unwrap().len(), 1);
        assert!(!root.exists());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn anonymous_profile_is_ephemeral_and_maximum_privacy() {
        let tmp = std::env::temp_dir().join(format!("apb-test-{}", Uuid::new_v4()));
        let mgr = ProfileManager::open(&tmp).unwrap();
        let anon = mgr.create_anonymous().unwrap();
        assert_eq!(anon.storage_mode, StorageMode::Ephemeral);
        assert_eq!(anon.privacy_level, PrivacyLevel::Maximum);
        std::fs::remove_dir_all(&tmp).ok();
    }
}

// Made by MrDuck && Ox-Alpha