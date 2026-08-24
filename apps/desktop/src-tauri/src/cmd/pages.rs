#![allow(unused_imports)]

use crate::shell::PageTab;
use tauri::{WebviewBuilder, WebviewUrl};
use crate::state::{AppState, SharedState};
use tauri::{AppHandle, Manager, State};

use tauri::{Emitter, LogicalPosition, LogicalSize, Position, Rect, Window};
use crate::shell::{content_rect, relayout, on_main_thread, page_rect, PageTabs, HIDDEN_RECT};
use crate::cmd::downloads::{download_dir_custom, unique_path, DownloadItem, DownloadsLog};
use crate::cmd::history::invoke_record_visit;
#[tauri::command]
pub(crate) async fn page_eval(app: AppHandle, id: String, js: String) -> Result<(), String> {
    let label = {
        let tabs = app.state::<PageTabs>();
        let guard = tabs.tabs.lock().unwrap();
        guard
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.label.clone())
            .ok_or_else(|| "вкладка не найдена".to_string())?
    };
    let app_for_main = app.clone();
    on_main_thread(&app, move || {
        let wv = app_for_main
            .get_webview(&label)
            .ok_or_else(|| "вкладка не найдена".to_string())?;
        wv.eval(&js).map_err(|e| e.to_string())
    })?
}

// Релей горячих клавиш из сайтов-вкладок: пока фокус в нативном вебвью,
// шелловые keydown не срабатывают. Этот скрипт инжектится В КАЖДУЮ вкладку
// (initialization_script — переживает навигации) и пересылает Ctrl+K/Ctrl+T
// в окно оболочки через shell_hotkey.
const HOTKEY_RELAY_JS: &str = r#"(function(){
  if (window.__apbHotkeyRelay) return; window.__apbHotkeyRelay = true;
  function send(k){
    try {
      var t = window.__TAURI__ && window.__TAURI__.core;
      if (t && t.invoke) t.invoke("shell_hotkey", { key: k });
    } catch(e){}
  }
  window.addEventListener("keydown", function(e){
    if (!(e.ctrlKey || e.metaKey)) return;
    var code = e.code || "";
    var k = (e.key || "").toLowerCase();
    if (code === "KeyK" || code === "KeyT" || code === "KeyF" ||
        k === "k" || k === "t" || k === "f") {
      e.preventDefault(); e.stopPropagation();
      var out = code === "KeyT" || k === "t" ? "t"
              : code === "KeyF" || k === "f" ? "f" : "k";
      send(out);
    }
  }, true);
})();"#;

#[tauri::command]
pub(crate) async fn shell_hotkey(app: AppHandle, key: String) -> Result<(), String> {
    let ch = key
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase())
        .unwrap_or(' ');
    if !ch.is_ascii_lowercase() {
        return Err("bad hotkey".into());
    }
    // Синтетическое событие в шелле: существующие обработчики (палитра,
    // новая вкладка) отработают сами.
    let js = format!(
        "document.dispatchEvent(new KeyboardEvent('keydown',{{key:'{}',ctrlKey:true,bubbles:true,cancelable:true}}));",
        ch
    );
    let app_for_main = app.clone();
    on_main_thread(&app, move || {
        let wv = app_for_main
            .get_webview("shell")
            .ok_or_else(|| "нет окна оболочки".to_string())?;
        wv.eval(&js).map_err(|e| e.to_string())
    })?
}

// ---------------------------------------------------------------------
// Workspaces — named groups of tabs per profile (workspaces.json).
// The frontend orchestrates switching; the backend only stores the doc.
// ---------------------------------------------------------------------

#[tauri::command]

pub(crate) fn page_extract_text(url: String) -> Result<serde_json::Value, String> {
    let parsed: tauri::Url = url.parse().map_err(|_| format!("неверный URL: {url}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("поддерживаются только http(s)-страницы".into());
    }
    let body = ureq::get(url.as_str())
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) APB-browser",
        )
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let body = if body.len() > 400_000 { body[..400_000].to_string() } else { body };

    // Title
    let title = body
        .find("<title")
        .and_then(|i| body[i..].find('>').map(|j| i + j + 1))
        .and_then(|s| body[s..].find("</title>").map(|e| body[s..s + e].to_string()))
        .unwrap_or_default();

    // Crude tag strip: remove scripts/styles/tags, decode few entities
    let mut txt = body.to_string();
    for tag in ["script", "style", "noscript", "svg", "head"] {
        let open = format!("<{tag}");
        while let Some(i) = txt.to_lowercase().find(&open) {
            match txt[i..].to_lowercase().find(&format!("</{tag}>")) {
                Some(e) => {
                    let end = i + e + tag.len() + 3;
                    txt.replace_range(i..end.min(txt.len()), " ");
                }
                None => break,
            }
        }
    }
    let mut out = String::with_capacity(txt.len());
    let mut in_tag = false;
    for ch in txt.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let clipped: String = collapsed.chars().take(8000).collect();
    Ok(serde_json::json!({ "title": title.trim(), "text": clipped }))
}

