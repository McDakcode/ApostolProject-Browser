// Made by MrDuck
// ---------------------------------------------------------------------
// Address bar — smart URL vs. search resolution.
// ---------------------------------------------------------------------

function resolveAddressInput(raw) {
  const value = raw.trim();
  if (/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(value)) return value;
  if (/^localhost(:\d+)?(\/.*)?$/i.test(value)) return "http://" + value;
  if (!/\s/.test(value) && /^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)+(:\d+)?(\/.*)?$/.test(value)) {
    return "https://" + value;
  }
  const base = SEARCH_ENGINES[getSearchEngine()] || SEARCH_ENGINES.duckduckgo;
  return base + encodeURIComponent(value);
}

document.getElementById("addressForm").addEventListener("submit", (e) => {
  e.preventDefault();
  const input = document.getElementById("addressInput");
  let raw = input.value.trim();
  if (!raw) return;
  // Режим названия сайта: в поле написано имя, а не адрес. Enter — просто
  // перейти (перезагрузить) текущую страницу, а не искать это имя.
  const t = (typeof currentTabObj === "function") ? currentTabObj() : null;
  if (t && !t.isNew && t.url && raw === (t.label || "")) raw = t.url;
  const url = resolveAddressInput(raw);
  navigateActiveTab(url, smartLabel(raw, url));
});

// ---------------------------------------------------------------------
// Bookmarks — v2: ПАПКИ (дерево, вложенность) + БУКМАРКЛЕТЫ.
// Папки хранит бэкенд (bookmark_folders, folder_id у закладки); панель
// строит дерево через bookmarks_tree одним пакетом. Букмарклет — закладка
// с URL «javascript:…»: клик исполняет код В АКТИВНОЙ вкладке (page_eval),
// ⚡ в строке. Поиск — по-прежнему substring (search_bookmarks).
// ---------------------------------------------------------------------

let bmFolders = []; // [{id, parent_id, name}]

function bmFolderLabel(id) {
  const f = bmFolders.find((x) => String(x.id) === String(id));
  return f ? f.name : "";
}

// каскад имён папки для селектов: «Работа / Подработка»
function bmPathOf(id, guard = 0) {
  if (guard > 8) return "";
  const f = bmFolders.find((x) => String(x.id) === String(id));
  if (!f) return "";
  const parent = f.parent_id ? bmPathOf(f.parent_id, guard + 1) : "";
  return parent ? parent + " / " + f.name : f.name;
}

function bmFillFolderSelects() {
  const opts = ['<option value="">— без папки —</option>']
    .concat(bmFolders.map((f) => `<option value="${f.id}">${escapeHtml(bmPathOf(f.id))}</option>`));
  document.getElementById("bmFolder").innerHTML = opts.join("");
  const popts = ['<option value="">— в корне —</option>']
    .concat(bmFolders.map((f) => `<option value="${f.id}">${escapeHtml(bmPathOf(f.id))}</option>`));
  document.getElementById("bmFolderParent").innerHTML = popts.join("");
}

// строка закладки (или букмарклета)
function bmRow(b) {
  const li = document.createElement("li");
  const isJs = /^javascript:/i.test(b.url);
  li.innerHTML =
    `<div style="min-width:0"><div class="title">${isJs ? "⚡ " : ""}${escapeHtml(b.title)}</div>` +
    `<div class="meta">${escapeHtml(isJs ? "букмарклет — код исполняется на странице" : b.url)}${b.tags.length ? " · " + escapeHtml(b.tags.join(", ")) : ""}</div></div>`;
  li.onclick = () => {
    if (isJs) {
      // букмарклет: код — в активную вкладку (page_eval)
      const code = b.url.replace(/^javascript:/i, "");
      if (typeof activeTabId !== "undefined" && activeTabId) {
        invoke("page_eval", { id: activeTabId, js: code }).catch(() => {});
      }
      return;
    }
    navigateActiveTab(b.url, b.title || undefined);
  };
  // ✕ — удалить закладку (файлы/страницу не трогаем)
  const del = document.createElement("button");
  del.className = "close";
  del.textContent = "✕";
  del.title = "Удалить закладку";
  del.onclick = (ev) => {
    ev.stopPropagation();
    invoke("bookmark_delete", { id: String(b.id) }).then(() => refreshBookmarks()).catch(() => {});
  };
  li.appendChild(del);
  return li;
}

async function refreshBookmarks(query = "") {
  const list = document.getElementById("bmList");
  if (query) {
    // поиск — плоский список (как раньше), но со ⚡ и ✕
    const results = await invoke("search_bookmarks", { query });
    list.innerHTML = "";
    if (!results.length) {
      list.innerHTML = `<li class="empty">Ничего не найдено</li>`;
      return;
    }
    for (const b of results) list.appendChild(bmRow(b));
    return;
  }
  // дерево: bookmarks_tree одним пакетом
  let tree;
  try { tree = await invoke("bookmarks_tree", {}); } catch { tree = null; }
  if (!tree) { list.innerHTML = `<li class="empty">Ошибка загрузки</li>`; return; }
  bmFolders = tree.folders || [];
  bmFillFolderSelects();
  list.innerHTML = "";
  const items = tree.items || [];
  if (!items.length && !bmFolders.length) {
    list.innerHTML = `<li class="empty">Пока нет закладок</li>`;
    return;
  }
  // корневые закладки (без папки)
  for (const b of items.filter((x) => !x.folder_id)) list.appendChild(bmRow(b));
  // папки: <details> с детьми (вложенность — каскадом имён)
  const byParent = new Map();
  for (const f of bmFolders) {
    const key = f.parent_id ? String(f.parent_id) : "";
    if (!byParent.has(key)) byParent.set(key, []);
    byParent.get(key).push(f);
  }
  const emitFolder = (folderId, host, depth) => {
    const kids = (byParent.get(String(folderId)) || []);
    for (const f of kids) {
      const det = document.createElement("details");
      det.className = "bm-folder";
      det.style.marginLeft = (depth * 10) + "px";
      const sum = document.createElement("summary");
      sum.textContent = "📁 " + f.name;
      det.appendChild(sum);
      host.appendChild(det);
      const inner = document.createElement("ul");
      inner.className = "list";
      det.appendChild(inner);
      for (const b of items.filter((x) => String(x.folder_id) === String(f.id))) {
        inner.appendChild(bmRow(b));
      }
      emitFolder(f.id, inner, depth + 1);
    }
  };
  emitFolder(null, list, 0);
}

document.getElementById("bmSearch").addEventListener("input", (e) => refreshBookmarks(e.target.value));

document.getElementById("bmAddBtn").addEventListener("click", async () => {
  const title = document.getElementById("bmTitle").value.trim();
  const url = document.getElementById("bmUrl").value.trim();
  const tags = document.getElementById("bmTags").value.split(",").map((t) => t.trim()).filter(Boolean);
  if (!title || !url) return;
  const folderId = document.getElementById("bmFolder").value || null;
  // add_bookmark с папкой: бэкенд принимает folderId (Option<Uuid>)
  await invoke("add_bookmark", { title, url, tags, note: null, folderId });
  document.getElementById("bmTitle").value = "";
  document.getElementById("bmUrl").value = "";
  document.getElementById("bmTags").value = "";
  await refreshBookmarks();
});

document.getElementById("bmFolderBtn").addEventListener("click", async () => {
  const name = document.getElementById("bmFolderName").value.trim();
  if (!name) return;
  const parentId = document.getElementById("bmFolderParent").value || null;
  await invoke("bookmark_folder_create", { name, parentId }).catch(() => {});
  document.getElementById("bmFolderName").value = "";
  await refreshBookmarks();
});

// ---------------------------------------------------------------------
// History
// ---------------------------------------------------------------------

