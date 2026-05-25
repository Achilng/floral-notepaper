<!-- markdownlint-disable -->

<div align="center">

<img src="./src-tauri/icons/icon.png" width="120" alt="Floral Notepaper Icon">

# Floral Notepaper

A lightweight, elegant, and modern sticky note app for your desktop<br>
Built with Tauri 2 + React

[简体中文](README.md) · [繁體中文](README_zh-HK.md)<br>
[Report an Issue](https://github.com/Achilng/floral-notepaper/issues) · [Changelog](https://github.com/Achilng/floral-notepaper/releases)

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

## Why Floral Notepaper

Most note-taking or sticky note apps out there are either bloated and steep to learn, or dated and long abandoned. Floral Notepaper was built to be different — quick to summon, light to use, and a pleasure to look at.

## Features

- **Markdown Editing & Preview** — Full GitHub Flavored Markdown support with seamless toggling between edit and preview modes

  ![Main window screenshot](Docs/images/主窗口截图.png)

- **Quick Note** — Summon a note window instantly from the system tray or via a global hotkey (default: `Ctrl+Space`)

  ![Multi-window example](Docs/images/小窗多开示例.gif)

- **Pin Mode** — Pin a note to a fixed spot on your desktop for quick reference and easy copying

  ![Pin mode example](Docs/images/AI绘画截图.png)

- **Import & Export** — Import and export notes as `.md` files

## Use Cases

- Use it as an always-visible clipboard to stash and copy text on the fly
- Jot things down while gaming or watching videos
- Capture a quick thought or burst of inspiration
- Keep a to-do list right on your desktop

## Download

Head over to [GitHub Releases](https://github.com/Achilng/floral-notepaper/releases) to download the latest version.

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri CLI 2](https://tauri.app/)

### Steps

```bash
git clone https://github.com/Achilng/floral-notepaper.git
cd floral-notepaper

npm install

# Development mode
npm run tauri dev

# Production build
npm run tauri build
```

Build artifacts are output to `src-tauri/target/release/bundle/`.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Achilng/floral-notepaper&type=Date&legend=top-left)](https://star-history.com/#Achilng/floral-notepaper&Date)

## 🌟 Contributors

[![contrib.rocks](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)](https://contrib.rocks/image?repo=Achilng/floral-notepaper&max=1000)

## License

[MIT](LICENSE)
