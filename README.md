<!-- markdownlint-disable -->

<div align="center">

<img src="./src-tauri/icons/icon.png" width="120" alt="花笺图标">

# 花笺 Floral Notepaper

轻量、优雅、现代化的本地便签工具<br>
基于 Tauri 2 + React 构建

[繁體中文](README_zh-HK.md) · [English](README_en-US.md)<br>
[反馈问题](https://github.com/Achilng/floral-notepaper/issues) · [更新日志](https://github.com/Achilng/floral-notepaper/releases)

[![Version](https://img.shields.io/github/v/release/Achilng/floral-notepaper)](https://github.com/Achilng/floral-notepaper/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Stars](https://img.shields.io/github/stars/Achilng/floral-notepaper?color=ffcb47&labelColor=black)</br>
![React 19](https://img.shields.io/badge/React-19-blue?logo=react)
![Tauri v2](https://img.shields.io/badge/Tauri-v2-%2324C8D8?logo=tauri)
![Rust Edition 2021](https://img.shields.io/badge/Rust-2021-%23000000?logo=rust)<br>
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Achilng/floral-notepaper)

</div>

<!-- markdownlint-restore -->

---

## 为什么选择花笺

市面上现有的笔记或便签软件，要么功能繁重、上手门槛高，要么界面陈旧、久未更新。花笺因此而生，其特点是轻便、随呼随用，同时提供现代化的界面与舒适的编辑体验。

## 功能特点

- **Markdown 编辑与预览** — 支持 GitHub Flavored Markdown 语法，实时切换编辑和预览模式

  ![主窗口截图](Docs/images/主窗口截图.png)

- **快捷便签** — 通过托盘或全局快捷键（默认 `Ctrl+Space`）随时唤出便签窗口

  ![小窗多开示例](Docs/images/小窗多开示例.gif)

- **磁贴模式** — 将笔记固定在桌面某处，以便快速查阅和复制

  ![磁贴示例](Docs/images/AI绘画截图.png)

- **导入导出** — 支持 `.md` 文件的导入和导出

## 应用场景

- 当作随时可见的剪贴板，快速暂存和复制文本
- 游戏、看视频时随手记点东西
- 临时记录思路或灵感
- 桌面待办清单

## 下载安装

前往 [GitHub Releases](https://github.com/Achilng/floral-notepaper/releases) 下载最新版本。

## 从源码构建

### 环境要求

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### 步骤

```bash
git clone https://github.com/Achilng/floral-notepaper.git
cd floral-notepaper

npm install

# 开发模式
npm run tauri dev

# 构建发布版本
npm run tauri build
```

构建产物输出到 `src-tauri/target/release/bundle/`。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Achilng/floral-notepaper&type=Date&legend=top-left)](https://star-history.com/#Achilng/floral-notepaper&Date)

## 🌟 贡献者

[![contrib.rocks](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)

## 许可证

[MIT](LICENSE)
