# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | **Deutsch** | [Français](README.fr.md)

**APB** ist ein datenschutzorientierter Desktop-Browser, der in einer einzigen App vereint:

- einen vollwertigen Browser mit nativen Tabs,
- einen Arbeitsbereich (Workspaces & Profile),
- eine Wissensbasis (Notizen + Graph),
- einen Passwort-Manager,
- einen KI-Assistenten.

Kein Electron. Gebaut mit **Tauri 2 + Rust** und nativen **WebView2**-Tabs.

> Frühe Entwicklungsphase (v0.2.x). Derzeit nur Windows.

---

## Warum APB

Die meisten „privaten“ Browser sind entweder Chromium/Firefox-Forks mit einem Haufen Erweiterungen oder dünne Hüllen. APB geht einen anderen Weg:

- **Echte Isolation.** Jedes Profil ist ein separater Speicherordner. Lesezeichen, Verlauf, Notizen, Datenschutz, Netzwerk und Tresor vermischen sich nicht zwischen Profilen.
- **Eigene Proxy-Schicht.** Ein lokaler HTTP-Proxy, der Tracker blockiert, Header umschreibt, Ketten aufbaut (HTTP/SOCKS5) und Live-Statistiken zeigt.
- **Wissensbasis im Browser.** Markdown-Notizen mit Bildern, Freihandzeichnen, LaTeX und einem interaktiven Verbindungsgraphen.
- **Minimaler Stack.** Vanilla-JS-Frontend — kein npm, kein React/Vue. Relativ leichtes Binary.

---

## Funktionen

### Browser & Tabs
- Native WebView2-Tabs in einem einzigen Shell-Fenster
- Schlafende Tabs, Drag & Drop, geteilte Ansicht
- Favicons, einklappbare Seitenleiste
- Workspaces — umschaltbare Tab-Sätze
- Download-Abfangen und Sitzungswiederherstellung

### Datenschutz & Netzwerk
- Datenschutzstufen + Notfallmodus (Panic-Button)
- Lokaler Proxy mit Tracker-Blockierung und Live-Statistiken
- DNS / DoH, Proxy-Ketten
- Ausnahmen pro Website
- Einstellungs-Audit (DNS, Proxy, Erweiterungen, KI, Tresor)

### Notizen & Wissensgraph
- Markdown-Editor mit Vorschau
- Bilder, Freihandzeichnen, LaTeX-Teilmenge (`$...$`)
- Ordner und `.md`-Export
- Unendliche Leinwand mit Physik-Layout
- Notizkarten, Verbindungen, Undo/Redo, PNG-Export

### Profile & Daten
- Vollständig isolierte Profile (inkl. anonym)
- Verlauf und Lesezeichen auf SQLite
- Intelligente Omnibox-Vorschläge

### Tresor (Vault)
- Passwort-Manager
- AES-256-GCM + Argon2id
- CSV-Import / -Export
- Passwort-Generator

### Sonstiges
- KI-Chat (OpenAI-kompatible Anbieter + lokales Ollama)
- Befehlspalette (Ctrl+K) — englische und russische Schlüsselwörter
- Dunkle / helle Themes, UI-Anpassung
- Einführungs-Tour
- Erweiterungssystem (in Entwicklung)

---

## Tech-Stack

| Schicht    | Technologie |
|------------|-------------|
| Backend    | Rust-Workspace (`crates/*`), Tauri 2, wry / WebView2 |
| Frontend   | Vanilla HTML / CSS / JS — kein Bundler, kein npm, keine Frameworks |
| Daten      | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |
| Build      | Frontend wird in die Binärdatei eingebettet (`frontendDist = "../ui"`) |

### Projektstruktur

```
apps/desktop/
  src-tauri/        Rust-Backend (shell, pages, Domänenbefehle cmd/*)
  ui/               Shell-Frontend (index.html + js-Module)
crates/             Domänen-Crates:
                    notes, vault, privacy, network, history,
                    bookmarks, profiles, extensions, ai ...
```

---

## Aus dem Quellcode bauen

**Voraussetzungen:**
- [Rust](https://rustup.rs) (stable, MSVC-Toolchain)
- WebView2-Runtime (unter Windows 10/11 vorinstalliert)

```powershell
cd apps/desktop/src-tauri
cargo build

# Debug-Build ausführen:
../../target/debug/apb-desktop.exe
```

Release-Build:

```powershell
cargo build --release
```

---

## Roadmap

1. Datenschutz-Engine vollständig auf echten Traffic anwenden (Tracker-Blockierung / DNS / Proxy)
2. Extension-Runtime (Content Scripts + Sandbox)
3. UI-Lokalisierung (derzeit hauptsächlich Russisch)
4. Automatische Updates über GitHub Releases + tauri-plugin-updater
5. Vollständiger NSIS-Installer
6. Weitere Verbesserungen bei Tab-Performance und Stabilität

---

## Mitwirkende

Erstellt von **MrDuck** (Idee, Produktvision, Design-Entscheidungen, Tests).  
Der Großteil des Codes wurde mit Hilfe von KI geschrieben.

## Lizenz

[MIT](LICENSE)
