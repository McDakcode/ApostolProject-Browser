// Made by MrDuck
#![allow(unused_imports)]

use crate::shell::PageTab;
use crate::cmd::debug::append_backend_log;
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
      // Дочерние вебвью не имеют window.__TAURI__ (есть только
      // __TAURI_INTERNALS__) — invoke зовём через неё.
      var i = window.__TAURI_INTERNALS__;
      if (i && i.invoke) i.invoke("shell_hotkey", { key: k });
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

// Перехват target=_blank / window.open + КАСТОМНОЕ КОНТЕКСТНОЕ МЕНЮ внутри
// вкладок: ПКМ по ссылке/картинке показывает тёмное меню APB с «Открыть в
// новой вкладке» (нативное меню WebView2 не умеет открывать вкладки в шелле).
const NEW_TAB_RELAY_JS: &str = r#"(function(){
  if (window.__apbNewTabRelay) return; window.__apbNewTabRelay = true;
  function schemeFallback(u){
    // Фолбэк без IPC: навигация на спец-схему перехватывается on_navigation
    // в Rust (false = отмена), URL открывается вкладкой в шелле.
    try { location.assign("apb-newtab:" + encodeURIComponent(String(u))); } catch(e){}
  }
  function openInTab(u){
    if (!u) return null;
    try {
      var i = window.__TAURI_INTERNALS__;
      if (i && i.invoke) {
        // ВАЖНО: invoke() может реджектнуться (ACL запретил команду для
        // remote-origin, аргумент не прошёл валидацию и т.п.) — раньше
        // reject тут никак не обрабатывался и вкладка просто не
        // открывалась без единой ошибки на экране. Теперь при reject
        // едем в schemeFallback вместо тишины.
        var p = i.invoke("shell_open_tab", { url: String(u) });
        if (p && typeof p.catch === "function") {
          p.catch(function(err){
            try { console.error("[apb] shell_open_tab failed:", err); } catch(e2){}
            schemeFallback(u);
          });
        }
        return null;
      }
    } catch(e){}
    schemeFallback(u);
    return null;
  }
  window.open = function(u, name){
    try {
      var s = String(u == null ? "" : u);
      if (s && s !== "about:blank" && (!name || name === "_blank")) {
        openInTab(s);
      }
    } catch(e){}
    return null;
  };
  document.addEventListener("click", function(e){
    try {
      if (e.defaultPrevented || e.button !== 0) return;
      var a = e.target && e.target.closest ? e.target.closest("a[href]") : null;
      if (!a) return;
      var tgt = a.getAttribute("target") || "";
      if (tgt === "_blank" || ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey)) {
        e.preventDefault(); e.stopPropagation();
        openInTab(a.href);
      }
    } catch(err){}
  }, true);

  // ---- Кастомное контекстное меню (ссылки и картинки) ----
  var ctx = null;
  function closeCtx(){
    if (!ctx) return;
    ctx.remove(); ctx = null;
    document.removeEventListener("pointerdown", ctxDown, true);
    window.removeEventListener("keydown", ctxKey, true);
  }
  function ctxDown(e){ if (ctx && !ctx.contains(e.target)) closeCtx(); }
  function ctxKey(e){ if (e.key === "Escape") closeCtx(); }
  function copyText(s){ try { navigator.clipboard.writeText(s); } catch(e){} }
  function openCtx(e, items){
    closeCtx();
    ctx = document.createElement("div");
    ctx.style.cssText = "position:fixed;z-index:2147483647;min-width:220px;padding:6px;"
      + "background:rgba(15,15,21,.94);backdrop-filter:blur(20px) saturate(150%);"
      + "-webkit-backdrop-filter:blur(20px) saturate(150%);"
      + "border:1px solid rgba(127,176,255,.25);border-radius:12px;"
      + "box-shadow:0 12px 40px rgba(0,0,0,.5);font:12.5px system-ui,sans-serif;color:#ececf1;";
    for (var i = 0; i < items.length; i++) {
      (function(it){
        if (it.sep) {
          var hr = document.createElement("div");
          hr.style.cssText = "height:1px;background:rgba(255,255,255,.12);margin:4px 8px";
          ctx.appendChild(hr);
          return;
        }
        var b = document.createElement("button");
        b.textContent = it.label;
        b.style.cssText = "display:block;width:100%;text-align:left;padding:8px 12px;"
          + "background:none;border:none;border-radius:8px;color:inherit;font:inherit;"
          + "cursor:pointer;white-space:nowrap";
        b.onmouseenter = function(){ b.style.background = "rgba(255,255,255,.10)"; };
        b.onmouseleave = function(){ b.style.background = "none"; };
        b.onclick = function(){ closeCtx(); try { it.fn(); } catch(e){} };
        ctx.appendChild(b);
      })(items[i]);
    }
    document.documentElement.appendChild(ctx);
    var mw = ctx.offsetWidth || 220, mh = ctx.offsetHeight || 120;
    ctx.style.left = Math.max(4, Math.min(e.clientX, window.innerWidth - mw - 8)) + "px";
    ctx.style.top = Math.max(4, Math.min(e.clientY, window.innerHeight - mh - 8)) + "px";
    setTimeout(function(){
      document.addEventListener("pointerdown", ctxDown, true);
      window.addEventListener("keydown", ctxKey, true);
    }, 0);
  }
  document.addEventListener("contextmenu", function(e){
    try {
      var a = e.target && e.target.closest ? e.target.closest("a[href]") : null;
      var img = e.target && e.target.closest ? e.target.closest("img") : null;
      var items = [];
      if (a) {
        var href = a.href || "";
        if (href && href.indexOf("javascript:") !== 0) {
          items.push({ label: "🔗 Открыть в новой вкладке", fn: function(){ openInTab(href); } });
          items.push({ label: "📋 Копировать адрес ссылки", fn: function(){ copyText(href); } });
        }
      }
      if (img) {
        var src = img.currentSrc || img.src || "";
        if (src) {
          if (items.length) items.push({ sep: true });
          items.push({ label: "🖼 Открыть изображение в новой вкладке", fn: function(){ openInTab(src); } });
          items.push({ label: "📋 Копировать адрес изображения", fn: function(){ copyText(src); } });
        }
      }
      if (!items.length) return; // не ссылка/картинка — нативное меню как раньше
      e.preventDefault(); e.stopPropagation();
      openCtx(e, items);
    } catch(err){}
  }, true);
})();"#;

