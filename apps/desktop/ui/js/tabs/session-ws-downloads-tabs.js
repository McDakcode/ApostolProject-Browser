// Made by MrDuck
// ---------------------------------------------------------------------
// Session persistence — open tabs are restored after a restart
// ---------------------------------------------------------------------

let activeProfileName = "";
let activeStorageMode = "Persistent";
let sessionTimer = null;

// Вкладки, которые попадают в сессию/воркспейс: пустые «новые вкладки»
// (пилюля без вебвью) сохранять нечего — фильтруем.
const savableTabs = () => tabs.filter((t) => !t.isNew && t.url);

// ---------------------------------------------------------------------
// Workspaces — named tab groups per profile (Zen-style pills)
// ---------------------------------------------------------------------

let wsDoc = null;

async function wsInit() {
  try { wsDoc = await invoke("workspaces_get"); } catch { return; }
  if (!wsDoc || !Array.isArray(wsDoc.list) || !wsDoc.list.length) return;
  const cur = wsDoc.list[wsDoc.current] || wsDoc.list[0];
  // Migrate: if the workspace is empty but a session was restored, adopt it
  if (cur && (!Array.isArray(cur.tabs) || !cur.tabs.length) && tabs.length) {
    const st = savableTabs();
    cur.tabs = st.map((t) => t.url);
    cur.active = Math.max(0, st.findIndex((t) => t.id === activeTabId));
    await saveWs();
  }
  renderWsPills();
}

async function saveWs() {
  if (!wsDoc) return;
  try { await invoke("workspaces_set", { data: wsDoc }); } catch { /* non-critical */ }
}

function renderWsPills() {
  const strip = document.getElementById("wsStrip");
  if (!strip || !wsDoc) return;
  strip.innerHTML = "";
  (wsDoc.list || []).forEach((w, i) => {
    const pill = document.createElement("div");
    pill.className = "ws-pill" + (i === wsDoc.current ? " active" : "");
    pill.dataset.i = i; // для контекст-меню
    pill.dataset.letter = (w.name || "?").trim()[0] || "?"; // иконка в свёрнутом рельсе
    pill.title = w.name + " · двойной клик — переименовать";
    const nm = document.createElement("span");
    nm.textContent = w.name;
    pill.appendChild(nm);
    const cnt = document.createElement("span");
    cnt.className = "ws-count";
    cnt.textContent = Array.isArray(w.tabs) ? w.tabs.length : 0;
    pill.appendChild(cnt);
    if ((wsDoc.list || []).length > 1) {
      const x = document.createElement("button");
      x.className = "ws-x";
      x.textContent = "×";
      x.title = "Удалить воркспейс";
      x.onclick = (ev) => { ev.stopPropagation(); deleteWs(i); };
      pill.appendChild(x);
    }
    pill.onclick = () => switchWs(i);
    pill.ondblclick = async (ev) => {
      ev.stopPropagation();
      const nn = await prompt("Имя воркспейса:", w.name);
      if (nn && nn.trim()) { w.name = nn.trim(); await saveWs(); renderWsPills(); }
    };
    strip.appendChild(pill);
  });
  // Mirror into the top strip (tabs-on-top mode)
  const top = document.getElementById("wsStripTop");
  if (top) {
    top.innerHTML = "";
    for (const child of [...strip.children]) top.appendChild(child.cloneNode(true));
  }
}

async function closeAllPageTabs() {
  for (const t of [...tabs]) {
    try { await invoke("page_close", { id: t.id }); } catch { /* already gone */ }
  }
  tabs = [];
  activeTabId = null;
}

async function enterWorkspace(w) {
  await closeAllPageTabs();
  const list = Array.isArray(w.tabs) ? w.tabs : [];
  for (const u of list.slice(-15)) {
    try { await createTab(u, null, { append: true }); } catch { /* skip */ }
  }
  const ai = typeof w.active === "number" && tabs[w.active] ? w.active : tabs.length - 1;
  if (tabs[ai]) switchTab(tabs[ai].id);
  else { openNewTabPage(); }
  renderWsPills();
  scheduleSessionSave();
}

async function switchWs(i) {
  if (!wsDoc || i === wsDoc.current) return;
  await persistSessionNow();          // store tabs into the OLD workspace
  wsDoc.current = i;
  await saveWs();
  await enterWorkspace(wsDoc.list[i]);
}

document.getElementById("wsAdd").addEventListener("click", async () => {
  if (!wsDoc || !Array.isArray(wsDoc.list)) return;
  const name = await prompt("Название нового воркспейса:", "Воркспейс " + (wsDoc.list.length + 1));
  if (!name || !name.trim()) return;
  await persistSessionNow();          // keep current tabs in their workspace
  wsDoc.current = wsDoc.list.length;
  wsDoc.list.push({ id: Date.now(), name: name.trim(), tabs: [], active: 0 });
  await saveWs();
  await enterWorkspace(wsDoc.list[wsDoc.current]);
});

