# AP Browser (ApostolProject Browser)

[Русский](README.md) | **English** | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

**APB** is a privacy-focused desktop browser that combines in a single app:

- a full browser with native tabs,
- a workspace (workspaces & profiles),
- a knowledge base (notes + graph),
- a password manager,
- an AI assistant.

No Electron. Built with **Tauri 2 + Rust** and native **WebView2** tabs.

> Early development (v0.2.x). Windows only for now.

---

## Why APB

Most “private” browsers are either Chromium/Firefox forks with a pile of extensions, or thin wrappers. APB takes a different approach:

- **Real isolation.** Every profile is a separate storage root. Bookmarks, history, notes, privacy settings, network config and vault never leak between profiles.
- **Own proxy layer.** A local HTTP proxy that can block trackers, rewrite headers, build proxy chains (HTTP/SOCKS5) and show live blocking stats.
- **Knowledge base inside the browser.** Markdown notes with images, freehand drawing, LaTeX, plus an interactive graph of connections.
- **Minimal stack.** Vanilla JS frontend — no npm, no React/Vue. Relatively lightweight binary.

---

## Features

### Browser & tabs
- Native WebView2 tabs inside a single shell window
- Sleeping tabs, drag & drop, split view
- Favicons, collapsible sidebar
- Workspaces — switchable sets of tabs
- Download interception and session restore

### Privacy & network
- Privacy levels + emergency / panic mode
- Local proxy with tracker blocking and live statistics
- DNS / DoH, proxy chains
- Per-site rule overrides
- Settings audit (DNS, proxy, extensions, AI, vault)

### Notes & knowledge graph
- Markdown editor with live preview
- Images, freehand drawing, LaTeX subset (`$...$`)
- Folders and `.md` export
- Infinite canvas with physics layout
- Note cards, links, undo/redo, PNG export

### Profiles & data
- Fully isolated profiles (including anonymous)
- SQLite-backed history and bookmarks
- Smart omnibox suggestions

### Vault
- Password manager
- AES-256-GCM + Argon2id
- CSV import / export
- Password generator

### Other
- AI chat (OpenAI-compatible providers + local Ollama)
- Command palette (Ctrl+K) — English & Russian keywords
- Dark / light themes, UI customization
- Onboarding tour
- Extension system (work in progress)

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

1. Fully apply the privacy engine to real traffic (tracker blocking / DNS / proxy)
2. Extension runtime (content scripts + sandbox)
3. UI localization (currently primarily Russian)
4. Auto-updates via GitHub Releases + tauri-plugin-updater
5. Proper NSIS installer
6. Further tab performance and stability work

---

## Credits

Created by **MrDuck** (idea, product vision, design decisions, testing).  
Most of the codebase was written with the help of AI.

## License

[MIT](LICENSE)
