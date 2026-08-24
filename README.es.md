# AP Browser (ApostolProject Browser)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh-CN.md) | **Español** | [Deutsch](README.de.md) | [Français](README.fr.md)

Un navegador de escritorio centrado en la privacidad que combina **navegador + espacio de trabajo + base de conocimiento + IA** en una sola aplicación. Sin Electron: construido con **Tauri 2 / Rust** y pestañas nativas WebView2.

> Fase temprana de desarrollo (v0.1.0), por ahora solo Windows.

## Características

- **Pestañas** — pestañas WebView2 nativas en una única ventana shell: pestañas inactivas, arrastrar y soltar, vista dividida, favicons, barra lateral plegable
- **Notas** — editor Markdown con vista previa, imágenes, dibujo a mano alzada, subconjunto de LaTeX (`$...$`), carpetas, exportación `.md`
- **Grafo de conocimiento** — lienzo infinito con distribución física, tarjetas de notas, conexiones, deshacer/rehacer, exportación PNG
- **Espacios de trabajo y perfiles** — perfiles aislados por carpeta de almacenamiento, conjuntos de pestañas por espacio de trabajo
- **Caja fuerte** — gestor de contraseñas con cifrado AES-256-GCM, derivación de claves Argon2id, importación/exportación CSV
- **Historial y marcadores** — respaldados en SQLite, historial por perfil, sugerencias en la omnibox
- **Paleta de comandos** — Ctrl+K, búsqueda por palabras clave en inglés y ruso
- **Chat de IA** — proveedores compatibles con OpenAI y Ollama local
- **Intercepción de descargas**, temas oscuro/claro, personalización de la interfaz, tour de bienvenida

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
crates/             Crates de dominio: notes, grafo, vault, privacy,
                    network, history, bookmarks, profiles, extensions, ai...
```

## Hoja de ruta

1. Aplicar el motor de privacidad al tráfico real (bloqueo de rastreadores / DNS / capa proxy)
2. Runtime de extensiones (content scripts)
3. Localización además del ruso
4. Actualizaciones automáticas (tauri-plugin-updater + GitHub Releases)
5. Instalador NSIS

## Créditos

Creado por **MrDuck** (idea, visión de producto, decisiones de diseño, pruebas) y **Ox-Alpha** (ingeniero de software IA — autor de la mayor parte del código).

## Licencia

[MIT](LICENSE)
