// Made by MrDuck
// ---------------------------------------------------------------------
// First-run onboarding — shown exactly once, at first launch
// ---------------------------------------------------------------------

function showOnboarding() {
  const ov = document.createElement("div");
  ov.id = "onboardingOverlay";
  ov.style.cssText =
    "position:fixed;inset:0;z-index:1000;display:flex;align-items:center;justify-content:center;background:var(--bg)";
  ov.innerHTML = `
    <div style="width:460px;max-width:92vw;text-align:center;padding:36px 34px;
                background:var(--surface);border:1px solid var(--border-strong);
                border-radius:20px;animation:riseIn .35s ease both">
      <div class="brand-mark" style="width:52px;height:52px;border-radius:16px;margin:0 auto 14px"></div>
      <h1 style="font-size:24px;font-weight:800;margin:0 0 6px">Добро пожаловать в APB</h1>
      <p class="hint" style="margin-bottom:22px">Первоначальная настройка — займёт полминуты.</p>

      <div style="text-align:left;margin-bottom:18px">
        <div class="panel-subtitle" style="margin-top:0">Тема</div>
        <div class="theme-buttons">
          <button id="obDark" data-theme-choice="dark">🌑 Чёрная</button>
          <button id="obLight" data-theme-choice="light">🌕 Белая</button>
        </div>
      </div>

      <div style="text-align:left;margin-bottom:18px">
        <div class="panel-subtitle" style="margin-top:0">Город для погоды</div>
        <input id="obCity" type="text" placeholder="Например: Москва"
               style="width:100%;background:var(--surface);border:1px solid var(--border);
                      border-radius:8px;color:var(--text);padding:9px 12px" />
      </div>

      <div style="text-align:left;margin-bottom:24px">
        <div class="panel-subtitle" style="margin-top:0">Системный браузер</div>
        <button id="obDefaultBtn" class="ghost-btn" style="width:100%">Открыть настройки Windows → «Приложения по умолчанию»</button>
        <p class="hint" style="margin-top:6px">Выберите APB там, если хотите открывать ссылки через него.</p>
      </div>

      <button id="obDone" class="primary-btn" style="width:100%;padding:11px">Начать работу →</button>
      <button id="obSkip" class="ghost-btn" style="width:100%;margin-top:8px;border:none;background:none">Пропустить</button>
    </div>`;
  document.body.appendChild(ov);

  const finish = () => {
    localStorage.setItem("apb-onboarded", "1");
    const city = ov.querySelector("#obCity").value.trim();
    if (city) localStorage.setItem("apb-weather", JSON.stringify({ city, ts: 0 }));
    ov.remove();
    showHome();
    // После онбординга — игровой тур подсказок (один раз)
    if (localStorage.getItem("apb-ui-tour") !== "done") {
      setTimeout(() => startUITour(false), 600);
    }
  };
  ov.querySelector("#obDone").addEventListener("click", finish);
  ov.querySelector("#obSkip").addEventListener("click", finish);
  ov.querySelector("#obDefaultBtn").addEventListener("click", () => {
    invokeV2("open_in_system", { url: "ms-settings:defaultapps" }).catch(() => alert("Не удалось открыть настройки Windows."));
  });
}

// ---------------------------------------------------------------------
// Personalization (Vivaldi-style): accent, radius, density, sidebar
// width, glass, motion — persisted in localStorage ("apb-ui").
// ---------------------------------------------------------------------

const PZ_DEFAULTS = { accent: "", radius: 12, sidebar: 232, density: "normal", glass: true, motion: true, glassA: "", tabsPos: "left", sidebarSide: "left", panelSide: "left", hideTools: false, wsCount: true, sideHover: false, font: "system", fontSize: 13, bgColor: "", bgImg: "", bgDim: 35, thBg: "", thSoft: "", thText: "", sound: true, settingsCols: false, settingsW: 640 };

// Curated font stacks for the UI font setting
const AP_FONTS = {
  system: '-apple-system, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif',
  segoe: '"Segoe UI", system-ui, sans-serif',
  arial: 'Arial, Helvetica, sans-serif',
  georgia: 'Georgia, "Times New Roman", serif',
  mono: 'Consolas, ui-monospace, monospace',
};

function pzLoad() {
  try { return { ...PZ_DEFAULTS, ...(JSON.parse(localStorage.getItem("apb-ui")) || {}) }; }
  catch { return { ...PZ_DEFAULTS }; }
}

