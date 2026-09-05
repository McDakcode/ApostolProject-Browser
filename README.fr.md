# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | [Español](README.es.md) | [Deutsch](README.de.md) | **Français**

**APB** est un navigateur de bureau axé sur la confidentialité qui réunit dans une seule application :

- un navigateur complet avec onglets natifs,
- un espace de travail (workspaces et profils),
- une base de connaissances (notes + graphe),
- un gestionnaire de mots de passe,
- un assistant IA.

Pas d'Electron. Construit avec **Tauri 2 + Rust** et des onglets natifs **WebView2**.

> Build de développement stable **v0.2.7**. Windows 10/11 uniquement pour l'instant.

---

## Pourquoi APB

La plupart des navigateurs « privés » sont soit des forks de Chromium/Firefox avec une pile d'extensions, soit de minces enveloppes. APB prend une autre voie :

- **Isolation réelle.** Chaque profil est un dossier de stockage séparé. Favoris, historique, notes, confidentialité, réseau et coffre-fort ne se mélangent pas entre profils.
- **Couche proxy propre.** Un proxy HTTP local qui bloque les traceurs/pubs/malwares par domaine, réécrit les en-têtes, construit des chaînes (HTTP/SOCKS5) et affiche des statistiques en direct.
- **Base de connaissances dans le navigateur.** Notes Markdown avec images, dessin à main levée, LaTeX et un graphe interactif de connexions.
- **Stack minimal.** Frontend en Vanilla JS — pas de npm, pas de React/Vue. Binaire relativement léger.

---

## Fonctionnalités

### Navigateur et onglets
- Onglets WebView2 natifs dans une unique fenêtre shell
- Onglets en veille, glisser-déposer, épinglage, dossiers d'onglets (groupes), vue partagée
- Favicons et titres réels, barre latérale repliable, menus contextuels d'onglets
- Workspaces — ensembles d'onglets interchangeables
- Interception des téléchargements (moteur dédié : annulation, reprise, progression avec vitesse) et restauration de session

### Confidentialité et réseau
- Niveaux de confidentialité + mode d'urgence (panic button)
- Proxy de filtrage local : blocage des traceurs/pubs/malwares par domaine et statistiques en direct
- DNS / DoH, serveurs DNS personnalisés, HTTPS-only, contrôle des cookies et du Referer, isolation WebRTC, chaînes de proxy
- Exceptions par site
- Audit des réglages (DNS, proxy, extensions, IA, coffre-fort)

### Notes et graphe de connaissances
- Éditeur Markdown avec aperçu
- Images (y compris SVG), dessin à main levée, sous-ensemble LaTeX (`$...$`)
- Dossiers et export `.md`
- Canevas infini avec disposition physique
- Cartes de notes, liens, annuler/rétablir, export PNG

### Profils et données
- Profils totalement isolés (y compris anonyme)
- Historique et favoris sur SQLite (dossiers de favoris, `javascript:` bookmarklets)
- Suggestions dans l'omnibox : historique + favoris + complétions de domaines + recherche — tout en local, sans serveurs de suggestions externes

### Coffre-fort (Vault)
- Gestionnaire de mots de passe
- AES-256-GCM + Argon2id
- Import / export CSV
- Générateur de mots de passe

### Autres
- Chat IA (fournisseurs compatibles OpenAI + Ollama local)
- Palette de commandes (Ctrl+K) — mots-clés anglais et russe
- Thèmes sombre / clair, thème sombre intelligent pour les sites, personnalisation de l'interface (`.apbtheme`), transparence/vitrage de la fenêtre
- Localisation : interface en russe et en anglais
- Visite guidée
- Système d'extensions : runtime v1 (content scripts par masques d'URL), interface de gestion masquée pour l'instant

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

1. Runtime d'extensions complet v2 — les content scripts par masques d'URL s'exécutent déjà (v1) ; ensuite : interface de gestion et API plus large
2. Localisation au-delà de RU/EN
3. Installateur NSIS personnalisé (actuellement celui par défaut de tauri)
4. Téléchargements derrière authentification : transmettre les cookies du profil au moteur de téléchargement
5. DoT (DNS over TLS) — DoH et serveurs DNS personnalisés sont pris en charge aujourd'hui
6. Améliorations supplémentaires de performance et de stabilité des onglets

---

## Limitations connues / bugs actuels

- **Téléchargements authentifiés.** Le moteur de téléchargement dédié ne transmet pas les cookies WebView2, donc les fichiers derrière un lien protégé peuvent échouer (les téléchargements publics fonctionnent).
- **Politiques de cookies.** Elles s'appliquent à la création de l'onglet — après une modification de politique, il faut rouvrir l'onglet ; `Set-Cookie` dans les réponses HTTPS n'est pas réécrit par le proxy.
- **DNS.** DoH et serveurs DNS personnalisés fonctionnent ; DoT n'est pas pris en charge. Le trafic vers les adresses LAN/IP va toujours en direct.
- **Extensions.** Le runtime v1 exécute les content scripts par masques, mais l'interface de gestion est masquée et l'API est limitée.
- **Localisation.** Russe et anglais uniquement.
- **LaTeX dans les notes.** Un sous-ensemble est pris en charge (`$...$`, `$$...$$`).

---

## Crédits

Créé par **MrDuck** (idée, vision produit, décisions de conception, tests).  
La majeure partie du code a été écrite avec l'aide de l'IA.

## Licence

[MIT](LICENSE)