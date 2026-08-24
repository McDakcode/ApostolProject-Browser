# AP Browser (ApostolProject Browser)

[English](README.md) | [Русский](README.ru.md) | **中文** | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

一款注重隐私的桌面浏览器，将**浏览器 + 工作区 + 知识库 + AI** 融合于单一应用。非 Electron——基于 **Tauri 2 / Rust** 构建，使用原生 WebView2 标签页。

> 早期开发阶段（v0.1.0），目前仅支持 Windows。

## 功能特性

- **标签页** — 单一外壳窗口中的原生 WebView2 标签页：休眠标签页、拖拽排序、分屏视图、网站图标、可折叠侧边栏
- **笔记** — Markdown 编辑器：实时预览、图片、手绘、LaTeX 子集（`$...$`）、文件夹、`.md` 导出
- **知识图谱** — 无限画布、物理布局、笔记卡片、连接线、撤销/重做、PNG 导出
- **工作区与配置档案** — 按存储文件夹隔离的配置档案，每个工作区独立的标签页集合
- **密码库** — 密码管理器：AES-256-GCM 加密、Argon2id 密钥派生、CSV 导入/导出
- **历史与书签** — 基于 SQLite，按配置档案保存历史记录，地址栏智能建议
- **命令面板** — Ctrl+K，支持英文/俄文关键词搜索
- **AI 对话** — 兼容 OpenAI 的服务商及本地 Ollama
- **下载拦截**、深色/浅色主题、界面自定义、新手引导

## 技术栈

| 层级 | 技术 |
|---|---|
| 后端 | Rust workspace（`crates/*`）、Tauri v2、wry/WebView2 |
| 前端 | 原生 HTML/CSS/JS — 无打包器、无 npm、无框架 |
| 数据 | `%APPDATA%/dev.apb.browser/`（SQLite、Markdown、JSON） |

前端在构建时嵌入二进制文件（`frontendDist = "../ui"`）。

## 从源码构建

前置要求：[Rust](https://rustup.rs)（stable，MSVC 工具链）、WebView2 运行时（Windows 10/11 已预装）。

```powershell
cd apps/desktop/src-tauri
cargo build
# 运行调试版本：
../../target/debug/apb-desktop.exe
```

## 项目结构

```
apps/desktop/
  src-tauri/        Rust 后端（shell、pages、cmd/* 领域命令）
  ui/               外壳前端（index.html + js 模块，按顺序加载）
crates/             领域 crate：notes、图谱、vault、privacy、
                    network、history、bookmarks、profiles、extensions、ai 等
```

## 路线图

1. 将隐私引擎应用于真实流量（跟踪器拦截 / DNS / 代理层）
2. 扩展运行时（content scripts）
3. 除俄语之外的本地化
4. 自动更新（tauri-plugin-updater + GitHub Releases）
5. NSIS 安装程序

## 许可证

[MIT](LICENSE)