#[tauri::command]

pub(crate) async fn page_open(app: AppHandle, url: String) -> Result<String, String> {
    let parsed: tauri::Url = url.parse().map_err(|_| format!("неверный URL: {url}"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let label = format!("page-{id}");

    // Hide all current tabs before adding the new one.
    {
        let tabs = app.state::<PageTabs>();
        for t in tabs.tabs.lock().unwrap().iter_mut() {
            t.visible = false;
        }
    }
    relayout(&app);

    let measured = {
        let tabs = app.state::<PageTabs>();
        let rect = *tabs.measured_rect.lock().unwrap();
        rect
    };
    let app_for_main = app.clone();
    let label_for_main = label.clone();
    on_main_thread(&app, move || -> Result<(), String> {
        let window = app_for_main.get_window("shell").ok_or_else(|| "нет окна оболочки".to_string())?;
        let (x, y, width, height) =
            measured.unwrap_or_else(|| content_rect(&window, false));
        let builder = WebviewBuilder::new(&label_for_main, WebviewUrl::External(parsed))
            .initialization_script(HOTKEY_RELAY_JS)
            .on_download(|webview, event| {
                let handle = webview.app_handle();
                match event {
                    tauri::webview::DownloadEvent::Requested { url, destination } => {
                        // Default: keep WebView2's suggestion (the OS Downloads
                        // folder). If the user picked a custom dir, use it.
                        let custom = handle
                            .try_state::<SharedState>()
                            .and_then(|_| handle.path().app_data_dir().ok())
                            .map(|root| root.join("downloads-dir.txt"))
                            .filter(|p| p.exists())
                            .and_then(|p| std::fs::read_to_string(p).ok())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty());
                        let final_path = if let Some(dir) = custom {
                            let dir = std::path::PathBuf::from(&dir);
                            let _ = std::fs::create_dir_all(&dir);
                            let name = destination
                                .file_name()
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_else(|| format!("file-{}", chrono::Utc::now().timestamp_millis()));
                            unique_path(&dir, &name)
                        } else {
                            destination.clone()
                        };
                        *destination = final_path.clone();
                        let item = DownloadItem {
                            id: uuid::Uuid::new_v4().to_string(),
                            url: url.to_string(),
                            file_name: final_path
                                .file_name()
                                .map(|s| s.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            path: final_path.to_string_lossy().into_owned(),
                            status: "downloading".into(),
                        };
                        if let Ok(mut log) = handle.state::<DownloadsLog>().inner().0.lock() {
                            log.push(item.clone());
                        }
                        let _ = handle.emit("dl-update", item);
                        true
                    }
                    tauri::webview::DownloadEvent::Finished { url, path, success } => {
                        let status = if success { "done" } else { "failed" };
                        let mut updated: Option<DownloadItem> = None;
                        if let Ok(mut log) = handle.state::<DownloadsLog>().inner().0.lock() {
                            let pos = log.iter().rposition(|d| {
                                d.status == "downloading"
                                    && (path.as_ref().map(|p| p.to_string_lossy() == d.path).unwrap_or(false)
                                        || d.url == url.to_string())
                            });
                            if let Some(i) = pos {
                                log[i].status = status.into();
                                updated = Some(log[i].clone());
                            }
                        }
                        if let Some(item) = updated {
                            let _ = handle.emit("dl-update", item);
                        }
                        true
                    }
                    _ => true,
                }
            });
        window
            .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        Ok(())
    })??;

    {
        let tabs = app.state::<PageTabs>();
        let mut guard = tabs.tabs.lock().unwrap();
        guard.push(PageTab { id: id.clone(), label: label.clone(), url: url.clone(), visible: true });
    }

    invoke_record_visit(&app, &url);
    Ok(id)
}

#[tauri::command]
pub(crate) async fn page_navigate(app: AppHandle, id: String, url: String) -> Result<(), String> {
    let parsed: tauri::Url = url.parse().map_err(|_| format!("неверный URL: {url}"))?;
    let label = {
        let tabs = app.state::<PageTabs>();
        let guard = tabs.tabs.lock().unwrap();
        guard
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.label.clone())
            .ok_or_else(|| "вкладка не найдена".to_string())?
    };

    let app_for_main = app.clone();
    let label_for_main = label.clone();
    let script = format!("location.replace({:?})", parsed.as_str());
    on_main_thread(&app, move || -> Result<(), String> {
        let webview = app_for_main.get_webview(&label_for_main).ok_or_else(|| "вкладка не найдена".to_string())?;
        webview.eval(&script).map_err(|e| e.to_string())
    })??;

    let tabs = app.state::<PageTabs>();
    let mut g = tabs.tabs.lock().unwrap();
    if let Some(t) = g.iter_mut().find(|t| t.id == id) {
        t.url = url.clone();
    }
    drop(g);
    invoke_record_visit(&app, &url);
    Ok(())
}

#[tauri::command]
pub(crate) async fn page_activate(app: AppHandle, id: String) -> Result<(), String> {
    let tabs = app.state::<PageTabs>();
    {
        let mut guard = tabs.tabs.lock().unwrap();
        for t in guard.iter_mut() {
            t.visible = t.id == id;
        }
    }
    relayout(&app);
    Ok(())
}

// ---------------------------------------------------------------------
// Split view — две живые вкладки рядом (50/50)
// ---------------------------------------------------------------------

/// Включить разделённый экран: left_id — левая половина, right_id — правая.
#[tauri::command]
pub(crate) async fn page_split_set(
    app: AppHandle,
    left_id: String,
    right_id: String,
) -> Result<(), String> {
    if left_id == right_id {
        return Err("нужны две разные вкладки".into());
    }
    let tabs = app.state::<PageTabs>();
    {
        let guard = tabs.tabs.lock().unwrap();
        for id in [&left_id, &right_id] {
            if !guard.iter().any(|t| &t.id == id) {
                return Err(format!("вкладка не найдена: {id}"));
            }
        }
    }
    *tabs.split.lock().unwrap() = Some((left_id, right_id));
    relayout(&app);
    Ok(())
}

/// Выключить разделённый экран.
#[tauri::command]
pub(crate) async fn page_split_off(app: AppHandle) -> Result<(), String> {
    let tabs = app.state::<PageTabs>();
    *tabs.split.lock().unwrap() = None;
    relayout(&app);
    Ok(())
}

/// Hide every page webview — used when an internal page (settings, vault,
/// extensions) or the home screen takes over the content area.
#[tauri::command]
pub(crate) async fn page_hide_all(app: AppHandle) -> Result<(), String> {
    {
        let tabs = app.state::<PageTabs>();
        let mut guard = tabs.tabs.lock().unwrap();
        for t in guard.iter_mut() {
            t.visible = false;
        }
    }
    relayout(&app);
    Ok(())
}

#[tauri::command]
pub(crate) async fn page_close(app: AppHandle, id: String) -> Result<bool, String> {
    let tabs = app.state::<PageTabs>();
    // Если закрыли члена сплита — сплит распадается, партнёр остаётся видимым
    {
        let mut split = tabs.split.lock().unwrap();
        if let Some((l, r)) = split.as_ref() {
            if *l == id || *r == id {
                let other = if *l == id { r.clone() } else { l.clone() };
                *split = None;
                if let Some(t) = tabs.tabs.lock().unwrap().iter_mut().find(|t| t.id == other) {
                    t.visible = true;
                }
            }
        }
    }
    let removed = {
        let mut guard = tabs.tabs.lock().unwrap();
        let pos = guard.iter().position(|t| t.id == id);
        match pos {
            Some(i) => guard.remove(i),
            None => return Ok(false),
        }
    };

    let was_visible = removed.visible;
    let removed_label = removed.label;
    let app_for_main = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(webview) = app_for_main.get_webview(&removed_label) {
            // Detach by moving offscreen; destroying via the parent window
            // isn't exposed — hide instead, cheap and reliable everywhere.
            let _ = webview.set_bounds(page_rect(-60000.0, -60000.0, 1.0, 1.0));
        }
    });

    // Only steal focus if we actually closed the tab that was visible —
    // closing a background tab must never jump you away from what you're
    // looking at.
    if was_visible {
        let next = {
            let guard = tabs.tabs.lock().unwrap();
            guard.last().map(|t| t.id.clone())
        };
        if let Some(next_id) = next {
            let mut guard = tabs.tabs.lock().unwrap();
            for t in guard.iter_mut() {
                t.visible = t.id == next_id;
            }
        }
    }
    relayout(&app);
    Ok(true)
}

#[tauri::command]
pub(crate) async fn page_relayout(
    app: AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let tabs = app.state::<PageTabs>();
    *tabs.measured_rect.lock().unwrap() =
        Some((x, y, width.max(50.0), height.max(50.0)));
    drop(tabs);
    relayout(&app);
    Ok(())
}

#[tauri::command]
pub(crate) fn open_in_system(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(&url).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&url).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}