async function deleteWs(i) {
  if (!wsDoc || wsDoc.list.length <= 1) return;
  const w = wsDoc.list[i];
  if (!(await confirm(`Удалить воркспейс «${w.name}»? Его вкладки будут потеряны.`))) return;
  const wasCurrent = i === wsDoc.current;
  wsDoc.list.splice(i, 1);
  if (wsDoc.current >= i) wsDoc.current = Math.max(0, wsDoc.current - 1);
  await saveWs();
  if (wasCurrent) await enterWorkspace(wsDoc.list[wsDoc.current]);
  else renderWsPills();
}

async function persistSessionNow() {
  const st = savableTabs();
  try {
    await invoke("session_save", {
      session: {
        tabs: st.map((t) => ({ url: t.url, label: t.label })),
        active: Math.max(0, st.findIndex((t) => t.id === activeTabId)),
      },
    });
  } catch { /* non-critical */ }
  // Mirror into the current workspace
  if (wsDoc && Array.isArray(wsDoc.list) && wsDoc.list.length) {
    const w = wsDoc.list[wsDoc.current];
    if (w) {
      w.tabs = st.map((t) => t.url);
      w.active = Math.max(0, st.findIndex((t) => t.id === activeTabId));
      saveWs();
    }
  }
}

// ---------------------------------------------------------------------
// Downloads — engine-level interception, rendered live via events
// ---------------------------------------------------------------------

let dlItems = [];

function hostOf(url) {
  try { return hostnameOf(url); } catch { return url; }
}

function dlRowEl(item) {
  const li = document.createElement("li");
  const cls = item.status === "done" ? "st-done" : item.status === "failed" ? "st-failed" : "st-active";
  li.innerHTML = `<div class="dl-main"><div class="dl-nm"></div><div class="meta"><span class="dl-chip ${cls}"></span> <span class="dl-host"></span></div></div>`;
  li.querySelector(".dl-nm").textContent = item.file_name.length > 42
    ? item.file_name.slice(0, 30) + "…" + item.file_name.slice(-10)
    : item.file_name;
  li.querySelector(".dl-chip").textContent =
    item.status === "done" ? "готово" : item.status === "failed" ? "ошибка" : "загружается…";
  li.querySelector(".dl-host").textContent = hostOf(item.url);
  li.title = item.path;
  li.onclick = async () => {
    const dir = item.path.replace(/[\\/][^\\/]+$/, "");
    invoke("open_in_system", { url: dir }).catch(() => {});
  };
  return li;
}

function renderDownloads() {
  const ul = document.getElementById("dlList");
  if (!ul) return;
  ul.innerHTML = "";
  if (!dlItems.length) {
    ul.innerHTML = '<li class="empty">Загрузок пока нет</li>';
    return;
  }
  for (const it of dlItems) ul.appendChild(dlRowEl(it));
}

async function refreshDownloads() {
  try {
    dlItems = await invoke("downloads_list");
    renderDownloads();
  } catch { /* ignore */ }
}

try {
// Made by MrDuck
  window.__TAURI__.event.listen("dl-update", (e) => {
    const it = e.payload;
    const i = dlItems.findIndex((d) => d.id === it.id || (d.path === it.path && d.status === "downloading"));
    if (i >= 0) dlItems[i] = it; else dlItems.unshift(it);
    renderDownloads();
  });
} catch { /* events unavailable */ }

document.getElementById("dlOpenFolder")?.addEventListener("click", async () => {
  try {
    const dir = await invoke("downloads_dir");
    invoke("open_in_system", { url: dir }).catch(() => {});
  } catch (e) { alert(e); }
});

function scheduleSessionSave() {
  if (activeStorageMode !== "Persistent") return;
  clearTimeout(sessionTimer);
  sessionTimer = setTimeout(persistSessionNow, 700);
}

// Перед закрытием окна дампнем сессию немедленно (без 700мс-задержки), чтобы
// перезапуск поднял ровно тот же набор вкладок и URL — включая SPA-переходы,
// чей URL пришёл через page-url-changed прямо перед закрытием.
window.addEventListener("beforeunload", () => { void persistSessionNow(); });
window.addEventListener("pagehide", () => { void persistSessionNow(); });

// ---------------------------------------------------------------------
// Tabs — in-window, rendered as <iframe> panes. No native windows, so
// nothing to spawn or hang.
//
// Known limitation: sites sending X-Frame-Options / CSP frame-ancestors
// (Google, YouTube, most banks, many modern SPAs) refuse to render
// inside any iframe, by design of those sites — this is a browser-wide
// constraint of the iframe approach, not a bug here.
// ---------------------------------------------------------------------

let tabs = [];
let activeTabId = null;

function activeTab() {
  return tabs.find((t) => t.id === activeTabId) || null;
}

function makeTabId() {
  return "tab-" + Date.now().toString(36) + "-" + Math.random().toString(36).slice(2, 8);
}

// ---------------------------------------------------------------------
// Favicon кэш + адресная строка «название сайта / полный URL»
// ---------------------------------------------------------------------

