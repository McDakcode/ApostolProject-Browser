#![allow(unused_imports)]

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Position, Rect, Size, Window};
use std::sync::Mutex;
use crate::cmd::downloads::{DownloadItem, DownloadsLog};

pub(crate) fn page_rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        position: Position::Logical(LogicalPosition::new(x, y)),
        size: Size::Logical(LogicalSize::new(width, height)),
    }
}

pub(crate) const HIDDEN_RECT: Rect = Rect {
    position: tauri::Position::Logical(tauri::LogicalPosition::new(-30000.0, -30000.0)),
    size: tauri::Size::Logical(tauri::LogicalSize::new(1.0, 1.0)),
};

/// Run `f` on the main thread and block the calling thread until it
/// finishes, returning whatever `f` produced. Required for any code that
/// touches a `Webview` (create/eval/set_bounds) — see the module doc for
/// why. Call this only from `async` commands (or plain threads): those run
/// off the main thread, so blocking here is safe; from the main thread it
/// would execute inline or deadlock.
pub(crate) fn on_main_thread<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|_| "главный поток не ответил".to_string())
}

// ---------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------


const HEADER_H: f64 = 88.0;
const RAIL_W: f64 = 60.0;
const MARGIN: f64 = 4.0;
const EDITOR_MIN_W: f64 = 380.0;
const EDITOR_FRACTION: f64 = 0.46;

pub(crate) struct PageTab {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) url: String,
    pub(crate) visible: bool,
}

#[derive(Default)]
pub(crate) struct PageTabs {
    pub(crate) tabs: Mutex<Vec<PageTab>>,
    /// Content-area rectangle (logical px, client-relative) as measured by
    /// the shell DOM (`#browserView`). Authoritative when present — it
    /// automatically accounts for the tabstrip/toolbar heights, the rail,
    /// the side panel and the note editor pane.
    pub(crate) measured_rect: Mutex<Option<(f64, f64, f64, f64)>>,
    /// Split view: пара ID вкладок (левая половина, правая половина).
    /// Пока задана — relayout делит content-area пополам между ними,
    /// все остальные вкладки прячутся независимо от visible.
    pub(crate) split: Mutex<Option<(String, String)>>,
}

/// Content-area rectangle (logical px, relative to the window client area)
/// where the active page webview lives. Shrinks when the note editor pane
/// is open on the right.
pub(crate) fn content_rect(window: &Window, editor_open: bool) -> (f64, f64, f64, f64) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.inner_size().unwrap_or(tauri::PhysicalSize::new(1280, 800));
    let w = size.width as f64 / scale;
    let h = size.height as f64 / scale;
    let editor_w = if editor_open {
        (w * EDITOR_FRACTION).max(EDITOR_MIN_W)
    } else {
        0.0
    };
    (
        RAIL_W + MARGIN,
        HEADER_H + MARGIN,
        (w - RAIL_W - editor_w - MARGIN * 2.0).max(200.0),
        (h - HEADER_H - MARGIN * 2.0).max(150.0),
    )
}

pub(crate) fn relayout(app: &AppHandle) {
    let tabs = app.state::<PageTabs>();
    let measured = *tabs.measured_rect.lock().unwrap();
    // Snapshot first: MutexGuard isn't Send, so it can't cross the
    // main-thread hop below, and we don't want to hold the lock while
    // touching webviews anyway.
    let snapshot: Vec<(String, bool)> = tabs
        .tabs
        .lock()
        .unwrap()
        .iter()
        .map(|t| (t.label.clone(), t.visible))
        .collect();
    let split: Option<(String, String)> = {
        let s = tabs.split.lock().unwrap().clone();
        match s {
            Some((l, r)) => {
                let g = tabs.tabs.lock().unwrap();
                let label_of = |id: &str| {
                    g.iter()
                        .find(|t| t.id == id)
                        .map(|t| t.label.clone())
                };
                match (label_of(&l), label_of(&r)) {
                    (Some(ll), Some(rl)) => Some((ll, rl)),
                    _ => None, // один из членов закрыт — сплит больше не валиден
                }
            }
            None => None,
        }
    };

    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = app2.get_window("shell") else { return };
        // Базовый прямоугольник контентной области — общий для всех режимов
        let base = measured
            .map(|(x, y, w, h)| page_rect(x, y, w, h))
            .unwrap_or_else(|| {
                let (x, y, w, h) = content_rect(&window, false);
                page_rect(x, y, w, h)
            });

        // Половины для split view (левая/правая)
        let halves = |rect: Rect| -> (Rect, Rect) {
            if let Position::Logical(pos) = rect.position {
                if let Size::Logical(size) = rect.size {
                    let half_w = size.width / 2.0;
                    return (
                        page_rect(pos.x, pos.y, half_w, size.height),
                        page_rect(pos.x + half_w, pos.y, size.width - half_w, size.height),
                    );
                }
            }
            (rect, rect)
        };

        for (label, visible) in snapshot {
            if let Some(webview) = app2.get_webview(&label) {
                let rect = match &split {
                    Some((left_label, right_label)) => {
                        if &label == left_label {
                            halves(base).0
                        } else if &label == right_label {
                            halves(base).1
                        } else {
                            HIDDEN_RECT
                        }
                    }
                    None => {
                        if visible {
                            base
                        } else {
                            HIDDEN_RECT
                        }
                    }
                };
                let _ = webview.set_bounds(rect);
            }
        }
    });
}
