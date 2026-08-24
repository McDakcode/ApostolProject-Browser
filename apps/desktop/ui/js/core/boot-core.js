// Made by MrDuck && Ox-Alpha
const invoke = window.__TAURI__.core.invoke;

// ---------------------------------------------------------------------
// In-page dialogs. Native alert()/prompt()/confirm() spawn OS windows
// that end up BEHIND our native child webviews — invisible yet modal,
// freezing the whole app. They are fully replaced here.
// ---------------------------------------------------------------------

function uiDialog({ message = "", kind = "alert", value = "" } = {}) {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "dlg-overlay";
    overlay.innerHTML =
      `<div class="dlg" role="dialog"><div class="dlg-message"></div>` +
      (kind === "prompt" ? `<input class="dlg-input" type="text" />` : "") +
      `<div class="dlg-buttons">` +
      (kind !== "alert" ? `<button class="ghost-btn" data-r="cancel">Отмена</button>` : "") +
      `<button class="primary-btn" data-r="ok">ОК</button></div></div>`;
    overlay.querySelector(".dlg-message").textContent = message;
    document.body.appendChild(overlay);
    const input = overlay.querySelector(".dlg-input");
    if (input) input.value = value != null ? String(value) : "";
    const done = (v) => { overlay.remove(); resolve(v); };
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) done(kind === "prompt" ? null : kind === "confirm" ? false : undefined);
      const b = e.target.closest("button[data-r]");
      if (!b) return;
      if (b.dataset.r === "ok") done(kind === "prompt" ? input.value : kind === "confirm" ? true : undefined);
      else done(kind === "prompt" ? null : false);
    });
    overlay.addEventListener("keydown", (e) => {
      if (e.key === "Enter") { e.preventDefault(); done(kind === "prompt" ? input.value : kind === "confirm" ? true : undefined); }
      if (e.key === "Escape") done(kind === "prompt" ? null : kind === "confirm" ? false : undefined);
    });
    (input || overlay.querySelector('[data-r="ok"]')).focus();
    if (input) input.select();
  });
}
const alert = (m) => uiDialog({ message: String(m ?? ""), kind: "alert" });
const prompt = (m, v) => uiDialog({ message: String(m ?? ""), kind: "prompt", value: v });
const confirm = (m) => uiDialog({ message: String(m ?? ""), kind: "confirm" }).then((r) => r === true);

// ---------------------------------------------------------------------
// Custom titlebar window controls
// ---------------------------------------------------------------------

function tauriWin() {
  try {
    const W = window.__TAURI__.window;
    return (W.getCurrentWindow || W.getCurrent)();
  } catch { return null; }
}
document.getElementById("winMin").addEventListener("click", () => tauriWin()?.minimize());
document.getElementById("winMax").addEventListener("click", async () => {
  const w = tauriWin();
  if (w) await w.toggleMaximize();
});
document.getElementById("winClose").addEventListener("click", () => tauriWin()?.close());

// Visible error reporting (helps catch silent breakages) + toasts
window.addEventListener("error", (ev) => {
  try { toast("⚠ Ошибка: " + (ev.message || "неизвестно")); } catch { /* ignore */ }
});
// Тосты складываются в вертикальный стек (#toastStack), а не ложатся друг
// на друга. kind: "ok" | "err" — зелёная/красная рамка (graph.js уже передаёт).
function toast(msg, kind) {
  let stack = document.getElementById("toastStack");
  if (!stack) {
    stack = document.createElement("div");
    stack.id = "toastStack";
    document.body.appendChild(stack);
  }
  const t = document.createElement("div");
  t.className = "toast" + (kind ? " " + kind : "");
  t.textContent = msg;
  stack.appendChild(t);
  while (stack.children.length > 6) stack.firstElementChild?.remove(); // не даём расти бесконечно
  setTimeout(() => {
    t.classList.add("hide");
    setTimeout(() => t.remove(), 200);
  }, 3200);
}

