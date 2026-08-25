// Made by MrDuck && Ox-Alpha
// GUI-приложение без консольного окна (пустая консоль рядом с браузером
// сбивала с толку). Диагностика всё равно пишется в apb/logs/shell-debug.log.
#![windows_subsystem = "windows"]

//! APB desktop shell — Tauri backend (entry point).
//!
//! The backend is split into domain modules:
//!   * `state.rs`   — shared `AppState` / `ActiveProfile` / `SharedState`
//!   * `shell.rs`   — window-level layout machinery (`PageTabs`, `relayout`,
//!     the `#browserView` measuring contract, main-thread helper)
//!   * `util.rs`    — percent/base64 codecs, image magic-byte sniffing
//!   * `cmd/*`      — one module per feature domain, mirroring the frontend
//!     modules under `ui/js/` (mapping table in `cmd/mod.rs`)
//!
//! Threading rule (critical on Windows): any command that touches webviews
//! or blocks (network I/O, Argon2) must be `async`, so its body runs on a
//! worker thread; webview work is marshaled back via `shell::on_main_thread`.

mod cmd;
mod liveprivacy;
mod proxy;
mod shell;
mod state;
mod util;

use cmd::downloads::DownloadsLog;
use cmd::*;
use shell::{relayout, PageTabs};
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    // Privacy engine data plane must exist BEFORE the first webview: the
    // local filtering proxy port goes into the shared browser arguments.
    let filter = Arc::new(liveprivacy::LiveFilter::default());
    let proxy_port = proxy::spawn(filter.clone());
    liveprivacy::init_browser_args(proxy_port);

    tauri::Builder::default()
        .manage(filter)
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("./apb-data"));
            let bootstrapped = AppState::bootstrap(data_dir).map_err(|e: String| -> Box<dyn std::error::Error> { e.into() })?;
            app.manage(Mutex::new(bootstrapped));
            app.manage(PageTabs::default());
            app.manage(DownloadsLog::default());
            liveprivacy::sync_from_state(app.handle())?;

            // The shell window is created here (not from tauri.conf.json) so
            // it shares the exact same browser arguments as every tab —
            // WebView2 requires identical environment options per user-data
            // folder, and the tabs get the proxy flag.
            tauri::WebviewWindowBuilder::new(app, "shell", tauri::WebviewUrl::App("index.html".into()))
                .title("APB — ApostolProject Browser")
                .inner_size(1280.0, 820.0)
                .min_inner_size(900.0, 560.0)
                .resizable(true)
                .decorations(false)
                .additional_browser_args(liveprivacy::browser_args())
                .build()
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "shell" {
                if matches!(event, tauri::WindowEvent::Resized(_)) {
                    relayout(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            create_profile,
            create_anonymous_profile,
            switch_profile,
            rename_profile,
            active_profile,
            delete_profile,
            add_bookmark,
            search_bookmarks,
            record_visit,
            recent_history,
            clear_history,
            list_notes,
            create_note,
            read_note,
            backlinks,
            save_note_image,
            read_note_asset,
            search_commands,
            get_privacy_overview,
            set_privacy_level,
            update_privacy_policy,
            set_emergency_mode,
            add_blocklist,
            panic_button,
            get_network_settings,
            save_network_settings,
            route_preview,
            run_network_diagnostics,
            privacy_stats,
            privacy_reset_stats,
            vault_status,
            vault_create,
            vault_unlock,
            vault_lock,
            vault_add_entry,
            vault_list,
            vault_reveal,
            vault_generate_password,
            ai_get_config,
            ai_save_config,
            ai_chat,
            history_all_profiles,
            notes_graph,
            save_graph_positions,
            save_board_items,
            note_delete,
            notes_reindex,
            session_get,
            session_save,
            downloads_list,
            downloads_dir,
            dl_dir_get,
            dl_dir_set,
            save_text_file,
            save_image_file,
            debug_log_append,
            page_eval,
            shell_hotkey,
            workspaces_get,
            workspaces_set,
            page_extract_text,
            ext_install,
            ext_list,
            ext_grant,
            ext_approve_dangerous,
            ext_set_enabled,
            ext_sandbox_policy,
            page_open,
            page_navigate,
            page_activate,
            page_hide_all,
            page_close,
            page_relayout,
            page_split_set,
            page_split_off,
            open_in_system,
            app_version,
            update_check,
            update_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running APB");
}

// Made by MrDuck && Ox-Alpha