function pzApply() {
  const p = pzLoad();
  const rs = document.documentElement.style;
  if (p.accent) rs.setProperty("--accent", p.accent);
  else rs.removeProperty("--accent");
  rs.setProperty("--r-sm", Math.max(0, p.radius - 5) + "px");
  rs.setProperty("--r", Math.max(2, p.radius - 2) + "px");
  rs.setProperty("--r-lg", p.radius + "px");
  rs.setProperty("--sidebar-w", p.sidebar + "px");
  rs.setProperty("--panel-w", (p.panelW || 330) + "px");
  if (p.editorW) rs.setProperty("--editor-w", p.editorW + "px"); else rs.removeProperty("--editor-w");
  if (p.aiW) rs.setProperty("--ai-w", p.aiW + "px"); else rs.removeProperty("--ai-w");
  document.body.classList.remove("density-compact", "density-normal", "density-spacious");
  document.body.classList.add("density-" + (p.density || "normal"));
  document.body.classList.toggle("no-glass", !p.glass);
  document.body.classList.toggle("no-motion", !p.motion);

  // --- Appearance page ---
  // Font
  rs.setProperty("--font-body", AP_FONTS[p.font] || AP_FONTS.system);
  document.body.style.fontSize = (p.fontSize || 13) + "px";
  // Tabs position / tools visibility
  document.body.classList.toggle("tabbar-top", p.tabsPos === "top");
  document.body.classList.toggle("sidebar-right", p.sidebarSide === "right");
  document.body.classList.toggle("panel-right", p.panelSide === "right");
  document.body.classList.toggle("hide-tools", !!p.hideTools);
  document.body.classList.toggle("no-ws-count", !p.wsCount);
  document.body.classList.toggle("side-hover", !!p.sideHover);
  // Settings layout (optional 2 columns + adjustable width)
  document.body.classList.toggle("settings-2col", !!p.settingsCols);
  rs.setProperty("--settings-w", (p.settingsW || 640) + "px");
  // Glass transparency
  if (p.glassA) rs.setProperty("--glass-a", String(p.glassA / 100));
  else rs.removeProperty("--glass-a");
  // Custom theme overrides
  if (p.thBg) rs.setProperty("--bg", p.thBg); else rs.removeProperty("--bg");
  if (p.thSoft) rs.setProperty("--bg-soft", p.thSoft); else rs.removeProperty("--bg-soft");
  if (p.thText) { rs.setProperty("--text", p.thText); } else rs.removeProperty("--text");
  // Home background
  const be = document.getElementById("browserEmpty");
  if (be) {
    const dim = Math.max(0, Math.min(80, p.bgDim == null ? 35 : p.bgDim)) / 100;
    be.style.backgroundColor = p.bgColor || "";
    be.style.backgroundImage = p.bgImg
      ? `linear-gradient(rgba(0,0,0,${dim}), rgba(0,0,0,${dim})), url("${p.bgImg}")`
      : "";
    be.style.backgroundSize = p.bgImg ? "cover" : "";
    be.style.backgroundPosition = p.bgImg ? "center" : "";
    be.style.backgroundRepeat = "no-repeat";
    be.style.backgroundAttachment = p.bgImg ? "fixed" : "";
  }
}

function pzUpdate(patch) {
  const p = { ...pzLoad(), ...patch };
  localStorage.setItem("apb-ui", JSON.stringify(p));
  pzApply();
  if ("sidebar" in patch || "radius" in patch || "density" in patch) {
    syncPageLayout(true);
    setTimeout(syncPageLayout, 220);
  }
}

function pzSyncControls() {
  const p = pzLoad();
  document.getElementById("pzAccent").value = p.accent || "#8a8a92";
  document.getElementById("pzRadius").value = p.radius;
  document.getElementById("pzSidebar").value = p.sidebar;
  document.getElementById("pzDensity").value = p.density;
  document.getElementById("pzGlass").checked = !!p.glass;
  document.getElementById("pzMotion").checked = !!p.motion;
  // Appearance
  const ap = (id, fn) => { const el = document.getElementById(id); if (el) fn(el); };
  ap("apTabsPos", (el) => { el.value = p.tabsPos || "left"; });
  ap("apSidebarSide", (el) => { el.value = p.sidebarSide || "left"; });
  ap("apPanelSide", (el) => { el.value = p.panelSide || "left"; });
  ap("apHideTools", (el) => { el.checked = !!p.hideTools; });
  ap("apFontSel", (el) => { el.value = p.font || "system"; });
  ap("apFontSize", (el) => { el.value = p.fontSize || 13; });
  ap("thBg", (el) => { el.value = p.thBg || "#000000"; });
  ap("thSoft", (el) => { el.value = p.thSoft || "#0b0b0d"; });
  ap("thText", (el) => { el.value = p.thText || "#f2f2f4"; });
  ap("apBgColor", (el) => { el.value = p.bgColor || "#000000"; });
  ap("apBgImg", (el) => { el.value = p.bgImg && !p.bgImg.startsWith("data:") ? p.bgImg : ""; });
  ap("apGlassA", (el) => { el.value = p.glassA || (document.documentElement.getAttribute("data-theme") === "light" ? 90 : 85); });
  ap("apBgDim", (el) => { el.value = p.bgDim == null ? 35 : p.bgDim; });
  ap("apWsCount", (el) => { el.checked = p.wsCount !== false; });
  ap("apSideHover", (el) => { el.checked = !!p.sideHover; });
  ap("apSettingsCols", (el) => { el.checked = !!p.settingsCols; });
  ap("apSettingsW", (el) => { el.value = p.settingsW || 640; });
}

document.getElementById("pzAccent").addEventListener("input", (e) => pzUpdate({ accent: e.target.value }));
document.getElementById("pzRadius").addEventListener("input", (e) => pzUpdate({ radius: +e.target.value }));
document.getElementById("pzSidebar").addEventListener("input", (e) => pzUpdate({ sidebar: +e.target.value }));
document.getElementById("pzDensity").addEventListener("change", (e) => pzUpdate({ density: e.target.value }));
document.getElementById("pzGlass").addEventListener("change", (e) => pzUpdate({ glass: e.target.checked }));
document.getElementById("pzMotion").addEventListener("change", (e) => pzUpdate({ motion: e.target.checked }));
document.getElementById("pzReset").addEventListener("click", () => {
  localStorage.removeItem("apb-ui");
  pzApply();
  pzSyncControls();
  syncPageLayout(true);
});

