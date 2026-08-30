// Made by MrDuck
// ---------------------------------------------------------------------
// Theme (dark / light, persisted). Переключатель темы в тулбаре убран —
// выбор темы живёт в настройках (.theme-buttons).
// ---------------------------------------------------------------------

function applyTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  document.querySelectorAll(".theme-buttons button").forEach((b) => {
    b.classList.toggle("active", b.dataset.themeChoice === theme);
  });
  localStorage.setItem("apb-theme", theme);
}

document.querySelectorAll(".theme-buttons button").forEach((b) => {
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
});


// Made by MrDuck