// Иконки тянутся ТОЛЬКО с самого сайта (/favicon.ico) — без сторонних
// сервисов (приватность). Результат запоминается в localStorage: удачное —
// показываем сразу (в т.ч. у спящих вкладок и после перезапуска), неудачное
// («иконки нет») — чтобы не дёргать URL повторно каждым рендером.
const FAV_CACHE_KEY = "apb-favicons";
function favCacheLoad() {
  try {
    const c = JSON.parse(localStorage.getItem(FAV_CACHE_KEY));
    return c && typeof c === "object" ? c : {};
  } catch { return {}; }
}
function favCacheSave(c) { try { localStorage.setItem(FAV_CACHE_KEY, JSON.stringify(c)); } catch {} }
function favOriginOf(url) {
  if (!url) return null;
  try { const u = new URL(url); return /^https?:$/.test(u.protocol) ? u.origin : null; } catch { return null; }
}
function favCached(origin) {
  const c = favCacheLoad();
  return origin in c ? c[origin] : null; // "" = известно, что иконки нет
}
function favRemember(origin, src) {
  const c = favCacheLoad();
  c[origin] = src;
  favCacheSave(c);
}
/** Готовит <img> с фавиконом сайта. true = картинка показывается,
 *  false = остаётся буква-фолбэк (иконки нет / спящая вкладка без кэша). */
function favAttach(img, url, opts = {}) {
  const origin = favOriginOf(url);
  if (!origin) return false;
  const cached = favCached(origin);
  if (cached === "") return false;                       // иконки точно нет
  if (opts.noNet && cached == null) return false;        // спящая — сеть не дёргаем
  const src = cached != null ? cached : origin + "/favicon.ico";
  img.onload = () => favRemember(origin, src);
  img.onerror = () => { favRemember(origin, ""); img.remove(); };
  img.src = src;
  return true;
}
window.__favAttach = favAttach;
window.__favOriginOf = favOriginOf;
window.__favCached = favCached;
window.__favRemember = favRemember;

// Кэш реальных заголовков страниц (<title>): подстановка сразу, даже у
// спящих вкладок (восстановленных сессий) — «название видео» вместо домена.
const TITLE_CACHE_KEY = "apb-titles";
function titleCacheLoad() {
  try {
    const c = JSON.parse(localStorage.getItem(TITLE_CACHE_KEY));
    return c && typeof c === "object" ? c : {};
  } catch { return {}; }
}
function titleCacheSave(c) { try { localStorage.setItem(TITLE_CACHE_KEY, JSON.stringify(c)); } catch {} }
function titleCacheFor(url) {
  if (!url) return "";
  try {
    const c = titleCacheLoad();
    return c[url] || "";
  } catch { return ""; }
}
function titleCacheRemember(url, title) {
  if (!url || !title) return;
  try {
    const c = titleCacheLoad();
    if (c[url] === title) return;
    c[url] = title;
    const keys = Object.keys(c);
    if (keys.length > 800) {
      for (let i = 0; i < keys.length - 800; i++) delete c[keys[i]];
    }
    titleCacheSave(c);
  } catch {}
}
window.__titleCacheFor = titleCacheFor;

/** Лучшая известная подпись для URL: реальный <title> из кэша → переданный
 *  label → поисковый запрос из URL → домен. */
function smartTitle(url, label) {
  const c = titleCacheFor(url);
  if (c) return c;
  return label || labelFromUrl(url) || hostnameOf(url);
}
window.__smartTitle = smartTitle;

// Адресная строка: показываем НАЗВАНИЕ сайта + иконку слева; клик (фокус)
// раскрывает полный URL для редактирования; потеря фокуса без правок снова
// возвращает название.
const addrInput = document.getElementById("addressInput");
const addrFavEl = document.getElementById("addrFav");
let _addrUrl = "", _addrTitle = "";

function setAddrFav(url) {
  const form = document.getElementById("addressForm");
  if (!addrFavEl || !form) return;
  const origin = favOriginOf(url);
  if (!origin) {
    addrFavEl.classList.add("hidden");
    form.classList.remove("has-fav");
    return;
  }
  const cached = favCached(origin);
  if (cached === "") {
    addrFavEl.classList.add("hidden");
    form.classList.remove("has-fav");
    return;
  }
  const src = cached != null ? cached : origin + "/favicon.ico";
  addrFavEl.onload = () => favRemember(origin, src);
  addrFavEl.onerror = () => {
    favRemember(origin, "");
    addrFavEl.classList.add("hidden");
    form.classList.remove("has-fav");
  };
  addrFavEl.src = src;
  addrFavEl.classList.remove("hidden");
  form.classList.add("has-fav");
}

function updateAddressBar(url, title) {
  _addrUrl = url || "";
  _addrTitle = (title && title !== url ? title : "") || "";
  if (!addrInput) return;
  setAddrFav(_addrUrl);
  const focused = document.activeElement === addrInput;
  addrInput.value = (!focused && _addrTitle) ? _addrTitle : _addrUrl;
}
window.updateAddressBar = updateAddressBar;

