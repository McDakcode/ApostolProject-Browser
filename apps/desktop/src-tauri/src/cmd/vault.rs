#![allow(unused_imports)]

use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

use apb_vault::{EntryKind, GeneratorOptions};
use crate::util::decode_base64;
#[tauri::command]
pub(crate) fn vault_status(state: tauri::State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    Ok(serde_json::json!({
        "created": active.vault_path.exists(),
        "unlocked": active.vault.is_some(),
    }))
}

#[tauri::command]
pub(crate) async fn vault_create(
    state: tauri::State<'_, SharedState>,
    passphrase: String,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    if active.vault_path.exists() {
        return Err("сейф уже создан".into());
    }
    if passphrase.chars().count() < 8 {
        return Err("парольная фраза слишком короткая (минимум 8 символов)".into());
    }
    active.vault = Some(apb_vault::Vault::create(&active.vault_path, &passphrase).map_err(|e| e.to_string())?);
    Ok(())
}

#[tauri::command]
pub(crate) async fn vault_unlock(
    state: tauri::State<'_, SharedState>,
    passphrase: String,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    if !active.vault_path.exists() {
        return Err("сейф не создан".into());
    }
    if active.vault.is_some() {
        return Ok(());
    }
    let file = apb_vault::Vault::open(&active.vault_path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "файл сейфа пуст".to_string())?;
    active.vault = Some(file.unlock(&passphrase).map_err(|e| e.to_string())?);
    Ok(())
}

#[tauri::command]
pub(crate) fn vault_lock(state: tauri::State<'_, SharedState>) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    if let Some(v) = active.vault.as_mut() {
        v.lock();
    }
    active.vault = None;
    Ok(())
}

#[tauri::command]
pub(crate) fn vault_add_entry(
    state: tauri::State<'_, SharedState>,
    kind: EntryKind,
) -> Result<String, String> {
    let mut guard = state.lock().unwrap();
    let active = guard.active_mut_or_err()?;
    let vault = active.vault.as_mut().ok_or_else(|| "сейф закрыт".to_string())?;
    let entry = vault.add_entry(kind).map_err(|e| e.to_string())?;
    Ok(entry.id.to_string())
}

#[tauri::command]
pub(crate) fn vault_list(
    state: tauri::State<'_, SharedState>,
) -> Result<Vec<(uuid::Uuid, String, String)>, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let vault = active.vault.as_ref().ok_or_else(|| "сейф закрыт".to_string())?;
    vault.list_summaries().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn vault_reveal(state: tauri::State<'_, SharedState>, id: String) -> Result<apb_vault::Entry, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let vault = active.vault.as_ref().ok_or_else(|| "сейф закрыт".to_string())?;
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    vault.reveal_entry(uuid).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn vault_generate_password(length: Option<u16>) -> Result<String, String> {
    let opts = GeneratorOptions {
        length: length.unwrap_or(20).clamp(8, 128) as usize,
        ..Default::default()
    };
    apb_vault::generate_password(opts).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// AI assistant (§14) — Privacy Firewall runs on every request
// ---------------------------------------------------------------------
