//! apb-tabs
//!
//! In-memory tab model plus JSON session persistence. Tabs form a tree
//! (child tabs opened from a parent stay nested, à la vertical-tab browsers,
//! §25), can be pinned, grouped, and put to sleep to save memory. `Session`
//! is the crash-recovery unit (§33): serialized to disk on every mutation
//! debounce in the real app, and reloaded on next launch.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum TabError {
    #[error("tab not found: {0}")]
    NotFound(Uuid),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, TabError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub group_id: Option<Uuid>,
    pub title: String,
    pub url: String,
    pub pinned: bool,
    /// Sleeping tabs keep their metadata but the engine has discarded the
    /// underlying webview to free memory (§25 "sleeping tabs").
    pub sleeping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabGroup {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub collapsed: bool,
}

/// One workspace's worth of tabs — matches design doc §23/§25.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabTree {
    tabs: HashMap<Uuid, Tab>,
    groups: HashMap<Uuid, TabGroup>,
    /// Insertion/display order, independent of the parent/child nesting.
    order: Vec<Uuid>,
    active_tab: Option<Uuid>,
}

impl TabTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_tab(&mut self, url: &str, title: &str, parent_id: Option<Uuid>) -> Uuid {
        let tab = Tab {
            id: Uuid::new_v4(),
            parent_id,
            group_id: None,
            title: title.to_string(),
            url: url.to_string(),
            pinned: false,
            sleeping: false,
        };
        let id = tab.id;
        self.order.push(id);
        self.tabs.insert(id, tab);
        self.active_tab = Some(id);
        id
    }

    pub fn close_tab(&mut self, id: Uuid) -> Result<()> {
        if self.tabs.remove(&id).is_none() {
            return Err(TabError::NotFound(id));
        }
        self.order.retain(|t| *t != id);
        // Reparent orphaned children to the closed tab's parent, so the tree
        // doesn't dangle (mirrors Arc/Zen "close parent -> promote children").
        let orphan_parent = self.tabs.get(&id).and_then(|t| t.parent_id);
        for tab in self.tabs.values_mut() {
            if tab.parent_id == Some(id) {
                tab.parent_id = orphan_parent;
            }
        }
        if self.active_tab == Some(id) {
            self.active_tab = self.order.last().copied();
        }
        Ok(())
    }

    pub fn pin(&mut self, id: Uuid, pinned: bool) -> Result<()> {
        self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?.pinned = pinned;
        Ok(())
    }

    pub fn set_sleeping(&mut self, id: Uuid, sleeping: bool) -> Result<()> {
        self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?.sleeping = sleeping;
        Ok(())
    }

    pub fn create_group(&mut self, name: &str, color: &str) -> Uuid {
        let group = TabGroup {
            id: Uuid::new_v4(),
            name: name.to_string(),
            color: color.to_string(),
            collapsed: false,
        };
        let id = group.id;
        self.groups.insert(id, group);
        id
    }

    pub fn assign_group(&mut self, tab_id: Uuid, group_id: Option<Uuid>) -> Result<()> {
        self.tabs
            .get_mut(&tab_id)
            .ok_or(TabError::NotFound(tab_id))?
            .group_id = group_id;
        Ok(())
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active_tab.and_then(|id| self.tabs.get(&id))
    }

    pub fn set_active(&mut self, id: Uuid) -> Result<()> {
        if !self.tabs.contains_key(&id) {
            return Err(TabError::NotFound(id));
        }
        self.active_tab = Some(id);
        Ok(())
    }

    /// Ordered list of tabs, pinned tabs first (matches typical tab-bar UX).
    pub fn ordered_tabs(&self) -> Vec<&Tab> {
        let mut tabs: Vec<&Tab> = self.order.iter().filter_map(|id| self.tabs.get(id)).collect();
        tabs.sort_by_key(|t| !t.pinned);
        tabs
    }

    pub fn children_of(&self, parent: Uuid) -> Vec<&Tab> {
        self.tabs.values().filter(|t| t.parent_id == Some(parent)).collect()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

/// Crash-recovery / restart unit: every workspace's tab tree, keyed by
/// workspace id, plus which workspace was focused. See §33.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Session {
    pub workspaces: HashMap<Uuid, TabTree>,
    pub focused_workspace: Option<Uuid>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_close_and_reparent() {
        let mut tree = TabTree::new();
        let parent = tree.open_tab("https://a.com", "A", None);
        let child = tree.open_tab("https://a.com/child", "A child", Some(parent));
        assert_eq!(tree.len(), 2);

        tree.close_tab(parent).unwrap();
        assert_eq!(tree.len(), 1);
        // child should now be reparented to None (parent's parent, which was None)
        assert_eq!(tree.children_of(parent).len(), 0);
        let child_tab = tree.ordered_tabs().into_iter().find(|t| t.id == child).unwrap();
        assert_eq!(child_tab.parent_id, None);
    }

    #[test]
    fn pinned_tabs_sort_first() {
        let mut tree = TabTree::new();
        let a = tree.open_tab("https://a.com", "A", None);
        let _b = tree.open_tab("https://b.com", "B", None);
        tree.pin(a, true).unwrap();
        let ordered = tree.ordered_tabs();
        assert!(ordered[0].pinned);
    }

    #[test]
    fn session_roundtrip_via_json() {
        let mut session = Session::new();
        let ws = Uuid::new_v4();
        let mut tree = TabTree::new();
        tree.open_tab("https://example.com", "Example", None);
        session.workspaces.insert(ws, tree);
        session.focused_workspace = Some(ws);

        let tmp = std::env::temp_dir().join(format!("apb-session-{}.json", Uuid::new_v4()));
        session.save_to_file(&tmp).unwrap();
        let loaded = Session::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.focused_workspace, Some(ws));
        assert_eq!(loaded.workspaces.get(&ws).unwrap().len(), 1);
        std::fs::remove_file(&tmp).ok();
    }
}