// Заголовок записи истории: сначала реальный <title> из кэша вкладок (как во
// вкладках и адресной строке — «название видео» вместо домена); иначе если
// бэкенд записал голый хостнейм (поисковик), вытаскиваем поисковый запрос.
function histDisplayTitle(title, url) {
  const cached = (typeof titleCacheFor === "function") ? titleCacheFor(url) : "";
  const host = (() => { try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return ""; } })();
  if (cached && cached !== host && cached !== url) return cached;
  const t = (title || "").trim();
  try {
    if (!t || t === host || t === url) {
      const q = typeof labelFromUrl === "function" ? labelFromUrl(url) : null;
      if (q) return "🔍 " + q;
    }
  } catch { /* не URL */ }
  return t || "(без названия)";
}

async function refreshHistory() {
  const visits = await invoke("recent_history", { limit: 50 });
  const list = document.getElementById("historyList");
  list.innerHTML = "";
  if (visits.length === 0) {
    list.innerHTML = '<li class="empty">История пуста — либо профиль анонимный и не пишет историю</li>';
    return;
  }
  for (const v of visits) {
    const li = document.createElement("li");
    li.innerHTML = `<div><div class="title">${escapeHtml(histDisplayTitle(v.title, v.url))}</div><div class="meta">${escapeHtml(hostnameOf(v.url))} · ${new Date(v.visited_at).toLocaleString()}</div></div>`;
    li.title = v.url;
    li.onclick = () => createTab(v.url, null, { background: true }); // фоном: текущая вкладка не меняется
    list.appendChild(li);
  }
}

// ---------------------------------------------------------------------
// Full history (Settings → all profiles, detailed rows)
// ---------------------------------------------------------------------

let histAllCache = [];

async function loadFullHistory() {
  const ul = document.getElementById("histAllList");
  if (!ul) return;
  ul.innerHTML = '<li class="empty">Загрузка…</li>';
  const scope = document.getElementById("histScopeSelect").value;
  try {
    histAllCache = await invoke("history_all_profiles", { limitPerProfile: 200, scope });
  } catch {
    histAllCache = [];
  }
  renderFullHistory();
}

function renderFullHistory() {
  const ul = document.getElementById("histAllList");
  if (!ul) return;
  const q = (document.getElementById("histFilterInput").value || "").trim().toLowerCase();
  const rows = histAllCache.filter((r) =>
    !q || `${histDisplayTitle(r.title, r.url)} ${r.title} ${r.url} ${r.profile}`.toLowerCase().includes(q));
  const stats = document.getElementById("histStats");
  if (stats) stats.textContent = `Записей: ${rows.length}`;

  // Per-profile summary chips
  const summary = document.getElementById("histSummary");
  if (summary) {
    const byProf = {};
    for (const r of histAllCache) byProf[r.profile] = (byProf[r.profile] || 0) + 1;
    summary.innerHTML = "";
    for (const [prof, n] of Object.entries(byProf)) {
      const b = document.createElement("span");
      b.className = "badge";
      b.textContent = `${prof}: ${n}`;
      summary.appendChild(b);
    }
    if (!Object.keys(byProf).length) summary.innerHTML = '<span class="hint">Пока нет записей</span>';
  }

  ul.innerHTML = "";
  if (!rows.length) {
    ul.innerHTML = '<li class="empty">История пуста</li>';
    return;
  }
  let lastDay = "";
  for (const r of rows.slice(0, 500)) {
    const d = new Date(r.visited_at);
    const dayKey = d.toDateString();
    if (dayKey !== lastDay) {
      lastDay = dayKey;
      const today = new Date().toDateString() === dayKey;
      const yest = new Date(Date.now() - 864e5).toDateString() === dayKey;
      const head = document.createElement("li");
      head.className = "hist-day";
      head.textContent = today ? "Сегодня" : yest ? "Вчера" : d.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long" });
      ul.appendChild(head);
    }
    const li = document.createElement("li");
    li.innerHTML = `<div class="h-main"><div class="h-title"></div><div class="h-url"></div><div class="h-meta"></div></div><span class="h-prof"></span>`;
    li.querySelector(".h-title").textContent = histDisplayTitle(r.title, r.url);
    li.querySelector(".h-url").textContent = r.url;
    li.querySelector(".h-meta").textContent = d.toLocaleTimeString();
    li.querySelector(".h-prof").textContent = r.profile;
    li.onclick = () => createTab(r.url, null, { background: true }); // фоном: текущая вкладка не меняется
    ul.appendChild(li);
  }
}
document.getElementById("histReloadBtn").addEventListener("click", loadFullHistory);
document.getElementById("histScopeSelect").addEventListener("change", loadFullHistory);
document.getElementById("histFilterInput").addEventListener("input", renderFullHistory);
document.getElementById("histClearActiveBtn")?.addEventListener("click", async () => {
  if (!(await confirm("Очистить всю историю ТЕКУЩЕГО профиля?"))) return;
  await invoke("clear_history");
  await loadFullHistory();
});

async function clearHistoryAndRefresh() {
  await invoke("clear_history");
  await refreshHistory();
}
document.getElementById("historyClearBtn").addEventListener("click", clearHistoryAndRefresh);
document.getElementById("historyClearBtn2").addEventListener("click", clearHistoryAndRefresh);

// ---------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------

async function refreshNotes() {
  const notes = await invoke("list_notes"); // теперь с папками: "Folder/Note.md"
  const list = document.getElementById("notesList");
  list.innerHTML = "";
  if (!notes.length) {
    list.innerHTML = '<li class="empty">Пока нет заметок</li>';
    return;
  }
  const mk = (n) => {
    const li = document.createElement("li");
    li.dataset.file = n; // для контекст-меню
    const base = n.includes("/") ? n.slice(n.lastIndexOf("/") + 1) : n;
    li.innerHTML = `<span class="title">${escapeHtml(base.replace(/\.md$/, ""))}</span>`;
    li.title = n;
    li.onclick = () => openNote(n);
    return li;
  };
  // Группировка по папкам (корневые — сверху)
  const groups = new Map();
  for (const n of notes) {
    const i = n.lastIndexOf("/");
    const folder = i >= 0 ? n.slice(0, i) : "";
    if (!groups.has(folder)) groups.set(folder, []);
    groups.get(folder).push(n);
  }
  const folders = [...groups.keys()].sort((a, b) =>
    a === "" ? -1 : b === "" ? 1 : a.localeCompare(b));
  for (const folder of folders) {
    if (!folder) {
      for (const n of groups.get(folder)) list.appendChild(mk(n));
      continue;
    }
    const det = document.createElement("details");
    det.className = "note-folder";
    det.open = true;
    const sum = document.createElement("summary");
    sum.textContent = "📁 " + folder + " (" + groups.get(folder).length + ")";
    det.appendChild(sum);
    const ul = document.createElement("ul");
    ul.className = "list note-sub";
    for (const n of groups.get(folder)) ul.appendChild(mk(n));
    det.appendChild(ul);
    list.appendChild(det);
  }
}

async function openNote(path) {
  const content = await invoke("read_note", { path });
  openEditor(path, content);
}

document.getElementById("noteBackBtn").addEventListener("click", () => {
  document.getElementById("noteViewer").classList.add("hidden");
});

document.getElementById("noteAddBtn").addEventListener("click", async () => {
  let name = document.getElementById("noteName").value.trim();
  if (!name) return;
  if (!name.endsWith(".md")) name += ".md";
  const content = "# " + name.replace(/\.md$/, "") + "\n\n";
  try {
    await invoke("create_note", { path: name, content });
    document.getElementById("noteName").value = "";
    document.getElementById("noteContent").value = "";
    await refreshNotes();
    openEditor(name, content);
    document.getElementById("edText").focus();
  } catch (e) { alert(e); }
});

// ---------------------------------------------------------------------
// Omnibox suggestions — выпадающий список под адресной строкой:
// совпадения из истории при вводе. Стрелки ↑↓ + Enter, Esc — закрыть.
// ---------------------------------------------------------------------

const omniBox = document.getElementById("addressInput");
const omniForm = document.getElementById("addressForm");
const omniDrop = document.createElement("div");
omniDrop.id = "omniSuggest";
omniDrop.classList.add("hidden");
omniForm.appendChild(omniDrop);
let omniItems = [], omniIdx = -1;

