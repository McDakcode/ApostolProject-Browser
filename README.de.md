# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | **Deutsch** | [Français](README.fr.md)

Ein datenschutzorientierter Desktop-Browser, der **Browser + Arbeitsbereich + Wissensbasis + KI** in einer einzigen App vereint. Kein Electron — gebaut mit **Tauri 2 / Rust** und nativen WebView2-Tabs. Windows 10/11.

## Funktionen

- **Tabs** — native WebView2-Tabs in einem einzigen Shell-Fenster: schlafende Tabs, Drag & Drop, Anheften 📌, Tab-Ordner (Gruppen), geteilte Ansicht, echte Favicons und Seitentitel, einklappbare Seitenleiste, Kontextmenüs
- **Datenschutz im echten Traffic** — eingebauter Filter-Proxy blockt Tracker/Werbung/Malware nach Domäne; HTTPS-only-Upgrade, Cookie- und Referer-Kontrolle, WebRTC-Isolierung, DoH/DNS, externe Proxy-Ketten (bis zu 3 Hops), Ausnahmen pro Website, Live-Blockierstatistik
- **Notizen** — Markdown-Editor mit Vorschau, Bildern (inkl. SVG), Freihandzeichnen, LaTeX-Teilmenge (`$...$`), Ordnern, `.md`-Export
- **Wissensgraf** — unendliche Leinwand auf echtem `<canvas>` mit Physik-Layout, Notizkarten, Verbindungen, Undo/Redo, PNG-Export samt Blockinhalten
- **Workspaces & Profile** — isolierte Profile nach Speicherordner, Tab-Sätze pro Workspace, anonymer Modus
- **Tresor** — Passwort-Manager mit AES-256-GCM-Verschlüsselung, Argon2id-Schlüsselableitung, CSV-Import/Export
- **Lesezeichen & Verlauf** — SQLite-basiert, Verlauf pro Profil, Lesezeichen-Ordner, `javascript:`-Bookmarklets, Omnibox-Vorschläge
- **Befehlspalette** — Ctrl+K, Stichwortsuche auf Englisch und Russisch
- **KI-Chat** — OpenAI-kompatible Anbieter und lokales Ollama
- **Downloads** — eigener Motor mit echter Abbrechbarkeit, Wiederholung und Fortschrittsbalken mit Geschwindigkeit und Diagramm
- **Lokalisierung** — russische und englische Oberfläche
- Dunkle/helle Themes, intelligentes dunkles Theme für Websites, UI-Anpassung (`.apbtheme`), Fenster-Transparenz/Glas, Onboarding-Tour

## Roadmap

1. Vollständige Extension-Runtime (Content Scripts nach URL-Masken)
2. Lokalisierung über RU/EN hinaus
3. Benutzerdefinierter Installer (volle Installationskontrolle in Rust)

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
crates/             Domänen-Crates: notes, canvas, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai ...
```

## Mitwirkende

Erstellt von **MrDuck** (Idee, Produktvision, Design-Entscheidungen, Tests) und **Ox-Alpha** (KI-Softwareingenieur — hat den Großteil der Codebasis geschrieben).

## Lizenz

[MIT](LICENSE)
