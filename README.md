# 花笺

一款 Windows 上轻量、优雅的本地便签工具。

## 为什么选择花笺

市面上不缺功能强大的笔记软件，也不缺轻量的便签小工具，但很少有两者兼备的——既轻便、随呼随用，又有现代化的界面和舒适的编辑体验。花笺正是为此而生。

## 应用场景

- 当作随时可见的剪贴板，快速暂存和复制文本
- 游戏、看视频时随手记点东西
- 临时记录思路或灵感
- 桌面待办清单

## 功能

- **Markdown 编辑与预览** — 实时切换编辑和预览模式
- **快捷便签** — 通过托盘或全局快捷键（默认 `Ctrl+Space`）随时唤出便签窗口
- **磁贴模式** — 将笔记固定在桌面，支持置顶、拖拽和缩放
- **导入导出** — 支持 `.md` 文件的导入和导出

## 下载安装

前往 [GitHub Releases](https://github.com/Achilng/floral-notepaper/releases) 下载最新版本。

> 目前仅在 Windows 11 上测试，其他系统版本的兼容性尚未验证。

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

## 许可证

[MIT](LICENSE)
