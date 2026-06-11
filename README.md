**简体中文** | [繁體中文](README_zh-HK.md) | [English](README_en-US.md)

<!-- markdownlint-disable -->

<div align="center">

<img src="./src-tauri/icons/icon.png" width="120" alt="花笺图标">

# 花笺 Floral Notepaper

轻量、优雅、现代化的本地便签工具<br>
基于 Tauri 2 + React 构建

[反馈问题](https://github.com/Achilng/floral-notepaper/issues) · [更新日志](https://github.com/Achilng/floral-notepaper/releases) <br>
[快速开始](#快速开始) · [FAQ](https://github.com/Achilng/floral-notepaper/wiki) · [构建指南](#从源码构建)

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

## 快速开始

### 下载安装

#### 通过Mirror酱下载

> [!TIP]
> 如您的网络不便访问 GitHub，或下载速度过慢，您可以尝试通过Mirror酱下载花笺<br>
> 此外，您也可以通过使用Mirror酱下载花笺来赞助花笺的开发者，详见[Mirror酱官网](https://mirrorchyan.com/)

| 系统    | 架构                    | 下载链接                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Windows | x64                     | [![Windows x64 Setup](https://img.shields.io/badge/Setup-x64-blue?logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI1MTIiIGhlaWdodD0iNTEyIiB2aWV3Qm94PSIwIDAgNTEyIDUxMiI%2BPHBhdGggZmlsbD0iI2ZmZiIgZD0iTTAgMGgyNDJ2MjQySDB6TTI3MCAwaDI0MnYyNDJIMjcwek0wIDI3MGgyNDJ2MjQySDB6TTI3MCAyNzBoMjQydjI0MkgyNzB6Ii8%2BPC9zdmc%2B)](https://mirrorchyan.com/zh/projects?rid=floral&os=windows&arch=x64&channel=stable) |
| macOS   | AArch64 (Apple Silicon) | [![macOS Apple Silicon](https://img.shields.io/badge/DMG-Apple%20Silicon-%23000000.svg?logo=apple)](https://mirrorchyan.com/zh/projects?rid=floral&os=macos&channel=stable&arch=arm64)                                                                                                                                                                                                                                                                             |
| macOS   | x64 (Intel)             | [![macOS Apple Silicon](https://img.shields.io/badge/DMG-Intel%20X64-%2300A9E0.svg?logo=apple)](https://mirrorchyan.com/zh/projects?rid=floral&os=macos&channel=stable&arch=x64)                                                                                                                                                                                                                                                                                   |

#### 通过 GitHub 下载

请前往 [Release 页](https://github.com/Achilng/floral-notepaper/releases/latest) 下载花笺

##### 下载参考

| 系统    | 架构                    | 类型     | 文件名                                  |
| ------- | ----------------------- | -------- | --------------------------------------- |
| Windows | x64                     | 安装程序 | floral-notepaper\_版本号\_x64-setup.exe |
| Windows | x64                     | 便携版   | floral-notepaper\_版本号.exe            |
| macOS   | AArch64 (Apple Silicon) | DMG      | floral-notepaper\_版本号\_aarch64.dmg   |
| macOS   | x64 (Intel)             | DMG      | floral-notepaper\_版本号\_x64.dmg       |

#### macOS 版安装指引

如遇安装问题，请参考：

- Wiki 中的 [macOS 安装指引](https://github.com/Achilng/floral-notepaper/wiki/macOS-%E5%AE%89%E8%A3%85%E6%8C%87%E5%BC%95-%7C-macOS-Installation-Guidance)
- 或视频（Bilibili）：[Mac云课堂 - 在 Mac 上装软件，要学会和苹果斗智斗勇](https://www.bilibili.com/video/BV1tg411t7hN)

### 从源码构建

请参考 [CONTRIBUTING.md](CONTRIBUTING.md)

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Achilng/floral-notepaper&type=Date&legend=top-left)](https://star-history.com/#Achilng/floral-notepaper&Date)

## 🌟 贡献者

[![contrib.rocks](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)

## 许可证

[MIT](LICENSE)