function omniHide() {
  if (omniDrop.classList.contains("hidden")) return;
  omniDrop.classList.add("hidden");
  omniIdx = -1;
  // если прятали вебвью ради подсказок — возвращаем вкладку
  if (typeof activeTabId !== "undefined" && activeTabId && typeof switchTab === "function") {
    const t = currentTabObj ? currentTabObj() : null;
    if (t && !t.isNew) switchTab(t.id);
  }
}

function omniHighlight() {
  Array.from(omniDrop.children).forEach((c, i) => c.classList.toggle("sel", i === omniIdx));
}

// --- Suggest v3: как у нормального браузера — до 5 живых вариантов, всё ЛОКАЛЬНО
// (приватность: никаких внешних suggest-серверов — keystrokes не покидают браузер) ---
//  1. «Закладки» — твоё сохранённое (⭐, из search_bookmarks)
//  2. «История» — что уже частично искал/открывал (🕘, из recent_history)
//  3. «Продолжение» — домены на префикс (✨, история + офлайн-словарь)
//  4. «Прямой URL» (🌐) — если похоже на адрес
//  5. «Искать …» (🔍) — всегда последний, гарантированно через выбранный поисковик
function omniEsc(s) {
  return String(s || "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}
// подсветка совпадения внутри текста (безопасно: всё экранировано)
function omniHl(text, q) {
  const t = String(text || "");
  if (!q) return omniEsc(t);
  const i = t.toLowerCase().indexOf(String(q).toLowerCase());
  if (i < 0) return omniEsc(t);
  return omniEsc(t.slice(0, i))
    + '<b class="omni-hl">' + omniEsc(t.slice(i, i + q.length)) + "</b>"
    + omniEsc(t.slice(i + q.length));
}
// офлайн-словарь популярных доменов (только префиксные дополнения)
const OMNI_DOMAINS = [
  "youtube.com", "github.com", "gitlab.com", "vk.com", "habr.com",
  "reddit.com", "wikipedia.org", "telegram.org", "twitter.com", "x.com",
  "instagram.com", "facebook.com", "google.com", "translate.google.com",
  "mail.ru", "yandex.ru", "stackoverflow.com", "openai.com", "notion.so",
  "figma.com", "discord.com", "twitch.tv", "music.youtube.com", "rutracker.org",
];
// строит и добавляет пункт списка
function omniPush(kind, icon, mainHtml, sub, url, title) {
  omniItems.push({ kind, icon, mainHtml, sub, url, title });
}
function omniBuildDom(list) {
  // item DOM + мышь: mousedown (не click — фокус не теряем до перехода)
  omniDrop.innerHTML = "";
  omniItems.forEach((v, i) => {
    const d = document.createElement("div");
    d.className = "omni-item omni-" + v.kind + (i === omniIdx ? " sel" : "");
    const ic = document.createElement("span"); ic.className = "omni-ic"; ic.textContent = v.icon;
    const tx = document.createElement("span"); tx.className = "omni-t"; tx.innerHTML = v.mainHtml;
    const ur = document.createElement("span"); ur.className = "omni-u"; ur.textContent = v.sub || "";
    d.append(ic, tx, ur);
    d.onmousedown = (e) => {
      e.preventDefault(); // чтобы не терять фокус до перехода
      omniHide();
      navigateActiveTab(v.url, v.title || undefined);
    };
    omniDrop.appendChild(d);
  });
}
function omniRender(q) {
// Made by MrDuck
  const raw = (q || "").trim();
  const query = raw.toLowerCase();
  if (!query) { omniHide(); return; }
  // История + закладки параллельно: как в нормальном браузере подсказываем
  // и то, что уже открывал, и то, что сохранил в закладки.
  Promise.all([
    invoke("recent_history", { limit: 300 }).catch(() => []),
    typeof invoke === "function" ? invoke("search_bookmarks", { query: raw }).catch(() => []) : Promise.resolve([]),
  ]).then(([hist, bms]) => {
    hist = hist || [];
    bms = bms || [];
    omniItems = [];
    const seenUrl = new Set();
    const wantUrlish = /^[\wа-яё-]+(\.[\wа-яё-]+){1,}(\/\S*)?$/i.test(raw) || raw.includes("://");
    const bare = /^[\wа-яё-]+$/i.test(raw) && !raw.includes(" ");

    function hostOf(u) {
      try { return new URL(u).hostname.replace(/^www\./, ""); } catch { return ""; }
    }
    function pushItem(kind, icon, mainHtml, sub, url, title) {
      if (seenUrl.has(url)) return;
      seenUrl.add(url);
      omniPush(kind, icon, mainHtml, sub, url, title);
    }

    // --- 1. Закладки: «это у тебя сохранено» ---
    for (const b of bms) {
      const u = b.url || "";
      const t = (b.title || "") + " " + u;
      if (!t.toLowerCase().includes(query)) continue;
      const title = b.title && b.title !== u ? b.title : u;
      pushItem("bm", "⭐", omniHl(title, raw), hostOf(u), u, b.title || u);
      if (omniItems.length >= 2) break;
    }
    // --- 2. История: «уже частично искал/открывал» ---
    for (const v of hist) {
      const u = v.url || "";
      if (!u) continue;
      const ul = u.toLowerCase();
      const t = (v.title || "").toLowerCase();
      if (!ul.includes(query) && !t.includes(query)) continue;
      const title = v.title && v.title !== u ? v.title : u;
      const mainTitle = omniHl(title, raw);
      pushItem("hist", "🕘", mainTitle, hostOf(u), u, v.title || u);
      if (omniItems.length >= 4) break;
    }
    // --- 3. «Что я мог хотеть»: продолжения доменов (для «голого» слова) ---
    if (bare) {
      const comp = new Set();
      for (const v of hist) {
        try {
          const h = new URL(v.url).hostname.replace(/^www\./, "");
          if (h.startsWith(query) && h.length > raw.length) comp.add(h);
        } catch {}
      }
      for (const d of OMNI_DOMAINS) if (d.startsWith(query) && d !== raw) comp.add(d);
      for (const h of comp) {
        const url = "https://" + h + "/";
        const mainHtml = omniEsc(raw) + '<span class="omni-comp">' + omniEsc(h.slice(raw.length)) + "</span>";
        pushItem("want", "✨", mainHtml, t("продолжение"), url, h);
        if (omniItems.length >= 5) break;
      }
    }
    // --- 4. Прямой URL ---
    if (wantUrlish) {
      const url = raw.includes("://") ? raw : "https://" + raw;
      pushItem("url", "🌐", omniEsc(raw), t("открыть сайт"), url, raw);
    }
    // --- 5. Поиск всегда последним (как у всех) ---
    const eng = (typeof getSearchEngine === "function") ? getSearchEngine() : "duckduckgo";
    const tpl = (typeof SEARCH_ENGINES !== "undefined" && SEARCH_ENGINES[eng]) || "https://duckduckgo.com/?q=";
    const surl = tpl + encodeURIComponent(raw);
    pushItem("search", "🔍", t("Искать") + " «" + omniEsc(raw) + "»", t("поиск"), surl, raw);
    // Показываем максимум 5 вариантов (поиск всегда последний)
    const lastIsSearch = !!omniItems.length && omniItems[omniItems.length - 1].kind === "search";
    const searchItem = lastIsSearch ? omniItems[omniItems.length - 1] : null;
    omniItems = omniItems.slice(0, 5);
    if (searchItem && !omniItems.includes(searchItem)) {
      omniItems[omniItems.length - 1] = searchItem;
    }
    if (!omniItems.length) { omniHide(); return; }
    if (omniIdx >= omniItems.length) omniIdx = -1;
    omniBuildDom();
    // Палитра/подсказки — это HTML, а сайт рисуется ПОВЕРХ: прячем вебвью
    if (typeof activeTabId !== "undefined" && activeTabId) {
      const t = typeof currentTabObj === "function" ? currentTabObj() : null;
      if (t && !t.isNew) invokeV2("page_hide_all", {}).catch(() => {});
    }
    omniDrop.classList.remove("hidden");
  }).catch(() => omniHide());
}

omniBox.addEventListener("input", () => { omniIdx = -1; omniRender(omniBox.value); });
omniBox.addEventListener("keydown", (e) => {
  if (omniDrop.classList.contains("hidden")) return;
  if (e.key === "ArrowDown") {
    e.preventDefault();
    omniIdx = Math.min(omniIdx + 1, omniItems.length - 1);
    omniHighlight();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    omniIdx = Math.max(omniIdx - 1, -1);
    omniHighlight();
  } else if (e.key === "Enter" && omniIdx >= 0) {
    e.preventDefault();
    const v = omniItems[omniIdx];
    omniHide();
    navigateActiveTab(v.url, v.title || undefined);
  } else if (e.key === "Escape") {
    omniHide();
  }
});
omniBox.addEventListener("blur", () => setTimeout(omniHide, 160));

// ---------------------------------------------------------------------
// Command palette (Ctrl+K)
// ---------------------------------------------------------------------

const paletteOverlay = document.getElementById("paletteOverlay");
const paletteInput = document.getElementById("paletteInput");

// Палитра — HTML, а нативные вкладки рисуются ПОВЕРХ шелла. Поэтому при
// открытии прячем все вебвью, при закрытии возвращаем активную вкладку.
function paletteShowFix() {
  if (typeof activeTabId !== "undefined" && activeTabId) {
    try { invokeV2("page_hide_all", {}); } catch (_) {}
  }
}
function paletteCloseRestore() {
  if (typeof activeTabId !== "undefined" && activeTabId && typeof switchTab === "function") {
    try { switchTab(activeTabId); } catch (_) {}
  }
}

document.addEventListener("keydown", (e) => {
  // e.code — физическая клавиша: работает и на русской раскладке
  const k = (e.key || "").toLowerCase();
  if ((e.ctrlKey || e.metaKey) && (k === "k" || e.code === "KeyK")) {
    e.preventDefault();
    paletteOverlay.classList.toggle("hidden");
    if (!paletteOverlay.classList.contains("hidden")) {
      paletteInput.value = "";
      paletteInput.focus();
      renderPalette("");
      paletteShowFix();
    } else {
      paletteCloseRestore();
    }
  }
  if (e.key === "Escape") {
    const wasPalette = !paletteOverlay.classList.contains("hidden");
    paletteOverlay.classList.add("hidden");
    if (internalOpen) closeInternal();
    if (wasPalette) paletteCloseRestore();
  }
  if ((e.ctrlKey || e.metaKey) && (k === "t" || e.code === "KeyT")) {
    e.preventDefault();
    // Новая вкладка = главный экран (как кнопка «+»)
    document.getElementById("newTabBtn").click();
  }
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "e") {
    e.preventDefault();
    toggleEmergencyShortcut().catch((err) => alert(err));
  }
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "Delete" || e.key === "Backspace")) {
    e.preventDefault();
    runPanicButton().catch((err) => alert(err));
  }
});