// Sidebar collapse button — bound here at the END of the script so nothing
// above can prevent it from attaching. ВАЖНО: единственный обработчик
// (дубль в session-ws-downloads-tabs.js удалён — двойной toggle ломал кнопку).
{
  const cb = document.getElementById("collapseBtn");
  if (cb && !cb._bound) {
    cb._bound = true;
    cb.addEventListener("click", () => {
      document.body.classList.toggle("collapsed");
      const sb = document.getElementById("sidebar");
      sb.classList.remove("peek");
      try {
        localStorage.setItem("apb-side-collapsed",
          document.body.classList.contains("collapsed") ? "1" : "0");
      } catch (_) {}
      syncPageLayout(true);
      setTimeout(syncPageLayout, 240);
    });
  }
  // Restore saved collapse state after all modules are loaded (timers fire
  // only after the synchronous script queue, so syncPageLayout exists here).
  setTimeout(() => {
    let saved = null;
    try { saved = localStorage.getItem("apb-side-collapsed"); } catch (_) {}
    if (saved === "1" && !document.body.classList.contains("collapsed")) {
      document.body.classList.add("collapsed");
      if (typeof syncPageLayout === "function") syncPageLayout(true);
    }
  }, 0);
  // Peek: temporarily expand the collapsed rail while the cursor is over it.
  // Работает ТОЛЬКО если включена настройка (body.side-hover, чекбокс
  // «Открывать свёрнутую панель при наведении» в Оформлении). По умолчанию
  // панель открывается ТОЛЬКО кнопкой-стрелкой.
  const sb = document.getElementById("sidebar");
  if (sb && !sb._peekBound) {
    sb._peekBound = true;
    let peekTimer = null;
    sb.addEventListener("mouseenter", () => {
      if (!document.body.classList.contains("collapsed")) return;
      if (!document.body.classList.contains("side-hover")) return;
      clearTimeout(peekTimer);
      requestAnimationFrame(() => sb.classList.add("peek"));
    });
    sb.addEventListener("mouseleave", () => {
      if (!document.body.classList.contains("collapsed")) return;
      if (!document.body.classList.contains("side-hover")) return;
      peekTimer = setTimeout(() => {
        sb.classList.remove("peek");
        syncPageLayout();
      }, 160);
    });
  }
}

// ---------------------------------------------------------------------
// Force dark theme on sites that don't have one (smart invert)
// ---------------------------------------------------------------------

const tabDark = new Set();
const DARK_ON_JS =
  "(function(){if(document.getElementById('apbDark'))return;" +
  "var s=document.createElement('style');s.id='apbDark';" +
  "s.textContent='html{filter:invert(1) hue-rotate(180deg);background:#fff}" +
  "img,video,picture,svg:not(#apbDark svg),iframe,[style*=\"background-image:url\"]{filter:invert(1) hue-rotate(180deg)}';" +
  "document.documentElement.appendChild(s);})();";
const DARK_OFF_JS = "(function(){var s=document.getElementById('apbDark');if(s)s.remove();})();";

function applyDarkTab(id, on) {
  invokeV2("page_eval", { id, js: on ? DARK_ON_JS : DARK_OFF_JS }).catch(() => {});
}
function updateDarkSiteBtn() {
  const b = document.getElementById("darkSiteBtn");
  if (!b) return;
  const t = currentTabObj();
  const on = !!(t && tabDark.has(t.id));
  b.classList.toggle("active-dark", on);
  b.title = on ? "Выключить тёмную тему сайта" : "Тёмная тема для сайта (если у сайта её нет)";
  b.querySelector("use")?.setAttribute("href", on ? "#i-sun" : "#i-moon");
}
document.getElementById("darkSiteBtn")?.addEventListener("click", () => {
  const t = currentTabObj();
  if (!t) { toast("Сначала откройте вкладку"); return; }
  if (tabDark.has(t.id)) { tabDark.delete(t.id); applyDarkTab(t.id, false); }
  else { tabDark.add(t.id); applyDarkTab(t.id, true); uiSound(660, 0.05); }
  updateDarkSiteBtn();
});
function reapplyDarkAfterNav(id) {
  if (tabDark.has(id)) setTimeout(() => applyDarkTab(id, true), 1200);
}

// ---------------------------------------------------------------------

// Made by MrDuck && Ox-Alpha