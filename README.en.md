# AP Browser (ApostolProject Browser)

[Русский](README.md) | **English** | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

**APB** is a privacy-focused desktop browser that combines in a single app:

- a full browser with native tabs,
- a workspace (workspaces & profiles),
- a knowledge base (notes + graph),
- a password manager,
- an AI assistant.

No Electron. Built with **Tauri 2 + Rust** and native **WebView2** tabs.

> Stable development build **v0.2.7**. Windows 10/11 only for now.

---

## Why APB

Most “private” browsers are either Chromium/Firefox forks with a pile of extensions, or thin wrappers. APB takes a different approach:

- **Real isolation.** Every profile is a separate storage root. Bookmarks, history, notes, privacy settings, network config and vault never leak between profiles.
- **Own proxy layer.** A local HTTP proxy that can block trackers/ads/malware by domain, rewrite headers, build proxy chains (HTTP/SOCKS5) and show live blocking stats.
- **Knowledge base inside the browser.** Markdown notes with images, freehand drawing, LaTeX, plus an interactive graph of connections.
- **Minimal stack.** Vanilla JS frontend — no npm, no React/Vue. Relatively lightweight binary.

---

## Features

### Browser & tabs
- Native WebView2 tabs inside a single shell window
- Sleeping tabs, drag & drop, pinning, tab folders (groups), split view
- Real favicons and page titles, collapsible sidebar, tab context menus
- Workspaces — switchable sets of tabs
- Download interception (dedicated engine: cancel, retry, speed progress) and session restore

### Privacy & network
- Privacy levels + emergency / panic mode
- Local filtering proxy: domain-based tracker/ad/malware blocking with live statistics
- DNS / DoH, custom DNS servers, HTTPS-only, cookie & Referer control, WebRTC isolation, proxy chains
- Per-site rule overrides
- Settings audit (DNS, proxy, extensions, AI, vault)

### Notes & knowledge graph
- Markdown editor with live preview
- Images (including SVG), freehand drawing, LaTeX subset (`$...$`)
- Folders and `.md` export
- Infinite canvas with physics layout
- Note cards, links, undo/redo, PNG export

### Profiles & data
- Fully isolated profiles (including anonymous)
- SQLite-backed history and bookmarks (bookmark folders, `javascript:` bookmarks)
- Omnibox suggestions: history + bookmarks + domain completions + search — all local, no external suggest servers

### Vault
- Password manager
- AES-256-GCM + Argon2id
- CSV import / export
- Password generator

### Other
- AI chat (OpenAI-compatible providers + local Ollama)
- Command palette (Ctrl+K) — English & Russian keywords
- Dark / light themes, smart site dark theme, UI customization (`.apbtheme`), window transparency/glass
- Localization: Russian and English interfaces
- Onboarding tour
- Extension system: runtime v1 (content scripts by URL masks), management UI hidden for now

---

## Tech stack

| Layer    | Tech |
|----------|------|
| Backend  | Rust workspace (`crates/*`), Tauri 2, wry / WebView2 |
| Frontend | Vanilla HTML / CSS / JS — no bundler, no npm, no frameworks |
| Data     | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |
| Build    | Frontend is embedded into the binary (`frontendDist = "../ui"`) |

### Project layout

```
apps/desktop/
  src-tauri/        Rust backend (shell, pages, cmd/* domain commands)
  ui/               Shell frontend (index.html + js modules)
crates/             Domain crates:
                    notes, vault, privacy, network, history,
                    bookmarks, profiles, extensions, ai ...
```

---

## Build from source

**Requirements:**
- [Rust](https://rustup.rs) (stable, MSVC toolchain)
- WebView2 Runtime (preinstalled on Windows 10/11)

```powershell
cd apps/desktop/src-tauri
cargo build

# run the debug build:
../../target/debug/apb-desktop.exe
```

Release build:

```powershell
cargo build --release
```

---

## Roadmap

1. Full extension runtime v2 — content scripts by URL masks already run (v1), next: extension management UI and a wider API
2. Localization beyond RU/EN
3. Custom NSIS installer (currently the stock tauri one)
4. Downloads behind auth: pass cookies from the profile into the download engine
5. DoT (DNS over TLS) — DoH and custom DNS servers are supported today
6. Further tab performance and stability work

---

## Known limitations / current bugs

- **Authenticated downloads.** The dedicated download engine does not pass WebView2 cookies, so files behind a protected link can fail (public downloads work fine).
- **Cookie policies.** They apply at tab creation time — after changing a policy you must open the tab anew; `Set-Cookie` in HTTPS responses is not rewritten by the proxy.
- **DNS.** DoH and custom DNS servers work; DoT is not supported. Traffic to LAN/IP addresses always goes direct.
- **Extensions.** Runtime v1 executes content scripts by masks, but the management UI is hidden and the API is limited.
- **Localization.** Russian and English only.
- **LaTeX in notes.** A subset is supported (`$...$`, `$$...$$`).

---

## Credits

Created by **MrDuck** (idea, product vision, design decisions, testing).  
Most of the codebase was written with the help of AI.

## License

[MIT](LICENSE)