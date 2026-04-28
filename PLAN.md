# 花笺 Tauri 2 重构开发实施计划书

> 花笺是一款 Windows 轻量桌面笔记工具，主打轻量、快捷、随手可用。
> 本文档是当前重构阶段的主计划。开发前应先阅读 `PLAN.md` 与 `PROGRESS.md`。

---

## 一、项目概览

| 项目 | 说明 |
|------|------|
| 名称 | 花笺 |
| 定位 | Windows 轻量桌面笔记工具 |
| 重构目标技术栈 | Tauri 2 + React + TypeScript + Rust |
| 目标平台 | Windows 10/11 |
| 前端设计稿 | `huajian-demo-确认此方案/` |
| 旧实现状态 | 现有 C# + WPF 版本已从正式项目目录移除，本机保留 WPF 备份，不再作为后续主线继续扩展 |

### 1.1 重构目标

- 使用 Tauri 2 替换 WPF 外壳，前端以 React + TypeScript 实现界面和交互。
- 使用 Rust 承担本地文件、元数据、配置、窗口、托盘、全局快捷键等桌面能力。
- 继承旧 WPF 版本已经跑通的核心功能：主窗口笔记编辑、Markdown 预览、快捷便签、磁贴、托盘、自启动、导入导出。
- UI 以 `huajian-demo-确认此方案/` 为确认方案，保持纸面质感、墨色文字、竹绿色强调色、轻量低干扰的视觉方向。
- 继续坚持本地优先：笔记以 `.md` 文件保存，元数据和配置透明、可迁移。

### 1.2 非目标

- 当前不做云同步。
- 当前不做 Android / iOS 端。
- 当前不引入账号系统、协作编辑或远程服务。
- 不在重构初期追求兼容旧 WPF 的内部代码结构；只迁移用户可见功能和数据格式。

---

## 二、设计稿结论

确认版设计稿目录：`huajian-demo-确认此方案/`

该目录已经是 Tauri 2 + React + TypeScript + Vite 的原型项目，包含：

- `src/components/MainWindow.tsx`：主窗口布局，包含笔记列表、搜索、新建、编辑/预览/分栏、工具栏和状态栏。
- `src/components/NotePad.tsx`：快捷便签小窗，包含新建/打开模式、标题、正文、保存、钉住入口。
- `src/components/TileShowcase.tsx`：磁贴状态展示，包含多磁贴、拖拽握柄、关闭点、缩放提示、置顶/桌面模式提示。
- `src/App.css`：Tailwind 4 主题 token，定义纸张色、墨色、竹绿色、字体和动效。
- `src-tauri/`：Tauri 2 基础壳，目前仅有模板级 Rust 命令和 opener 插件。

设计稿可作为 UI 和项目脚手架参考，但不能直接视为生产实现。正式重构时需要拆分 mock 数据、补齐真实状态管理、接入 Rust 命令、配置 Tauri 权限，并处理多窗口生命周期。

---

## 三、核心功能模块

### 3.1 主窗口

主窗口是笔记浏览和编辑的主体。

- 左侧笔记列表：展示标题、摘要、更新时间、字数。
- 搜索：支持按标题和正文摘要过滤。
- 新建笔记：无需强制输入标题，标题可为空。
- 编辑器：Markdown 文本编辑。
- 预览：支持编辑、预览、分栏三种模式。
- 工具栏：侧栏折叠、钉为磁贴、导出、删除、基础 Markdown 快捷操作。
- 状态栏：行列信息、编码、文件大小或字数、保存状态。

**标题规则：**

- 新建笔记时不要求输入标题。
- 列表显示名优先取：用户标题 > 内容首行文本 > `无标题笔记`。
- 用户可以在主窗口随时修改标题。

### 3.2 便签窗口

便签窗口是快捷记录和磁贴的统一生命周期。

```
全局热键呼出 -> 编辑内容 -> 保存关闭
                         -> 转为磁贴 -> 右键转为小窗 -> 继续编辑
```

快捷记录状态：

- 通过全局热键呼出。
- 默认进入正文编辑区。
- 支持新建模式和打开已有笔记模式。
- 保存后写入笔记库并关闭窗口。
- 钉住后在同一窗口内转为磁贴状态。

磁贴状态：

