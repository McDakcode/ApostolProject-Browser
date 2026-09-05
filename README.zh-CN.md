# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | **中文** | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

一款注重隐私的桌面浏览器，将**浏览器 + 工作区 + 知识库 + AI** 融合于单一应用。非 Electron——基于 **Tauri 2 / Rust** 构建，使用原生 WebView2 标签页。适用于 Windows 10/11。

## 功能特性

- **标签页** — 单一外壳窗口中的原生 WebView2 标签页：休眠标签页、拖拽排序、固定 📌、标签页文件夹（分组）、分屏视图、真实网站图标与标题、可折叠侧边栏、右键菜单
- **真实流量隐私保护** — 内置过滤代理按域名拦截跟踪器/广告/恶意软件；强制 HTTPS 升级、Cookie 与 Referer 控制、WebRTC 隔离、DoH/DNS、外部代理链（最多 3 跳）、站点例外、实时拦截统计
- **笔记** — Markdown 编辑器：实时预览、图片（含 SVG）、手绘、LaTeX 子集（`$...$`）、文件夹、`.md` 导出
- **知识图谱** — 基于真实 `<canvas>` 的无限画布：物理布局、笔记卡片、连接线、撤销/重做、PNG 导出（含区块内容）
- **工作区与配置档案** — 按存储文件夹隔离的配置档案，每个工作区独立的标签页集合，匿名模式
- **密码库** — 密码管理器：AES-256-GCM 加密、Argon2id 密钥派生、CSV 导入/导出
- **书签与历史** — 基于 SQLite，按配置档案保存历史记录、书签文件夹、`javascript:` 书签小工具、地址栏智能建议
- **命令面板** — Ctrl+K，支持英文/俄文关键词搜索
- **AI 对话** — 兼容 OpenAI 的服务商及本地 Ollama
- **下载** — 专属引擎：真正可取消、可重试，进度条附带速度与图表
- **本地化** — 俄语与英语界面
- 深色/浅色主题、网站智能深色主题、界面自定义（`.apbtheme`）、窗口透明/玻璃效果、新手引导

## 路线图

1. 完整的扩展运行时（按 URL 掩码执行 content scripts）
2. 除 RU/EN 之外的本地化
3. 自定义安装程序（用 Rust 实现完整的安装控制）

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
crates/             领域 crate：notes、canvas、vault、privacy、
                    network、history、bookmarks、profiles、extensions、ai 等
```

## 制作人员

由 **MrDuck**（创意、产品方向、设计决策、测试）与 **Ox-Alpha**（AI 软件工程师——编写了本代码库的大部分）共同打造。

## 许可证

[MIT](LICENSE)
