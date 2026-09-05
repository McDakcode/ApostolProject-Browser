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
    pub(crate) status: String, // "downloading" | "done" | "failed" | "cancelled"
    #[serde(default)]
    pub(crate) progress: i64, // -1 = неизвестно; 0..100 — повторная закачка
    #[serde(default)]
    pub(crate) recv: u64, // скачано байт (retry-закачка)
    #[serde(default)]
    pub(crate) total: u64, // всего байт (Content-Length; 0 = неизвестно)
    #[serde(default)]
    pub(crate) source: String, // label webview-вкладки, запустившей закачку
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

impl DownloadsLog {
    /// Лог загрузок хранится на диске (app_data/downloads-log.json):
    /// переживает перезапуск — иначе после каждого старта приложения
    /// история загрузок была бы пустой (грабля: лог жил только в памяти).
    pub fn load_from_disk(app: &AppHandle) -> Self {
        let mut items: Vec<DownloadItem> = Vec::new();
        if let Ok(root) = app.path().app_data_dir() {
            if let Ok(s) = std::fs::read_to_string(root.join("downloads-log.json")) {
                // Deserialize вручную (DownloadItem только Serialize):
                #[derive(serde::Deserialize)]
                struct Di {
                    id: String, url: String, file_name: String, path: String,
                    status: String,
                    #[serde(default)]
                    progress: i64,
                    #[serde(default)]
                    recv: u64,
                    #[serde(default)]
                    total: u64,
                    #[serde(default)]
                    source: String,
                }
                if let Ok(v) = serde_json::from_str::<Vec<Di>>(&s) {
                    items = v
                        .into_iter()
                        .map(|mut d| {
                            // Перезапуск убил все идущие закачки: строка
                            // помечается «interrupted» — файл частичный,
                            // юзер решает: ↻ продолжить (перезакачать).
                            if d.status == "downloading" {
                                d.status = "interrupted".into();
                            }
                            DownloadItem {
                                id: d.id, url: d.url, file_name: d.file_name,
                                path: d.path, status: d.status, progress: d.progress,
                                recv: d.recv, total: d.total, source: d.source,
                            }
                        })
                        .collect();
                }
            }
        }
        DownloadsLog(Mutex::new(items))
    }
    pub fn save_to_disk(&self, app: &AppHandle) {
        if let Ok(root) = app.path().app_data_dir() {
            if let Ok(g) = self.0.lock() {
                // последние 200 записей — файл не растёт бесконечно
                let tail: Vec<&DownloadItem> = if g.len() > 200 { g[g.len()-200..].iter().collect() } else { g.iter().collect() };
                if let Ok(json) = serde_json::to_string_pretty(&tail) {
                    let _ = std::fs::write(root.join("downloads-log.json"), json);
                }
            }
        }
    }
}

#[tauri::command]
pub(crate) fn download_cancel(
    app: AppHandle,
    id: String,
    path: String,
) -> Result<(), String> {
    // ✕ = отменить ТОЛЬКО идущую закачку, мгновенно. Собственная закачка
    // (ureq-движок — все новые): флаг в DlOwnRuns → поток обрывает
    // соединение и удаляет .apbpart в пределах следующего чанка (≤250 мс),
    // тут же удаляем .apbpart и сами. Браузерная (старые строки): пометить
    // путь, файл удалит Finished. Готовые файлы юзер удаляет сам из папки.
    let item: Option<DownloadItem>;
    {
        let log = app.state::<DownloadsLog>();
        let mut g = log.0.lock().map_err(|e| e.to_string())?;
        let Some(i) = g.iter().position(|d| d.id == id || d.path == path) else {
            return Ok(());
        };
        if g[i].status != "downloading" {
            return Ok(());
        }
        g[i].status = "cancelled".into();
        item = Some(g[i].clone());
    }
    if let Some(it) = &item {
        // флаг в движок: поток убьёт соединение и .apbpart в ≤250 мс;
        // .apbpart сносим и сразу — если поток ещё не создал его, создаст
        // заново и тут же увидит флаг.
        let runs = app.state::<DlOwnRuns>();
        if let Ok(mut c) = runs.cancel.lock() {
            c.insert(it.id.clone(), true);
        }
        let tmp = std::path::Path::new(&it.path).with_extension("apbpart");
        let _ = std::fs::remove_file(&tmp);
    }
    if let Some(item) = item {
        let _ = tauri::Emitter::emit(&app, "dl-update", item);
    }
    if let Some(log) = app.try_state::<DownloadsLog>() {
        log.save_to_disk(&app);
    }
    Ok(())
}

