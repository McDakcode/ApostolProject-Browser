//! apb-bookmarks
//!
//! Bookmarks with nested folders, free-form tags, and per-bookmark notes
//! (design doc §7). One store per profile — the caller supplies an
//! already-open `apb_storage::Store` scoped to that profile's database.

use apb_storage::{Migration, Store, StorageError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum BookmarkError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("bookmark not found: {0}")]
    NotFound(Uuid),
}

pub type Result<T> = std::result::Result<T, BookmarkError>;

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "create bookmarks + folders + tags tables",
    sql: "
        CREATE TABLE IF NOT EXISTS bookmark_folders (
            id        TEXT PRIMARY KEY,
            parent_id TEXT REFERENCES bookmark_folders(id) ON DELETE CASCADE,
            name      TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS bookmarks (
            id         TEXT PRIMARY KEY,
            folder_id  TEXT REFERENCES bookmark_folders(id) ON DELETE SET NULL,
            title      TEXT NOT NULL,
            url        TEXT NOT NULL,
            note       TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS bookmark_tags (
            bookmark_id TEXT NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
            tag         TEXT NOT NULL,
            PRIMARY KEY (bookmark_id, tag)
        );
        CREATE INDEX IF NOT EXISTS idx_bookmarks_folder ON bookmarks(folder_id);
        CREATE INDEX IF NOT EXISTS idx_bookmark_tags_tag ON bookmark_tags(tag);
    ",
}];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: Uuid,
    pub folder_id: Option<Uuid>,
    pub title: String,
    pub url: String,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct BookmarkStore {
    store: Store,
}

impl BookmarkStore {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn create_folder(&self, name: &str, parent_id: Option<Uuid>) -> Result<Folder> {
        let folder = Folder {
            id: Uuid::new_v4(),
            parent_id,
            name: name.to_string(),
        };
        self.store.with_conn(|c| {
            c.execute(
                "INSERT INTO bookmark_folders (id, parent_id, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    folder.id.to_string(),
                    folder.parent_id.map(|p| p.to_string()),
                    folder.name
                ],
            )
        })?;
        Ok(folder)
    }

    pub fn add(
        &self,
        title: &str,
        url: &str,
        folder_id: Option<Uuid>,
        tags: &[&str],
        note: Option<&str>,
    ) -> Result<Bookmark> {
        let bm = Bookmark {
            id: Uuid::new_v4(),
            folder_id,
            title: title.to_string(),
            url: url.to_string(),
            note: note.map(str::to_string),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            created_at: chrono::Utc::now(),
        };
        self.store.with_conn(|c| {
            c.execute(
                "INSERT INTO bookmarks (id, folder_id, title, url, note, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    bm.id.to_string(),
                    bm.folder_id.map(|f| f.to_string()),
                    bm.title,
                    bm.url,
                    bm.note,
                    bm.created_at.to_rfc3339(),
                ],
            )?;
            for tag in &bm.tags {
                c.execute(
                    "INSERT INTO bookmark_tags (bookmark_id, tag) VALUES (?1, ?2)",
                    rusqlite::params![bm.id.to_string(), tag],
                )?;
            }
            Ok(())
        })?;
        Ok(bm)
    }

    /// Substring search across title, url, and tags — backs the Command
    /// Palette "Search bookmarks" action (§22).
    pub fn search(&self, query: &str) -> Result<Vec<Bookmark>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let ids: Vec<Uuid> = self.store.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT DISTINCT b.id FROM bookmarks b
                 LEFT JOIN bookmark_tags t ON t.bookmark_id = b.id
                 WHERE lower(b.title) LIKE ?1 OR lower(b.url) LIKE ?1 OR lower(t.tag) LIKE ?1",
            )?;
            let rows = stmt.query_map([&pattern], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect();

        ids.into_iter().map(|id| self.get(id)).collect()
    }

    pub fn get(&self, id: Uuid) -> Result<Bookmark> {
        let row = self.store.with_conn(|c| {
            c.query_row(
                "SELECT id, folder_id, title, url, note, created_at FROM bookmarks WHERE id = ?1",
                [id.to_string()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<String>>(4)?,
                        r.get::<_, String>(5)?,
                    ))
                },
            )
        });

        let (id_s, folder_s, title, url, note, created_s) = match row {
            Ok(r) => r,
            Err(StorageError::Sqlite(rusqlite::Error::QueryReturnedNoRows)) => {
                return Err(BookmarkError::NotFound(id))
            }
            Err(e) => return Err(e.into()),
        };

        let tags = self.store.with_conn(|c| {
            let mut stmt = c.prepare("SELECT tag FROM bookmark_tags WHERE bookmark_id = ?1")?;
            let rows = stmt.query_map([&id_s], |r| r.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;

        Ok(Bookmark {
            id: Uuid::parse_str(&id_s).unwrap_or(id),
            folder_id: folder_s.and_then(|s| Uuid::parse_str(&s).ok()),
            title,
            url,
            note,
            tags,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_s)
                .map(|d| d.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_search_bookmarks() {
        let store = Store::open_in_memory(MIGRATIONS).unwrap();
        let bs = BookmarkStore::new(store);

        let folder = bs.create_folder("Rust", None).unwrap();
        bs.add(
            "The Rust Book",
            "https://doc.rust-lang.org/book/",
            Some(folder.id),
            &["rust", "docs"],
            Some("Great intro"),
        )
        .unwrap();
        bs.add("Random", "https://example.com", None, &["misc"], None)
            .unwrap();

        let found = bs.search("rust").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "The Rust Book");
        assert_eq!(found[0].tags.len(), 2);
    }

    #[test]
    fn missing_bookmark_errors() {
        let store = Store::open_in_memory(MIGRATIONS).unwrap();
        let bs = BookmarkStore::new(store);
        assert!(matches!(bs.get(Uuid::new_v4()), Err(BookmarkError::NotFound(_))));
    }
}