paletteOverlay.addEventListener("click", (e) => {
  if (e.target === paletteOverlay) {
    paletteOverlay.classList.add("hidden");
    paletteCloseRestore();
  }
});

paletteInput.addEventListener("input", (e) => renderPalette(e.target.value));

async function renderPalette(query) {
  const results = await invoke("search_commands", { query });
  const list = document.getElementById("paletteList");
  list.innerHTML = "";
  for (const c of results) {
    const li = document.createElement("li");
    li.innerHTML = `<span class="title">${escapeHtml(c.title)}</span><span class="meta">${c.shortcut || ""}</span>`;
    li.onclick = () => runCommand(c.id);
    list.appendChild(li);
  }
}

async function runCommand(id) {
  paletteOverlay.classList.add("hidden");
  paletteCloseRestore();
  switch (id) {
    case "tabs.new":
      document.getElementById("newTabBtn").click();
      break;
    case "notes.create":
      openSidePanel("notes");
      document.querySelector("#notes .add-form").setAttribute("open", "");
      break;
    case "bookmarks.add":
      openSidePanel("bookmarks");
      document.querySelector("#bookmarks .add-form").setAttribute("open", "");
      break;
    case "profiles.switch":
      openInternal("settings");
      break;
    case "history.clear":
      await clearHistoryAndRefresh();
      break;
    case "privacy.emergency":
      await toggleEmergencyShortcut();
      break;
    case "privacy.audit":
      openInternal("privacy");
      await refreshPrivacy();
      break;
    case "vault.lock":
      await invoke("vault_lock");
      openInternal("vault");
      await refreshVault();
      break;
    case "ai.chat":
      toggleAiPane(true);
      break;
    case "panic.button":
      await runPanicButton();
      break;
  }
}

// ---------------------------------------------------------------------
// Privacy Engine
// ---------------------------------------------------------------------

const PRIVACY_FLAGS = [
  ["flagTrackers", "block_trackers"],
  ["flagAds", "block_ads"],
  ["flagFpScripts", "block_fingerprinting_scripts"],
  ["flagThirdPartyCookies", "block_third_party_cookies"],
  ["flagHttpsOnly", "https_only"],
  ["flagClearOnExit", "clear_cookies_on_exit"],
];

let currentPrivacyPolicy = null;

// ---------------------------------------------------------------------
// Shield-индикатор в тулбаре (виден всегда — тулбар не перекрывается
// вебвью). MAX = фиолетовый бейдж, Emergency = красный пульсирующий.
// ---------------------------------------------------------------------
async function updateShieldIndicator(ovPre) {
  const el = document.getElementById("shieldBadge");
  if (!el) return;
  try {
    const ov = ovPre || await invoke("get_privacy_overview");
    const emg = !!ov.emergency;
    const max = String(ov.level) === "Maximum";
    el.classList.toggle("emergency", emg);
    el.classList.toggle("max", !emg && max);
    if (emg) {
      el.textContent = "🚨 Экстренный режим";
      el.title = "Emergency Privacy Mode включён — клик открывает настройки приватности";
      el.classList.remove("hidden");
    } else if (max) {
      el.textContent = "🛡 МАКС защита";
      el.title = "Включён максимальный уровень приватности — клик открывает настройки";
      el.classList.remove("hidden");
    } else {
      el.classList.add("hidden");
    }
  } catch (e) { /* профиль ещё не готов */ }
}

document.getElementById("shieldBadge").addEventListener("click", async () => {
  openInternal("privacy");
  await refreshPrivacy();
});

setTimeout(() => { try { if (typeof updateShieldIndicator === "function") updateShieldIndicator(); } catch {} }, 0);