- 可拖拽、可缩放、内容只读展示；需要编辑时通过右键菜单转回小窗。
- 支持同时存在多个磁贴。
- 支持置顶悬浮。
- ~~支持贴在桌面模式。该能力需要 Rust 调用 Windows Win32 / WorkerW 相关 API，Tauri 本身只负责窗口外壳。~~ 当前不需要 WorkerW 桌面嵌入，磁贴保留置顶悬浮模式。
- 关闭后不自动恢复；后续如需恢复上次磁贴状态，单独作为新功能设计。

### 3.3 系统集成

- 系统托盘：托盘菜单包含打开主窗口、快速记录、开机自启、关闭到托盘、退出。
- 全局快捷键：默认用于呼出快捷便签，后续可配置。
- 开机自启：优先使用 Tauri 2 官方 autostart 插件；如不能满足 Windows 细节，再封装 Rust 平台实现。
- 窗口管理：主窗口、便签窗口、磁贴窗口使用稳定 label 管理生命周期。
- 关闭策略：关闭主窗口默认隐藏到托盘，退出必须走托盘或明确退出命令。

### 3.4 导入导出

- 导入单个 `.md` 文件到笔记库。
- 导出当前笔记为 `.md`。
- ~~后续支持批量导出到目录。~~ 已取消，当前不做。
- 文件选择优先使用 Tauri dialog 插件或 Rust 侧文件选择封装。

---

## 四、数据存储方案

### 4.1 存储边界

Rust 后端是文件系统和数据一致性的唯一权威。React 前端只通过 Tauri commands 读写笔记、元数据和配置，不直接写用户笔记目录。

### 4.2 笔记内容

- 每条笔记保存为独立 `.md` 文件。
- 默认目录：用户文档目录下的 `花笺/notes/`，允许用户后续在设置中修改。
- 文件名建议：`{id}_{safe_title}.md`；无标题时为 `{id}.md`。
- 文件名必须做非法字符清理，避免 Windows 路径保留字符和重名冲突。

### 4.3 元数据索引

初期继续使用 `metadata.json`，由 Rust 读写。

```json
{
  "notes": [
    {
      "id": "uuid",
      "title": "笔记标题",
      "fileName": "uuid_笔记标题.md",
      "createdAt": "2026-04-26T10:00:00",
      "updatedAt": "2026-04-26T12:00:00",
      "wordCount": 120
    }
  ]
}
```

原则：

- 元数据用于快速加载列表，不替代 `.md` 原文。
- 启动时应能发现元数据与文件不一致，并给出可恢复策略。
- 笔记量明显增长后，再评估 SQLite；重构初期不引入数据库。

### 4.4 应用配置

配置保存为 `config.json`，包含：

- 笔记目录。
- 默认快捷键。
- 关闭到托盘。
- 开机自启。
- 便签/磁贴自动保存开关。
- 窗口尺寸和位置。
- 编辑/预览默认模式。

配置可以使用 Rust JSON 文件实现；如使用 Tauri store 插件，必须明确 capabilities 权限。

---

## 五、技术方案

正式 Tauri 重构工程按用户要求放在 `src/花笺/` 内，并覆盖原 WPF 项目路径；`huajian-demo-确认此方案/` 仅作为只读前端设计稿参考，不在该目录内直接改动。

### 5.1 目标项目结构

```
花笺/src/花笺/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── app/
│   │   ├── routes.ts
│   │   └── providers.tsx
│   ├── components/
│   │   ├── main-window/
│   │   ├── note-pad/
│   │   ├── tile/
│   │   └── common/
│   ├── features/
│   │   ├── notes/
│   │   ├── markdown/
│   │   ├── settings/
│   │   └── windows/
│   ├── styles/
│   │   └── theme.css
│   └── types/
├── src-tauri/
│   ├── tauri.conf.json
│   ├── Cargo.toml
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── lib.rs
│       ├── main.rs
│       ├── commands/
│       ├── models/
│       ├── services/
│       │   ├── notes.rs
│       │   ├── metadata.rs
│       │   ├── config.rs
│       │   ├── windows.rs
│       │   ├── tray.rs
│       │   ├── hotkeys.rs
│       │   └── desktop_embed.rs
│       └── platform/
│           └── windows.rs
├── legacy-wpf/                 # 可选：迁移时暂存旧 WPF 项目，确认后再决定是否删除
└── README.md
```

### 5.2 前端技术点

