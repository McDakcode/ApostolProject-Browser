#![allow(unused_imports)]

use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

use apb_bookmarks::Bookmark;
#[tauri::command]
pub(crate) fn add_bookmark(
    state: tauri::State<'_, SharedState>,
    title: String,
    url: String,
    tags: Vec<String>,
    note: Option<String>,
) -> Result<Bookmark, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
    active
        .bookmarks
        .add(&title, &url, None, &tag_refs, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn search_bookmarks(state: tauri::State<'_, SharedState>, query: String) -> Result<Vec<Bookmark>, String> {
    let guard = state.lock().unwrap();
    guard.active_or_err()?.bookmarks.search(&query).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// History (§10A.3 — respects each profile's RecordingPolicy)
// ---------------------------------------------------------------------