// --- Appearance page listeners ---
const apOn = (id, ev, fn) => document.getElementById(id)?.addEventListener(ev, fn);
apOn("apTabsPos", "change", (e) => { pzUpdate({ tabsPos: e.target.value }); syncPageLayout(true); setTimeout(syncPageLayout, 240); });
apOn("apSidebarSide", "change", (e) => { pzUpdate({ sidebarSide: e.target.value }); syncPageLayout(true); setTimeout(syncPageLayout, 240); });
apOn("apPanelSide", "change", (e) => { pzUpdate({ panelSide: e.target.value }); syncPageLayout(true); setTimeout(syncPageLayout, 240); });
apOn("apHideTools", "change", (e) => pzUpdate({ hideTools: e.target.checked }));
apOn("apSideHover", "change", (e) => pzUpdate({ sideHover: e.target.checked }));
apOn("apFontSel", "change", (e) => pzUpdate({ font: e.target.value }));
apOn("apFontSize", "input", (e) => pzUpdate({ fontSize: +e.target.value }));
apOn("thBg", "input", (e) => pzUpdate({ thBg: e.target.value }));
apOn("thSoft", "input", (e) => pzUpdate({ thSoft: e.target.value }));
apOn("thText", "input", (e) => pzUpdate({ thText: e.target.value }));
apOn("thReset", "click", () => { pzUpdate({ thBg: "", thSoft: "", thText: "" }); pzSyncControls(); });
apOn("apBgColor", "input", (e) => pzUpdate({ bgColor: e.target.value }));
apOn("apBgImg", "change", (e) => pzUpdate({ bgImg: e.target.value.trim() }));
apOn("apBgReset", "click", () => { pzUpdate({ bgColor: "", bgImg: "" }); pzSyncControls(); });
apOn("exitTopBtn", "click", () => { pzUpdate({ tabsPos: "left" }); syncPageLayout(true); });
apOn("ntTopBtn", "click", () => document.getElementById("newTabBtn").click());
apOn("apGlassA", "input", (e) => pzUpdate({ glassA: +e.target.value }));
apOn("apSound", "change", (e) => pzUpdate({ sound: e.target.checked }));
apOn("apWsCount", "change", (e) => pzUpdate({ wsCount: e.target.checked }));
apOn("apBgDim", "input", (e) => pzUpdate({ bgDim: +e.target.value }));
apOn("apSettingsCols", "change", (e) => pzUpdate({ settingsCols: e.target.checked }));
apOn("apSettingsW", "input", (e) => pzUpdate({ settingsW: +e.target.value }));

// --- Downloads folder setting ---
// Made by MrDuck
async function dlDirRefresh() {
  try {
    const cur = await invoke("dl_dir_get");
    const inp = document.getElementById("dlDirInput");
    if (inp) inp.value = cur || "";
  } catch {}
}
apOn("dlDirSave", "click", async () => {
  const v = document.getElementById("dlDirInput").value.trim();
  try {
    await invoke("dl_dir_set", { path: v });
    toast(v ? "Папка загрузок сохранена" : "Возвращено значение по умолчанию");
  } catch (e) { alert(e); }
});
apOn("dlDirClear", "click", async () => {
  await invoke("dl_dir_set", { path: "" });
  document.getElementById("dlDirInput").value = "";
  toast("По умолчанию — Загрузки Windows");
});
apOn("dlDirOpen", "click", async () => {
  const dir = await invoke("downloads_dir");
  invokeV2("open_in_system", { url: dir }).catch(() => {});
});

// --- Custom theme presets ---
function getPresets() {
  try { return JSON.parse(localStorage.getItem("apb-themes")) || {}; } catch { return {}; }
}
function thPresetRefresh() {
  const sel = document.getElementById("thPresetSel");
  if (!sel) return;
  const ps = getPresets();
  sel.innerHTML = '<option value="">— мои темы —</option>' +
    Object.keys(ps).map((n) => `<option value="${escapeHtml(n)}">${escapeHtml(n)}</option>`).join("");
}
apOn("thApply", "click", () => {
  pzUpdate({
    thBg: document.getElementById("thBg").value,
    thSoft: document.getElementById("thSoft").value,
    thText: document.getElementById("thText").value,
    thLink: document.getElementById("thLink").value,
  });
  toast("Цвета применены");
});
apOn("thReset", "click", () => { pzUpdate({ thBg: "", thSoft: "", thText: "", thLink: "" }); pzSyncControls(); });
apOn("thPresetSave", "click", async () => {
  const name = await prompt("Название темы:", "Моя тема");
  if (!name || !name.trim()) return;
  const ps = getPresets();
  ps[name.trim()] = {
    thBg: document.getElementById("thBg").value,
    thSoft: document.getElementById("thSoft").value,
    thText: document.getElementById("thText").value,
    thLink: document.getElementById("thLink").value,
  };
  localStorage.setItem("apb-themes", JSON.stringify(ps));
  thPresetRefresh();
  document.getElementById("thPresetSel").value = name.trim();
  toast(`Тема «${name.trim()}» сохранена`);
});
apOn("thPresetLoad", "click", () => {
  const name = document.getElementById("thPresetSel").value;
  if (!name) return;
  const t = getPresets()[name];
  if (!t) return;
  pzUpdate({ thBg: t.thBg || "", thSoft: t.thSoft || "", thText: t.thText || "", thLink: t.thLink || "" });
  pzSyncControls();
  toast(`Тема «${name}» применена`);
});
apOn("thPresetDel", "click", () => {
  const name = document.getElementById("thPresetSel").value;
  if (!name) return;
  const ps = getPresets();
  delete ps[name];
  localStorage.setItem("apb-themes", JSON.stringify(ps));
  thPresetRefresh();
});
thPresetRefresh();