if (addrInput) {
  addrInput.addEventListener("focus", () => {
    if (_addrUrl && addrInput.value === _addrTitle) addrInput.value = _addrUrl;
    requestAnimationFrame(() => addrInput.select());
  });
  addrInput.addEventListener("blur", () => {
    if (_addrTitle && addrInput.value === _addrUrl) addrInput.value = _addrTitle;
  });
}

// Отслеживаем прошлый состав вкладок: анимацию появления вешаем ТОЛЬКО на
// новые пилюли, иначе весь стрип переигрывал бы анимацию при каждом рендере.
let _prevTabIds = new Set();

function renderTabStrip() {
  const scroll = document.getElementById("tabstripScroll");
  const prevIds = _prevTabIds;
  const curIds = new Set();
  scroll.innerHTML = "";
  for (const tab of tabs) {
    curIds.add(tab.id);
    const pill = document.createElement("div");
    pill.className = "tab-pill" + (tab.id === activeTabId ? " active" : "") +
      ((splitPair && (tab.id === splitPair.left || tab.id === splitPair.right)) ? " split" : "");
    if (!prevIds.has(tab.id)) pill.classList.add("tab-appear");
    pill.title = tab.url;
    const fav = document.createElement("span");
    fav.className = "tab-fav";
    if (tab.isNew) {
      fav.textContent = "+";
      fav.classList.add("tab-fav-new");
    } else {
      fav.textContent = ((tab.label || "?").trim()[0] || "?").toUpperCase();
    }
    // Настоящий фавикон сайта: тянем /favicon.ico САМОГО сайта (без сторонних
    // сервисов типа Google s2 — приватность). Буква остаётся под картинкой
    // как фолбэк, если иконки нет. Иконка запрашивается и для СПЯЩИХ вкладок
    // (юзер хочет логотип всегда), повторные 404 не дёргаются (кэш).
    if (!tab.url.startsWith("apb://")) {
      const img = document.createElement("img");
      img.className = "fav-img";
      img.loading = "lazy";
      img.alt = "";
      if (favAttach(img, tab.url)) fav.appendChild(img);
    }
    const title = document.createElement("span");
    title.className = "tab-pill-title";
    title.textContent = tab.label;
    // Двойной клик по названию — переименовать вкладку
    title.ondblclick = (ev) => {
      ev.stopPropagation();
      if (_tabDragSuppressed) return;
      renameTab(tab);
    };
    const closeBtn = document.createElement("button");
    closeBtn.className = "tab-pill-close";
    closeBtn.textContent = "×";
    closeBtn.onclick = (ev) => { ev.stopPropagation(); closeTab(tab.id); };
    pill.append(fav, title, closeBtn);
    pill.onclick = () => { if (_tabDragSuppressed) return; switchTab(tab.id); };
    scroll.appendChild(pill);
    pill._tabRef = tab;
    _makeTabDraggable(pill);
  }
  _prevTabIds = curIds; // запоминаем состав для следующего рендера
  const sb = document.getElementById("splitBtn");
  if (sb) sb.classList.toggle("active", !!splitPair);
  // Mirror into the horizontal top strip (tabs-on-top mode)
  const h = document.getElementById("tabstripH");
  if (h) {
    h.innerHTML = "";
    for (const child of [...scroll.children]) h.appendChild(child.cloneNode(true));
  }
  updateNavBtns();
}

function hostnameOf(url) {
  try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return url; }
}

// ---- Drag&drop: перетаскивание вкладок по вертикали с анимацией ----
let _tabDragSuppressed = false;

/** Переименование вкладки (двойной клик по названию). */
async function renameTab(tab) {
  if (!tab) return;
  const nn = await prompt("Название вкладки:", tab.label || "");
  if (nn == null) return;
  const v = nn.trim();
  if (!v) return;
  tab.userRenamed = true; // не перетирать ручное имя реальным <title>
  tab.label = v;
  renderTabStrip();
  scheduleSessionSave();
}

