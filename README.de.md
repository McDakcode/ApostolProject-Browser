# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | **Deutsch** | [Français](README.fr.md)

**APB** ist ein datenschutzorientierter Desktop-Browser, der in einer einzigen App vereint:

- einen vollwertigen Browser mit nativen Tabs,
- einen Arbeitsbereich (Workspaces & Profile),
- eine Wissensbasis (Notizen + Graph),
- einen Passwort-Manager,
- einen KI-Assistenten.

Kein Electron. Gebaut mit **Tauri 2 + Rust** und nativen **WebView2**-Tabs.

> Stabile Entwicklungsbuild **v0.2.7**. Derzeit nur Windows 10/11.

---

## Warum APB

Die meisten „privaten“ Browser sind entweder Chromium/Firefox-Forks mit einem Haufen Erweiterungen oder dünne Hüllen. APB geht einen anderen Weg:

- **Echte Isolation.** Jedes Profil ist ein separater Speicherordner. Lesezeichen, Verlauf, Notizen, Datenschutz, Netzwerk und Tresor vermischen sich nicht zwischen Profilen.
- **Eigene Proxy-Schicht.** Ein lokaler HTTP-Proxy, der Tracker/Werbung/Malware nach Domäne blockt, Header umschreibt, Ketten aufbaut (HTTP/SOCKS5) und Live-Statistiken zeigt.
- **Wissensbasis im Browser.** Markdown-Notizen mit Bildern, Freihandzeichnen, LaTeX und einem interaktiven Verbindungsgraphen.
- **Minimaler Stack.** Vanilla-JS-Frontend — kein npm, kein React/Vue. Relativ leichtes Binary.

---

## Funktionen

### Browser & Tabs
- Native WebView2-Tabs in einem einzigen Shell-Fenster
- Schlafende Tabs, Drag & Drop, Anheften, Tab-Ordner (Gruppen), geteilte Ansicht
- Echte Favicons und Seitentitel, einklappbare Seitenleiste, Tab-Kontextmenüs
- Workspaces — umschaltbare Tab-Sätze
- Download-Abfangen (eigener Motor: abbrechen, wiederholen, Fortschritt mit Geschwindigkeit) und Sitzungswiederherstellung

### Datenschutz & Netzwerk
- Datenschutzstufen + Notfallmodus (Panic-Button)
- Lokaler Filter-Proxy: domänenbasierte Blockierung von Trackern/Werbung/Malware und Live-Statistiken
- DNS / DoH, benutzerdefinierte DNS-Server, HTTPS-only, Cookie- und Referer-Kontrolle, WebRTC-Isolierung, Proxy-Ketten
- Ausnahmen pro Website
- Einstellungs-Audit (DNS, Proxy, Erweiterungen, KI, Tresor)

### Notizen & Wissensgraph
- Markdown-Editor mit Vorschau
- Bilder (inkl. SVG), Freihandzeichnen, LaTeX-Teilmenge (`$...$`)
- Ordner und `.md`-Export
- Unendliche Leinwand mit Physik-Layout
- Notizkarten, Verbindungen, Undo/Redo, PNG-Export

### Profile & Daten
- Vollständig isolierte Profile (inkl. anonym)
- Verlauf und Lesezeichen auf SQLite (Lesezeichen-Ordner, `javascript:`-Bookmarklets)
- Omnibox-Vorschläge: Verlauf + Lesezeichen + Domänen-Fortsetzungen + Suche — alles lokal, ohne externe Suggest-Server

### Tresor (Vault)
- Passwort-Manager
- AES-256-GCM + Argon2id
- CSV-Import / -Export
- Passwort-Generator

### Sonstiges
- KI-Chat (OpenAI-kompatible Anbieter + lokales Ollama)
- Befehlspalette (Ctrl+K) — englische und russische Schlüsselwörter
- Dunkle / helle Themes, intelligentes dunkles Theme für Websites, UI-Anpassung (`.apbtheme`), Fenster-Transparenz/Glas
- Lokalisierung: russische und englische Oberfläche
- Einführungs-Tour
- Erweiterungssystem: Runtime v1 (Content Scripts nach URL-Masken), Verwaltungs-UI vorerst ausgeblendet

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

1. Vollständige Extension-Runtime v2 — Content Scripts nach URL-Masken laufen bereits (v1); danach: Verwaltungs-UI und eine breitere API
2. Lokalisierung über RU/EN hinaus
3. Benutzerdefinierter NSIS-Installer (aktuell der Standard von tauri)
4. Downloads hinter Authentifizierung: Cookies aus dem Profil an den Download-Motor durchreichen
5. DoT (DNS over TLS) — heute werden DoH und benutzerdefinierte DNS-Server unterstützt
6. Weitere Verbesserungen bei Tab-Performance und Stabilität

---

## Bekannte Einschränkungen / aktuelle Fehler

- **Authentifizierte Downloads.** Der eigene Download-Motor reicht keine WebView2-Cookies durch, daher können Dateien hinter einem geschützten Link fehlschlagen (öffentliche Downloads funktionieren).
- **Cookie-Richtlinien.** Sie greifen beim Erstellen der Tabs — nach einer Richtlinienänderung muss der Tab neu geöffnet werden; `Set-Cookie` in HTTPS-Antworten wird vom Proxy nicht umgeschrieben.
- **DNS.** DoH und benutzerdefinierte DNS-Server funktionieren; DoT wird nicht unterstützt. Traffic zu LAN/IP-Adressen geht immer direkt.
- **Erweiterungen.** Die Runtime v1 führt Content Scripts nach Masken aus, aber die Verwaltungs-UI ist ausgeblendet und die API begrenzt.
- **Lokalisierung.** Nur Russisch und Englisch.
- **LaTeX in Notizen.** Es wird eine Teilmenge unterstützt (`$...$`, `$$...$$`).

---

## Mitwirkende

Erstellt von **MrDuck** (Idee, Produktvision, Design-Entscheidungen, Tests).  
Der Großteil des Codes wurde mit Hilfe von KI geschrieben.

## Lizenz

[MIT](LICENSE)