// Запоминание позиции видео: главный <video> страницы пишет currentTime в
// localStorage, при повторном открытии ролик продолжается с места остановки
// (в т.ч. после рестарта браузера). Короткие видео (<90с, реклама) не
// трогаем — иначе восстановление ломает рекламные ролики YouTube.
const VIDEO_RESUME_JS: &str = r#"(function(){
  if (window.__apbVideoResume) return; window.__apbVideoResume = true;
  // Ключ считаем В МОМЕНТ записи/чтения: YouTube и прочие SPA меняют URL
  // без перезагрузки страницы — ключ, вычисленный один раз, цеплял бы
  // позицию не к тому ролику. ПРОБЛЕМА, найденная у юзера: обычный
  // origin+pathname для всех роликов одинаков ("/watch"), а полный
  // search — НЕСТАБИЛЕН: YouTube сам добавляет/переставляет параметры
  // (pp, embeds, cbrd...), поэтому ключ «на сохранении» расходился с
  // ключом «при загрузке» ровно для того же ролика → время не находилось.
  // ФИКС: ключ строится строго по ИДЕНТИЧНОСТИ ролика — для /watch берём
  // ТОЛЬКО ?v=<id>, всё остальное выкидываем; shorts/live/embed уже
  // уникальны самим pathname (в нём id). Спам-параметры не влияют на ключ.
  function key(){
    try {
      var u = new URL(location.href);
      if (u.pathname === "/watch") {
        var v7 = u.searchParams.get("v");
        if (v7) return "apb-video-pos:" + u.origin + "/watch?v=" + v7;
      }
      u.searchParams.delete("t");
      return "apb-video-pos:" + u.origin + u.pathname + (u.search ? u.search : "");
    } catch(e){
      return "apb-video-pos:" + location.origin + location.pathname;
    }
  }
  // Диагностика в журнал бэкенда (метка [page]). Толстые строки не шлём —
  // только факты и цифры, чтобы по apb\logs\shell-debug.log понять, что
  // видит страница (p-push: записал/пропустил, p-restore: есть/нет/применил).
  function diag(m){
    try { var i = window.__TAURI_INTERNALS__; if (i && i.invoke) i.invoke("page_diag", { msg: m }).catch(function(){}); } catch(e){}
  }
  function mainVideo(){
    var vs = document.querySelectorAll("video");
    var best = null, bestA = 0;
    for (var i = 0; i < vs.length; i++) {
      var a = (vs[i].videoWidth * vs[i].videoHeight) || (vs[i].clientWidth * vs[i].clientHeight);
      if (a > bestA) { bestA = a; best = vs[i]; }
    }
    return best;
  }
  function resumable(v){
    return v && (!isFinite(v.duration) || v.duration >= 90);
  }
  var lastSaveDiag = 0;
  function save(v){
    try {
      if (!resumable(v)) return; // реклама/короткие вставки не трогаем
      var t = v.currentTime || 0;
      var nearEnd = isFinite(v.duration) && t >= v.duration - 10;
      if (t > 5 && !nearEnd) {
        localStorage.setItem(key(), String(Math.floor(t)));
        var now = Date.now();
        if (now - lastSaveDiag > 20000) { lastSaveDiag = now; diag("push k=" + key() + " t=" + Math.floor(t)); }
      } else if (t <= 5) {
        localStorage.removeItem(key());
      }
    } catch(e){}
  }
  function restore(v){
    // Диагностику держим В САМОМ НАЧАЛЕ (до resumable): если restore вообще
    // вызывается, но молча выходит на resumable()===false (duration ещё 0/NaN/
    // короткое на этапе, когда видео только появилось в DOM), без diag мы бы
    // ничего не узнали. Теперь видно каждый вызов и причину пропуска.
    try { if (!v.__apbDiag) diag("restore-call ct=" + Math.floor(v.currentTime) + " dur=" + (isFinite(v.duration) ? Math.floor(v.duration) : "inf") + " ready=" + v.readyState); } catch(e){}
    try {
      if (!resumable(v)) return;
      var kkey = key();
      var t = parseFloat(localStorage.getItem(kkey) || "0") || 0;
      if (t > 5 && v.currentTime < 5 && (!isFinite(v.duration) || t < v.duration - 10)) {
        v.currentTime = t;
        if (!v.__apbDiag) { v.__apbDiag = true; diag("restore k=" + kkey + " -> " + Math.floor(t)); }
      } else if (t > 5 && !v.__apbDiag) {
        v.__apbDiag = true;
        diag("restore-skip k=" + kkey + " v=" + Math.floor(t) + " ct=" + Math.floor(v.currentTime) + " dur=" + (isFinite(v.duration) ? Math.floor(v.duration) : "inf"));
      } else if (!v.__apbDiag) {
        v.__apbDiag = true;
        diag("restore-empty k=" + kkey);
      }
    } catch(e){}
  }
  var last = 0;
  setInterval(function(){
    var v = mainVideo();
    if (!v || v.paused) return;
    var now = Date.now();
    if (now - last < 4000) return;
    last = now; save(v);
  }, 2000);
  window.addEventListener("pagehide", function(){ var v = mainVideo(); if (v) save(v); });
  // НАДЁЖНЫЙ FALLBACK-ВОССТАНОВИТЕЛЬ: события loadedmetadata/play/observer
  // у YouTube с холодного старта НЕнадёжны (video появляется поздно, к моменту
  // навешивания hook может уже проскочить нужное состояние → restore молчит,
  // это и ловили в журнале 97). Поэтому дополнительно крутим ВРЕМЕННЫЙ цикл:
  // каждые 600мс в течение ~30с берём главное <video> и, если позицию ещё не
  // восстанавливали и она реально нужна, ставим currentTime. Не зависит от
  // событий/observer вообще. __apbRestored на самом <video> не даёт зациклиться.
  (function(){
    var attempt = 0, tried = 0;
    var timer = setInterval(function(){
      attempt++;
      if (attempt > 50) { clearInterval(timer); return; }
      var v = mainVideo();
      if (!v) return;
      if (v.__apbRestored) { clearInterval(timer); return; }
      var kkey = key();
      var t = parseFloat(localStorage.getItem(kkey) || "0") || 0;
      if (t > 5 && v.currentTime < 5 && (!isFinite(v.duration) || t < v.duration - 10)) {
        tried++;
        // duration могла быть ещё 0/NaN — позволяем установить, но требование 90с
        // для НЕ-числовой длительности держим (resumable: !isFinite допускается).
        try { v.currentTime = t; } catch(e){}
        if (v.currentTime >= 5 || tried > 12) { v.__apbRestored = true; clearInterval(timer); }
        else if (tried === 1) diag("fallback-restore t=" + Math.floor(t) + " current=" + Math.floor(v.currentTime));
      } else if (v.currentTime >= 5 || !(t > 5)) {
        clearInterval(timer);
      }
    }, 600);
  })();
  // SPA-навигация (pushState): YouTube подменяет ролик без перезагрузки —
  // после смены URL пробуем восстановить позицию уже нового видео.
  function hook(v){
    if (v.__apbVp) return; v.__apbVp = true;
    v.addEventListener("loadedmetadata", function(){ v.__apbRestored = false; restore(v); });
    v.addEventListener("play", function(){ if (!v.__apbRestored) { v.__apbRestored = true; restore(v); } });
    if (v.readyState >= 1) restore(v);
    v.addEventListener("pause", function(){ save(v); });
    v.addEventListener("seeked", function(){ save(v); });
  }
  try {
    var mo = new MutationObserver(function(){ document.querySelectorAll("video").forEach(hook); });
    mo.observe(document.documentElement, { childList: true, subtree: true });
  } catch(e){}
  try {
    var ps = history.pushState;
    history.pushState = function(){ var r = ps.apply(this, arguments); setTimeout(function(){ document.querySelectorAll("video").forEach(function(v){ v.__apbVp = false; v.__apbRestored = false; }); }, 60); return r; };
    window.addEventListener("popstate", function(){ setTimeout(function(){ document.querySelectorAll("video").forEach(function(v){ v.__apbVp = false; v.__apbRestored = false; }); }, 60); });
  } catch(e){}
  document.querySelectorAll("video").forEach(hook);
})();"#;

