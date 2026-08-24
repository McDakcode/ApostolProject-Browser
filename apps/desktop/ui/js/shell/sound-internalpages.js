// Made by MrDuck && Ox-Alpha
// ---------------------------------------------------------------------
// UI sounds — tiny WebAudio blips, no assets
// ---------------------------------------------------------------------

let _audioCtx = null;
function uiSound(freq = 660, dur = 0.05, type = "triangle", vol = 0.05) {
  if (!pzLoad().sound) return;
  try {
    _audioCtx = _audioCtx || new (window.AudioContext || window.webkitAudioContext)();
    const o = _audioCtx.createOscillator();
    const g = _audioCtx.createGain();
    o.type = type;
    o.frequency.value = freq;
    g.gain.setValueAtTime(vol, _audioCtx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.0001, _audioCtx.currentTime + dur);
    o.connect(g).connect(_audioCtx.destination);
    o.start();
    o.stop(_audioCtx.currentTime + dur);
  } catch { /* audio unavailable */ }
}

document.querySelectorAll(".rail-item").forEach((btn) => {
  btn.addEventListener("click", () => {
    if (btn.dataset.tab === "ai") { toggleAiPane(); return; }
    const isOpen = sidePanel.classList.contains("open");
    const isThisActive = btn.classList.contains("active");
    if (isOpen && isThisActive) {
      closeSidePanel();
    } else {
      openSidePanel(btn.dataset.tab);
    }
  });
});

document.getElementById("settingsBtn").addEventListener("click", () => {
  if (internalOpen === "settings") closeInternal();
  else openInternal("settings");
});


// ---------------------------------------------------------------------
// Internal pages (settings / vault / extensions) — "site-like" pages
// that take over the content area instead of living in a drawer.
// ---------------------------------------------------------------------

const internalHost = document.getElementById("internalHost");
let internalOpen = null;
let internalReturnId = null;

function closeInternal(restore = true) {
  if (!internalOpen) return;
  internalOpen = null;
  internalHost.classList.add("hidden");
  if (restore && internalReturnId && tabs.some((t) => t.id === internalReturnId)) {
    switchTab(internalReturnId);
  } else if (restore && !activeTabId) {
    showHome();
  }
}

async function openInternal(id) {
  await invokeV2("page_hide_all").catch(() => {});
  internalReturnId = activeTabId;
  activeTabId = null;
  internalOpen = id;
  sidePanel.classList.remove("open");
  document.querySelectorAll(".rail-item").forEach((b) => b.classList.remove("active"));
  internalHost.classList.remove("hidden");
  document.getElementById("browserEmpty").classList.add("hidden");
  document.querySelectorAll("#internalHost .panel").forEach((p) =>
    p.classList.toggle("active", p.id === id));
  document.querySelectorAll(".isec-link").forEach((b) =>
    b.classList.toggle("active", b.dataset.isec === id));
  renderTabStrip();
  if (id === "settings") {
    pzSyncControls();
  }
  if (id === "appearance") {
    pzSyncControls();
  }
  if (id === "weatherSettings") {
    renderWeatherPreview();
    const cc = document.getElementById("wxCurCity");
    try {
      const cfg = JSON.parse(localStorage.getItem("apb-weather"));
      if (cc) cc.textContent = cfg?.city ? `Текущий город: ${cfg.city}` : "Город не выбран";
    } catch {}
  }
  if (id === "history") {
    loadFullHistory();
  }
}

document.querySelectorAll(".isec-link").forEach((b) =>
  b.addEventListener("click", () => openInternal(b.dataset.isec)));

async function refreshSidePanels() {
  await Promise.all([
    refreshBookmarks(),
    refreshHistory(),
    refreshDownloads(),
    refreshNotes(),
    refreshPrivacy(),
    refreshNetwork(),
    refreshVault(),
    refreshExtensions(),
  ]).catch((e) => console.error("panel refresh", e));
}


// Made by MrDuck && Ox-Alpha