// Made by MrDuck && Ox-Alpha
#![allow(unused_imports)]

use apb_profiles::Profile;
use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(crate) fn list_profiles(state: tauri::State<'_, SharedState>) -> Result<Vec<Profile>, String> {
    state.lock().unwrap().profiles.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_profile(state: tauri::State<'_, SharedState>, name: String) -> Result<Profile, String> {
    state.lock().unwrap().profiles.create(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_anonymous_profile(state: tauri::State<'_, SharedState>) -> Result<Profile, String> {
    state
        .lock()
        .unwrap()
        .profiles
        .create_anonymous()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn switch_profile(state: tauri::State<'_, SharedState>, id: String) -> Result<Profile, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut guard = state.lock().unwrap();
    guard.activate(uuid)?;
    Ok(guard.active_or_err()?.profile.clone())
}

#[tauri::command]
pub(crate) fn active_profile(state: tauri::State<'_, SharedState>) -> Result<Profile, String> {
    Ok(state.lock().unwrap().active_or_err()?.profile.clone())
}

/// Удалить профиль вместе со всеми его данными (storage root стирается
/// целиком в ProfileManager::delete). Активный профиль удалить нельзя —
/// сначала переключиться на другой.
#[tauri::command]
pub(crate) fn delete_profile(state: tauri::State<'_, SharedState>, id: String) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut guard = state.lock().unwrap();
    if let Some(active) = guard.active.as_ref() {
        if active.profile.id == uuid {
            return Err("нельзя удалить активный профиль — сначала переключитесь на другой".into());
        }
    }
    if guard.profiles.list().map_err(|e| e.to_string())?.len() <= 1 {
        return Err("нельзя удалить последний профиль".into());
    }
    guard.profiles.delete(uuid).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// Bookmarks (§7)
// ---------------------------------------------------------------------

// Made by MrDuck && Ox-Alpha