// Кинорежим ютуба («широкий экран»): в обычных браузерах раз включённый —
// держится «навсегда, для любого ролика». Сам YouTube хранит это состояние
// в JS-конфигурации плеера нестабильно, поэтому у нас при открытии нового
// ролика (особенно с холодного старта) раскладка каждый раз стартует с
// дефолтной. Решение без сторонних API: запоминаем ВЫБОР юзера (кнопка
// `button.ytp-size-button` = переключатель «кинорежим», состояние видно по
// атрибуту `theater` на ytd-watch-flexy) в localStorage и, если на экране
// плеер не в том режиме, незаметно кликаем кнопку один раз. Работает и для
// SPA-переходов между роликами, и для холодной загрузки страницы ролика.
const THEATER_KEEP_JS: &str = r#"(function(){
  if (window.__apbTheater) return; window.__apbTheater = true;
  var PREF = "apb-theater";
  function diag(m){
    try { var i = window.__TAURI_INTERNALS__; if (i && i.invoke) i.invoke("page_diag", { msg: m }).catch(function(){}); } catch(e){}
  }
  function flexy(){ return document.querySelector("ytd-watch-flexy"); }
  function theaterOn(){
    var f = flexy();
    return !!(f && (f.hasAttribute("theater") || (f.className || "").indexOf("theater") !== -1));
  }
  function desired(){
    try { return localStorage.getItem(PREF) === "1"; } catch(e){ return false; }
  }
  function store(v){
    try { localStorage.setItem(PREF, v ? "1" : "0"); } catch(e){}
  }
  // Текущий выбор юзера фиксируем в момент его собственного переключения.
  function hookSizeButton(){
    try {
      var b = document.querySelector("button.ytp-size-button");
      if (b && !b.__apbTh) {
        b.__apbTh = true;
        b.addEventListener("click", function(){ setTimeout(function(){ store(theaterOn()); }, 400); });
      }
    } catch(e){}
  }
  function apply(){
    var f = flexy();
    if (!f) return;
    var cur = theaterOn();
    var want = desired();
    if (cur === want) return;
    hookSizeButton();
    var b = document.querySelector("button.ytp-size-button");
    if (b) b.click();
  }
  // SPA-переходы: yt-navigate-finish + наблюдение за появлением плеера.
  document.addEventListener("yt-navigate-finish", function(){ setTimeout(apply, 700); });
  try {
    var mo = new MutationObserver(function(){ hookSizeButton(); apply(); });
    mo.observe(document.documentElement, { childList: true, subtree: true });
  } catch(e){}
  // Холодная загрузка: несколько попыток с нарастающей задержкой.
  [0, 200, 800, 2000, 4000].forEach(function(d){ setTimeout(apply, d); });
  // УПОРСТВО: состояние ютуба нестабильно, одного клика может не хватить
  // (кнопка появляется позже, flexy ещё без атрибута at-theather). Крутим
  // цикл каждые 1.5с в течение ~30с: если расхождение «хочу-факт» есть и
  // кнопка в DOM — кликаем; останавливаемся, когда совпало. Ниже желаемое
  // ни разу не перетирается — юзер-выбор в записи переключателя сохраняется.
  (function(){
    var n = 0;
    var timer = setInterval(function(){
      n++;
      if (n > 20) { clearInterval(timer); return; }
      var f = flexy();
      if (!f) return;
      if (theaterOn() === desired()) { clearInterval(timer); return; }
      hookSizeButton();
      var b = document.querySelector("button.ytp-size-button");
      if (b) { b.click(); diag("theater-click want=" + (desired() ? "1" : "0")); }
    }, 1500);
  })();
  // Диагностика для юзера/лога: показываем, что видим (flexy есть/нет,
  // текущий театр-статус, желаемое, кнопка есть/нет) сразу при старте.
  try {
    var ff = flexy();
    diag("theater-init flexy=" + (ff ? "yes" : "no") + " cur=" + (theaterOn() ? "1" : "0") + " want=" + (desired() ? "1" : "0") + " btn=" + (document.querySelector("button.ytp-size-button") ? "yes" : "no"));
  } catch(e){}
})();"#;

