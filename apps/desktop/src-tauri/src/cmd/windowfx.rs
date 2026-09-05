// Made by MrDuck
//! Прозрачность/стекло окна браузера: юзер может сделать окно полностью
//! прозрачным с системным блюром (Acrylic/Blur — видно рабочий стол) или
//! отдельные слои UI — стеклом (CSS apb-glass). Окно обязано быть
//! transparent С МОМЕНТА СОЗДАНИЯ (Windows не даёт включить WS_EX_LAYERED
//! + effects на живом окне надёжно) — поэтому строим окно прозрачным
//! всегда, а «обычный» вид возвращаем командой: полностью непрозрачный
//! бэкграунд без эффектов.

use tauri::{AppHandle, Manager};

/// Режим прозрачности окна. "off" — обычное окно, "blur" — системный
/// блюр за прозрачным окном (Windows 10: Blur, 11: Acrylic; за окном
/// видно рабочий стол и другие окна), "clear" — просто прозрачное
/// (фон UI остаётся своим, сквозь пустоты видно рабочий стол).
#[tauri::command]
pub(crate) fn window_transparency(app: AppHandle, mode: String) -> Result<(), String> {
    let Some(win) = app.get_webview_window("shell") else {
        return Err("окно не найдено".into());
    };
    match mode.as_str() {
        "off" => {
            // вернуть обычный вид: движок красит своим цветом, эффекты прочь
            let _ = win.set_effects(None::<tauri::utils::config::WindowEffectsConfig>);
            let _ = win.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 255)));
        }
        "blur" => {
            // ГРАБЛЯ 36: окно transparent, но ДВИЖОК WebView2 красит свою
            // подложку сам (белым/чёрным) — системное стекло за ним не видно.
            // (0,0,0,0) = прозрачная подложка webview: за окном теперь
            // реально виден размытый Acrylic-стол.
            let _ = win.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 0)));
            let effects = tauri::window::EffectsBuilder::new()
                .effects([tauri::window::Effect::Acrylic, tauri::window::Effect::Blur])
                .state(tauri::window::EffectState::Active)
                .color(tauri::window::Color(18, 18, 24, 200))
                .build();
            let _ = win.set_effects(Some(effects));
        }
        "clear" => {
            // чистая прозрачность без системного блюра: подложка движка
            // прозрачна, за окном виден рабочий стол как есть
            let _ = win.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 0)));
            let _ = win.set_effects(None::<tauri::utils::config::WindowEffectsConfig>);
        }
        _ => return Err("режим: off | blur | clear".into()),
    }
    Ok(())
}
