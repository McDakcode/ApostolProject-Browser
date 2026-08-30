// Made by MrDuck
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

// Made by MrDuck
// ---------------------------------------------------------------------------
// Smooth manual window dragging (Windows)
//
// The default tao/start_dragging path enters the modal SC_MOVE loop, which
// starves the renderer: on a frameless window with WebView2 the content
// trails the cursor and stutters — very visible on high-refresh monitors.
// Instead we track the cursor in a tight dedicated thread and reposition the
// window ourselves (~500 Hz updates), so motion is as smooth as the display.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicBool, Ordering};

static DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub(crate) fn shell_begin_drag(app: AppHandle) -> Result<(), String> {
    if DRAG_ACTIVE.load(Ordering::SeqCst) {
        return Ok(());
    }
    let window = app.get_window("shell").ok_or_else(|| "нет окна оболочки".to_string())?;
    let hwnd_isize = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    if DRAG_ACTIVE.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let started = std::thread::Builder::new()
        .name("apb-drag".into())
        .spawn(move || {
            // На время перетаскивания анимации DWM прочь — иначе лаг.
            set_dwm_transitions_raw(hwnd_isize, false);
            #[cfg(windows)]
            unsafe { drag_loop(hwnd_isize) }
            set_dwm_transitions_raw(hwnd_isize, true);
            DRAG_ACTIVE.store(false, Ordering::SeqCst);
        });
    if started.is_err() {
        DRAG_ACTIVE.store(false, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(windows)]
unsafe fn drag_loop(hwnd_raw: isize) {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::System::Threading::Sleep;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetWindowRect, IsZoomed, SetWindowPos,
        ShowWindow, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SW_MAXIMIZE, SW_RESTORE,
    };
    const VK_LBUTTON: i32 = 0x01;
    // Курсор физически упирается в границу монитора (ОС сама его туда
    // клэмпит), поэтому пары пикселей допуска достаточно, чтобы поймать
    // жест «дотащили до края».
    const SNAP_EDGE_PX: i32 = 3;
    let hwnd = hwnd_raw as *mut core::ffi::c_void;

    let mut pt = POINT { x: 0, y: 0 };
    if GetCursorPos(&mut pt) == 0 {
        return;
    }
    let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    GetWindowRect(hwnd, &mut rc);
    let mut off_x = pt.x - rc.left;
    let off_y_base = pt.y - rc.top;

    // Dragging a maximized window: restore it first and keep the grab point
    // proportionally under the cursor (like native titlebar behaviour).
    if IsZoomed(hwnd) != 0 {
        let total_w = ((rc.right - rc.left) as f32).max(1.0);
        let rel = (((pt.x - rc.left) as f32) / total_w).clamp(0.08, 0.92);
        ShowWindow(hwnd, SW_RESTORE);
        Sleep(30);
        GetWindowRect(hwnd, &mut rc);
        let w = rc.right - rc.left;
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            pt.x - (rel * w as f32) as i32,
            pt.y - off_y_base.max(14),
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        GetWindowRect(hwnd, &mut rc);
        off_x = pt.x - rc.left;
    }

    let off_y = pt.y - rc.top;
    let _ = off_y_base;
    loop {
        Sleep(2);
        if GetAsyncKeyState(VK_LBUTTON) >= 0 {
            break; // button released
        }
        if GetCursorPos(&mut pt) == 0 {
            break;
        }
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            pt.x - off_x,
            pt.y - off_y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    // --- Aero Snap на отпускании ---
    // Наш ручной цикл заменяет системный SC_MOVE, который обычно сам ловит
    // подтаскивание к краю экрана — досюда добавляем то же поведение
    // вручную: у верхнего края монитора разворачиваем (через ShowWindow,
    // чтобы IsZoomed()-ветка выше корректно восстанавливала окно при
    // следующем перетаскивании), у левого/правого — прилепляем к половине
    // рабочей области (с учётом панели задач).
    if GetCursorPos(&mut pt) != 0 {
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) != 0 {
            let mon = mi.rcMonitor;
            let work = mi.rcWork;
            if pt.y <= mon.top + SNAP_EDGE_PX {
                ShowWindow(hwnd, SW_MAXIMIZE);
            } else if pt.x <= mon.left + SNAP_EDGE_PX {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    work.left,
                    work.top,
                    (work.right - work.left) / 2,
                    work.bottom - work.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            } else if pt.x >= mon.right - SNAP_EDGE_PX {
                let half_w = (work.right - work.left) / 2;
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    work.right - half_w,
                    work.top,
                    half_w,
                    work.bottom - work.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}

/// DWM-анимации сворачивания/разворачивания/открытия окна. Включены всегда —
/// гасятся только на время ручного перетаскивания (см. shell_begin_drag),
/// иначе каждое перемещение анимируется и окно «отстаёт» от курсора.
#[cfg(windows)]
fn set_dwm_transitions_raw(hwnd_raw: isize, enabled: bool) {
    unsafe {
        use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
        let disabled: i32 = if enabled { 0 } else { 1 };
        DwmSetWindowAttribute(
            hwnd_raw as *mut core::ffi::c_void,
            3, // DWMWA_TRANSITIONS_FORCEDISABLED
            &disabled as *const i32 as *const core::ffi::c_void,
            4,
        );
    }
}

#[cfg(not(windows))]
fn set_dwm_transitions_raw(_hwnd_raw: isize, _enabled: bool) {}

// Made by MrDuck
