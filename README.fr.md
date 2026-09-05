# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | **Français**

Un navigateur de bureau axé sur la confidentialité qui réunit **navigateur + espace de travail + base de connaissances + IA** dans une seule application. Pas d'Electron — construit avec **Tauri 2 / Rust** et des onglets WebView2 natifs. Windows 10/11.

## Fonctionnalités

- **Onglets** — onglets WebView2 natifs dans une unique fenêtre shell : onglets en veille, glisser-déposer, épinglage 📌, dossiers d'onglets (groupes), vue partagée, favicons et titres réels, barre latérale repliable, menus contextuels
- **Confidentialité sur le trafic réel** — proxy de filtrage intégré qui bloque traqueurs/pubs/malwares par domaine ; upgrade HTTPS-only, contrôle des cookies et du Referer, isolation WebRTC, DoH/DNS, chaînes de proxy externes (jusqu'à 3 sauts), exceptions par site, statistiques de blocage en direct
- **Notes** — éditeur Markdown avec aperçu, images (y compris SVG), dessin à main levée, sous-ensemble LaTeX (`$...$`), dossiers, export `.md`
- **Graphe de connaissances** — canevas infini sur un vrai `<canvas>` avec disposition physique, cartes de notes, liens, annuler/rétablir, export PNG avec le contenu des blocs
- **Espaces de travail et profils** — profils isolés par dossier de stockage, ensembles d'onglets par espace de travail, mode anonyme
- **Coffre-fort** — gestionnaire de mots de passe : chiffrement AES-256-GCM, dérivation de clé Argon2id, import/export CSV
- **Favoris et historique** — adossés à SQLite, historique par profil, dossiers de favoris, `javascript:` bookmarklets, suggestions dans l'omnibox
- **Palette de commandes** — Ctrl+K, recherche par mots-clés en anglais et en russe
- **Chat IA** — fournisseurs compatibles OpenAI et Ollama local
- **Téléchargements** — moteur dédié avec annulation réelle, reprise et barre de progression avec vitesse et graphique
- **Localisation** — interface en russe et en anglais
- Thèmes sombre/clair, thème sombre intelligent pour les sites, personnalisation de l'interface (`.apbtheme`), transparence/vitrage de la fenêtre, visite guidée

## Feuille de route

1. Runtime d'extensions complet (content scripts par masques d'URL)
2. Localisation au-delà de RU/EN
3. Installateur personnalisé (contrôle total de l'installation en Rust)

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
crates/             Crates métier : notes, canvas, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai...
```

## Crédits

Créé par **MrDuck** (idée, vision produit, décisions de conception, tests) et **Ox-Alpha** (ingénieur logiciel IA — a écrit la majeure partie du code).

## Licence

[MIT](LICENSE)