// --- Export / import full settings profile (.apbtheme) ---
apOn("skinExportBtn", "click", async () => {
  const payload = {
    app: "apb-settings", version: 1, exported_at: new Date().toISOString(),
    ui: pzLoad(),
    widgets: getWidgetCfg(),
    theme: localStorage.getItem("apb-theme") || "dark",
    pinned: getPinned(),
  };
  try {
    const path = await invoke("save_text_file", {
      name: "apb-settings-" + new Date().toISOString().slice(0, 10) + ".apbtheme",
      contents: JSON.stringify(payload, null, 2),
    });
    toast("Настройки сохранены: " + path);
  } catch (e) { alert("Ошибка экспорта: " + e); }
});
apOn("skinImportBtn", "click", () => document.getElementById("skinImportFile").click());
apOn("skinImportFile", "change", async (e) => {
  const file = e.target.files && e.target.files[0];
  e.target.value = "";
  if (!file) return;
  let data;
  try { data = JSON.parse(await file.text()); } catch { alert("Файл повреждён или это не .apbtheme."); return; }
  if (data.app !== "apb-settings") { alert("Это не файл настроек APB."); return; }
  if (!(await confirm("Применить настройки из файла? Текущие будут заменены."))) return;
  if (data.ui) localStorage.setItem("apb-ui", JSON.stringify(data.ui));
  if (data.widgets) localStorage.setItem("apb-widgets", JSON.stringify(data.widgets));
  if (data.theme) localStorage.setItem("apb-theme", data.theme);
  if (Array.isArray(data.pinned)) localStorage.setItem("apb-pinned", JSON.stringify(data.pinned));
  applyTheme(localStorage.getItem("apb-theme") || "dark");
  pzApply(); pzSyncControls(); applyWidgetCfg(); renderWsPills(); renderHome(); syncPageLayout(true);
  setTimeout(syncPageLayout, 240);
  toast("Настройки применены ✓");
});
// Background image from local file — downscaled and stored as data-URL
apOn("apBgFileBtn", "click", () => document.getElementById("apBgFile").click());
apOn("apBgFile", "change", (e) => {
  const file = e.target.files && e.target.files[0];
  if (!file) return;
  const img = new Image();
  const reader = new FileReader();
  reader.onload = () => { img.src = reader.result; };
  img.onload = () => {
    // Downscale so it fits comfortably into localStorage
    const maxW = 1920;
    const scale = Math.min(1, maxW / img.naturalWidth);
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(img.naturalWidth * scale);
    canvas.height = Math.round(img.naturalHeight * scale);
    canvas.getContext("2d").drawImage(img, 0, 0, canvas.width, canvas.height);
    let data;
    try { data = canvas.toDataURL("image/jpeg", 0.85); } catch { alert("Не удалось обработать картинку"); return; }
    if (data.length > 4_500_000) { alert("Картинка слишком большая даже после сжатия — выберите другую."); return; }
    pzUpdate({ bgImg: data });
    const inp = document.getElementById("apBgImg");
    if (inp) inp.value = "";
  };
  reader.readAsDataURL(file);
  e.target.value = "";
});

pzApply();

// ---------------------------------------------------------------------
// Resizable panes — drag the left edge of editor/AI panes and the right
// edge of the side panel to resize them with the mouse.
// ---------------------------------------------------------------------

function makeResizer(handleId, paneId, { persistKey, min = 280 }) {
  const h = document.getElementById(handleId);
  const p = document.getElementById(paneId);
  if (!h || !p) return;
  let startX = 0, startW = 0;
  h.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    h.setPointerCapture(e.pointerId);
    startX = e.clientX;
    startW = p.getBoundingClientRect().width;
    document.body.classList.add("resizing");
    h.classList.add("active");
  });
  h.addEventListener("pointermove", (e) => {
    if (!h.hasPointerCapture || !h.hasPointerCapture(e.pointerId)) return;
    const w = Math.round(startW + (startX - e.clientX));
    p.style.width = Math.min(Math.max(w, min), Math.round(window.innerWidth * 0.7)) + "px";
    p.style.flex = "0 0 " + p.style.width;
  });
  const end = (e) => {
    if (!h.hasPointerCapture || !h.hasPointerCapture(e.pointerId)) return;
    h.releasePointerCapture(e.pointerId);
    document.body.classList.remove("resizing");
    h.classList.remove("active");
    syncPageLayout(true);
    setTimeout(syncPageLayout, 80);
    if (persistKey) {
      const w = parseInt(p.style.width, 10);
      if (w) pzUpdate({ [persistKey]: w });
    }
  };
  h.addEventListener("pointerup", end);
  h.addEventListener("pointercancel", end);
}
makeResizer("edResizer", "editorPane", { persistKey: "editorW", min: 340 });
makeResizer("aiResizer", "aiPane", { persistKey: "aiW", min: 320 });
makeResizer("panelResizer", "sidePanel", { persistKey: "panelW", min: 240 });

