# AP Browser (ApostolProject Browser)

**English** | [Русский](README.ru.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

A privacy-focused desktop browser that combines **browser + workspace + knowledge base + AI** in one app. Not Electron — built on **Tauri 2 / Rust** with native WebView2 tabs.

> Early development (v0.1.0), Windows only for now.

## Features

- **Tabs** — native WebView2 tabs in a single shell window: sleeping tabs, drag & drop, split view, favicons, collapsible sidebar
- **Notes** — Markdown editor with preview, images, freehand drawing, LaTeX subset (`$...$`), folders, `.md` export
- **Knowledge graph** — infinite canvas with physics layout, note cards, links, undo/redo, PNG export
- **Workspaces & profiles** — isolated profiles by storage folder, per-workspace tab sets
- **Vault** — password manager with AES-256-GCM encryption and Argon2id key derivation, CSV import/export
- **History & bookmarks** — SQLite-backed, per-profile history, omnibox suggestions
- **Command palette** — Ctrl+K, English/Russian keyword search
- **AI chat** — OpenAI-compatible providers and local Ollama
- **Downloads interception**, dark/light themes, UI customization, onboarding tour

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
crates/             Domain crates: notes, graph data, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai...
```

## Roadmap

1. Apply privacy engine to real traffic (tracker blocking / DNS / proxy layer)
2. Extension runtime (content scripts)
3. Localization beyond Russian
4. Auto-updates (tauri-plugin-updater + GitHub Releases)
5. NSIS installer

## License

[MIT](LICENSE)