async function refreshPrivacy() {
  const ov = await invoke("get_privacy_overview");
  currentPrivacyPolicy = ov.policy;

  const levelSel = document.getElementById("privacyLevelSelect");
  levelSel.value = String(ov.level);

  document.getElementById("emergencyBtn").classList.toggle("active-danger", !!ov.emergency);
  document.getElementById("emergencyBtn").textContent =
    (ov.emergency ? "🚨 Emergency Mode ВКЛЮЧЁН — выключить" : "🚨 Emergency Privacy Mode");

  for (const [elId, key] of PRIVACY_FLAGS) {
    const el = document.getElementById(elId);
    el.checked = !!ov.policy[key];
    el.onchange = async () => {
      if (!currentPrivacyPolicy) return;
      currentPrivacyPolicy[key] = el.checked;
      currentPrivacyPolicy.level = "Custom";
      await invoke("update_privacy_policy", { policy: currentPrivacyPolicy });
      await refreshPrivacy();
    };
  }

  const wrSel = document.getElementById("webrtcSelect");
  wrSel.value = ov.policy.webrtc || "Enabled";
  wrSel.onchange = async () => {
    if (!currentPrivacyPolicy) return;
    currentPrivacyPolicy.webrtc = wrSel.value;
    currentPrivacyPolicy.level = "Custom";
    await invoke("update_privacy_policy", { policy: currentPrivacyPolicy });
    await refreshPrivacy();
  };

  const eh = document.getElementById("enforceHint");
  if (eh && ov.enforcement) {
    eh.textContent = `Маршрут: ${ov.enforcement.upstream || "прямой"} · DNS: ${ov.enforcement.doh || "системный"} · ${ov.enforcement.webrtc || ""}`;
  }

  const auditList = document.getElementById("auditList");
  auditList.innerHTML = "";
  for (const f of ov.findings || []) {
    const li = document.createElement("li");
    const cls = f.status === "Ok" ? "audit-ok" : f.status === "Critical" ? "audit-critical" : "audit-warn";
    li.innerHTML = `<span class="dot ${cls}"></span><div><div class="title">${escapeHtml(f.area)}</div><div class="meta">${escapeHtml(f.message)}</div></div>`;
    auditList.appendChild(li);
  }

  const bc = document.getElementById("blockedCounter");
  const st = ov.stats || {};
  bc.textContent = `Заблокировано запросов: ${st.total_blocked || 0}` +
    (st.proxy_live ? "" : " (прокси не активен)") +
    (st.rules ? ` · правил в базе: ${st.rules}` : "");

  let rst = document.getElementById("blockedResetBtn");
  if (!rst) {
    rst = document.createElement("button");
    rst.id = "blockedResetBtn";
    rst.className = "ghost-btn";
    rst.style.cssText = "margin-top:6px;font-size:12px";
    rst.textContent = "⟳ Сбросить счётчик блокировок";
    rst.addEventListener("click", async () => {
      await invoke("privacy_reset_stats");
      await refreshPrivacy();
    });
    bc.parentElement.insertBefore(rst, bc.nextSibling);
  }

  let bs = document.getElementById("blockedSites");
  if (!bs) {
    bs = document.createElement("ul");
    bs.id = "blockedSites";
    bs.className = "list";
    rst.parentElement.insertBefore(bs, rst.nextSibling);
  }
  bs.innerHTML = "";
  for (const s of (st.per_site || [])) {
    const li = document.createElement("li");
    li.innerHTML = `<div class="meta">🚫 ${escapeHtml(s.site)} — <strong>${s.count}</strong></div>`;
    bs.appendChild(li);
  }

  // Свои списки блокировки: показываем и даём удалить
  const addForm = document.getElementById("blocklistAddBtn")?.closest("details");
  if (addForm) {
    let cl = document.getElementById("customLists");
    if (!cl) {
      cl = document.createElement("ul");
      cl.id = "customLists";
      cl.className = "list";
      addForm.parentElement.insertBefore(cl, addForm.nextSibling);
    }
    cl.innerHTML = "";
    for (const l of (ov.custom_lists || [])) {
      const li = document.createElement("li");
      const lines = l.text.split("\n").filter((s) => s.trim() && !s.trim().startsWith("#")).length;
      li.innerHTML = `<div><div class="title">${escapeHtml(l.name)}</div>` +
        `<div class="meta">${lines} доменов · ${escapeHtml(l.category)}</div></div>`;
      const x = document.createElement("button");
      x.className = "close";
      x.textContent = "✕";
      x.title = "Удалить список";
      x.onclick = async (ev) => {
        ev.stopPropagation();
        if (!(await confirm(`Удалить список «${l.name}» (${lines} доменов)?`))) return;
        try {
          await invoke("remove_blocklist", { name: l.name });
          await refreshPrivacy();
          toast("Список удалён");
        } catch (err) { alert("Ошибка: " + err); }
      };
      li.appendChild(x);
      cl.appendChild(li);
    }
  }

  // Исключения сайтов: сетевой блокировщик пропускает домен + поддомены
  const ovList = document.getElementById("siteOverridesList");
  if (ovList) {
    ovList.innerHTML = "";
    for (const o of (ov.site_overrides || [])) {
      const li = document.createElement("li");
      li.innerHTML =
        `<div><div class="title">🛡 ${escapeHtml(o.host)}</div>` +
        `<div class="meta">${o.allow_trackers ? "трекеры разрешены" : "в списке исключений"}</div></div>`;
      const x = document.createElement("button");
      x.className = "close";
      x.textContent = "✕";
      x.title = "Убрать исключение";
      x.onclick = async (ev) => {
        ev.stopPropagation();
        try {
          await invoke("site_override_remove", { host: o.host });
          await refreshPrivacy();
          toast("Исключение убрано");
        } catch (err) { alert("Ошибка: " + err); }
      };
      li.appendChild(x);
      ovList.appendChild(li);
    }
  }

  const dashboard = ov.dashboard || [];
  if (dashboard.length && !document.getElementById("fpDash")) {
    const p = document.createElement("p");
    p.className = "panel-subtitle";
    p.textContent = "Fingerprint-панель";
    auditList.parentElement.insertBefore(p, document.getElementById("blockedCounter").nextSibling);
    const ul = document.createElement("ul");
    ul.id = "fpDash";
    ul.className = "list";
    auditList.parentElement.insertBefore(ul, document.getElementById("blockedCounter"));
  }
  const fp = document.getElementById("fpDash");
  if (fp) {
    fp.innerHTML = "";
    for (const s of dashboard) {
      const li = document.createElement("li");
      li.innerHTML = `<div class="meta">${escapeHtml(s.surface)}: <strong>${escapeHtml(s.status)}</strong></div>`;
      fp.appendChild(li);
    }
  }

  updateShieldIndicator(ov);
}

document.getElementById("privacyLevelSelect").addEventListener("change", async (e) => {
  await invoke("set_privacy_level", { level: e.target.value });
  await refreshPrivacy();
});

document.getElementById("emergencyBtn").addEventListener("click", async () => {
  const on = !document.getElementById("emergencyBtn").classList.contains("active-danger");
  await invoke("set_emergency_mode", { on });
  await refreshPrivacy();
});

document.getElementById("blocklistAddBtn").addEventListener("click", async () => {
  const name = document.getElementById("blocklistName").value.trim() || "Мой список";
  const text = document.getElementById("blocklistText").value;
  if (!text.trim()) return;
  const added = await invoke("add_blocklist", { name, category: "Advertising", text });
  alert(`Добавлено доменов: ${added}`);
  document.getElementById("blocklistText").value = "";
  await refreshPrivacy();
});

document.getElementById("overrideAllowBtn").addEventListener("click", async () => {
  const input = document.getElementById("overrideHostInput");
  let host = input.value.trim();
  if (!host) {
    // Пустой ввод → берём домен активной вкладки (если это сайт)
    const t = typeof activeTab === "function" ? activeTab() : null;
    if (t && t.url && /^https?:/i.test(t.url)) {
      try { host = new URL(t.url).hostname; } catch (e) {}
    }
  }
  if (!host) return alert("Укажите домен, например example.com");
  try {
    const saved = await invoke("site_override_set", { host, allow: true });
    input.value = "";
    toast(`«${saved}» исключён из блокировки`);
    await refreshPrivacy();
  } catch (err) { alert("Ошибка: " + err); }
});

async function toggleEmergencyShortcut() {
  const ov = await invoke("get_privacy_overview");
  await invoke("set_emergency_mode", { on: !ov.emergency });
  openInternal("privacy");
  await refreshPrivacy();
}

// ---------------------------------------------------------------------
// Panic Button (Ctrl+Shift+Delete)
// ---------------------------------------------------------------------

async function runPanicButton() {
// Made by MrDuck
  const done = await invoke("panic_button");
  openInternal("privacy");
  await Promise.all([refreshHistory(), refreshPrivacy()]);
  alert("Panic Button выполнено:\n\n• " + done.join("\n• "));
}

// ---------------------------------------------------------------------
// Network
// ---------------------------------------------------------------------

let networkSettings = null;