// ---------------------------------------------------------------------
// UI TOUR — игровые подсказки: мини-панель возле элемента + пульсирующая
// подсветка. Экран НЕ блокирует (без оверлея), интерфейс остаётся живым.
// Тур идёт НЕ только по главной: шаги сами открывают внутренние страницы
// (настройки/оформление/история/сейф/расширения) и боковые панели.
// Показывается один раз после онбординга; повтор — Настройки → Оформление.
// ---------------------------------------------------------------------

const TOUR_KEY = "apb-ui-tour";
// Made by MrDuck
const TOUR_STEPS = [
  { center: true, title: "👋 Добро пожаловать в AP Browser",
    text: "AP Browser (ApostolProject Browser) — десктопный браузер с упором на приватность. Внутри: настоящие вкладки на движке WebView2, воркспейсы со своими наборами вкладок, изолированные профили, заметки с графом знаний, сейф паролей и AI-ассистент, который умеет работать полностью локально. Никакой телеметрии — все данные остаются только на твоём компьютере. Жми «Далее» — проведу по главным кнопкам." },
  { sel: "#collapseBtn", title: "Сворачивание панели",
    text: "Схлопывает боковую панель в узкий рельс с иконками. Повторный клик разворачивает обратно. Открытие по наведению включается в Оформлении." },
  { sel: "#newTabBtn", title: "Новая вкладка",
    text: "Кнопка или Ctrl+T. Новые вкладки появляются вверху списка, у каждой открытой — иконка сайта." },
  { sel: "#tabstripScroll", title: "Вкладки",
    text: "Клик — перейти, × — закрыть, ПКМ по вкладке — контекстное меню." },
  { sel: "#wsStrip", title: "Воркспейсы",
    text: "Отдельные рабочие пространства со своим набором вкладок. + — создать, двойной клик — переименовать, ПКМ — дублировать или удалить." },
  { sel: "#addressInput", title: "Омнибокс",
    text: "Адрес или поисковый запрос — Enter. Кнопка ☾ справа включает принудительную тёмную тему для сайтов без своей." },
  { sel: ".nav-group", title: "Навигация",
    text: "Назад, вперёд и обновить. История ведётся отдельно для каждой вкладки." },
  { sel: "#profileSelect", title: "Профили",
    text: "Полная изоляция: свои cookie, история и загрузки. Кнопка + рядом создаёт профиль, в том числе анонимный без следов." },
  { sel: ".side-tools", title: "Инструменты",
    text: "Закладки, история, загрузки, заметки с графом связей, приватность и AI-чат — всегда внизу панели." },

  { action: () => { openInternal("settings"); }, sel: ".isec-nav",
    title: "Внутренние страницы",
    text: "Настройки, оформление, история, пароли и расширения живут в одной оболочке — переключаются этими вкладками." },
  { action: () => { openInternal("appearance"); }, sel: "#appearance .internal-title",
    title: "Оформление",
    text: "Темы, акценты, шрифты, фон главной страницы. Здесь же экспорт/импорт настроек и кнопка ❓ повторного тура подсказок." },
  { action: () => { openInternal("history"); }, sel: "#histScopeSelect",
    title: "История",
    text: "Полная история с фильтром. Можно смотреть либо текущий профиль, либо сразу все — записи сгруппированы по дням." },
  { action: () => { openInternal("vault"); }, sel: ["#vaultContentBox", "#vaultSetupBox"],
    title: "Сейф паролей",
    text: "Локальное хранилище паролей: AES-256-GCM + Argon2id. Данные никогда не покидают компьютер. Импорт/экспорт JSON." },
  { action: () => { openInternal("extensions"); }, sel: "#extensions .internal-title",
    title: "Расширения",
    text: "Установка из папки с manifest.json, разрешения выдаются по профилю." },

  { action: () => {
      const ih = document.getElementById("internalHost");
      if (ih && !ih.classList.contains("hidden")) closeInternal(false);
      showHome();
      const p = document.getElementById("aiPane");
      if (p.classList.contains("hidden")) document.querySelector('.rail-item[data-tab="ai"]').click();
    }, sel: "#aiPane .editor-head",
    title: "AI-ассистент",
    text: "Чат прямо в браузере: Ollama работает полностью локально, поддерживаются OpenAI-совместимые API. Умеет учитывать текст открытой страницы и переводить её." },
  { action: () => {
      const p = document.getElementById("aiPane");
      if (!p.classList.contains("hidden")) document.getElementById("aiClose").click();
      document.querySelector('.rail-item[data-tab="bookmarks"]').click();
    }, sel: "#bookmarks .panel-title",
    title: "Закладки",
    text: "Поиск по названию, адресу и тегам. Добавление — через + или ПКМ на странице." },
  { action: () => {
      document.querySelector('.rail-item[data-tab="downloads"]').click();
    }, sel: "#downloads .panel-title",
    title: "Загрузки",
    text: "Файлы ловятся автоматически и складываются в папку загрузок активного профиля. Клик по строке — открыть папку." },
  { action: () => {
      // Гарантированно открываем секцию заметок и форму создания
      openSidePanel("notes");
      const det = document.querySelector("#notes .add-form");
      if (det) det.setAttribute("open", "");
    }, sel: "#notes .panel-title",
    title: "Заметки — markdown с суперсилой",
    text: "Создавай заметку кнопкой «Сохранить» — имя может содержать папку: Работа/Идея.md. Внутри: markdown, [[вики-ссылки]] между заметками, #теги, чекбоксы, картинки и даже формулы LaTeX ($E=mc^2$ или $$\\frac{a}{b}$$). Кнопка «?» на панели редактора — шпаргалка по синтаксису." },
  { action: async () => {
      // Открываем первую заметку в редакторе (если есть)
      const li = document.querySelector("#notesList li[data-file]");
      if (!li) return;
      try {
        const content = await invoke("read_note", { path: li.dataset.file });
        openEditor(li.dataset.file, content);
      } catch (_) {}
    }, sel: ".editor-tabs",
    title: "Редактор: Рисование и Просмотр",
    text: "Над текстом четыре вкладки. «Рисование» — рисуй мышью пером, маркером и фигурами, а кнопка «⬇ В заметку» вставит рисунок прямо в текст. «Просмотр» — красивый рендер: заголовки, списки, таблицы, картинки и формулы LaTeX ($x^2$, $$\\frac{a}{b}$$). Кнопка «⬇» в шапке сохраняет заметку как .md файл." },
  { action: async () => {
      // Открываем первую заметку и сразу вид «Граф» (если заметки есть)
      const li = document.querySelector("#notesList li[data-file]");
      if (!li) return;
      const file = li.dataset.file;
      try {
        const content = await invoke("read_note", { path: file });
        openEditor(file, content);
        setEtab("graph");
      } catch (_) {}
    }, sel: "#etabGraph .graph-tools",
    title: "Граф знаний",
    text: "Визуальная доска связей. 2×ПКМ по фону — создать блок, ПКМ-тянуть от фигуры или за ● точку на её краю — провести связь, колесо — зум, ПКМ — панорама, ЛКМ-рамка — выделить группу, Del удаляет блоки и линии, Ctrl+Z отменяет. Блоки можно перетаскивать, клик по заметке открывает её. Все жесты — в ❓ Подсказке внутри графа." },
  { action: () => {
      const p = document.getElementById("aiPane");
      if (!p.classList.contains("hidden")) document.getElementById("aiClose").click();
      closeSidePanel();
      const ec = document.getElementById("edClose");
      if (ec) ec.click();
      showHome();
    }, center: true, title: "🎉 Готово! Вы во всём разобрались",
    text: "Шпаргалка на будущее: Ctrl+T — новая вкладка, Ctrl+K — палитра команд, ПКМ почти везде открывает своё меню. Профили полностью изолируют данные, сейф паролей шифруется AES-256-GCM с ключом Argon2id и не покидает компьютер, а AI-чат работает даже без интернета через Ollama. Заметки понимают markdown, [[вики-ссылки]] и рисуются графом связей — открой любую заметку → вкладка «Граф». Вернуть этот тур: Настройки → Оформление → ❓ Подсказки. Приятного пользования!" },
];

