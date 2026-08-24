# AP Browser (ApostolProject Browser)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | **Deutsch** | [Français](README.fr.md)

Ein datenschutzorientierter Desktop-Browser, der **Browser + Arbeitsbereich + Wissensbasis + KI** in einer einzigen App vereint. Kein Electron — gebaut mit **Tauri 2 / Rust** und nativen WebView2-Tabs.

> Frühe Entwicklungsphase (v0.1.0), derzeit nur Windows.

## Funktionen

- **Tabs** — native WebView2-Tabs in einem einzigen Shell-Fenster: schlafende Tabs, Drag & Drop, geteilte Ansicht, Favicons, einklappbare Seitenleiste
- **Notizen** — Markdown-Editor mit Vorschau, Bildern, Freihandzeichnen, LaTeX-Teilmenge (`$...$`), Ordnern, `.md`-Export
- **Wissensgraf** — unendliche Leinwand mit Physik-Layout, Notizkarten, Verbindungen, Undo/Redo, PNG-Export
- **Workspaces & Profile** — isolierte Profile nach Speicherordner, Tab-Sätze pro Workspace
- **Tresor** — Passwort-Manager mit AES-256-GCM-Verschlüsselung, Argon2id-Schlüsselableitung, CSV-Import/Export
- **Verlauf & Lesezeichen** — SQLite-basiert, Verlauf pro Profil, Omnibox-Vorschläge
- **Befehlspalette** — Ctrl+K, Schlüsselwortsuche auf Englisch und Russisch
- **KI-Chat** — OpenAI-kompatible Anbieter und lokales Ollama
- **Download-Abfangen**, dunkle/helle Themes, UI-Anpassung, Onboarding-Tour

## Tech-Stack

| Schicht | Technologie |
|---|---|
| Backend | Rust-Workspace (`crates/*`), Tauri v2, wry/WebView2 |
| Frontend | Vanilla HTML/CSS/JS — kein Bundler, kein npm, keine Frameworks |
| Daten | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |

Das Frontend wird zur Build-Zeit in die Binärdatei eingebettet (`frontendDist = "../ui"`).

## Aus dem Quellcode bauen

Voraussetzungen: [Rust](https://rustup.rs) (stable, MSVC-Toolchain), WebView2-Runtime (unter Windows 10/11 vorinstalliert).

```powershell
cd apps/desktop/src-tauri
cargo build
# Debug-Build ausführen:
../../target/debug/apb-desktop.exe
```

## Projektstruktur

```
apps/desktop/
  src-tauri/        Rust-Backend (shell, pages, Domänenbefehle cmd/*)
  ui/               Shell-Frontend (index.html + js-Module, der Reihe nach geladen)
crates/             Domänen-Crates: notes, Graf, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai ...
```

## Roadmap

1. Datenschutz-Engine auf den echten Traffic anwenden (Tracker-Blockierung / DNS / Proxy-Schicht)
2. Extension-Runtime (Content Scripts)
3. Lokalisierung über das Russische hinaus
4. Automatische Updates (tauri-plugin-updater + GitHub Releases)
5. NSIS-Installer

## Mitwirkende

Erstellt von **MrDuck** (Idee, Produktvision, Design-Entscheidungen, Tests) und **Ox-Alpha** (KI-Softwareingenieur — hat den Großteil der Codebasis geschrieben).

## Lizenz

[MIT](LICENSE)
