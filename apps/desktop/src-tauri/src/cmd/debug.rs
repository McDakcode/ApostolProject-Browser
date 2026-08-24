// Made by MrDuck && Ox-Alpha
//! Debug logging for the demo/debug build.
//!
//! The frontend buffers diagnostics (JS errors, unhandled rejections,
//! failed invokes, graph lifecycle events) and flushes them here.
//!
//! КУДА ПИШЕМ (в порядке приоритета):
//!   1. `%APB_LOG_DIR%`, если переменная задана
//!   2. `<корень проекта>/logs/` — ищем вверх от exe каталог с Cargo.toml
//!      И подпапкой apps/ (это workspace-корень apb/) → логи лежат рядом
//!      с проектом: G:\APB AI\apb\logs\shell-debug.log
//!   3. фолбэк — AppData
//!
//! Rotated at ~2 MB → shell-debug.log.old. Strip this whole module before
//! any public/release build.

use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;

const MAX_BYTES: u64 = 2_000_000;

fn logs_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(v) = std::env::var("APB_LOG_DIR") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        for anc in exe.ancestors().skip(1) {
            // именно workspace-корень: и манифест, и структура apps/
            if anc.join("Cargo.toml").is_file() && anc.join("apps").is_dir() {
                return anc.join("logs");
            }
        }
    }
    app.path()
        .app_data_dir()
        .map(|p| p.join("logs"))
        .unwrap_or_else(|_| PathBuf::from("logs"))
}

#[tauri::command]
pub(crate) fn debug_log_append(
    app: tauri::AppHandle,
    lines: Vec<String>,
) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    let dir = logs_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("shell-debug.log");

    // Simple size-based rotation: keep exactly one previous generation.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_BYTES {
            let _ = std::fs::rename(&path, dir.join("shell-debug.log.old"));
        }
    }

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    for line in lines {
        let _ = writeln!(f, "{line}");
    }
    Ok(())
}


// Made by MrDuck && Ox-Alpha