let _tourIdx = -1;
let _tourDimEl = null, _tourGhostEl = null;

function _tourEls() {
  return {
    hl: document.getElementById("tourHl"),
    pop: document.getElementById("tourPop"),
  };
}

function _tourTarget(sel) {
  const list = Array.isArray(sel) ? sel : [sel];
  for (const s of list) {
    const el = document.querySelector(s);
    if (!el) continue;
    const r = el.getBoundingClientRect();
    if (r.width < 2 && r.height < 2) continue;
    return el;
  }
  return null;
}

function startUITour(force) {
  if (!force && localStorage.getItem(TOUR_KEY) === "done") return;
  stopUITour(true); // перезапуск поверх

  // Затемнение-блокировка: ровно 4 шторы вокруг одной «дырки» (цели).
  // Изначально каждая покрывает весь экран → на старте плавно схлопываются.
  const dim = document.createElement("div");
  dim.id = "tourDim";
  for (let i = 0; i < 4; i++) {
    const s = document.createElement("div");
    s.className = "td-shade";
    s.style.top = "0px";
    s.style.left = "0px";
    s.style.width = window.innerWidth + "px";
    s.style.height = window.innerHeight + "px";
    dim.appendChild(s);
  }
  document.body.appendChild(dim);
  _tourDimEl = dim;
  requestAnimationFrame(() => dim.classList.add("show"));

  // Прозрачный «мост» над кнопками окна: они остаются ЗАТЕМНЁННЫМИ
  // визуально, но клики сквозь шторы пробрасываются на настоящие кнопки.
  const ghost = document.createElement("div");
  ghost.id = "tourWcGhost";
  const fwd = (e) => {
    ghost.style.pointerEvents = "none";
    const el = document.elementFromPoint(e.clientX, e.clientY);
    ghost.style.pointerEvents = "";
    if (!el || el === ghost) return;
    const Ev = e.type.startsWith("pointer") ? PointerEvent : MouseEvent;
    el.dispatchEvent(new Ev(e.type, {
      bubbles: true, cancelable: true,
      clientX: e.clientX, clientY: e.clientY, button: 0,
    }));
    e.preventDefault();
  };
  ghost.addEventListener("pointerdown", fwd);
  ghost.addEventListener("pointerup", fwd);
  ghost.addEventListener("click", fwd);
  document.body.appendChild(ghost);
  _tourGhostEl = ghost;

  const hl = document.createElement("div");
  hl.id = "tourHl";
  const pop = document.createElement("div");
  pop.id = "tourPop";
  document.body.append(hl, pop);
  window.addEventListener("resize", _tourOnResize);
  document.addEventListener("keydown", _tourOnKey, true);
  // Стартуем с чистого экрана: закрываем настройки/панели и идём на главную
  try {
    const ih = document.getElementById("internalHost");
    if (ih && !ih.classList.contains("hidden")) closeInternal(false);
    const apn = document.getElementById("aiPane");
    if (apn && !apn.classList.contains("hidden")) {
      const x = document.getElementById("aiClose");
      if (x) x.click();
    }
    closeSidePanel();
    showHome();
  } catch (_) {}
  _tourIdx = -1;
  _tourNext();
}

