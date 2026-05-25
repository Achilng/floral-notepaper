<!-- markdownlint-disable -->

<div align="center">

<img src="./src-tauri/icons/icon.png" width="120" alt="花箋圖示">

# 花箋 Floral Notepaper

輕巧、優雅、現代化的本機便箋工具<br>
基於 Tauri 2 + React 構建

[简体中文](README.md) · [English](README_en-US.md)<br>
[回報問題](https://github.com/Achilng/floral-notepaper/issues) · [更新日誌](https://github.com/Achilng/floral-notepaper/releases)

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

## 下載安裝

前往 [GitHub Releases](https://github.com/Achilng/floral-notepaper/releases) 下載最新版本。

## 從原始碼構建

### 環境需求

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### 步驟

```bash
git clone https://github.com/Achilng/floral-notepaper.git
cd floral-notepaper

npm install

# 開發模式
npm run tauri dev

# 構建發佈版本
npm run tauri build
```

構建產物輸出至 `src-tauri/target/release/bundle/`。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Achilng/floral-notepaper&type=Date&legend=top-left)](https://star-history.com/#Achilng/floral-notepaper&Date)

## 🌟 貢獻者

[![contrib.rocks](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)

## 授權條款

[MIT](LICENSE)
