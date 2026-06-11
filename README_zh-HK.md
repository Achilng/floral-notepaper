[简体中文](README.md) | **繁體中文** | [English](README_en-US.md)

<!-- markdownlint-disable -->

<div align="center">

<img src="./src-tauri/icons/icon.png" width="120" alt="花箋圖示">

# 花箋 Floral Notepaper

輕巧、優雅、現代化的本機便箋工具<br>
基於 Tauri 2 + React 構建

[回報問題](https://github.com/Achilng/floral-notepaper/issues) · [更新日誌](https://github.com/Achilng/floral-notepaper/releases) <br>
[快速開始](#快速開始) · [FAQ](https://github.com/Achilng/floral-notepaper/wiki) · [構建指南](#從原始碼構建)

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

## 為什麼選擇花箋

市面上現有的筆記或便箋軟件，要麼功能繁重、上手門檻高，要麼介面陳舊、久未更新。花箋因此而生，其特點是輕巧、隨呼隨用，同時提供現代化的介面與舒適的編輯體驗。

## 功能特點

- **Markdown 編輯與預覽** — 支援 GitHub Flavored Markdown 語法，可即時切換編輯及預覽模式

  ![主視窗截圖](Docs/images/主窗口截图.png)

- **快速便箋** — 透過系統匣或全域快速鍵（預設 `Ctrl+Space`）隨時喚出便箋視窗

  ![小視窗多開示例](Docs/images/小窗多开示例.gif)

- **磁貼模式** — 將筆記固定於桌面某處，以便快速查閱及複製

  ![磁貼示例](Docs/images/AI绘画截图.png)

- **匯入匯出** — 支援 `.md` 檔案的匯入及匯出

## 應用場景

- 用作隨時可見的剪貼簿，快速暫存及複製文字
- 遊戲、觀看影片時隨手記錄
- 臨時記錄思路或靈感
- 桌面待辦清單

## 快速開始

### 下載安裝

#### 透過 Mirror 醬下載

> [!TIP]
> 如您的網絡不便訪問 GitHub，或下載速度過慢，您可以嘗試透過 Mirror 醬下載花箋<br>
> 此外，您也可以透過使用 Mirror 醬下載花箋來贊助花箋的開發者，詳見 [Mirror 醬官網](https://mirrorchyan.com/)

| 系統    | 架構                    | 下載連結                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Windows | x64                     | [![Windows x64 Setup](https://img.shields.io/badge/Setup-x64-blue?logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI1MTIiIGhlaWdodD0iNTEyIiB2aWV3Qm94PSIwIDAgNTEyIDUxMiI%2BPHBhdGggZmlsbD0iI2ZmZiIgZD0iTTAgMGgyNDJ2MjQySDB6TTI3MCAwaDI0MnYyNDJIMjcwek0wIDI3MGgyNDJ2MjQySDB6TTI3MCAyNzBoMjQydjI0MkgyNzB6Ii8%2BPC9zdmc%2B)](https://mirrorchyan.com/zh/projects?rid=floral&os=windows&arch=x64&channel=stable) |
| macOS   | AArch64 (Apple Silicon) | [![macOS Apple Silicon](https://img.shields.io/badge/DMG-Apple%20Silicon-%23000000.svg?logo=apple)](https://mirrorchyan.com/zh/projects?rid=floral&os=macos&channel=stable&arch=arm64)                                                                                                                                                                                                                                                                             |
| macOS   | x64 (Intel)             | [![macOS Intel](https://img.shields.io/badge/DMG-Intel%20X64-%2300A9E0.svg?logo=apple)](https://mirrorchyan.com/zh/projects?rid=floral&os=macos&channel=stable&arch=x64)                                                                                                                                                                                                                                                                                           |

#### 透過 GitHub 下載

請前往 [Release 頁](https://github.com/Achilng/floral-notepaper/releases/latest) 下載花箋

##### 下載參考

| 系統    | 架構                    | 類型     | 檔案名稱                                |
| ------- | ----------------------- | -------- | --------------------------------------- |
| Windows | x64                     | 安裝程式 | floral-notepaper\_版本號\_x64-setup.exe |
| Windows | x64                     | 可攜版   | floral-notepaper\_版本號.exe            |
| macOS   | AArch64 (Apple Silicon) | DMG      | floral-notepaper\_版本號\_aarch64.dmg   |
| macOS   | x64 (Intel)             | DMG      | floral-notepaper\_版本號\_x64.dmg       |

#### macOS 版安裝指引

如遇安裝問題，請參考：

- Wiki 中的 [macOS 安裝指引](https://github.com/Achilng/floral-notepaper/wiki/macOS-%E5%AE%89%E8%A3%85%E6%8C%87%E5%BC%95-%7C-macOS-Installation-Guidance)
- 或影片（Bilibili）：[Mac雲課堂 - 在 Mac 上裝軟件，要學會和蘋果鬥智鬥勇](https://www.bilibili.com/video/BV1tg411t7hN)

### 從原始碼構建

請參考 [CONTRIBUTING.md](CONTRIBUTING.md)

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Achilng/floral-notepaper&type=Date&legend=top-left)](https://star-history.com/#Achilng/floral-notepaper&Date)

## 🌟 貢獻者

[![contrib.rocks](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)

## 授權條款

[MIT](LICENSE)
