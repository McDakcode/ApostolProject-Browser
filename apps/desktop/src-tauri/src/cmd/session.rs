// Made by MrDuck
#![allow(unused_imports)]

use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(crate) fn session_get(state: tauri::State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let path = guard.profiles.storage_root(active.profile.id).join("session.json");
    Ok(std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "tabs": [], "active": 0 })))
}

#[tauri::command]
pub(crate) fn session_save(
    state: tauri::State<'_, SharedState>,
    session: serde_json::Value,
) -> Result<(), String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let root = guard.profiles.storage_root(active.profile.id);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    std::fs::write(
        root.join("session.json"),
        serde_json::to_string(&session).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// Downloads commands
// ---------------------------------------------------------------------

// Made by MrDuck