| 功能 | 技术方案 |
|------|----------|
| UI 框架 | React + TypeScript |
| 构建工具 | Vite |
| 样式 | Tailwind 4 + 主题 token，参考确认版设计稿 |
| Markdown 编辑 | 初期可使用 `<textarea>` 跑通流程；后续评估 CodeMirror 6 |
| Markdown 预览 | 前端 Markdown 渲染库，需支持软换行策略 |
| 状态管理 | 优先使用 React hooks + feature 内聚状态；复杂后再引入轻量 store |
| Tauri 调用 | 封装 `features/*/api.ts`，避免组件直接散落 `invoke` |

### 5.3 Rust / Tauri 技术点

| 功能 | 技术方案 |
|------|----------|
| 桌面外壳 | Tauri 2 |
| 本地文件 | Rust 标准库 / `serde_json` / `uuid` / `chrono` |
| Tauri commands | `notes_*`、`config_*`、`window_*`、`import_export_*` 分组 |
| 多窗口 | Tauri WebviewWindow / Rust Window 管理，使用 query 或 route 区分主窗口、便签、磁贴 |
| 系统托盘 | Tauri tray API，需要启用 `tray-icon` feature |
| 全局快捷键 | `tauri-plugin-global-shortcut` |
| 开机自启 | 优先 `tauri-plugin-autostart` |
| 文件选择 | `tauri-plugin-dialog` |
| 打开文件夹 | `tauri-plugin-opener` |
| WorkerW 桌面嵌入 | 已取消，当前不需要桌面嵌入功能 |
| 权限 | 严格维护 `src-tauri/capabilities/default.json`，只开放必要命令和插件权限 |

### 5.4 参考文档

- Tauri Vite 前端配置：`https://v2.tauri.app/start/frontend/vite/`
- Tauri 系统托盘：`https://v2.tauri.app/zh-cn/learn/system-tray/`
- Tauri 全局快捷键插件：`https://v2.tauri.app/zh-cn/plugin/global-shortcut/`
- Tauri autostart 插件：`https://v2.tauri.app/plugin/autostart/`
- Tauri dialog 插件：`https://v2.tauri.app/plugin/dialog/`
- Tauri store / 权限参考：`https://v2.tauri.app/plugin/store/`

---

## 六、重构阶段规划

### Phase R0：重构准备

**目标**：冻结旧 WPF 主线，确认 Tauri 2 方案和设计稿输入。

- [x] 确认目标技术栈为 Tauri 2 + React + TypeScript + Rust。
- [x] 确认设计稿目录为 `huajian-demo-确认此方案/`。
- [x] 检查设计稿结构和 TypeScript 编译状态。
- [x] 更新 `PLAN.md` 和 `PROGRESS.md`。
- [x] 决定旧 WPF 项目迁移期间的目录策略：先复制一份 WPF 备份，再用 Tauri 工程覆盖 `src/花笺/` 正式项目路径。

### Phase R1：Tauri 基础骨架

**目标**：建立可运行的 Tauri 2 应用骨架。

- [x] 在 `src/花笺/` 建立 Tauri + Vite + React + TypeScript 工程结构。
- [x] 修正 `package.json`、`Cargo.toml`、`tauri.conf.json` 的产品名、identifier、窗口尺寸和构建命令。
- [x] 配置 Vite 固定端口、`frontendDist`、`src-tauri` watch ignore。
- [x] 配置基础 capabilities。
- [x] 移植设计稿主题 token 和基础布局。
- [ ] 保证 `npm.cmd run build`、`cargo test`、`npm.cmd run tauri dev` 的基础路径清晰。当前 `npm.cmd run build` 已通过，`cargo test` 可通过显式使用 `D:\Rust` 下 stable toolchain 运行。

### Phase R2：Rust 数据核心

**目标**：先跑通不依赖 UI 的笔记数据闭环。

- [x] 定义 `Note`、`NoteMetadata`、`AppConfig`、错误类型和 command DTO。
- [x] 实现笔记目录初始化。
- [x] 实现 `.md` 创建、读取、更新、删除。
- [x] 实现 `metadata.json` 读写和损坏恢复策略。
- [x] 实现 `config.json` 读写。
- [x] 为 Rust 服务添加单元测试，测试数据放在临时目录。

### Phase R3：主窗口功能迁移

**目标**：主窗口可真实创建、编辑、预览和保存笔记。

