# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | **Français**

**APB** est un navigateur de bureau axé sur la confidentialité qui réunit dans une seule application :

- un navigateur complet avec onglets natifs,
- un espace de travail (workspaces et profils),
- une base de connaissances (notes + graphe),
- un gestionnaire de mots de passe,
- un assistant IA.

Pas d'Electron. Construit avec **Tauri 2 + Rust** et des onglets natifs **WebView2**.

> Début de développement (v0.2.x). Windows uniquement pour l'instant.

---

## Pourquoi APB

La plupart des navigateurs « privés » sont soit des forks de Chromium/Firefox avec une pile d'extensions, soit de minces enveloppes. APB prend une autre voie :

- **Isolation réelle.** Chaque profil est un dossier de stockage séparé. Favoris, historique, notes, confidentialité, réseau et coffre-fort ne se mélangent pas entre profils.
- **Couche proxy propre.** Un proxy HTTP local qui bloque les traceurs, réécrit les en-têtes, construit des chaînes (HTTP/SOCKS5) et affiche des statistiques en direct.
- **Base de connaissances dans le navigateur.** Notes Markdown avec images, dessin à main levée, LaTeX et un graphe interactif de connexions.
- **Stack minimal.** Frontend en Vanilla JS — pas de npm, pas de React/Vue. Binaire relativement léger.

---

## Fonctionnalités

### Navigateur et onglets
- Onglets WebView2 natifs dans une unique fenêtre shell
- Onglets en veille, glisser-déposer, vue partagée
- Favicons, barre latérale repliable
- Workspaces — ensembles d'onglets interchangeables
- Interception des téléchargements et restauration de session

### Confidentialité et réseau
- Niveaux de confidentialité + mode d'urgence (panic button)
- Proxy local avec blocage des traceurs et statistiques en direct
- DNS / DoH, chaînes de proxy
- Exceptions par site
- Audit des réglages (DNS, proxy, extensions, IA, coffre-fort)

### Notes et graphe de connaissances
- Éditeur Markdown avec aperçu
- Images, dessin à main levée, sous-ensemble LaTeX (`$...$`)
- Dossiers et export `.md`
- Canevas infini avec disposition physique
- Cartes de notes, liens, annuler/rétablir, export PNG

### Profils et données
- Profils totalement isolés (y compris anonyme)
- Historique et favoris sur SQLite
- Suggestions intelligentes dans l'omnibox

### Coffre-fort (Vault)
- Gestionnaire de mots de passe
- AES-256-GCM + Argon2id
- Import / export CSV
- Générateur de mots de passe

### Autres
- Chat IA (fournisseurs compatibles OpenAI + Ollama local)
- Palette de commandes (Ctrl+K) — mots-clés anglais et russe
- Thèmes sombre / clair, personnalisation de l'interface
- Visite guidée
- Système d'extensions (en cours de développement)

---

## Stack technique

| Couche     | Technologie |
|------------|-------------|
| Backend    | Workspace Rust (`crates/*`), Tauri 2, wry / WebView2 |
| Frontend   | HTML / CSS / JS vanilla — sans bundler, sans npm, sans framework |
| Données    | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |
| Compilation| Le frontend est incorporé dans le binaire (`frontendDist = "../ui"`) |

### Structure du projet

```
apps/desktop/
  src-tauri/        Backend Rust (shell, pages, commandes métier cmd/*)
  ui/               Frontend du shell (index.html + modules js)
crates/             Crates métier :
                    notes, vault, privacy, network, history,
                    bookmarks, profiles, extensions, ai ...
```

---

## Compiler depuis les sources

**Prérequis :**
- [Rust](https://rustup.rs) (stable, toolchain MSVC)
- Runtime WebView2 (préinstallé sous Windows 10/11)

```powershell
cd apps/desktop/src-tauri
cargo build

# lancer la build de débogage :
../../target/debug/apb-desktop.exe
```

Build de release :

```powershell
cargo build --release
```

---

## Feuille de route

1. Appliquer le moteur de confidentialité au trafic réel (blocage des traceurs / DNS / proxy)
2. Runtime d'extensions (content scripts + sandbox)
3. Localisation de l'interface (actuellement principalement en russe)
4. Mises à jour automatiques via GitHub Releases + tauri-plugin-updater
5. Installateur NSIS complet
6. Améliorations supplémentaires de performance et de stabilité des onglets

---

## Crédits

Créé par **MrDuck** (idée, vision produit, décisions de conception, tests).  
La majeure partie du code a été écrite avec l'aide de l'IA.

## Licence

[MIT](LICENSE)
