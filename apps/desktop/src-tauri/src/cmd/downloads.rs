// Made by MrDuck
#![allow(unused_imports)]

use std::sync::Mutex;
use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

use crate::shell::on_main_thread;
#[derive(Clone, serde::Serialize)]
pub(crate) struct DownloadItem {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) file_name: String,
    pub(crate) path: String,
    pub(crate) status: String, // "downloading" | "done" | "failed"
}

pub(crate) fn unique_path(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let mut p = dir.join(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let ext = p
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    let mut n = 1u32;
    while p.exists() {
        n += 1;
        p = dir.join(format!("{stem}-{n}{ext}"));
    }
    p
}

/// Build a `Rect` from logical coordinates.

#[derive(Default)]
pub struct DownloadsLog(pub Mutex<Vec<DownloadItem>>);

#[tauri::command]
pub(crate) fn downloads_list(log: tauri::State<'_, DownloadsLog>) -> Result<Vec<DownloadItem>, String> {
    let guard = log.inner().0.lock().map_err(|e| e.to_string())?;
    Ok(guard.iter().rev().cloned().collect())
}

#[tauri::command]
pub(crate) fn downloads_dir(state: tauri::State<'_, SharedState>, app: AppHandle) -> Result<String, String> {
    let custom = download_dir_custom(&app)?;
    if let Some(d) = custom {
        return Ok(d);
    }
    let guard = state.lock().unwrap();
    let active = guard.active_or_err()?;
    let dir = guard.profiles.storage_root(active.profile.id).join("downloads");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().into_owned())
}

pub(crate) fn download_dir_custom(app: &AppHandle) -> Result<Option<String>, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let p = root.join("downloads-dir.txt");
    if !p.exists() {
        return Ok(None);
    }
    let s = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let s = s.trim().to_string();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

#[tauri::command]
pub(crate) fn dl_dir_get(app: AppHandle) -> Result<String, String> {
    Ok(download_dir_custom(&app)?.unwrap_or_default())
}

#[tauri::command]
pub(crate) fn dl_dir_set(app: AppHandle, path: String) -> Result<(), String> {
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let p = root.join("downloads-dir.txt");
    if path.trim().is_empty() {
        let _ = std::fs::remove_file(&p);
    } else {
        std::fs::write(&p, path.trim()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Save text to the user's Downloads folder (used for exports).
#[tauri::command]
pub(crate) fn save_text_file(name: String, contents: String) -> Result<String, String> {
    let downloads = std::env::var("USERPROFILE")
        .map(|h| std::path::PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let path = unique_path(&downloads, &name);
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Save binary (base64) to the user's Downloads folder — PNG-экспорт графа.
#[tauri::command]
pub(crate) fn save_image_file(
    app: AppHandle,
    name: String,
    data_base64: String,
) -> Result<String, String> {
    let _ = app;
    let bytes = crate::util::decode_base64(&data_base64).ok_or("неверный base64")?;
    let downloads = std::env::var("USERPROFILE")
        .map(|h| std::path::PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let path = unique_path(&downloads, &name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------
// Find-in-page — evaluate arbitrary JS inside a page tab (used to inject
// the self-contained find bar into the active webview).
// ---------------------------------------------------------------------

// Made by MrDuck