function _makeTabDraggable(pill) {
  let startX = 0, startY = 0, dragging = false;
  let items = [], slots = [], oIdx = 0, cIdx = 0, baseTop = 0, hPill = 0, minY = 0, maxY = 0;

  const beginDrag = () => {
    dragging = true;
    pill.classList.add("dragging");
    const parent = pill.parentElement;
    items = Array.from(parent.children);
    slots = items.map((c) => { const r = c.getBoundingClientRect(); return { top: r.top, h: r.height }; });
    oIdx = items.indexOf(pill);
    cIdx = oIdx;
    const pr = pill.getBoundingClientRect();
    baseTop = pr.top;
    hPill = pr.height;
    const lr = parent.getBoundingClientRect();
    minY = lr.top - baseTop;                       // верх списка
    maxY = lr.bottom - pr.height - baseTop;        // низ списка
    // Соседи будут ПЛАВНО ехать на свои новые места
    for (const c of parent.children) {
      if (c !== pill) c.style.transition = "transform 0.24s cubic-bezier(0.33,1,0.68,1)";
    }
  };

  // Сдвиги соседей относительно ИСХОДНЫХ позиций — без накопительных ошибок,
  // движение всегда к абсолютной цели → анимация непрерывная и мягкая.
  const applyOffsets = () => {
    let i = 0;
    for (const c of pill.parentElement.children) {
      if (c === pill) { i++; continue; }
      let off = 0;
      if (oIdx < cIdx && i > oIdx && i <= cIdx) off = -hPill;
      else if (cIdx < oIdx && i >= cIdx && i < oIdx) off = hPill;
      c.style.transform = off ? "translateY(" + off + "px)" : "";
      i++;
    }
  };

  pill.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    if (e.target.closest(".tab-pill-close")) return; // крестик не тащим
    startX = e.clientX; startY = e.clientY; dragging = false;

    let lastY = 0;
// Made by MrDuck
    const move = (ev) => {
      if (!dragging) {
        if (Math.hypot(ev.clientX - startX, ev.clientY - startY) < 6) return;
        beginDrag();
      }
      // Тащим строго в пределах списка — никуда «до бесконечности» уехать нельзя
      let y = ev.clientY - startY;
      y = Math.min(Math.max(y, minY), maxY);
      lastY = y;
      pill.style.transform = "translateY(" + y + "px)";
      // Целевой индекс — ближайший центр слота к курсору
      let bd = Infinity;
      slots.forEach((s, i) => {
        const d = Math.abs(s.top + s.h / 2 - ev.clientY);
        if (d < bd) { bd = d; cIdx = i; }
      });
      applyOffsets();
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      if (!dragging) return;
      _tabDragSuppressed = true;
      setTimeout(() => { _tabDragSuppressed = false; }, 260);
      // Плавная доводка пилюли в её слот (без «прыжка» при отпускании)
      const targetY = slots[cIdx].top - baseTop;
      pill.style.transition = "transform 0.18s cubic-bezier(0.33,1,0.68,1)";
      pill.style.transform = "translateY(" + targetY + "px)";
      setTimeout(() => {
        pill.classList.remove("dragging");
        pill.style.transition = "";
        pill.style.transform = "";
        for (const c of pill.parentElement.children) {
          if (c !== pill) { c.style.transition = ""; c.style.transform = ""; }
        }
        if (cIdx !== oIdx) {
          const arr = [...tabs];
          const movedTab = arr.splice(oIdx, 1)[0];
          arr.splice(cIdx, 0, movedTab);
          tabs.length = 0;
          for (const t of arr) tabs.push(t);
          scheduleSessionSave();
        }
        renderTabStrip(); // нормализация стилей/иконок после переноса
      }, 190);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  });
}

function showEmptyState(show) {
  document.getElementById("browserEmpty").classList.toggle("hidden", !show);
}

// Each tab = a native WebView2 child webview embedded INSIDE this single
// shell window (real browser engine, no iframes — any site works).
const invokeV2 = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

// ---------------------------------------------------------------------
// Split view — две живые вкладки рядом (50/50)
// ---------------------------------------------------------------------

let splitPair = null; // { left, right } — id вебвью-вкладок

/** Тихо погасить сплит (без смены активной вкладки). */
function splitExitSilent() {
  if (!splitPair) return;
  splitPair = null;
  invokeV2("page_split_off", {}).catch(() => {});
}

/** Разделить экран: активная вкладка слева, otherId — справа. */
async function apbSplitWith(otherId) {
  const a = tabs.find((t) => t.id === activeTabId);
  const b = tabs.find((t) => t.id === otherId);
  if (!a || !b || a.id === b.id) { toast("Нужны две открытые вкладки", "err"); return; }
  if (a.isNew || b.isNew || a.asleep || b.asleep) { toast("Сначала откройте обе вкладки", "err"); return; }
  try {
    await invokeV2("page_split_set", { leftId: a.id, rightId: b.id });
    splitPair = { left: a.id, right: b.id };
    window.__apbSplitPair = splitPair;
    activeTabId = a.id;
    updateAddressBar(a.url, a.label);
    showEmptyState(false);
    renderTabStrip();
    syncPageLayout(true);
    toast("Разделённый экран: «" + a.label + "» и «" + b.label + "»", "ok");
  } catch (e) { toast("Split: " + e, "err"); }
}

/** Выйти из разделения, оставить фокус на указанной вкладке. */
async function apbSplitExit(focusId) {
  splitPair = null;
  window.__apbSplitPair = null;
  await invokeV2("page_split_off", {}).catch(() => {});
  renderTabStrip();
  const fid = focusId || activeTabId || (tabs[0] && tabs[0].id);
  if (fid) switchTab(fid); else showHome();
}
window.apbSplitWith = apbSplitWith;
window.apbSplitExit = apbSplitExit;