// Спа-сайты (YouTube) меняют URL через pushState без топ-навигации, поэтому
// on_navigation о URL нового ролика НЕ сообщает. Сторона страницы сама
// замечает смену location.href (pushState/replaceState/popstate/hashchange
// + лёгкий поллинг на случай экзотики) и докладывает в бэкенд через
// __TAURI_INTERNALS__.invoke → page_url_push (команда получает вебвью-автора,
// поэтому id эмитится голым, как и остальные события). Схема-none,
// навигацию НЕ трогаем — только invoke (location.assign на apb-* уже один
// раз ломал страницы). Если invoke вдруг недоступен — сценарий молча
// вырубается, URL просто останется старым (названия продолжают работать
// через on_document_title_changed).
const URL_CHANGE_JS: &str = r#"(function(){
  if (window.__apbUrlPush) return; window.__apbUrlPush = true;
  function report(){
    try {
      var h = location.href;
      if (h === window.__apbLastUrl) return;
      window.__apbLastUrl = h;
      var i = window.__TAURI_INTERNALS__;
      if (i && i.invoke) i.invoke("page_url_push", { url: h }).catch(function(){});
    } catch(e){}
  }
  try {
    var ps = history.pushState, rs = history.replaceState;
    history.pushState = function(){ var r = ps.apply(this, arguments); report(); return r; };
    history.replaceState = function(){ var r = rs.apply(this, arguments); report(); return r; };
  } catch(e){}
  window.addEventListener("popstate", report);
  window.addEventListener("hashchange", report);
  setInterval(report, 1200);
  var base = document.title;
  try {
    var t0 = new MutationObserver(function(){ if (document.title !== base) { report(); base = document.title; } });
    t0.observe(document.querySelector("head") || document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true });
  } catch(e){}
})();"#;