/// «✕ История»: очистить список загрузок (файлы на диске НЕ трогаем).
#[tauri::command]
pub(crate) fn downloads_clear(app: AppHandle) -> Result<(), String> {
    let log = app.state::<DownloadsLog>();
    {
        let mut g = log.0.lock().map_err(|e| e.to_string())?;
        g.clear();
    }
    log.save_to_disk(&app);
    Ok(())
}

/// Собственные закачки (ureq-потоки): cancel — флаги отмены по кнопке ✕
/// (true = убить поток), active — живые потоки (дедуп: двойной клик по ↻
/// не должен поднимать два потока на один файл). ГРАБЛЯ: не смешивать их
/// в одной карте — true «запущен» и true «отменить» в одном HashMap
/// мгновенно убивали каждую закачку на первом же чанке.
#[derive(Default)]
pub struct DlOwnRuns {
    pub cancel: std::sync::Mutex<std::collections::HashMap<String, bool>>,
    pub active: std::sync::Mutex<std::collections::HashSet<String>>,
}

/// Эмит dl-update (прогресс/статус).
fn emit_dl(app: &AppHandle, id: &str, url: &str, path: &str, status: &str, pct: i64, recv: u64, total: u64, source: &str) {
    let p = std::path::Path::new(path);
    let item = DownloadItem {
        id: id.to_string(),
        url: url.to_string(),
        file_name: p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path.to_string(),
        status: status.to_string(),
        progress: pct,
        recv,
        total,
        source: source.to_string(),
    };
    let _ = tauri::Emitter::emit(app, "dl-update", item);
}

