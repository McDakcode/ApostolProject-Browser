// Made by MrDuck
// ---------------------------------------------------------------------
// Theme (dark / light, persisted). Переключатель темы в тулбаре убран —
// выбор темы живёт в настройках (.theme-buttons).
// Хранение ДВОЙНОЕ: localStorage (мгновенно для UI) + видимый файл
// %APPDATA%/dev.apb.browser/settings.json + themes/theme.css (юзер может
// посмотреть/забэкапить; см. cmd/userfiles.rs).
// ---------------------------------------------------------------------

function applyTheme(theme) {
  // Гварды: theme-buttons стилизует и блок сейфа (без data-theme-choice) —
  // раньше клик по «Создать сейф» звал applyTheme(undefined) и ЗАПИСЫВАЛ
  // "undefined" в localStorage. Битое значение восстанавливаем в dark.
  if (!theme || theme === "undefined" || theme === "null") theme = "dark";
  document.documentElement.setAttribute("data-theme", theme);
  document.querySelectorAll(".theme-buttons button[data-theme-choice]").forEach((b) => {
    b.classList.toggle("active", b.dataset.themeChoice === theme);
  });
  localStorage.setItem("apb-theme", theme);
  try { invoke("settings_theme_save", { theme }); } catch { /* файл — удобная копия, не критично */ }
}

document.querySelectorAll(".theme-buttons button[data-theme-choice]").forEach((b) => {
  b.addEventListener("click", () => applyTheme(b.dataset.themeChoice));
});

applyTheme(localStorage.getItem("apb-theme") || "dark");

// ---------------------------------------------------------------------
// Search engine preference (used by the address-bar fallback)
// ---------------------------------------------------------------------

const SEARCH_ENGINES = {
  duckduckgo: "https://duckduckgo.com/?q=",
  google: "https://www.google.com/search?q=",
  bing: "https://www.bing.com/search?q=",
  startpage: "https://www.startpage.com/sp/search?query=",
};

function getSearchEngine() {
  return localStorage.getItem("apb-search-engine") || "duckduckgo";
}

const searchEngineSelect = document.getElementById("searchEngineSelect");
searchEngineSelect.value = getSearchEngine();
searchEngineSelect.addEventListener("change", (e) => {
  localStorage.setItem("apb-search-engine", e.target.value);
  try { invoke("settings_search_save", { engine: e.target.value }); } catch {}
});
// Первичная синхронизация текущего значения в settings.json.
try { invoke("settings_search_save", { engine: getSearchEngine() }); } catch {}


// Made by MrDuck