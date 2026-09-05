# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | **Español** | [Deutsch](README.de.md) | [Français](README.fr.md)

**APB** es un navegador de escritorio centrado en la privacidad que combina en una sola aplicación:

- un navegador completo con pestañas nativas,
- un espacio de trabajo (workspaces y perfiles),
- una base de conocimiento (notas + grafo),
- un gestor de contraseñas,
- un asistente de IA.

Sin Electron. Construido con **Tauri 2 + Rust** y pestañas nativas **WebView2**.

> Compilación de desarrollo estable **v0.2.7**. Por ahora solo Windows 10/11.

---

## Por qué APB

La mayoría de los navegadores «privados» son forks de Chromium/Firefox con un montón de extensiones, o envoltorios finos. APB toma otro camino:

- **Aislamiento de verdad.** Cada perfil es una carpeta de almacenamiento separada. Marcadores, historial, notas, privacidad, red y caja fuerte no se mezclan entre perfiles.
- **Capa de proxy propia.** Un proxy HTTP local que bloquea rastreadores/anuncios/malware por dominio, reescribe cabeceras, construye cadenas (HTTP/SOCKS5) y muestra estadísticas en vivo.
- **Base de conocimiento dentro del navegador.** Notas Markdown con imágenes, dibujo a mano alzada, LaTeX e un grafo interactivo de conexiones.
- **Stack mínimo.** Frontend en Vanilla JS — sin npm, sin React/Vue. Binario relativamente ligero.

---

## Características

### Navegador y pestañas
- Pestañas WebView2 nativas en una única ventana shell
- Pestañas inactivas, arrastrar y soltar, fijación, carpetas de pestañas (grupos), vista dividida
- Favicons y títulos reales, barra lateral plegable, menús contextuales de pestañas
- Workspaces — conjuntos de pestañas intercambiables
- Intercepción de descargas (motor propio: cancelar, reintentar, progreso con velocidad) y restauración de sesión

### Privacidad y red
- Niveles de privacidad + modo de emergencia (panic button)
- Proxy local de filtrado: bloqueo de rastreadores/anuncios/malware por dominio y estadísticas en vivo
- DNS / DoH, servidores DNS personalizados, solo-HTTPS, control de cookies y Referer, aislamiento WebRTC, cadenas de proxy
- Excepciones por sitio
- Auditoría de ajustes (DNS, proxy, extensiones, IA, caja fuerte)

### Notas y grafo de conocimiento
- Editor Markdown con vista previa
- Imágenes (incl. SVG), dibujo a mano alzada, subconjunto LaTeX (`$...$`)
- Carpetas y exportación `.md`
- Lienzo infinito con distribución física
- Tarjetas de notas, conexiones, deshacer/rehacer, exportación PNG

### Perfiles y datos
- Perfiles totalmente aislados (incluido anónimo)
- Historial y marcadores en SQLite (carpetas de marcadores, `javascript:` bookmarklets)
- Sugerencias en la omnibox: historial + marcadores + continuaciones de dominio + búsqueda — todo local, sin servidores de sugerencias externos

### Caja fuerte (Vault)
- Gestor de contraseñas
- AES-256-GCM + Argon2id
- Importación / exportación CSV
- Generador de contraseñas

### Otros
- Chat de IA (proveedores compatibles con OpenAI + Ollama local)
- Paleta de comandos (Ctrl+K) — palabras clave en inglés y ruso
- Temas oscuro / claro, tema oscuro inteligente para sitios, personalización de la interfaz (`.apbtheme`), transparencia/vidrio de ventana
- Localización: interfaz en ruso e inglés
- Tour de bienvenida
- Sistema de extensiones: runtime v1 (content scripts por máscaras de URL), interfaz de gestión oculta por ahora

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

1. Runtime de extensiones completo v2 — los content scripts por máscaras de URL ya se ejecutan (v1); después: interfaz de gestión y una API más amplia
2. Localización más allá de RU/EN
3. Instalador NSIS personalizado (actualmente el estándar de tauri)
4. Descargas tras autenticación: pasar las cookies del perfil al motor de descargas
5. DoT (DNS over TLS) — hoy se admiten DoH y servidores DNS personalizados
6. Más trabajo en rendimiento y estabilidad de pestañas

---

## Limitaciones conocidas / errores actuales

- **Descargas autenticadas.** El motor de descargas propio no pasa las cookies de WebView2, por lo que los archivos tras un enlace protegido pueden fallar (las descargas públicas funcionan).
- **Políticas de cookies.** Se aplican al crear la pestaña — tras cambiar una política hay que abrir la pestaña de nuevo; el proxy no reescribe `Set-Cookie` en respuestas HTTPS.
- **DNS.** DoH y servidores DNS personalizados funcionan; DoT no se admite. El tráfico a direcciones LAN/IP siempre va directo.
- **Extensiones.** El runtime v1 ejecuta content scripts por máscaras, pero la interfaz de gestión está oculta y la API es limitada.
- **Localización.** Solo ruso e inglés.
- **LaTeX en notas.** Se admite un subconjunto (`$...$`, `$$...$$`).

---

## Créditos

Creado por **MrDuck** (idea, visión de producto, decisiones de diseño, pruebas).  
Gran parte del código fue escrita con la ayuda de IA.

## Licencia

[MIT](LICENSE)