async function refreshNetwork() {
  networkSettings = await invoke("get_network_settings");
  const mode = networkSettings.dns?.mode || "System";
  document.getElementById("dnsModeSelect").value = mode;
  document.getElementById("dohUrlInput").value = networkSettings.dns?.doh_url || "";

  const chainName = networkSettings.default_chain;
  let hop = null;
  if (chainName) hop = (networkSettings.chains?.[chainName]?.hops || [])[0] || null;
  document.getElementById("proxyTypeSelect").value = hop ? hop.kind : "";
  document.getElementById("proxyHostInput").value = hop ? hop.host : "";
  document.getElementById("proxyPortInput").value = hop ? hop.port : "";
}

document.getElementById("netSaveBtn").addEventListener("click", async () => {
  if (!networkSettings) networkSettings = {};
  networkSettings.dns = {
    mode: document.getElementById("dnsModeSelect").value,
    doh_url: document.getElementById("dohUrlInput").value.trim(),
    dot_host: "",
    custom_servers: [],
  };
  const ptype = document.getElementById("proxyTypeSelect").value;
  if (ptype) {
    const host = document.getElementById("proxyHostInput").value.trim();
    const port = parseInt(document.getElementById("proxyPortInput").value, 10) || 1080;
    networkSettings.add_chain = { name: "default", hops: [{ kind: ptype, host, port, username: null, password: null }] };
  } else {
    networkSettings.default_chain = null;
  }
  await invoke("save_network_settings", { settings: networkSettings });
  await refreshNetwork();
  alert("Настройки сети сохранены");
});

document.getElementById("routePreviewBtn").addEventListener("click", async () => {
  const host = document.getElementById("routeHostInput").value.trim();
  if (!host) return;
  const route = await invoke("route_preview", { host });
  const ul = document.getElementById("routePreview");
  ul.innerHTML = "";
  for (const n of route) {
    const li = document.createElement("li");
    li.innerHTML = `<div class="meta">${n.encrypted ? "🔐" : "→"} ${escapeHtml(n.label)}</div>`;
    li.title = n.note || "";
    ul.appendChild(li);
  }
});

document.getElementById("netDiagBtn").addEventListener("click", async () => {
  const results = await invoke("run_network_diagnostics");
  const ul = document.getElementById("diagList");
  ul.innerHTML = "";
  for (const r of results) {
    const li = document.createElement("li");
    li.innerHTML = `<div><div class="title">${r.ok ? "✅" : "❌"} ${escapeHtml(r.name)}</div><div class="meta">${escapeHtml(r.detail || "")}</div></div>`;
    ul.appendChild(li);
  }
});

// ---------------------------------------------------------------------
// Secure Vault
// ---------------------------------------------------------------------

function setVaultUi(created, unlocked) {
  const line = document.getElementById("vaultStatusLine");
  if (line) line.textContent = created ? (unlocked ? "Сейф разблокирован" : "Сейф заблокирован — введите фразу") : "Сейф не создан";
  document.getElementById("vaultSetupBox").classList.toggle("hidden", created && unlocked);
  document.getElementById("vaultContentBox").classList.toggle("hidden", !unlocked);
  document.getElementById("vaultLockBtn").classList.toggle("hidden", !unlocked);
}

let vaultCache = [];

async function refreshVault() {
  let st;
  try { st = await invoke("vault_status"); } catch { return; }
  setVaultUi(st.created, st.unlocked);
  vaultCache = [];
  if (st.unlocked) {
    try { vaultCache = await invoke("vault_list"); } catch { /* locked meanwhile */ }
  }
  renderVaultEntries();
}

function renderVaultEntries() {
  const grid = document.getElementById("vaultEntries");
  if (!grid) return;
  const qEl = document.getElementById("vaultSearch");
  const q = qEl ? qEl.value.trim().toLowerCase() : "";
  grid.innerHTML = "";
  const rows = vaultCache.filter((r) => {
    if (!q) return true;
    const hay = `${r[1]} ${r[2]}`.toLowerCase();
    return hay.includes(q);
  });
  if (!rows.length) {
    grid.innerHTML = '<div class="hint" style="grid-column:1/-1;text-align:center;padding:26px 0">' +
      (q ? "Ничего не найдено" : "Записей пока нет — добавьте первую через форму ниже") + "</div>";
    return;
  }
  for (const [id, title, meta] of rows) {
    const card = document.createElement("div");
    card.className = "pw-card";
    const head = document.createElement("div");
    head.className = "pc-title";
    const fav = document.createElement("span");
    fav.className = "pc-fav";
    fav.textContent = (title[0] || "?").toUpperCase();
    const tt = document.createElement("span");
    tt.textContent = title;
    head.append(fav, tt);
    const user = document.createElement("div");
    user.className = "pc-user";
    user.textContent = meta && meta !== "—" ? meta : "без логина";
    const btns = document.createElement("div");
    btns.className = "pc-btns";
    const mk = (label, fn) => {
      const b = document.createElement("button");
      b.textContent = label;
      b.onclick = fn;
      return b;
    };
    btns.appendChild(mk("👁 Показать", async () => {
      try {
        const e = await invoke("vault_reveal", { id });
        uiDialog({
          message:
            `${e.title}\n\nЛогин: ${e.username || "—"}\nПароль: ${e.password || "—"}` +
            (e.url ? `\nURL: ${e.url}` : ""),
          kind: "alert",
        });
      } catch (err) { alert(err); }
    }));
    btns.appendChild(mk("📋 Логин", async () => {
      try {
        const e = await invoke("vault_reveal", { id });
        if (e.username) { await navigator.clipboard.writeText(e.username); toast("Логин скопирован"); }
      } catch (err) { alert(err); }
    }));
    btns.appendChild(mk("🔑 Пароль", async () => {
      try {
        const e = await invoke("vault_reveal", { id });
        if (e.password) { await navigator.clipboard.writeText(e.password); toast("Пароль скопирован"); }
      } catch (err) { alert(err); }
    }));
    card.append(head, user, btns);
    grid.appendChild(card);
  }
}
document.getElementById("vaultSearch")?.addEventListener("input", renderVaultEntries);

document.getElementById("vaultCreateBtn").addEventListener("click", async () => {
  const pass = document.getElementById("vaultPassInput").value;
  if (!pass) return;
  try {
    await invoke("vault_create", { passphrase: pass });
    document.getElementById("vaultPassInput").value = "";
  } catch (e) { alert(e); }
  await refreshVault();
});

document.getElementById("vaultUnlockBtn").addEventListener("click", async () => {
  const pass = document.getElementById("vaultPassInput").value;
  if (!pass) return;
  try {
    await invoke("vault_unlock", { passphrase: pass });
    document.getElementById("vaultPassInput").value = "";
  } catch (e) { alert(e); }
  await refreshVault();
});

document.getElementById("vaultLockBtn").addEventListener("click", async () => {
  await invoke("vault_lock");
  await refreshVault();
});

document.getElementById("veAddBtn").addEventListener("click", async () => {
  const title = document.getElementById("veTitle").value.trim();
  const username = document.getElementById("veUser").value.trim();
  const password = document.getElementById("vePass").value;
  const url = document.getElementById("veUrl").value.trim() || null;
  if (!title || !password) return;
  try {
    await invoke("vault_add_entry", {
      // Backend serde tag is "kind" with snake_case variants.
      kind: { kind: "password", title, username, password, url, totp_secret: null },
    });
    document.getElementById("veTitle").value = "";
    document.getElementById("veUser").value = "";
    document.getElementById("vePass").value = "";
    document.getElementById("veUrl").value = "";
  } catch (e) { alert(e); }
  await refreshVault();
});

// Highlight vault buttons once the passphrase is typed
document.getElementById("vaultPassInput")?.addEventListener("input", (e) => {
  const ready = e.target.value.trim().length >= 8;
  document.getElementById("vaultCreateBtn").classList.toggle("ready", ready);
  document.getElementById("vaultUnlockBtn").classList.toggle("ready", ready);
});

