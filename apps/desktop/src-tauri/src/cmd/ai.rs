// Made by MrDuck
#![allow(unused_imports)]

use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

use apb_ai::{build_context, AiClient, ChatMessage, ContextPermissions, ContextPiece, ContextSource, ProviderConfig, UreqTransport};
pub(crate) fn ai_config_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("ai.json")
}

pub(crate) fn load_ai_config(root: &std::path::Path) -> ProviderConfig {
    std::fs::read_to_string(ai_config_path(root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]

pub(crate) fn ai_get_config(state: tauri::State<'_, SharedState>) -> Result<ProviderConfig, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let root = guard.profiles.storage_root(active.profile.id);
    Ok(load_ai_config(&root))
}

#[tauri::command]
pub(crate) fn ai_save_config(
    state: tauri::State<'_, SharedState>,
    config: ProviderConfig,
) -> Result<(), String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let root = guard.profiles.storage_root(active.profile.id);
    std::fs::write(ai_config_path(&root), serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn ai_chat(
    state: tauri::State<'_, SharedState>,
    prompt: String,
    page_title: Option<String>,
    page_content: Option<String>,
) -> Result<apb_ai::ChatReport, String> {
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let profile_root = guard.profiles.storage_root(active.profile.id);
    let config = load_ai_config(&profile_root);

    let perms = ContextPermissions::default();
    let mut pieces: Vec<ContextPiece> = Vec::new();
    if let (Some(t), Some(c)) = (page_title, page_content) {
        pieces.push(ContextPiece { source: ContextSource::Page, title: t, body: c });
    }
    let (context, stripped) = build_context(&perms, &pieces);

    let client = AiClient::new(config, UreqTransport::new());
    let mut report = client
        .chat(&[ChatMessage::user(prompt)], &context)
        .map_err(|e| e.to_string())?;
    report.secrets_blocked += stripped;
    Ok(report)
}

// ---------------------------------------------------------------------
// Extensions (per-profile sandboxed installs, §10A.22-23)
// ---------------------------------------------------------------------

// Made by MrDuck