// Точка входа для бэкенда (shell_open_tab): target=_blank-ссылки и window.open
// из вкладок открываются новой вкладкой здесь, в шелле.
// Правило юзера (инверсно к Chrome): ЛКМ по target=_blank и попапы
// (window.open) → focus=true, СРАЗУ перейти; ПКМ «Открыть в новой вкладке»
// и Ctrl+клик → фоном, не уходить со страницы.
window.__apbOpenTab = (url, focus) => createTab(url, null, { background: !focus });

// События от бэкенда о жизни вкладок-вебвью:
try {
  const ev = window.__TAURI__.event;
  // Сайт сам сменил страницу (клик по ссылке/редирект) — синхронизируем
  // вкладку, историю и омнибокс, иначе адресная строка показывает старое.
  ev.listen("page-url-changed", (e) => {
    const { id, url } = e.payload || {};
    if (window.__apbLog) window.__apbLog("INFO", `url-changed ${id} ${url}`);
    const t = tabs.find((x) => x.id === id);
    if (!t || !url || t.url === url) return;
    t.url = url;
    if (!t.userRenamed) t.label = smartTitle(url, "");
    if (t.hist[t.hi] !== url) {
      t.hist = t.hist.slice(0, t.hi + 1);
      t.hist.push(url);
      t.hi = t.hist.length - 1;
    }
    if (id === activeTabId) updateAddressBar(url, t.label);
    renderTabStrip();
    updateNavBtns();
    scheduleSessionSave();
  });
  // Реальный заголовок страницы (<title>) с бэкенда — настоящее название
  // сайта в адресной строке и в кладке (если юзер не переименовал вручную).
  ev.listen("page-title-changed", (e) => {
    const { id, title } = e.payload || {};
    if (window.__apbLog) window.__apbLog("INFO", `title-changed ${id} ${title}`);
    const t = tabs.find((x) => x.id === id);
    if (!t || !title) return;
    const v = String(title).trim().slice(0, 200);
    if (!v || t.userRenamed) return;
    t.label = v;
    titleCacheRemember(t.url, v);
    if (id === activeTabId) updateAddressBar(t.url, t.label);
    renderTabStrip();
    scheduleSessionSave();
  });
  // Нативное меню «Открыть в новом окне» (ПКМ на YouTube и т.п.) — бэкенд
  // запретил ОС-окно и просит открыть ссылку вкладкой.
  ev.listen("page-open-tab", (e) => {
    const u = e.payload && e.payload.url;
    const f = !!(e.payload && e.payload.focus);
    if (u) window.__apbOpenTab(u, f);
  });
} catch { /* event API недоступен — живём как раньше */ }

/** Кнопка ⬓ в тулбаре: вкл/выкл сплит одной кнопкой. */
document.getElementById("splitBtn").addEventListener("click", () => {
  if (splitPair) { apbSplitExit(activeTabId); return; }
  const other = tabs.find((t) => t.id !== activeTabId && !t.isNew && !t.asleep && t.url);
  if (!other) { toast("Нужна вторая открытая вкладка для сплита", "err"); return; }
  apbSplitWith(other.id);
});

let relayoutTimer = null;

// Measure the actual content-area hole (#browserView) and hand it to the
// backend, which positions native tab webviews there. This automatically
// accounts for the tabstrip/toolbar heights, the rail, the side panel and
// the note editor pane — no pixel constants duplicated in Rust.
//
// BUG FIX: `getBoundingClientRect()` used to be read once, synchronously,
// right when syncPageLayout() was CALLED — before the requested delay.
// For the debounced (non-immediate) path that's the wrong moment: e.g.
// opening the toolbar side panel calls this right after adding the
// `.open` class, but the panel's width transition (~0.16s) hasn't
// actually run yet, so the captured rect still reflected the OLD,
// pre-panel (full-width) layout. 170ms later we'd still push that STALE
// rect to the backend, so the real tab's native webview got positioned
// at its old, too-wide bounds — sitting right on top of where the side
// panel had since opened, instead of alongside it. That's the "opens a
// toolbar tab over a live site and it's just black/shows the site
// squeezed weirdly" bug. Fix: measure fresh at the moment we actually
// push, not at the moment we schedule the push.
function syncPageLayout(immediate = false) {
  const push = () => {
    const view = document.getElementById("browserView").getBoundingClientRect();
    invokeV2("page_relayout", {
      x: view.left,
      y: view.top,
      width: view.width,
      height: view.height,
    }).catch(() => {});
  };
  clearTimeout(relayoutTimer);
  if (immediate) push();
  else relayoutTimer = setTimeout(push, 170); // > 0.16s panel transition
}

window.addEventListener("resize", () => {
  clearTimeout(relayoutTimer);
  relayoutTimer = setTimeout(syncPageLayout, 120);
});