document.getElementById("pwGenBtn").addEventListener("click", async () => {
  const pw = await invoke("vault_generate_password", { length: 20 });
  document.getElementById("pwGenOut").textContent = pw;
  document.getElementById("pwGenRow").classList.remove("hidden");
});
document.getElementById("pwGenAgain")?.addEventListener("click", async () => {
  const pw = await invoke("vault_generate_password", { length: 20 });
  document.getElementById("pwGenOut").textContent = pw;
});
document.getElementById("pwGenCopy")?.addEventListener("click", async () => {
  const v = document.getElementById("pwGenOut").textContent;
  if (v) await navigator.clipboard.writeText(v);
});

// --- Import / export passwords (CSV, Chrome-compatible) ---

function csvEscape(v) {
  const s = String(v ?? "");
  return /[",\r\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s;
}
function parseCSV(text) {
  // RFC-4180-ish parser
  const rows = [];
  let row = [], cur = "", inQ = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (inQ) {
      if (ch === '"') {
        if (text[i + 1] === '"') { cur += '"'; i++; }
        else inQ = false;
      } else cur += ch;
    } else if (ch === '"') inQ = true;
    else if (ch === ",") { row.push(cur); cur = ""; }
    else if (ch === "\n" || ch === "\r") {
      if (ch === "\r" && text[i + 1] === "\n") i++;
      row.push(cur); cur = "";
      if (row.length > 1 || row[0] !== "") rows.push(row);
      row = [];
    } else cur += ch;
  }
  if (cur !== "" || row.length) { row.push(cur); rows.push(row); }
  return rows;
}

document.getElementById("pwExportBtn")?.addEventListener("click", async () => {
  try {
    if (!vaultCache.length) { alert("Сейф пуст — нечего экспортировать."); return; }
    const entries = [];
    for (const [id] of vaultCache) {
      try {
        const e = await invoke("vault_reveal", { id });
        entries.push(e);
      } catch { /* skip unreadable */ }
    }
    // \uFEFF (BOM) в начале — иначе Excel открывает UTF-8 CSV с битой кириллицей.
    let csv = "\uFEFFname,url,username,password\r\n";
    for (const e of entries) {
      csv += [csvEscape(e.title), csvEscape(e.url), csvEscape(e.username), csvEscape(e.password)].join(",") + "\r\n";
    }
    const path = await invoke("save_text_file", {
      name: "apb-passwords-" + new Date().toISOString().slice(0, 10) + ".csv",
      contents: csv,
    });
    toast("Экспортировано: " + path);
  } catch (e) { alert("Ошибка экспорта: " + e); }
});

document.getElementById("pwImportBtn")?.addEventListener("click", () => document.getElementById("pwImportFile").click());
document.getElementById("pwImportFile")?.addEventListener("change", async (e) => {
  const file = e.target.files && e.target.files[0];
  e.target.value = "";
  if (!file) return;
// Made by MrDuck
  let rows;
  try { rows = parseCSV(await file.text()); }
  catch { alert("Не удалось прочитать файл как CSV."); return; }
  if (!rows.length) { alert("Файл пуст."); return; }
  // Header detection: сначала точные имена колонок Chrome/Bitwarden, потом
  // общие слова. \uFEFF срезаем — наш экспорт кладёт BOM ради Excel.
  const head = rows[0].map((h) => h.replace(/^\uFEFF/, "").trim().toLowerCase());
  const findCol = (...patterns) => {
    for (const re of patterns) {
      const i = head.findIndex((h) => re.test(h));
      if (i >= 0) return i;
    }
    return -1;
  };
  // "username" содержит "name" — отсекаем негативным lookahead, иначе
  // имя записи уедет в логин (например, на CSV из Firefox).
  let iName = findCol(/^(name|title|имя)$/, /^(?!.*(user|логин)).*(name|title|имя)/);
  let iUrl = findCol(/^(url|login_uri)$/, /url|uri|website|сайт/);
  let iUser = findCol(/^(username|login_username)$/, /username|login|user|логин/);
  let iPass = findCol(/^(password|login_password)$/, /pass|пароль/);
  let dataRows;
  if (iPass < 0) { // заголовок не распознан: считаем колонки name,url,user,pass с первой строки
    iName = 0; iUrl = 1; iUser = 2; iPass = 3;
    dataRows = rows;
  } else {
    dataRows = rows.slice(1);
  }
  const hostOf = (u) => { try { return new URL(u).hostname.replace(/^www\./, ""); } catch { return ""; } };
  const valid = dataRows
    .map((r) => ({
      title: (r[iName] || "").trim(),
      url: (r[iUrl] || "").trim() || null,
      username: (iUser >= 0 ? (r[iUser] || "") : "").trim() || null,
      password: (r[iPass] || "").trim(),
    }))
    .map((r) => ({ ...r, title: r.title || hostOf(r.url || "") })) // Firefox отдаёт записи без name-колонки
    .filter((r) => r.title && r.password && r.password.toLowerCase() !== "password");
  if (!valid.length) { alert("Не найдено ни одной записи с названием и паролем."); return; }
  if (!(await confirm(`Импортировать ${valid.length} записей в сейф?`))) return;
  let ok = 0;
  for (const r of valid) {
    try {
      await invoke("vault_add_entry", {
        kind: { kind: "password", title: r.title, username: r.username, password: r.password, url: r.url, totp_secret: null },
      });
      ok++;
    } catch { /* skip broken row */ }
  }
  toast(`Импортировано записей: ${ok} из ${valid.length}`);
  await refreshVault();
});

// ---------------------------------------------------------------------
// AI assistant — right-side chat pane (like the note editor).
// First open shows provider setup; after saving, a normal chat.
// ---------------------------------------------------------------------

const AI_KIND_TO_ENUM = {
  ollama: "Ollama",
  open_ai_compatible: "OpenAiCompatible",
  anthropic_compatible: "AnthropicCompatible",
  custom_http: "CustomHttp",
};

const aiPane = document.getElementById("aiPane");

function aiConfigured() { return localStorage.getItem("apb-ai-configured") === "1"; }

function showAiSetup() {
  document.getElementById("aiSetup").classList.remove("hidden");
  document.getElementById("aiChatWrap").classList.add("hidden");
}
function showAiChat() {
  document.getElementById("aiSetup").classList.add("hidden");
  document.getElementById("aiChatWrap").classList.remove("hidden");
}

async function toggleAiPane(forceOpen = false) {
  const willOpen = forceOpen || aiPane.classList.contains("hidden");
  if (!willOpen) {
    aiPane.classList.add("hidden");
    syncPageLayout();
    return;
  }
  closeInternal(false);
  closeSidePanelKeepRect();
  aiPane.classList.remove("hidden");
  syncPageLayout(true);
  setTimeout(syncPageLayout, 220);
  await refreshAi();
  if (aiConfigured()) showAiChat(); else showAiSetup();
}

function closeSidePanelKeepRect() {
  sidePanel.classList.remove("open");
  document.querySelectorAll(".rail-item").forEach((b) => b.classList.toggle("active", b.dataset.tab === "ai"));
}

document.getElementById("aiClose").addEventListener("click", () => toggleAiPane(false));
document.getElementById("aiCfgToggle").addEventListener("click", () => {
  const setupHidden = document.getElementById("aiSetup").classList.contains("hidden");
  if (setupHidden) showAiSetup(); else showAiChat();
});

async function refreshAi() {
  let cfg;
  try { cfg = await invoke("ai_get_config"); } catch { return; }
  const enumToValue = Object.fromEntries(Object.entries(AI_KIND_TO_ENUM).map(([k, v]) => [v, k]));
  document.getElementById("aiKindSelect").value = enumToValue[cfg.kind] || "ollama";
  document.getElementById("aiBaseUrl").value = cfg.base_url || "";
  document.getElementById("aiModel").value = cfg.model || "";
  document.getElementById("aiKeyEnv").value = cfg.api_key_env || "";
}