function stopUITour(markDone) {
  const { hl, pop } = _tourEls();
  if (hl) hl.remove();
  if (pop) pop.remove();
  if (_tourDimEl) {
    const d = _tourDimEl;
    d.classList.remove("show");           // плавное затухание
    setTimeout(() => d.remove(), 300);
  }
  _tourDimEl = null;
  if (_tourGhostEl) { _tourGhostEl.remove(); _tourGhostEl = null; }
// Made by MrDuck
  window.removeEventListener("resize", _tourOnResize);
  document.removeEventListener("keydown", _tourOnKey, true);
  _tourIdx = -1;
  if (markDone === true) localStorage.setItem(TOUR_KEY, "done");
}

// Спотлайт: 4 шторы с фиксированными ролями вокруг одной дырки (цели).
// Роли не меняются между шагами → шторы плавно переезжают, без прыжков.
// Кнопки окна остаются затемнёнными, но ghost-слой пробрасывает им клики.
function _tourSpotlight(targetRect) {
  if (!_tourDimEl) return;
  const vw = window.innerWidth, vh = window.innerHeight, pad = 8;
  let hx = 0, hy = 0, hw = 0, hh = 0;
  if (targetRect) {
    hx = Math.max(0, targetRect.left - pad);
    hy = Math.max(0, targetRect.top - pad);
    hw = Math.min(vw - hx, targetRect.width + pad * 2);
    hh = Math.min(vh - hy, targetRect.height + pad * 2);
  }
  const st = (el, o) => {
    for (const k of ["top", "left", "width", "height"]) el.style[k] = o[k] + "px";
  };
  const shades = _tourDimEl.querySelectorAll(".td-shade");
  if (!shades || shades.length < 4) return;
  st(shades[0], { top: 0, left: 0, width: vw, height: hy });                 // сверху
  st(shades[1], { top: hy + hh, left: 0, width: vw, height: vh - hy - hh }); // снизу
  st(shades[2], { top: hy, left: 0, width: hx, height: hh });                // слева
  st(shades[3], { top: hy, left: hx + hw, width: vw - hx - hw, height: hh }); // справа

  if (_tourGhostEl) {
    const wc = document.querySelector(".win-controls");
    if (wc) {
      const wr = wc.getBoundingClientRect();
      _tourGhostEl.style.display = "";
      _tourGhostEl.style.top = wr.top - 4 + "px";
      _tourGhostEl.style.left = wr.left - 4 + "px";
      _tourGhostEl.style.width = wr.width + 8 + "px";
      _tourGhostEl.style.height = wr.height + 8 + "px";
    } else {
      _tourGhostEl.style.display = "none";
    }
  }
}

function _tourOnResize() {
  if (_tourIdx < 0 || _tourIdx >= TOUR_STEPS.length) return;
  const st = TOUR_STEPS[_tourIdx];
  if (st.center) return; // центральная карточка не зависит от ресайза целей
  const el = _tourTarget(st.sel);
  if (el) {
    _tourSpotlight(el.getBoundingClientRect());
    _tourRender(st, el);
  }
}
function _tourOnKey(e) {
  if (e.key !== "Escape") return;
  e.stopPropagation();
  stopUITour(true);
}

function _tourNext() {
  void _tourAdvance(1);
}

function _tourBack() {
  void _tourAdvance(-1);
}

/** Общий проход по шагам: выполняет action (может быть async — например,
 *  открыть заметку и включить вид «Граф»), затем рендерит подходящий шаг. */
async function _tourAdvance(dir) {
  for (let guard = 0; guard < 60; guard++) {
    const ni = _tourIdx + dir;
    if (ni < 0 || ni >= TOUR_STEPS.length) {
      if (dir > 0) {
        stopUITour(true);
        toast("Тур завершён. Вернуть: Настройки → Оформление → ❓ Подсказки", "ok");
      }
      return;
    }
    _tourIdx = ni;
    const st = TOUR_STEPS[ni];
    try { if (st.action) await st.action(); } catch (_) {}
    if (st.center || _tourTarget(st.sel)) return _tourRender(st);
    // скрытая цель — молча идём дальше в том же направлении
  }
}

