# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | **Español** | [Deutsch](README.de.md) | [Français](README.fr.md)

Un navegador de escritorio centrado en la privacidad que combina **navegador + espacio de trabajo + base de conocimiento + IA** en una sola aplicación. Sin Electron: construido con **Tauri 2 / Rust** y pestañas nativas WebView2. Windows 10/11.

## Características

- **Pestañas** — pestañas WebView2 nativas en una única ventana shell: pestañas inactivas, arrastrar y soltar, fijación 📌, carpetas de pestañas (grupos), vista dividida, favicons y títulos reales, barra lateral plegable, menús contextuales
- **Privacidad en el tráfico real** — proxy de filtrado integrado que bloquea rastreadores/anuncios/malware por dominio; actualización solo-HTTPS, control de cookies y Referer, aislamiento WebRTC, DoH/DNS, cadenas de proxy externas (hasta 3 saltos), excepciones por sitio, estadísticas de bloqueo en vivo
- **Notas** — editor Markdown con vista previa, imágenes (incl. SVG), dibujo a mano alzada, subconjunto de LaTeX (`$...$`), carpetas, exportación `.md`
- **Grafo de conocimiento** — lienzo infinito sobre un `<canvas>` real con distribución física, tarjetas de notas, conexiones, deshacer/rehacer, exportación PNG con el contenido de los bloques
- **Espacios de trabajo y perfiles** — perfiles aislados por carpeta de almacenamiento, conjuntos de pestañas por espacio de trabajo, modo anónimo
- **Caja fuerte** — gestor de contraseñas con cifrado AES-256-GCM, derivación de claves Argon2id, importación/exportación CSV
- **Marcadores e historial** — respaldados en SQLite, historial por perfil, carpetas de marcadores, javascript:`bookmarklets`, sugerencias en la omnibox
- **Paleta de comandos** — Ctrl+K, búsqueda por palabras clave en inglés y ruso
- **Chat de IA** — proveedores compatibles con OpenAI y Ollama local
- **Descargas** — motor propio con cancelación real, reintento y barra de progreso con velocidad y gráfico
- **Localización** — interfaz en ruso e inglés
- Temas oscuro/claro, tema oscuro inteligente para sitios, personalización de la interfaz (`.apbtheme`), transparencia/vidrio de ventana, visita guiada

## Hoja de ruta

1. Runtime de extensiones completo (content scripts por máscaras de URL)
2. Localización más allá de RU/EN
3. Instalador personalizado (control total de la instalación en Rust)

## Stack tecnológico

| Capa | Tecnología |
|---|---|
| Backend | Workspace de Rust (`crates/*`), Tauri v2, wry/WebView2 |
| Frontend | HTML/CSS/JS vanilla — sin bundler, sin npm, sin frameworks |
| Datos | `%APPDATA%/dev.apb.browser/` (SQLite, Markdown, JSON) |

El frontend se incrusta en el binario durante la compilación (`frontendDist = "../ui"`).

## Compilar desde el código fuente

Requisitos previos: [Rust](https://rustup.rs) (stable, toolchain MSVC), runtime de WebView2 (preinstalado en Windows 10/11).

```powershell
cd apps/desktop/src-tauri
cargo build
# ejecutar la compilación de depuración:
../../target/debug/apb-desktop.exe
```

## Estructura del proyecto

```
apps/desktop/
  src-tauri/        Backend Rust (shell, pages, comandos de dominio cmd/*)
  ui/               Frontend del shell (index.html + módulos js, cargados en orden)
crates/             Crates de dominio: notes, canvas, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai...
```

## Créditos

Creado por **MrDuck** (idea, visión de producto, decisiones de diseño, pruebas) y **Ox-Alpha** (ingeniero de software IA — autor de la mayor parte del código).

## Licencia

[MIT](LICENSE)