document.getElementById("aiSaveCfgBtn").addEventListener("click", async () => {
  const status = document.getElementById("aiSetupStatus");
  const value = document.getElementById("aiKindSelect").value;
  try {
    await invoke("ai_save_config", {
      config: {
        kind: AI_KIND_TO_ENUM[value],
        base_url: document.getElementById("aiBaseUrl").value.trim(),
        model: document.getElementById("aiModel").value.trim() || "llama3.2",
        api_key_env: document.getElementById("aiKeyEnv").value.trim() || null,
        max_context_chars: 6000,
      },
    });
    localStorage.setItem("apb-ai-configured", "1");
    status.textContent = "";
    showAiChat();
  } catch (e) {
    status.textContent = "Ошибка сохранения: " + e;
  }
});

function chatMsgEl(role, text) {
  const d = document.createElement("div");
  d.className = "chat-msg " + role;
  d.textContent = text;
  return d;
}
function scrollChat() {
  const log = document.getElementById("aiReply");
  log.scrollTop = log.scrollHeight;
}
function autosizeAi() {
  const t = document.getElementById("aiPrompt");
  t.style.height = "auto";
  t.style.height = Math.min(t.scrollHeight, 120) + "px";
}
document.getElementById("aiPrompt").addEventListener("input", autosizeAi);

async function sendAi() {
  const input = document.getElementById("aiPrompt");
  const text = input.value.trim();
  const useCtx = document.getElementById("aiCtxBtn")?.classList.contains("active");
  if (!text && !useCtx) return;
  const log = document.getElementById("aiReply");
  const meta = document.getElementById("aiMeta");
  log.appendChild(chatMsgEl("user", text || "(про текущую страницу)"));
  input.value = "";
  autosizeAi();
  scrollChat();
  const btn = document.getElementById("aiAskBtn");
  btn.disabled = true;
  meta.textContent = "Думаю…";

  let pageTitle = null;
  let pageContent = null;
  if (useCtx && activeTab()) {
    try {
      const ext = await invoke("page_extract_text", { url: activeTab().url });
      pageTitle = ext.title || activeTab().label;
      pageContent = ext.text;
      meta.textContent = "Страница прочитана, спрашиваю модель…";
    } catch (e) {
      log.appendChild(chatMsgEl("err", "Не удалось прочитать страницу: " + e));
      btn.disabled = false;
      meta.textContent = "";
      scrollChat();
      return;
    }
  }
  try {
    const report = await invoke("ai_chat", {
      prompt: text || "Кратко перескажи страницу ниже.",
      pageTitle,
      pageContent,
    });
    log.appendChild(chatMsgEl("bot", report.reply));
    meta.textContent =
      `${report.provider_local ? "локальный" : "облачный"} провайдер · секретов отфильтровано: ${report.secrets_blocked}`;
    uiSound(920, 0.05);
  } catch (e) {
    log.appendChild(chatMsgEl("err", "Ошибка: " + e));
    meta.textContent = "";
  }
  btn.disabled = false;
  scrollChat();
}
document.getElementById("aiAskBtn").addEventListener("click", sendAi);
document.getElementById("aiPrompt").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendAi(); }
});
document.getElementById("aiCtxBtn")?.addEventListener("click", (e) => {
  const on = e.currentTarget.classList.toggle("active");
  e.currentTarget.title = on
    ? "Текст страницы ДОБАВЛЯЕТСЯ к запросу (клик — выключить)"
    : "Добавлять текст активной страницы в запрос";
});

// Translate the active page via the configured AI provider
document.getElementById("aiTranslateBtn")?.addEventListener("click", async () => {
  const t = currentTabObj();
  if (!t) { alert("Нет открытой страницы для перевода."); return; }
  toggleAiPane(true);
  const lang = await prompt("Перевести страницу на:", "русский");
  if (!lang) return;
  const log = document.getElementById("aiReply");
  const meta = document.getElementById("aiMeta");
  const btn = document.getElementById("aiAskBtn");
  log.appendChild(chatMsgEl("user", `🌍 Переведи страницу «${t.label}» на ${lang}`));
  scrollChat();
  btn.disabled = true;
  meta.textContent = "Читаю страницу…";
  try {
    const ext = await invoke("page_extract_text", { url: t.url });
    const body = (ext.text || "").slice(0, 5000);
    if (!body) throw new Error("пустой текст страницы");
    meta.textContent = "Перевожу…";
    const report = await invoke("ai_chat", {
      prompt: `Переведи следующий текст на ${lang}. Выведи только перевод без комментариев:\n\n${body}`,
      pageTitle: null,
      pageContent: null,
    });
    log.appendChild(chatMsgEl("bot", report.reply));
    meta.textContent = `переведено через ${report.provider_local ? "локальную" : "облачную"} модель`;
  } catch (e) {
    log.appendChild(chatMsgEl("err", "Ошибка перевода: " + e));
    meta.textContent = "";
  }
  btn.disabled = false;
  scrollChat();
});

// ---------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------

async function refreshExtensions() {
  // Расширения СПРЯТАНЫ (сессия 111): страницы нет в DOM — тихо пропускаем,
  // код оставлен для расширений v2.
  if (!document.getElementById("extList")) return;
  const exts = await invoke("ext_list");
  const ul = document.getElementById("extList");
  ul.innerHTML = "";
  if (exts.length === 0) {
    ul.innerHTML = '<li class="empty">Расширения не установлены</li>';
    return;
  }
  for (const e of exts) {
    const li = document.createElement("li");
    const masks = (e.manifest.matches || []).join(", ") || "—";
    li.innerHTML = `<div><div class="title">${escapeHtml(e.manifest.name)} v${escapeHtml(e.manifest.version)}</div><div class="meta">id: ${escapeHtml(e.manifest.id)} · ${e.enabled_globally ? "включено" : "выключено"} · скрипт: ${escapeHtml(e.manifest.entry_point)} · сайты: ${escapeHtml(masks)}${e.manifest.permissions?.length ? " · права: " + e.manifest.permissions.join(", ") : ""}</div></div>`;

    const toggle = document.createElement("button");
    toggle.className = "ghost-btn";
    toggle.textContent = e.enabled_globally ? "Выключить" : "Включить";
    toggle.onclick = async () => {
      await invoke("ext_set_enabled", { extId: e.manifest.id, enabled: !e.enabled_globally });
      await refreshExtensions();
    };

    // Выдача прав: кнопка на каждое ЗАЯВЛЕННОЕ в манифесте право (выдать
    // можно только заявленное — бэкенд это проверяет). Опасные права после
    // выдачи требуют отдельного подтверждения (ext_approve_dangerous).
    const DANGEROUS = ["all_websites", "cookies", "history", "network", "filesystem", "ai_context"];
    for (const p of e.manifest.permissions || []) {
      const pb = document.createElement("button");
      pb.className = "ghost-btn";
      pb.textContent = "＋ " + p;
      pb.onclick = async () => {
        try {
          await invoke("ext_grant", { extId: e.manifest.id, perms: [p] });
          if (DANGEROUS.includes(p)) {
            const ok = await confirm(`Расширение «${e.manifest.name}» получит опасное право ${p}. Подтвердить?`);
            if (ok) await invoke("ext_approve_dangerous", { extId: e.manifest.id, perm: p });
          }
          const pol = await invoke("ext_sandbox_policy", { extId: e.manifest.id });
          alert("Право выдано: " + p + ".\nАктивные возможности: " + (pol.capabilities.join("; ") || "нет"));
        } catch (err) { alert(err); }
        await refreshExtensions();
      };
      li.appendChild(pb);
    }

    li.appendChild(toggle);
    ul.appendChild(li);
  }
}

document.getElementById("extInstallBtn")?.addEventListener("click", async () => {
  const path = document.getElementById("extPathInput").value.trim();
  if (!path) return;
  try {
    const installed = await invoke("ext_install", { path });
    alert(`Установлено: ${installed.manifest.name}. По умолчанию прав нет — выдайте нужные явно.`);
  } catch (e) { alert("Ошибка установки: " + e); }
  await refreshExtensions();
});


// Made by MrDuck