async function createTab(url, label, opts = {}) {
  splitExitSilent();
  try {
    // Фоновая вкладка: создаётся скрытой, ТЕКУЩАЯ вкладка остаётся
    // активной (омнибокс/главный экран не трогаем) — как Ctrl+клик в
    // обычных браузерах.
    const cmd = opts.background ? "page_open_bg" : "page_open";
    const id = await invokeV2(cmd, { url });
    const t = { id, url, label: smartTitle(url, label), hist: [url], hi: 0 };
    // Новые вкладки по умолчанию встают В НАЧАЛО списка (сверху).
    // opts.append — для восстановления сессий/воркспейсов, чтобы сохранить
    // исходный порядок.
    if (opts.append) tabs.push(t); else tabs.unshift(t);
    if (!opts.background) {
      activeTabId = id;
      internalOpen = null;
      internalHost.classList.add("hidden");
      updateAddressBar(url, t.label);
      showEmptyState(false);
    }
    renderTabStrip();
    if (!opts.background) syncPageLayout(true);
    scheduleSessionSave();
    return id;
  } catch (e) {
    alert("Не удалось открыть страницу: " + e);
    return null;
  }
}

/** Подпись вкладки: домен для адресов, но текст запроса — если юзер искал. */
function smartLabel(rawInput, resolvedUrl) {
  const r = (rawInput || "").trim();
  if (!r) return hostnameOf(resolvedUrl);
  const looksLikeAddress =
    /^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(r) ||
    /^localhost(:\d+)?/i.test(r) ||
    (!/\s/.test(r) && /^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)+(:\d+)?(\/.*)?$/.test(r));
  if (looksLikeAddress) return hostnameOf(resolvedUrl);
  return r.length > 26 ? r.slice(0, 26) + "…" : r;
}

/** Достаём поисковый запрос из URL (?q=/?query=/?text=...) — чтобы вкладка,
 *  открытая из истории/закладок, называлась тем, что искал юзер, а не
 *  «duckduckgo.com». Если параметров нет — null. */
function labelFromUrl(url) {
  try {
    const u = new URL(url);
    for (const key of ["q", "query", "text", "search", "word", "p", "ask"]) {
      const v = u.searchParams.get(key);
      const dec = v && v.trim();
      if (dec) return dec.length > 26 ? dec.slice(0, 26) + "…" : dec;
    }
  } catch { /* не URL — ладно */ }
  return null;
}

function navigateActiveTab(url, label, opts = {}) {
  const cur = currentTabObj();
  if (!activeTabId || !cur) { createTab(url, label, opts); return; }
  // Навигация в «новой вкладке» будит её в настоящий сайт
  if (cur.isNew) { wakeAs(cur, url, label); return; }
  const tab = cur;
  tab.url = url;
  tab.label = smartTitle(url, label);
  // Record navigation history for the back/forward buttons.
  if (tab.hist[tab.hi] !== url) {
    tab.hist = tab.hist.slice(0, tab.hi + 1);
    tab.hist.push(url);
  }
  tab.hi = tab.hist.length - 1;
  updateAddressBar(url, tab.label);
  invokeV2("page_navigate", { id: tab.id, url }).catch(() => {});
  renderTabStrip();
  scheduleSessionSave();
}

// ---- Back / forward / reload (per-tab JS-side history) ----

// Made by MrDuck
function currentTabObj() {
  return tabs.find((t) => t.id === activeTabId) || null;
}

function updateNavBtns() {
  const t = currentTabObj();
  const back = document.getElementById("navBack");
  const fwd = document.getElementById("navFwd");
  const rel = document.getElementById("navReload");
  if (!back || !fwd || !rel) return;
  back.disabled = !t || t.hi <= 0;
  fwd.disabled = !t || t.hi >= t.hist.length - 1;
  rel.disabled = !t;
}

function jumpHistory(dir) {
  const t = currentTabObj();
  if (!t || t.isNew) return;
  const ni = t.hi + dir;
  if (ni < 0 || ni >= t.hist.length) return;
  t.hi = ni;
const url = t.hist[ni];
    t.url = url;
    t.label = smartTitle(url, "");
  updateAddressBar(url, t.label);
  invokeV2("page_navigate", { id: t.id, url }).catch(() => {});
  renderTabStrip();
}

document.getElementById("navBack").addEventListener("click", () => jumpHistory(-1));
document.getElementById("navFwd").addEventListener("click", () => jumpHistory(1));
document.getElementById("navReload").addEventListener("click", () => {
  const t = currentTabObj();
  if (t && !t.isNew) invokeV2("page_navigate", { id: t.id, url: t.url }).catch(() => {});
});

// Sidebar collapse toggle живёт в boot-core.js (тут был ДУБЛЬ обработчика —
// двойной toggle делал кнопку нерабочей).

/** Спящая вкладка: есть в списке, но вебвью не создаётся (старт с главного
 *  экрана). Просыпается при клике — тогда открывается сайт. */
function addSleepingTab(url, label) {
  tabs.unshift({
    id: makeTabId(), url,
    label: smartTitle(url, label),
    hist: [url], hi: 0, asleep: true,
  });
}