/// Общий движок собственной закачки (ureq): главный поток скачивания APB.
/// WebView2/wry не умеет прерывать закачку в полёте — браузерный механизм
/// нельзя отменить по кнопке. Поэтому Requested-обработчик отдаёт WebView2
/// false (SetCancel — браузер вообще не начинает качать), а файл тянет
/// этот поток. Отмена проверяется каждый чанк: флаг в DlOwnRuns →
/// обрыв соединения, удаление .apbpart, статус «cancelled» мгновенно.
/// Кнопка ↻ (retry) использует тот же движок.
pub(crate) fn spawn_own_download(app: &AppHandle, id: String, url: String, path: String, source: String) {
    let app = app.clone();
    {
        let runs = app.state::<DlOwnRuns>();
        let guard = runs.active.lock();
        if let Ok(mut a) = guard {
            if a.contains(&id) {
                return; // уже качает
            }
            a.insert(id.clone());
        }
    }
    // строка обязана быть «downloading» в ЛОГЕ (не только в UI-эмите):
    // download_cancel проверяет статус по логу — иначе ✕ у retry-закачки
    // молча ничего не делал.
    if let Ok(mut log) = app.state::<DownloadsLog>().inner().0.lock() {
        if let Some(i) = log.iter().position(|d| d.id == id) {
            log[i].status = "downloading".into();
        }
    }
    std::thread::spawn(move || {
        // замыкания живут внутри потока: всё, что они заимствуют —
        // локали самого потока (урок из retry: снаружи = E0373/E0505)
        let cancelled = |app: &AppHandle| -> bool {
            app.state::<DlOwnRuns>()
                .cancel
                .lock()
                .map(|m| m.get(&id).copied().unwrap_or(false))
                .unwrap_or(false)
        };
        let fin = |status: &str, recv: u64, all: u64| {
            emit_dl(&app, &id, &url, &path, status, -1, recv, all, &source);
            if let Ok(mut a) = app.state::<DlOwnRuns>().active.lock() {
                a.remove(&id);
            }
            if let Ok(mut c) = app.state::<DlOwnRuns>().cancel.lock() {
                c.remove(&id);
            }
            if let Ok(mut log) = app.state::<DownloadsLog>().inner().0.lock() {
                if let Some(i) = log.iter().position(|d| d.id == id) {
                    log[i].status = status.into();
                    let _ = tauri::Emitter::emit(&app, "dl-update", log[i].clone());
                }
            }
            if let Some(dl_log) = app.try_state::<DownloadsLog>() {
                dl_log.save_to_disk(&app);
            }
        };
        let resp = match ureq::get(&url).timeout(std::time::Duration::from_secs(600)).call() {
            Ok(r) => r,
            Err(_) => { fin("failed", 0, 0); return; }
        };
        let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut done: u64 = 0;
        let mut reader = resp.into_reader();
        let tmp = std::path::Path::new(&path).with_extension("apbpart");
        let mut file = match std::fs::File::create(&tmp) {
            Ok(f) => f,
            Err(_) => { fin("failed", 0, total); return; }
        };
        use std::io::{Read, Write};
        let mut buf = [0u8; 65536];
        let mut last_emit = std::time::Instant::now();
        loop {
            if cancelled(&app) {
                let _ = std::fs::remove_file(&tmp);
                fin("cancelled", done, total);
                return;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if file.write_all(&buf[..n]).is_err() { let _ = std::fs::remove_file(&tmp); fin("failed", done, total); return; }
                    done += n as u64;
                    // прогресс не чаще 4 раз в секунду — UI не флудим
                    if last_emit.elapsed() >= std::time::Duration::from_millis(250) {
                        last_emit = std::time::Instant::now();
                        let pct = if total > 0 { (done * 100 / total) as i64 } else { -1 };
                        emit_dl(&app, &id, &url, &path, "downloading", pct, done, total, &source);
                    }
                }
                Err(_) => { let _ = std::fs::remove_file(&tmp); fin("failed", done, total); return; }
            }
        }
        if file.flush().is_err() { let _ = std::fs::remove_file(&tmp); fin("failed", done, total); return; }
        drop(file);
        if cancelled(&app) {
            // отмена прилетела между концом закачки и переименованием
            let _ = std::fs::remove_file(&tmp);
            fin("cancelled", done, total);
            return;
        }
        if std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
            fin("failed", done, total);
            return;
        }
        fin("done", done, total);
    });
}

/// Перезакачка файла по URL (кнопка «↻ повторить» / упавшая загрузка):
/// честный прогресс 0..100 в dl-update, статус done/failed в конце,
/// пишет в тот же путь через временный файл и переименовывает по готовности.
#[tauri::command]
pub(crate) fn download_retry(app: AppHandle, id: String, url: String, path: String) -> Result<(), String> {
    // у строки уже может быть source — сохраняем вкладку-источник
    let mut src = String::new();
    if let Ok(log) = app.state::<DownloadsLog>().inner().0.lock() {
        if let Some(d) = log.iter().find(|d| d.id == id || d.path == path) {
            src = d.source.clone();
        }
    }
    // сброс старого флага отмены: иначе ↻ после ✕ навсегда молчал бы
    {
        let runs = app.state::<DlOwnRuns>();
        let guard = runs.cancel.lock();
        if let Ok(mut c) = guard {
            c.remove(&id);
        }
    }
    emit_dl(&app, &id, &url, &path, "downloading", 0, 0, 0, &src);
    spawn_own_download(&app, id, url, path, src);
    Ok(())
}

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