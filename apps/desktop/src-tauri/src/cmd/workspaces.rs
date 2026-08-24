// Made by MrDuck && Ox-Alpha
#![allow(unused_imports)]

use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(crate) fn workspaces_get(state: tauri::State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let path = guard.profiles.storage_root(active.profile.id).join("workspaces.json");
    Ok(std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "current": 0,
                "list": [ { "id": 0, "name": "Основное", "tabs": [], "active": 0 } ]
            })
        }))
}

/// Fetch a page server-side and return readable text for the AI context.
/// (WebView2 eval cannot return values across origins, so we fetch
/// ourselves; JS-rendered SPAs will yield limited text — honest boundary.)
#[tauri::command]

pub(crate) fn workspaces_set(state: tauri::State<'_, SharedState>, data: serde_json::Value) -> Result<(), String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let root = guard.profiles.storage_root(active.profile.id);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    std::fs::write(
        root.join("workspaces.json"),
        serde_json::to_string(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

// Made by MrDuck && Ox-Alpha