/** Разбудить спящую вкладку: открыть вебвью и подменить временный id. */
async function wakeTab(tab) {
  splitExitSilent();
  try {
    const realId = await invokeV2("page_open", { url: tab.url });
    _prevTabIds.delete(tab.id); // новый id → пилюля честно проиграет анимацию пробуждения
    tab.id = realId;
    tab.asleep = false;
    tab.hist = [tab.url];
    tab.hi = 0;
    activeTabId = realId;
    internalOpen = null;
    internalHost.classList.add("hidden");
    updateAddressBar(tab.url, tab.label);
    showEmptyState(false);
    renderTabStrip();
    syncPageLayout(true);
    scheduleSessionSave();
  } catch (e) {
    alert("Не удалось открыть страницу: " + e);
  }
}

/** Новая вкладка-«страница»: пилюля есть, вебвью нет; внутри — главный
 *  экран. При вводе адреса превращается в обычную вкладку (wakeAs). */
function openNewTabPage() {
  splitExitSilent();
  const t = { id: makeTabId(), url: "", label: "Новая вкладка", hist: [""], hi: 0, isNew: true };
  tabs.unshift(t);
  activeTabId = t.id;
  internalOpen = null;
  internalHost.classList.add("hidden");
  // КРИТИЧНО: нативные вебвью рисуются ПОВЕРХ HTML — прячем их все
  invokeV2("page_hide_all", {}).catch(() => {});
  updateAddressBar("", "");
  showHome();
  renderTabStrip();
  syncPageLayout();
  scheduleSessionSave();
  updateNavBtns();
  const input = document.getElementById("homeSearchInput");
  if (input) input.focus();
}

/** Первая навигация в новой вкладке: заводим настоящее вебвью. */
async function wakeAs(tab, url, label) {
  splitExitSilent();
  try {
    const realId = await invokeV2("page_open", { url });
    _prevTabIds.delete(tab.id);
    tab.id = realId;
    tab.url = url;
    tab.label = smartTitle(url, label);
    tab.hist = [url];
    tab.hi = 0;
    tab.isNew = false;
    activeTabId = realId;
    updateAddressBar(url, tab.label);
    showEmptyState(false);
    renderTabStrip();
    syncPageLayout(true);
    scheduleSessionSave();
    if (typeof updateDarkSiteBtn === "function") updateDarkSiteBtn();
  } catch (e) {
    alert("Не удалось открыть страницу: " + e);
  }
}

async function switchTab(id) {
  closeInternal(false);
  const tab = tabs.find((t) => t.id === id);
  if (!tab) return;
  // Клик по члену сплита — просто переключаем фокус, сплит живёт
  if (splitPair && (id === splitPair.left || id === splitPair.right)) {
    activeTabId = id;
    updateAddressBar(tab.url, tab.label);
    renderTabStrip();
    updateNavBtns();
    return;
  }
  // Клик по сторонней вкладке гасит сплит и открывает её на весь экран
  if (splitPair) await apbSplitExit(id);
  if (tab.asleep) { await wakeTab(tab); return; } // спящая — сначала открыть сайт
  if (tab.isNew) {
    // Новая вкладка-страница: пилюля есть, вебвью нет — показываем главную
    activeTabId = tab.id;
    invokeV2("page_hide_all", {}).catch(() => {});
    updateAddressBar("", "");
    showHome();
    renderTabStrip();
    scheduleSessionSave();
    updateNavBtns();
    return;
  }
  activeTabId = id;
  updateAddressBar(tab.url, tab.label);
  showEmptyState(false);
  invokeV2("page_activate", { id }).catch(() => {});
  renderTabStrip();
  scheduleSessionSave();
}

function closeTab(id) {
  // Партнёр по сплиту остаётся видимым (бэкенд сам гасит сплит)
  let splitPartner = null;
  if (splitPair && (id === splitPair.left || id === splitPair.right)) {
    splitPartner = id === splitPair.left ? splitPair.right : splitPair.left;
  }
  invokeV2("page_close", { id }).catch(() => {});
  tabs = tabs.filter((t) => t.id !== id);
  tabDark.delete(id);
  splitPair = null;
  window.__apbSplitPair = null;

  if (activeTabId === id) {
    activeTabId = null;
    if (splitPartner) {
      switchTab(splitPartner);
    } else if (tabs.length > 0) {
      // список теперь «новые сверху» — активируем верхнюю оставшуюся
      switchTab(tabs[0].id);
    } else {
      updateAddressBar("", "");
      showHome();
      syncPageLayout();
    }
  }
  renderTabStrip();
  scheduleSessionSave();
  updateDarkSiteBtn();
  ensureHomeVisible();
}

document.getElementById("newTabBtn").addEventListener("click", () => {
  // «+» и Ctrl+T создают НАСТОЯЩУЮ вкладку-пилюлю (внутри — главный экран;
  // ввод адреса превращает её в сайт)
  openNewTabPage();
});

// Extra guards: after closing/switching tabs the content must never stay dark.
// Lives here (not in 04) because it wraps syncPageLayout declared above —
// cross-file function hoisting disappeared when the monolith was split.
const _origSync = syncPageLayout;
syncPageLayout = function (...a) { _origSync(...a); ensureHomeVisible(); };

// Made by MrDuck