- [x] 将确认版 `MainWindow` 拆为生产组件。
- [x] 接入 Rust commands 加载笔记列表。
- [x] 接入新建、选择、编辑、保存、删除。
- [x] 实现保存状态和自动保存策略。
- [x] 接入 Markdown 预览。
- [x] 实现搜索过滤。
- [x] 实现钉为磁贴入口，但磁贴窗口创建可在 R4 完成。

### Phase R4：便签与磁贴窗口迁移

**目标**：恢复旧 WPF 版本的快捷记录和磁贴能力。

- [x] 将确认版 `NotePad` 拆为生产组件。
- [x] 设计窗口路由：`main`、`notepad`、`tile:{noteId}`。
- [x] 实现从 Rust 或前端创建便签窗口。
- [x] 实现快捷便签保存到笔记库。
- [x] 实现打开已有笔记模式。
- [x] 实现钉住为磁贴。
- [x] 实现多磁贴生命周期管理。
- [x] 实现磁贴拖拽、缩放、置顶。

### Phase R5：Windows 系统集成

**目标**：恢复常驻桌面工具体验。

- [x] 接入系统托盘和托盘菜单。
- [x] 实现关闭到托盘。
- [x] 接入全局快捷键呼出便签。
- [x] 接入开机自启。
- [x] ~~迁移 WorkerW 桌面嵌入能力。~~ 已取消，当前不需要此功能。
- [x] ~~处理 Explorer 重启、多显示器、DPI 缩放等 Windows 边界场景。~~ 改为用户侧手动验收，不作为 R5 施工阻塞项。

### Phase R6：导入导出与设置

**目标**：补齐文件流转和基础配置。

- [x] 导入单个 `.md` 文件。
- [x] ~~导入目录或批量 `.md` 文件。~~ 已取消，当前不做。
- [x] 导出当前笔记。
- [x] ~~批量导出。~~ 已取消，当前不做。
- [x] 设置页：笔记目录、快捷键、关闭到托盘、开机自启、便签自动保存、默认视图模式。
- [ ] 迁移旧版本数据目录识别策略。

### Phase R7：打包、验证与打磨

**目标**：形成可试用的 Windows 构建。

- [ ] 配置应用图标和 bundle 信息。
- [ ] 配置 release 构建命令。
- [ ] 验证安装包或独立可执行文件。
- [ ] 做主窗口、便签、磁贴的视觉回归检查。
- [ ] 做异常处理和日志。
- [ ] 清理旧 WPF 项目的最终去留。

---

## 七、迁移原则

1. **先数据，后界面**：Rust 笔记服务先稳定，再接完整 UI。
2. **前端不直接掌控文件系统**：所有笔记写入经 Rust 统一处理，减少权限扩散。
3. **保留已验证的 Windows 经验**：热键、托盘等旧实现是迁移参考；WorkerW 桌面嵌入当前不迁移。
4. **设计稿是视觉基准，不是生产架构**：样式和布局可复用，mock 状态必须拆除。
5. **每阶段可运行**：每个阶段结束都应能启动应用并验证核心路径。
6. **不升级系统环境**：遇到 Node、Rust、Tauri CLI 环境版本问题时，先记录并询问，不擅自升级。
7. **临时文件统一放到 `D:\Agent\Agent_temp`**：测试数据、临时构建输出不写入 C 盘。

---

## 八、待定事项

| 事项 | 状态 | 备注 |
|------|------|------|
| 旧 WPF 项目目录处理 | 待定 | 建议先保留，Tauri MVP 可用后再决定移动或删除 |
| Markdown 编辑器 | 待定 | 初期 textarea，后续评估 CodeMirror 6 |
| 元数据格式 | 暂定 JSON | 数据量增大后再评估 SQLite |
| WorkerW 实现边界 | 已取消 | 当前不需要桌面嵌入功能，保留置顶悬浮磁贴 |
| 快捷键默认值 | 暂定 Ctrl+Space | 需处理冲突和用户自定义 |
| 打包形式 | 待定 | 优先 Tauri 默认 Windows bundle，后续看分发需求 |
| 磁贴视觉方案 | 已迁移 | 清新淡雅风格，四角 L 形装饰角标，无可见功能控件；已从 `D:\Agent\Agent_temp\tile-design/` 迁移到正式项目磁贴态 |
