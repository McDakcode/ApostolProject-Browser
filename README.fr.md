# AP Browser (ApostolProject Browser)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | **Français**

Un navigateur de bureau axé sur la confidentialité qui réunit **navigateur + espace de travail + base de connaissances + IA** dans une seule application. Pas d'Electron — construit avec **Tauri 2 / Rust** et des onglets WebView2 natifs.

> Début de développement (v0.1.0), Windows uniquement pour l'instant.

## Fonctionnalités

- **Onglets** — onglets WebView2 natifs dans une unique fenêtre shell : onglets en veille, glisser-déposer, vue partagée, favicons, barre latérale repliable
- **Notes** — éditeur Markdown avec aperçu, images, dessin à main levée, sous-ensemble LaTeX (`$...$`), dossiers, export `.md`
- **Graphe de connaissances** — canevas infini avec disposition physique, cartes de notes, liens, annuler/rétablir, export PNG
- **Espaces de travail et profils** — profils isolés par dossier de stockage, ensembles d'onglets par espace de travail
- **Coffre-fort** — gestionnaire de mots de passe : chiffrement AES-256-GCM, dérivation de clé Argon2id, import/export CSV
- **Historique et favoris** — adossés à SQLite, historique par profil, suggestions dans l'omnibox
- **Palette de commandes** — Ctrl+K, recherche par mots-clés en anglais et en russe
- **Chat IA** — fournisseurs compatibles OpenAI et Ollama local
- **Interception des téléchargements**, thèmes sombre/clair, personnalisation de l'interface, visite guidée

## Stack technique

| Couche | Technologie |
|---|---|
| Backend | Workspace Rust (`crates/*`), Tauri v2, wry/WebView2 |
| Frontend | HTML/CSS/JS vanilla — sans bundler, sans npm, sans framework |
| Données | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |

Le frontend est incorporé dans le binaire au moment de la compilation (`frontendDist = "../ui"`).

## Compiler depuis les sources

Prérequis : [Rust](https://rustup.rs) (stable, toolchain MSVC), runtime WebView2 (préinstallé sous Windows 10/11).

```powershell
cd apps/desktop/src-tauri
cargo build
# lancer la build de débogage :
../../target/debug/apb-desktop.exe
```

## Structure du projet

```
apps/desktop/
  src-tauri/        Backend Rust (shell, pages, commandes métier cmd/*)
  ui/               Frontend du shell (index.html + modules js, chargés dans l'ordre)
crates/             Crates métier : notes, graphe, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai...
```

## Feuille de route

1. Appliquer le moteur de confidentialité au trafic réel (blocage des traqueurs / DNS / couche proxy)
2. Runtime d'extensions (content scripts)
3. Localisation au-delà du russe
4. Mises à jour automatiques (tauri-plugin-updater + GitHub Releases)
5. Installateur NSIS

## Licence

[MIT](LICENSE)
