// Made by MrDuck
// ---------------------------------------------------------------------
// APB FX — живой фон-созвездие + стеклянный интерфейс.
//
// СОЗВЕЗДИЕ (particles): canvas ПОД всем UI (#apbFxCanvas, z-index:0,
// pointer-events:none), точки дрейфуют, между близкими — линии. Ближние
// к курсору точки притягиваются ИЛИ отталкиваются — клик по пустому
// месту шелла переключает режим (сразу видно «приближаются/удаляются»).
// Нативные вебвью сайтов рисуются ПОВЕРХ шелла — созвездие видно в
// сайдбаре/панелях (в glass-режиме — сквозь них) и на «домашней»
// странице. rAF (на 200 Гц — 200 кадров/с), пауза при скрытом окне.
// prefers-reduced-motion — созвездие не стартует вовсе.
//
// GLASS (<html>.apb-glass): панели и хром становятся полупрозрачными с
// blur — сквозь них видно созвездие. Хранение: localStorage (как тема).
// ---------------------------------------------------------------------

(function () {
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // === Созвездие ===
  const canvas = document.createElement("canvas");
  canvas.id = "apbFxCanvas";
  document.body.appendChild(canvas);
  const ctx = canvas.getContext("2d");
  let W = 0, H = 0, DPR = 1;
  let pts = [];
  let pkgs = [];  // «пакеты» network-анимации: искры бегут по линиям сети
  const PKG_MAX = 14;
  let mouse = { x: -9999, y: -9999, active: false };
  let repel = true; // true = точки разбегаются от курсора, false = тянутся к нему
  let running = false;
  let inited = false;

  function cssVar(v) {
    return getComputedStyle(document.documentElement).getPropertyValue(v).trim();
  }

  function accentColor() {
    const a = cssVar("--accent") || "#7fb0ff";
    return a.startsWith("#") ? a.slice(0, 7) : "#7fb0ff";
  }

  function hexRgb(hex) {
    const h = hex.replace("#", "");
    const v = h.length === 3 ? h.split("").map((c) => c + c).join("") : h.padEnd(6, "0").slice(0, 6);
    return parseInt(v.slice(0, 2), 16) + "," + parseInt(v.slice(2, 4), 16) + "," + parseInt(v.slice(4, 6), 16);
  }

  function resize() {
    DPR = Math.min(window.devicePixelRatio || 1, 2);
    W = Math.max(1, window.innerWidth);
    H = Math.max(1, window.innerHeight);
    canvas.width = W * DPR; canvas.height = H * DPR;
    ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
  }

  function spawn() {
    // плотность: точка на ~23k px² — на 1440p ~90 точек, живо и дёшево
    const n = Math.min(140, Math.max(36, Math.round((W * H) / 23000)));
    pts = [];
    for (let i = 0; i < n; i++) {
      pts.push({
        x: Math.random() * W,
        y: Math.random() * H,
        vx: (Math.random() - 0.5) * 0.35,
        vy: (Math.random() - 0.5) * 0.35,
        r: Math.random() * 1.6 + 0.7,
      });
    }
  }

  function tick() {
    if (!running) return;
    if (document.hidden) { requestAnimationFrame(tick); return; }
    const acc = accentColor();
    const R = Math.min(W, H) * 0.22; // радиус влияния курсора
    const LINK = 130;               // дистанция линии между точками
    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      p.x += p.vx; p.y += p.vy;
      if (p.x < 0 || p.x > W) { p.vx *= -1; p.x = Math.max(0, Math.min(W, p.x)); }
      if (p.y < 0 || p.y > H) { p.vy *= -1; p.y = Math.max(0, Math.min(H, p.y)); }
      if (mouse.active) {
        const dx = mouse.x - p.x, dy = mouse.y - p.y;
        const d = Math.hypot(dx, dy);
        if (d < R && d > 0.001) {
          const f = (1 - d / R) * 0.06; // сила ~ близости к курсору
          if (repel) { p.vx -= (dx / d) * f; p.vy -= (dy / d) * f; }
          else { p.vx += (dx / d) * f * 0.7; p.vy += (dy / d) * f * 0.7; }
          const sp = Math.hypot(p.vx, p.vy), MAX = 2.6;
          if (sp > MAX) { p.vx = p.vx / sp * MAX; p.vy = p.vy / sp * MAX; }
        }
      }
      p.vx *= 0.985; p.vy *= 0.985; // вязкость: скорости гаснут к дрейфу
    }
    ctx.lineWidth = 1;
    const links = []; // [a, b] — рёбра этого кадра (для пакетов-искр)
    for (let i = 0; i < pts.length; i++) {
      for (let j = i + 1; j < pts.length; j++) {
        const a = pts[i], b = pts[j];
        const dx = a.x - b.x, dy = a.y - b.y;
        const d2 = dx * dx + dy * dy;
        if (d2 < LINK * LINK) {
          const al = (1 - Math.sqrt(d2) / LINK) * 0.28;
          ctx.strokeStyle = "rgba(" + hexRgb(acc) + "," + al.toFixed(3) + ")";
          ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
          links.push(a, b);
        }
      }
    }
    // Network animation: «пакеты данных» — маленькие аккуратные искры
    // бегут по линиям сети, как трафик на диаграмме. Ядро 1.2px без
    // гало-круга + короткий хвост (два затухающих сегмента позади).
    // Ровная скорость у всех, рождение только из узлов (не из середины).
    while (pkgs.length < PKG_MAX && links.length && Math.random() < 0.3) {
      const k = (Math.random() * links.length / 2 | 0) * 2;
      pkgs.push({
        a: links[k], b: links[k + 1],
        t: 0,                         // стартуем из узла — не из середины пути
        sp: 0.0045 + Math.random() * 0.0025, // узкий диапазон = ровный поток
      });
    }
    for (let i = pkgs.length - 1; i >= 0; i--) {
      const g = pkgs[i];
      g.t += g.sp;
      if (g.t >= 1 || !pts.includes(g.a) || !pts.includes(g.b)) {
        pkgs.splice(i, 1); // доехал (или ребро распалось) — переродится сам
        continue;
      }
      const x = g.a.x + (g.b.x - g.a.x) * g.t;
      const y = g.a.y + (g.b.y - g.a.y) * g.t;
      // хвост: два сегмента позади с затуханием (направление a→b)
      const dx = g.b.x - g.a.x, dy = g.b.y - g.a.y;
      for (let s = 1; s <= 2; s++) {
        const tt = g.t - s * 0.035;
        if (tt < 0) break;
        ctx.fillStyle = "rgba(" + hexRgb(acc) + "," + (0.30 / s).toFixed(2) + ")";
        ctx.beginPath();
        ctx.arc(g.a.x + dx * tt, g.a.y + dy * tt, 1.1 - 0.3 * s, 0, 6.2832);
        ctx.fill();
      }
      // ядро пакета
      ctx.fillStyle = "rgba(" + hexRgb(acc) + ",0.95)";
      ctx.beginPath(); ctx.arc(x, y, 1.3, 0, 6.2832); ctx.fill();
    }
    for (const p of pts) {
      ctx.fillStyle = "rgba(" + hexRgb(acc) + ",0.55)";
      ctx.beginPath(); ctx.arc(p.x, p.y, p.r, 0, 6.2832); ctx.fill();
    }
    requestAnimationFrame(tick);
  }

  function startFx() {
    if (reduced) return;
    running = true;
    if (!inited) {
      inited = true;
      resize(); spawn();
      window.addEventListener("resize", () => { resize(); spawn(); });
      document.addEventListener("mousemove", (e) => {
        mouse.x = e.clientX; mouse.y = e.clientY; mouse.active = true;
      });
      document.addEventListener("mouseleave", () => { mouse.active = false; });
      // клик по пустому месту (не по контролам) — притяжение ⇄ отталкивание
      document.addEventListener("click", (e) => {
        if (e.target.closest("button, a, input, select, textarea, .tab-pill, .rail-item, li, details, summary, .omni-item, .widget, .recent-chip")) return;
        repel = !repel;
      });
    } else {
      resize();
    }
    requestAnimationFrame(tick);
  }

  function stopFx() {
    running = false;
    ctx.clearRect(0, 0, W, H);
  }

  // === Стеклянный интерфейс ===
  function fxParticles() {
    return localStorage.getItem("apb-fx") !== "0";
  }
  function isGlass() {
    return localStorage.getItem("apb-glass") === "1";
  }
  function applyGlass(on) {
    document.documentElement.classList.toggle("apb-glass", !!on);
    localStorage.setItem("apb-glass", on ? "1" : "0");
    syncFxToggles();
  }
  function applyParticles(on) {
    localStorage.setItem("apb-fx", on ? "1" : "0");
    syncFxToggles();
    if (on) startFx(); else stopFx();
  }
  function syncFxToggles() {
    const gp = document.getElementById("fxGlassToggle");
    const pp = document.getElementById("fxParticlesToggle");
    if (gp) gp.checked = isGlass();
    if (pp) pp.checked = fxParticles();
  }

  // === Прозрачность окна (видно рабочий стол) ===
  // mode: off | blur | clear — применяет нативную команду и сохраняет выбор.
  function winMode() {
    return localStorage.getItem("apb-winmode") || "off";
  }
  function applyWinMode(mode) {
    const m = ["off", "blur", "clear"].includes(mode) ? mode : "off";
    localStorage.setItem("apb-winmode", m);
    invoke("window_transparency", { mode: m }).catch(() => {});
    // CSS: оба прозрачных режима гасят html-фон; blur дополнительно
    // оставляет системный Acrylic за тонировкой хрома
    document.documentElement.classList.toggle("apb-window-clear", m === "clear");
    document.documentElement.classList.toggle("apb-window-blur", m === "blur");
    syncFxToggles();
  }
  function syncWinButtons() {
    const m = winMode();
    document.querySelectorAll(".fx-win").forEach((b) => {
      b.classList.toggle("active", b.dataset.winmode === m);
    });
  }

  // глобал для панели настроек
  window.__apbFx = { applyGlass, applyParticles, startFx, stopFx, syncFxToggles, isGlass, fxParticles, applyWinMode, winMode };

  // тумблеры в Настройках
  document.addEventListener("change", (e) => {
    if (e.target && e.target.id === "fxParticlesToggle") applyParticles(e.target.checked);
    if (e.target && e.target.id === "fxGlassToggle") applyGlass(e.target.checked);
  });
  document.addEventListener("click", (e) => {
    const b = e.target.closest(".fx-win");
    if (b) applyWinMode(b.dataset.winmode);
  });
  syncFxToggles();
  syncWinButtons();

  // старт по сохранённым настройкам
  if (isGlass()) document.documentElement.classList.add("apb-glass");
  if (winMode() !== "off") applyWinMode(winMode()); // повторно применить нативный эффект
  else {
    // off при старте: движок может помнить прозрачную подложку с прошлого
    // запуска — возвращаем непрозрачный фон
    invoke("window_transparency", { mode: "off" }).catch(() => {});
  }
  if (fxParticles()) startFx();

  // === Ripple: кольцо от точки клика на любой кнопке ===
  // Делегированный pointerdown: один листенер на весь документ.
  document.addEventListener("pointerdown", (e) => {
    const b = e.target.closest("button");
    if (!b || b.disabled) return;
    const r = b.getBoundingClientRect();
    const d = Math.max(r.width, r.height) * 2.2; // кольцо шире кнопки
    const s = document.createElement("span");
    s.className = "apb-ripple";
    s.style.width = s.style.height = d + "px";
    s.style.left = (e.clientX - r.left) + "px";
    s.style.top = (e.clientY - r.top) + "px";
    b.appendChild(s);
    setTimeout(() => s.remove(), 600);
  }, { passive: true });
})();
