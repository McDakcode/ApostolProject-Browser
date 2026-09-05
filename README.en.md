# AP Browser (ApostolProject Browser)

[Русский](README.md) | **English** | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

A privacy-focused desktop browser that combines **browser + workspace + knowledge base + AI** in one app. Not Electron — built on **Tauri 2 / Rust** with native WebView2 tabs. Windows 10/11.

## Features

- **Tabs** — native WebView2 tabs in a single shell window: sleeping tabs, drag & drop, pinning 📌, tab folders (groups), split view, real favicons and page titles, collapsible sidebar, context menus
- **Real-traffic privacy** — built-in filtering proxy blocks trackers/ads/malware by domain; HTTPS-only upgrade, cookie and Referer control, WebRTC isolation, DoH/DNS, external proxy chains (up to 3 hops), per-site exceptions, live blocking stats
- **Notes** — Markdown editor with preview, images (including SVG), freehand drawing, LaTeX subset (`$...$`), folders, `.md` export
- **Knowledge graph** — infinite canvas on a real `<canvas>` with physics layout, note cards, links, undo/redo, PNG export including block contents
- **Workspaces & profiles** — isolated profiles by storage folder, per-workspace tab sets, anonymous mode
- **Vault** — password manager with AES-256-GCM encryption and Argon2id key derivation, CSV import/export
- **Bookmarks & history** — SQLite-backed, per-profile history, bookmark folders, `javascript:` bookmarkets, omnibox suggestions
- **Command palette** — Ctrl+K, English/Russian keyword search
- **AI chat** — OpenAI-compatible providers and local Ollama
- **Downloads** — dedicated engine with real cancellation, retry, and a progress bar with speed and a graph
- **Localization** — Russian and English interfaces
- Dark/light themes, smart site dark theme, UI customization (`.apbtheme`), window transparency/glass, onboarding tour

## Roadmap

1. Full extension runtime (content scripts by URL masks)
2. Localization beyond RU/EN
3. Custom installer (full install control in Rust)

## Tech stack

| Layer | Tech |
|---|---|
| Backend | Rust workspace (`crates/*`), Tauri v2, wry/WebView2 |
| Frontend | Vanilla HTML/CSS/JS — no bundler, no npm, no frameworks |
| Data | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |

The frontend is embedded into the binary at build time (`frontendDist = "../ui"`).

## Build from source

Prerequisites: [Rust](https://rustup.rs) (stable, MSVC toolchain), WebView2 Runtime (preinstalled on Windows 10/11).

```powershell
cd apps/desktop/src-tauri
cargo build
# run the debug build:
../../target/debug/apb-desktop.exe
```

## Project layout

```
apps/desktop/
  src-tauri/        Rust backend (shell, pages, cmd/* domain commands)
  ui/               Shell frontend (index.html + js modules, loaded in order)
crates/             Domain crates: notes, canvas, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai...
```

## Credits

Built by **MrDuck** (idea, product vision, design decisions, testing) and **Ox-Alpha** (AI software engineer — wrote most of the codebase).

## License

[MIT](LICENSE)
