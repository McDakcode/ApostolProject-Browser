# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | **Español** | [Deutsch](README.de.md) | [Français](README.fr.md)

**APB** es un navegador de escritorio centrado en la privacidad que combina en una sola aplicación:

- un navegador completo con pestañas nativas,
- un espacio de trabajo (workspaces y perfiles),
- una base de conocimiento (notas + grafo),
- un gestor de contraseñas,
- un asistente de IA.

Sin Electron. Construido con **Tauri 2 + Rust** y pestañas nativas **WebView2**.

> Fase temprana de desarrollo (v0.2.x). Por ahora solo Windows.

---

## Por qué APB

La mayoría de los navegadores «privados» son forks de Chromium/Firefox con un montón de extensiones, o envoltorios finos. APB toma otro camino:

- **Aislamiento de verdad.** Cada perfil es una carpeta de almacenamiento separada. Marcadores, historial, notas, privacidad, red y caja fuerte no se mezclan entre perfiles.
- **Capa de proxy propia.** Un proxy HTTP local que bloquea rastreadores, reescribe cabeceras, construye cadenas (HTTP/SOCKS5) y muestra estadísticas en vivo.
- **Base de conocimiento dentro del navegador.** Notas Markdown con imágenes, dibujo a mano alzada, LaTeX e un grafo interactivo de conexiones.
- **Stack mínimo.** Frontend en Vanilla JS — sin npm, sin React/Vue. Binario relativamente ligero.

---

## Características

### Navegador y pestañas
- Pestañas WebView2 nativas en una única ventana shell
- Pestañas inactivas, arrastrar y soltar, vista dividida
- Favicons, barra lateral plegable
- Workspaces — conjuntos de pestañas intercambiables
- Intercepción de descargas y restauración de sesión

### Privacidad y red
- Niveles de privacidad + modo de emergencia (panic button)
- Proxy local con bloqueo de rastreadores y estadísticas en vivo
- DNS / DoH, cadenas de proxy
- Excepciones por sitio
- Auditoría de ajustes (DNS, proxy, extensiones, IA, caja fuerte)

### Notas y grafo de conocimiento
- Editor Markdown con vista previa
- Imágenes, dibujo a mano alzada, subconjunto LaTeX (`$...$`)
- Carpetas y exportación `.md`
- Lienzo infinito con distribución física
- Tarjetas de notas, conexiones, deshacer/rehacer, exportación PNG

### Perfiles y datos
- Perfiles totalmente aislados (incluido anónimo)
- Historial y marcadores en SQLite
- Sugerencias inteligentes en la omnibox

### Caja fuerte (Vault)
- Gestor de contraseñas
- AES-256-GCM + Argon2id
- Importación / exportación CSV
- Generador de contraseñas

### Otros
- Chat de IA (proveedores compatibles con OpenAI + Ollama local)
- Paleta de comandos (Ctrl+K) — palabras clave en inglés y ruso
- Temas oscuro / claro, personalización de la interfaz
- Tour de bienvenida
- Sistema de extensiones (en desarrollo)

---

## Stack tecnológico

| Capa       | Tecnología |
|------------|------------|
| Backend    | Workspace Rust (`crates/*`), Tauri 2, wry / WebView2 |
| Frontend   | HTML / CSS / JS vanilla — sin bundler, sin npm, sin frameworks |
| Datos      | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |
| Compilación| El frontend se incrusta en el binario (`frontendDist = "../ui"`) |

### Estructura del proyecto

```
apps/desktop/
  src-tauri/        Backend Rust (shell, pages, comandos de dominio cmd/*)
  ui/               Frontend del shell (index.html + módulos js)
crates/             Crates de dominio:
                    notes, vault, privacy, network, history,
                    bookmarks, profiles, extensions, ai ...
```

---

## Compilar desde el código fuente

**Requisitos:**
- [Rust](https://rustup.rs) (stable, toolchain MSVC)
- Runtime de WebView2 (preinstalado en Windows 10/11)

```powershell
cd apps/desktop/src-tauri
cargo build

# ejecutar la compilación de depuración:
../../target/debug/apb-desktop.exe
```

Compilación de release:

```powershell
cargo build --release
```

---

## Hoja de ruta

1. Aplicar el motor de privacidad al tráfico real (bloqueo de rastreadores / DNS / proxy)
2. Runtime de extensiones (content scripts + sandbox)
3. Localización de la interfaz (actualmente principalmente en ruso)
4. Actualizaciones automáticas vía GitHub Releases + tauri-plugin-updater
5. Instalador NSIS completo
6. Más trabajo en rendimiento y estabilidad de pestañas

---

## Créditos

Creado por **MrDuck** (idea, visión de producto, decisiones de diseño, pruebas).  
Gran parte del código fue escrita con la ayuda de IA.

## Licencia

[MIT](LICENSE)