#[tauri::command]
pub(crate) async fn page_url_push(webview: tauri::Webview, url: String) -> Result<(), String> {
    if url.trim().is_empty() {
        return Ok(());
    }
    let raw_label = webview.label().to_string();
    let bare = raw_label
        .strip_prefix("page-")
        .unwrap_or(&raw_label)
        .to_string();
    append_backend_log(&format!("[url-push] {bare} {url}"));
    let app = webview.app_handle().clone();
    let payload = serde_json::json!({ "id": bare, "url": url });
    let _ = tauri::Emitter::emit(&app, "page-url-changed", payload);
    Ok(())
}

/// Служебный канал диагностики из СТОРОНЫ СТРАНИЦЫ в журнал бэкенда
/// (apb\logs\shell-debug.log, метка [page]). Сделан командой, как
/// page_url_push: tauri сам подставляет вебвью-автора. Нужен, чтобы
/// страничные скрипты (позиция видео, кинорежим ютуба) могли доказать,
/// что происходит «на поле», не полагаясь на догадки.
#[tauri::command]
pub(crate) async fn page_diag(webview: tauri::Webview, msg: String) -> Result<(), String> {
    let raw_label = webview.label().to_string();
    let bare = raw_label
        .strip_prefix("page-")
        .unwrap_or(&raw_label)
        .to_string();
    append_backend_log(&format!("[page] {bare} {msg}"));
    Ok(())
}

