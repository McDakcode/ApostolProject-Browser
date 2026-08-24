//! apb-notes
//!
//! Notes are plain `.md` files on disk — the vault IS the filesystem
//! directory (design doc §13, §30: "never lock the user into a proprietary
//! database"). This crate's SQLite index (backlinks, tags, full-text) is a
//! disposable cache that can always be regenerated with `reindex`. Losing
//! `index.sqlite` never loses data — only search/graph performance until the
//! next reindex.

use apb_storage::{Migration, Store, StorageError};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum NotesError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("note not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, NotesError>;

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "create notes index tables",
    sql: "
        CREATE TABLE IF NOT EXISTS notes (
            path  TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS note_tags (
            note_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
            tag       TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS note_links (
            from_path TEXT NOT NULL REFERENCES notes(path) ON DELETE CASCADE,
            to_title  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_note_links_to ON note_links(to_title);
        CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag);
    ",
}];

#[derive(Debug, Clone, Serialize)]
pub struct NoteMeta {
    pub path: String,
    pub title: String,
    pub tags: Vec<String>,
    pub outgoing_links: Vec<String>,
}

pub struct Vault {
    root: PathBuf,
    index: Store,
}

impl Vault {
    /// `root` is the profile's `notes/` directory (plain files).
    /// `index_path` is a sidecar SQLite file, safe to delete at any time.
    pub fn open(root: impl AsRef<Path>, index_path: impl AsRef<Path>) -> Result<Self> {
        std::fs::create_dir_all(&root)?;
        let index = Store::open(index_path, MIGRATIONS)?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            index,
        })
    }

    #[cfg(test)]
    pub fn open_with_memory_index(root: impl AsRef<Path>) -> Result<Self> {
        std::fs::create_dir_all(&root)?;
        let index = Store::open_in_memory(MIGRATIONS)?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            index,
        })
    }

    /// Create/overwrite a note file, then update the index for it.
    pub fn write_note(&self, relative_path: &str, content: &str) -> Result<()> {
        let full = self.root.join(relative_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, content)?;
        self.index_one(relative_path, content)?;
        Ok(())
    }

    pub fn read_note(&self, relative_path: &str) -> Result<String> {
        let full = self.root.join(relative_path);
        std::fs::read_to_string(&full).map_err(|_| NotesError::NotFound(relative_path.to_string()))
    }

    fn index_one(&self, relative_path: &str, content: &str) -> Result<()> {
        let title = extract_title(content, relative_path);
        let tags = extract_tags(content);
        let links = extract_wikilinks(content);

        self.index.with_conn(|c| {
            c.execute(
                "INSERT INTO notes (path, title) VALUES (?1, ?2)
                 ON CONFLICT(path) DO UPDATE SET title = excluded.title",
                rusqlite::params![relative_path, title],
            )?;
            c.execute("DELETE FROM note_tags WHERE note_path = ?1", [relative_path])?;
            for tag in &tags {
                c.execute(
                    "INSERT INTO note_tags (note_path, tag) VALUES (?1, ?2)",
                    rusqlite::params![relative_path, tag],
                )?;
            }
            c.execute("DELETE FROM note_links WHERE from_path = ?1", [relative_path])?;
            for link in &links {
                c.execute(
                    "INSERT INTO note_links (from_path, to_title) VALUES (?1, ?2)",
                    rusqlite::params![relative_path, link],
                )?;
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Rebuild the entire index from disk. Safe to call any time (e.g. after
    /// deleting index.sqlite, or on first run against a vault that already
    /// has notes from another APB installation).
    pub fn reindex(&self) -> Result<usize> {
        self.index.with_conn(|c| {
            c.execute("DELETE FROM notes", [])?;
            c.execute("DELETE FROM note_tags", [])?;
            c.execute("DELETE FROM note_links", [])?;
            Ok(())
        })?;

        let mut count = 0;
        for entry in walk_markdown(&self.root)? {
            let rel = entry
                .strip_prefix(&self.root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read_to_string(&entry)?;
            self.index_one(&rel, &content)?;
            count += 1;
        }
        Ok(count)
    }

    /// Notes whose content contains `[[Title]]` pointing at `title` — the
    /// backlinks panel (§13/§14).
    pub fn backlinks(&self, title: &str) -> Result<Vec<String>> {
        self.index
            .with_conn(|c| {
                let mut stmt =
                    c.prepare("SELECT DISTINCT from_path FROM note_links WHERE to_title = ?1")?;
                let rows = stmt.query_map([title], |r| r.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(Into::into)
    }

    pub fn notes_with_tag(&self, tag: &str) -> Result<Vec<String>> {
        self.index
            .with_conn(|c| {
                let mut stmt = c.prepare("SELECT note_path FROM note_tags WHERE tag = ?1")?;
                let rows = stmt.query_map([tag], |r| r.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(Into::into)
    }

    /// Full graph as (from_title, to_title) edges — feeds the Graph View
    /// (§14). Unresolved links (pointing at a title with no matching note)
    /// are included too, so the UI can render them as "ghost" nodes.
    pub fn graph_edges(&self) -> Result<Vec<(String, String)>> {
        self.index
            .with_conn(|c| {
                let mut stmt = c.prepare(
                    "SELECT n.title, l.to_title FROM note_links l
                     JOIN notes n ON n.path = l.from_path",
                )?;
                let rows = stmt.query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(Into::into)
    }
}

fn extract_title(content: &str, fallback_path: &str) -> String {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    fallback_path.trim_end_matches(".md").to_string()
}

fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let bytes: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '#' && (i == 0 || bytes[i - 1].is_whitespace()) {
            let mut j = i + 1;
            let mut tag = String::new();
            while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '-' || bytes[j] == '_') {
                tag.push(bytes[j]);
                j += 1;
            }
            if !tag.is_empty() {
                tags.push(tag);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    tags
}

/// Все `[[вики-ссылки]]` из текста заметки (порядок вхождения, без дублей
/// подряд — дедупликация на совести вызывающего).
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let link = after[..end].trim().to_string();
            if !link.is_empty() {
                links.push(link);
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    links
}

fn walk_markdown(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_markdown(&path)?);
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_note_indexes_title_tags_links() {
        let tmp = std::env::temp_dir().join(format!("apb-vault-{}", uuid_like()));
        let vault = Vault::open_with_memory_index(&tmp).unwrap();

        vault
            .write_note(
                "Rust Browser Architecture.md",
                "# Rust Browser Architecture\n\n#rust #architecture\n\nSee [[Network Layer]] and [[Extension System]].",
            )
            .unwrap();

        let backlinks = vault.backlinks("Network Layer").unwrap();
        assert_eq!(backlinks, vec!["Rust Browser Architecture.md"]);

        let tagged = vault.notes_with_tag("rust").unwrap();
        assert_eq!(tagged.len(), 1);

        let edges = vault.graph_edges().unwrap();
        assert_eq!(edges.len(), 2);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn reindex_rebuilds_from_disk_only() {
        let tmp = std::env::temp_dir().join(format!("apb-vault-{}", uuid_like()));
        let vault = Vault::open_with_memory_index(&tmp).unwrap();
        vault.write_note("A.md", "# A\n\nLinks to [[B]].").unwrap();
        vault.write_note("B.md", "# B\n\nNo links here.").unwrap();

        let n = vault.reindex().unwrap();
        assert_eq!(n, 2);
        assert_eq!(vault.backlinks("B").unwrap(), vec!["A.md"]);

        std::fs::remove_dir_all(&tmp).ok();
    }

    // Tiny local counter to avoid pulling `uuid` into this crate just for tests.
    fn uuid_like() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())
    }
}
