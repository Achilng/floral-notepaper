# 花笺

一款 windows 上轻量、优雅、现代化的本地便签工具。

## 为什么要开发花笺
目前市面上不乏功能强大且现代的笔记软件，也存在轻量化的网页便签工具。但却没有能集成：轻便、页面现代、易上手、随呼随用

## 应用场景
- AI绘画时存放提示词，并且随时快速复制

## 功能

- **Markdown 编辑与预览** — 支持实时切换编辑和预览模式
- **快捷便签** — 通过托盘或全局快捷键（默认 `Ctrl+Space`）随时唤出小窗记录
- **磁贴模式** — 将笔记固定在桌面，支持置顶、拖拽和缩放
- **导入导出** — 支持 `.md` 文件的导入和导出

## 下载安装

前往 [GitHub Releases](https://github.com/Achilng/floral-notepaper/releases) 下载最新版本

> 开发环境为win11系统，暂未测试其它系统版本兼容性。

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

# 开发模式运行
npm run tauri dev

# 构建发布版本
npm run tauri build
```

构建产物输出到 `src-tauri/target/release/bundle/`。

## 许可证

[MIT](LICENSE)
