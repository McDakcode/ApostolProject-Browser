# AP Browser (ApostolProject Browser)

[Русский](README.md) | [English](README.en.md) | **中文** | [Español](README.es.md) | [Deutsch](README.de.md) | [Français](README.fr.md)

**APB** 是一款注重隐私的桌面浏览器，将以下功能融合于单一应用：

- 带原生标签页的完整浏览器，
- 工作区（工作空间与配置档案），
- 知识库（笔记 + 图谱），
- 密码管理器，
- AI 助手。

非 Electron。基于 **Tauri 2 + Rust** 构建，使用原生 **WebView2** 标签页。

> 早期开发阶段（v0.2.x）。目前仅支持 Windows。

---

## 为什么选择 APB

大多数「隐私」浏览器要么是带一堆扩展的 Chromium/Firefox 分支，要么是薄薄的外壳。APB 走的是另一条路：

- **真正的隔离。** 每个配置档案都是独立的存储文件夹。书签、历史、笔记、隐私设置、网络与密码库在配置档案之间互不泄露。
- **自有代理层。** 本地 HTTP 代理可拦截跟踪器、改写请求头、构建代理链（HTTP/SOCKS5）并显示实时统计。
- **浏览器内的知识库。** 支持图片、手绘、LaTeX 的 Markdown 笔记，以及交互式连接图谱。
- **极简技术栈。** 前端为原生 JS——无 npm、无 React/Vue。二进制体积相对较小。

---

## 功能特性

### 浏览器与标签页
- 单一外壳窗口中的原生 WebView2 标签页
- 休眠标签、拖拽排序、分屏视图
- 网站图标、可折叠侧边栏
- 工作空间——可快速切换的标签页集合
- 下载拦截与会话恢复

### 隐私与网络
- 隐私级别 + 紧急模式（panic button）
- 带跟踪器拦截与实时统计的本地代理
- DNS / DoH、代理链
- 按网站覆盖规则
- 设置审计（DNS、代理、扩展、AI、密码库）

### 笔记与知识图谱
- 带实时预览的 Markdown 编辑器
- 图片、手绘、LaTeX 子集（`$...$`）
- 文件夹与 `.md` 导出
- 带物理布局的无限画布
- 笔记卡片、连接线、撤销/重做、PNG 导出

### 配置档案与数据
- 完全隔离的配置档案（含匿名）
- 基于 SQLite 的历史与书签
- 地址栏智能建议

### 密码库（Vault）
- 密码管理器
- AES-256-GCM + Argon2id
- CSV 导入 / 导出
- 密码生成器

### 其他
- AI 对话（兼容 OpenAI 的服务商 + 本地 Ollama）
- 命令面板（Ctrl+K）——支持英文/俄文关键词
- 深色 / 浅色主题、界面自定义
- 新手引导
- 扩展系统（开发中）

---

## 技术栈

| 层级   | 技术 |
|--------|------|
| 后端   | Rust workspace（`crates/*`）、Tauri 2、wry / WebView2 |
| 前端   | 原生 HTML / CSS / JS — 无打包器、无 npm、无框架 |
| 数据   | `%APPDATA%/dev.apb.browser/`（SQLite、Markdown、JSON） |
| 构建   | 前端在编译时嵌入二进制文件（`frontendDist = "../ui"`） |

### 项目结构

```
apps/desktop/
  src-tauri/        Rust 后端（shell、pages、cmd/* 领域命令）
  ui/               外壳前端（index.html + js 模块）
crates/             领域 crate：
                    notes、vault、privacy、network、history、
                    bookmarks、profiles、extensions、ai ...
```

---

## 从源码构建

**前置要求：**
- [Rust](https://rustup.rs)（stable，MSVC 工具链）
- WebView2 运行时（Windows 10/11 已预装）

```powershell
cd apps/desktop/src-tauri
cargo build

# 运行调试版本：
../../target/debug/apb-desktop.exe
```

发布构建：

```powershell
cargo build --release
```

---

## 路线图

1. 将隐私引擎完全应用于真实流量（跟踪器拦截 / DNS / 代理）
2. 扩展运行时（content scripts + 沙箱）
3. 界面本地化（目前以俄语为主）
4. 通过 GitHub Releases + tauri-plugin-updater 自动更新
5. 完整的 NSIS 安装程序
6. 进一步提升标签页性能与稳定性

---

## 制作人员

由 **MrDuck**（创意、产品方向、设计决策、测试）创建。  
大部分代码在 AI 的帮助下完成。

## 许可证

[MIT](LICENSE)