#[tauri::command]
pub(crate) async fn shell_open_tab(app: AppHandle, url: String) -> Result<(), String> {
    let parsed: tauri::Url = url.parse().map_err(|_| format!("неверный URL: {url}"))?;
    // Глобальную функцию открывает session-ws-downloads-tabs.js (createTab).
    let js = format!("window.__apbOpenTab && window.__apbOpenTab({:?})", parsed.as_str());
    let app_for_main = app.clone();
    on_main_thread(&app, move || {
        let wv = app_for_main
            .get_webview("shell")
            .ok_or_else(|| "нет окна оболочки".to_string())?;
        wv.eval(&js).map_err(|e| e.to_string())
    })?
}

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

// Made by MrDuck
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
    // Голый id вкладки (page_open возвращает ИМЕННО его). События обязаны
    // нести id, иначе фронтенд tabs.find() по "page-<uuid>" промахивается
    // мимо вкладки с id "<uuid>" — заголовки и смена URL не доходили до UI
    // (корень бага «названий нет», найден по логам: [nav] идут, а вкладка
    // не находится).
    let nav_id = id.clone();

    // Fingerprint spoofing script for this profile (injected before page JS).
    // None when protection is Off — zero overhead for Standard level.
    let fingerprint_js: Option<String> = {
        let state = app.state::<crate::state::SharedState>();
        let guard = state.lock().unwrap();
        guard.active_or_err().ok().and_then(|a| {
            let pol = a.privacy.effective_policy();
            if pol.fingerprint_protection == apb_privacy::FingerprintLevel::Off {
                None
            } else {
                Some(
                    apb_privacy::FingerprintPersona::derive(a.profile.id, pol.fingerprint_protection)
                        .injection_script(),
                )
            }
        })
    };

    // Cookie/storage isolation + Referer-policy shim for this tab's snapshot
    // of the policy. Covers HTTPS traffic where the proxy cannot look inside
    // the tunnel: in cross-origin frames document.cookie is frozen (and
    // storage writes are no-ops under strict isolation), plus a
    // <meta name="referrer"> is planted for the referrer policy. Applies to
    // webviews created after the last policy change (same as fingerprint).
    let privacy_js: Option<String> = {
        let state = app.state::<crate::state::SharedState>();
        let guard = state.lock().unwrap();
        guard.active_or_err().ok().and_then(|a| {
            let pol = a.privacy.effective_policy();
            let cookie_shim =
                pol.block_third_party_cookies || pol.strict_storage_isolation;
            if !cookie_shim && pol.referrer == apb_privacy::ReferrerPolicy::Default {
                None
            } else {
                Some(privacy_shim_js(
                    pol.block_third_party_cookies,
                    pol.strict_storage_isolation,
                    pol.referrer,
                ))
            }
        })
    };

    // Cosmetic ad filtering: AdBlock-grade element hiding. When the profile
    // blocks ads, plant the full stylesheet + DOM-sweeper at document-start
    // so blocked networks leave no empty boxes. The sweeper *removes*
    // banner-shaped nodes and blanks media (gif/swf/flash) URLs, so static /
    // animated / flash test banners are truly gone (clientWidth/Height → 0).
    // Single always-on AdBlock-style mode — no mild/aggressive switch.
    let cosmetic_js: Option<String> = {
        let state = app.state::<crate::state::SharedState>();
        let guard = state.lock().unwrap();
        guard
            .active_or_err()
            .ok()
            .filter(|a| a.privacy.effective_policy().block_ads)
            .map(|_| apb_privacy::blocklists::aggressive_filter_script())
    };

    // In-page request blocker (extension-grade layer): sendBeacon/fetch/XHR
    // to known ad/tracker endpoints abort in-page — the proxy can't see
    // these inside HTTPS tunnels. The full banner/flash/creative path set is
    // always active (the extension-grade mode).
    let req_js: Option<String> = {
        let state = app.state::<crate::state::SharedState>();
        let guard = state.lock().unwrap();
        guard
            .active_or_err()
            .ok()
            .filter(|a| {
                let p = a.privacy.effective_policy();
                p.block_ads || p.block_trackers
            })
            .map(|_| apb_privacy::blocklists::request_blocker_script(
                &apb_privacy::blocklists::builtin_request_patterns_aggressive(),
            ))
    };

    on_main_thread(&app, move || -> Result<(), String> {
        let window = app_for_main.get_window("shell").ok_or_else(|| "нет окна оболочки".to_string())?;
        let (x, y, width, height) =
            measured.unwrap_or_else(|| content_rect(&window, false));
        // Сайт сам сменил страницу (клик по ссылке, редирект) — сообщаем
        // шеллу, чтобы омнибокс/история вкладки не оставались на старом URL.
        let nav_app = app_for_main.clone();
        let nav_label = label_for_main.clone();
        // Нативное меню WebView2 «Открыть ссылку в новом окне» и не пойманные
        // шимом window.open: ОС-окно НЕ создаём — просим шелл открыть вкладку.
        let nw_app = app_for_main.clone();
        let mut builder = WebviewBuilder::new(&label_for_main, WebviewUrl::External(parsed))
            .on_navigation(move |url| {
                // Фолбэк-канал «открыть вкладкой» без IPC (если remote-invoke
                // запрещён): шим в странице ведёт на apb-newtab:<url>,
                // навигация отменяется, URL уезжает в шелл новой вкладкой.
                if url.scheme() == "apb-newtab" {
                    let raw = url.as_str().trim_start_matches("apb-newtab:");
                    // crate::util::percent_decode вместо несуществующего
                    // percent_decode_str (внешний крейт percent-encoding не
                    // подключён и нигде не импортирован — билд падал с
                    // E0425). При неудачном декодировании открываем как есть
                    // — лучше сырой URL, чем сломанная вкладка.
                    let target = crate::util::percent_decode(raw).unwrap_or_else(|| raw.to_string());
                    let payload = serde_json::json!({ "url": target });
                    let _ = tauri::Emitter::emit(&nav_app, "page-open-tab", payload);
                    return false;
                }
                append_backend_log(&format!("[nav] {nav_label} {url}"));
                let payload = serde_json::json!({ "id": nav_id, "url": url.to_string() });
                let _ = tauri::Emitter::emit(&nav_app, "page-url-changed", payload);
                true
            })
            // Реальный заголовок страницы (<title>) — для названия сайта в
            // омнибоксе и подписей вкладок. Приходит как только WebView2
            // прогрузит документ. id — голый uuid (как page_open возвращает),
            // а не "page-"-label: фронтенд ищет вкладку ТОЛЬКО по нему.
            .on_document_title_changed(move |w, title| {
                let t = title.trim().to_string();
                let raw_label = w.label().to_string();
                let bare = raw_label
                    .strip_prefix("page-")
                    .unwrap_or(&raw_label)
                    .to_string();
                append_backend_log(&format!("[title-ev] {bare} {t}"));
                if t.is_empty() { return; }
                let app = w.app_handle().clone();
                let payload = serde_json::json!({ "id": bare, "title": t });
                let _ = tauri::Emitter::emit(&app, "page-title-changed", payload);
            })
            .on_new_window(move |url, _features| {
                let payload = serde_json::json!({ "url": url.to_string() });
                let _ = tauri::Emitter::emit(&nw_app, "page-open-tab", payload);
                tauri::webview::NewWindowResponse::Deny
            })
            .initialization_script(HOTKEY_RELAY_JS)
            .initialization_script(NEW_TAB_RELAY_JS)
            .initialization_script(VIDEO_RESUME_JS)
            .initialization_script(THEATER_KEEP_JS)
            .initialization_script(URL_CHANGE_JS)
            // MUST match the shell window's args exactly (same user-data
            // folder = same WebView2 environment options requirement).
            .additional_browser_args(crate::liveprivacy::browser_args())
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
        if let Some(js) = fingerprint_js {
            builder = builder.initialization_script(js);
        }
        if let Some(js) = privacy_js {
            builder = builder.initialization_script(js);
        }
        if let Some(js) = cosmetic_js {
            builder = builder.initialization_script(js);
        }
        if let Some(js) = req_js {
            builder = builder.initialization_script(js);
        }
        window
            .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        append_backend_log(&format!("[open] {label_for_main} ok"));
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

// Made by MrDuck
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
// Made by MrDuck
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
            // Сначала останавливаем страницу: полного уничтожения webview в
            // tauri 2 нет (закрыто только "спрятать за экран"), а скрытый
            // WebView2 продолжает молотить и играть аудио/видео — юзер видел
            // баг «выключил видео вкладкой, а звук остался навсегда».
            // Навигация на about:blank мгновенно выгружает страницу и её
            // медиа-процессы — звук умирает вместе с ней.
            let _ = webview.navigate(tauri::Url::parse("about:blank").unwrap());
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

/// Build the cookie/storage/referrer enforcement script. Injected into every
/// frame of a tab webview (initialization scripts survive navigations).
fn privacy_shim_js(
    block_tp_cookies: bool,
    strict_storage: bool,
    referrer: apb_privacy::ReferrerPolicy,
) -> String {
    use apb_privacy::ReferrerPolicy as RP;
    let meta: String = match referrer {
        RP::Default => "",
        RP::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
        RP::SameOriginOnly => "same-origin",
        RP::NeverCrossOrigin => "no-referrer",
    }
    .to_string();
    let tp = if block_tp_cookies { "true" } else { "false" };
    let ss = if strict_storage { "true" } else { "false" };
    format!(
        r#"(() => {{
  const REF = "{meta}";
  try {{
    if (REF && !document.querySelector('meta[name="referrer"]')) {{
      const m = document.createElement("meta");
      m.setAttribute("name", "referrer");
      m.setAttribute("content", REF);
      (document.head || document.documentElement).appendChild(m);
    }}
  }} catch (e) {{}}
  const TP = {tp}, SS = {ss};
  if (!TP && !SS) return;
  let inFrame = false;
  try {{ inFrame = window.top !== window.self; }} catch (e) {{ inFrame = false; }}
  if (!inFrame) return;
  let cross = false;
  try {{
    const t = window.top.location;
    cross = t.host !== window.location.host || t.protocol !== window.location.protocol;
  }} catch (e) {{ cross = true; }}
  if (!cross) return;
  if (TP) {{
    try {{
      Object.defineProperty(document, "cookie", {{
        configurable: false,
        get: () => "",
        set: () => {{}},
      }});
    }} catch (e) {{}}
  }}
  if (SS) {{
    for (const name of ["localStorage", "sessionStorage"]) {{
      try {{
        const s = window[name];
        for (const k of ["setItem", "removeItem", "clear"]) {{
          try {{ s[k] = () => {{ throw new Error("apb-isolated"); }}; }} catch (e) {{}}
        }}
      }} catch (e) {{}}
    }}
  }}
}})();"#
    )
}

// Made by MrDuck