function _tourRender(step, targetEl) {
  const { hl, pop } = _tourEls();
  if (!pop) { stopUITour(true); return; }

  let r = null;
  if (step.center) {
    // Центральная карточка: без кольца, спотлайт схлопывается в точку
    if (hl) hl.remove();
    _tourSpotlight(null);
    pop.classList.add("tp-center");
    pop.style.left = ""; pop.style.top = "";
  } else {
    pop.classList.remove("tp-center");
    const el = targetEl || _tourTarget(step.sel);
    // ВАЖНО: hl здесь может быть null (после центр-шага кольцо удалено) —
    // это не ошибка, кольцо пересоздаётся ниже. Глушим тур только без цели.
    if (!el) { stopUITour(true); return; }
    r = el.getBoundingClientRect();
    _tourSpotlight(r);
    // Кольцо пересоздаётся каждый шаг — анимации появления переигрываются
    if (hl) hl.remove();
    const ring = document.createElement("div");
    ring.id = "tourHl";
    ring.style.left = r.left - 4 + "px";
    ring.style.top = r.top - 4 + "px";
    ring.style.width = r.width + 8 + "px";
    ring.style.height = r.height + 8 + "px";
    document.body.appendChild(ring);
  }

  const last = _tourIdx === TOUR_STEPS.length - 1;
  pop.innerHTML = `
    <div class="tp-in">
    <div class="tp-head">
      <span class="tp-step">${_tourIdx + 1} / ${TOUR_STEPS.length}</span>
      <button class="tp-x" title="Закрыть подсказки">×</button>
    </div>
    <h3>${step.title}</h3>
    <p>${step.text}</p>
    <div class="tp-foot">
      <button class="ghost-btn small tp-skip">Пропустить всё</button>
      ${_tourIdx === 0 ? "" : `<button class="ghost-btn small tp-back">Назад</button>`}
      <button class="primary-btn tp-next">${last ? "Готово ✓" : "Далее →"}</button>
    </div>
    </div>`;

  // Центральная карточка: без позиционирования (CSS .tp-center), только хендлеры
  if (step.center) {
    pop.querySelector(".tp-x").onclick = () => stopUITour(true);
    pop.querySelector(".tp-skip").onclick = () => stopUITour(true);
    pop.querySelector(".tp-next").onclick = () => _tourNext();
    const bbC = pop.querySelector(".tp-back");
    if (bbC) bbC.onclick = () => _tourBack();
    return;
  }

  // УМНОЕ ПОЗИЦИРОВАНИЕ: выбираем сторону с максимальным запасом места,
  // приоритет снизу→сверху→справа→слева; высокие цели обходим сбоку.
  pop.style.visibility = "hidden";
  pop.style.left = "0px"; pop.style.top = "0px";
  requestAnimationFrame(() => {
    const pw = pop.offsetWidth, ph = pop.offsetHeight;
    const vw = window.innerWidth, vh = window.innerHeight, m = 12, edge = 12;
    const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
    const tall = r.height > vh * 0.5;
    let cands;
    if (tall) {
      cands = [
        { x: r.right + m, y: Math.min(r.top + 8, vh - ph - edge) },
        { x: r.left - pw - m, y: Math.min(r.top + 8, vh - ph - edge) },
      ];
    } else {
      cands = [
        { x: cx - pw / 2, y: r.bottom + m },          // снизу (приоритет)
        { x: cx - pw / 2, y: r.top - ph - m },        // сверху
        { x: r.right + m, y: cy - ph / 2 },           // справа
        { x: r.left - pw - m, y: cy - ph / 2 },       // слева
      ];
    }
    let best = null;
    for (let i = 0; i < cands.length; i++) {
      const c = cands[i];
      const ox = Math.max(0, edge - c.x) + Math.max(0, c.x + pw - (vw - edge));
      const oy = Math.max(0, edge - c.y) + Math.max(0, c.y + ph - (vh - edge));
      const fits = ox === 0 && oy === 0;
      // чем раньше в списке и чем меньше переполнение — тем лучше
      const score = (fits ? 1e6 : 0) - (ox + oy) * 10 - i;
      if (!best || score > best.score) best = { x: c.x, y: c.y, score };
    }
    const clampX = (v) => Math.min(Math.max(v, edge), Math.max(edge, vw - pw - edge));
    const clampY = (v) => Math.min(Math.max(v, edge), Math.max(edge, vh - ph - edge));
    pop.style.left = clampX(best.x) + "px";
    pop.style.top = clampY(best.y) + "px";
    pop.style.visibility = "";
  });

  pop.querySelector(".tp-x").onclick = () => stopUITour(true);
  pop.querySelector(".tp-skip").onclick = () => stopUITour(true);
  pop.querySelector(".tp-next").onclick = () => _tourNext();
  const bb = pop.querySelector(".tp-back");
  if (bb) bb.onclick = () => _tourBack();
}

// Автозапуск для тех, кто уже прошёл онбординг, но тур не видел
setTimeout(() => {
  if (document.getElementById("onboardingOverlay")) return; // онбординг сам запустит
  if (localStorage.getItem(TOUR_KEY) !== "done") startUITour(false);
}, 1400);

apOn("tourReplayBtn", "click", () => startUITour(true));


// Made by MrDuck