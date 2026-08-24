// Made by MrDuck && Ox-Alpha
// ---------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------

async function loadProfiles() {
  const profiles = await invoke("list_profiles");
  const active = await invoke("active_profile");
  activeProfileName = active.name || "";
  activeStorageMode = active.storage_mode === "Ephemeral" ? "Ephemeral" : "Persistent";

  const select = document.getElementById("profileSelect");
  select.innerHTML = "";
  for (const p of profiles) {
    const opt = document.createElement("option");
    opt.value = p.id;
    opt.textContent = p.name;
    if (p.id === active.id) opt.selected = true;
    select.appendChild(opt);
  }

  const list = document.getElementById("profilesList");
  list.innerHTML = "";
  for (const p of profiles) {
    const li = document.createElement("li");
    li.className = p.id === active.id ? "active-profile" : "";
    li.dataset.id = p.id;       // для контекст-меню
    li.dataset.name = p.name;
    const anonBadge = p.storage_mode === "Ephemeral" ? " · анонимный" : "";
    li.innerHTML = `<div><div class="title">${escapeHtml(p.name)}${p.id === active.id ? " (текущий)" : ""}</div><div class="meta">${p.privacy_level}${anonBadge}</div></div>`;
    if (p.id !== active.id) {
      li.onclick = async () => {
        await invoke("switch_profile", { id: p.id });
        await loadProfiles();
        await refreshSidePanels();
      };
    }
    list.appendChild(li);
  }

  const badges = document.getElementById("privacyBadges");
  badges.innerHTML = `
    <span class="badge">${active.privacy_level}</span>
    <span class="badge">${active.storage_mode === "Ephemeral" ? "история не пишется" : "история сохраняется"}</span>
    <span class="badge">поиск: ${active.search_engine}</span>
  `;
}

document.getElementById("profileSelect").addEventListener("change", async (e) => {
  await invoke("switch_profile", { id: e.target.value });
  await loadProfiles();
  await refreshSidePanels();
});

async function createProfileFromForm() {
  const name = document.getElementById("newProfileName").value.trim();
  const anon = document.getElementById("newProfileAnon").checked;
  if (!anon && !name) return;

  const profile = anon ? await invoke("create_anonymous_profile") : await invoke("create_profile", { name });
  document.getElementById("newProfileName").value = "";
  document.getElementById("newProfileAnon").checked = false;

  await invoke("switch_profile", { id: profile.id });
  await loadProfiles();
  await refreshSidePanels();
}

document.getElementById("createProfileBtn").addEventListener("click", createProfileFromForm);

document.getElementById("quickCreateProfileBtn").addEventListener("click", async () => {
  await openInternal("settings");
  const details = document.querySelector("#settings .add-form");
  details.setAttribute("open", "");
  details.scrollIntoView({ block: "nearest" });
  document.getElementById("newProfileName").focus();
});


// ---------------------------------------------------------------------
// Side panel (Bookmarks / History / Notes / Settings) — a collapsible
// drawer next to the rail. Clicking the active item again closes it,
// so the browser view gets full width back.
// ---------------------------------------------------------------------

const sidePanel = document.getElementById("sidePanel");

function openSidePanel(tabId) {
  closeInternal(false);
  document.querySelectorAll(".rail-item").forEach((b) => b.classList.toggle("active", b.dataset.tab === tabId));
  document.querySelectorAll(".panel").forEach((p) => p.classList.toggle("active", p.id === tabId));
  sidePanel.classList.add("open");
  syncPageLayout();
}

function closeSidePanel() {
  document.querySelectorAll(".rail-item").forEach((b) => b.classList.remove("active"));
  sidePanel.classList.remove("open");
  syncPageLayout();
  ensureHomeVisible();
}

// Safety net: if nothing is visible in the content area, bring the home
// screen back (fixes rare "black canvas" states).
function ensureHomeVisible() {
  const tabAlive = activeTabId && tabs.some((t) => t.id === activeTabId);
  const internalVisible = !internalHost.classList.contains("hidden") && internalOpen;
  if (!tabAlive && !internalVisible) {
    const be = document.getElementById("browserEmpty");
    if (be.classList.contains("hidden")) showHome();
  }
}
// NOTE: the syncPageLayout wrapper lives at the END of
// 06-session-ws-downloads-tabs.js — it must run AFTER the real
// syncPageLayout declaration (no cross-file hoisting anymore).

// Made by MrDuck && Ox-Alpha