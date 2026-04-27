# 花笺 开发进度

## 当前阶段：R1 — Tauri 2 基础骨架启动

当前主线已从 C# + WPF 转为 Tauri 2 + React + TypeScript + Rust。旧 WPF 版本不再作为继续扩展的主线，但其已经完成的功能和 Windows 经验会作为迁移参考。

---

## 一、重构决策

| 事项 | 当前结论 |
|------|----------|
| 目标技术栈 | Tauri 2 + React + TypeScript + Rust |
| 目标平台 | Windows 10/11 |
| 前端设计稿 | `huajian-demo-确认此方案/` |
| UI 风格 | 纸面质感、墨色文字、竹绿色强调色、轻量低干扰 |
| 数据策略 | `.md` 原文文件 + `metadata.json` 元数据 + `config.json` 配置 |
| 旧 WPF 代码 | 已从正式项目路径 `src/花笺/` 移除；本机备份位于 `src/花笺-WPF备份-20260428-002118/`，不推送到远端 |

---

## 二、设计稿检查结果

确认版设计稿目录：`huajian-demo-确认此方案/`

已查看的关键文件：

- `package.json`
- `src/App.tsx`
- `src/App.css`
- `src/components/MainWindow.tsx`
- `src/components/NotePad.tsx`
- `src/components/TileShowcase.tsx`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/default.json`

设计稿现状：

- 已具备 Vite + React + TypeScript + Tauri 2 基础结构。
- 前端包含主窗口、便签小窗、磁贴展示三类核心界面。
- 当前数据均为 mock，Markdown 预览是示例级解析，不是生产级实现。
- Rust 侧仍是 Tauri 模板级代码，仅有 `greet` 示例命令和 opener 插件。
- `tauri.conf.json`、`Cargo.toml`、package 名称仍是临时项目名，正式重构时必须改为花笺的产品信息。

验证记录：

- 已运行：`npm.cmd exec tsc -- --noEmit`
- 结果：通过。
- 说明：PowerShell 直接运行 `npm` 会被本机执行策略拦截 `npm.ps1`；在 PowerShell 中应使用 `npm.cmd`，不需要修改系统执行策略。

---

## 三、旧 WPF 版本功能快照

旧 WPF 版本已完成到 Phase 4，可作为迁移验收清单。

### Phase 1：基础骨架

| 任务 | 状态 |
|------|------|
| 搭建项目结构（.NET 8 + WPF） | 已完成 |
| 实现 NoteService：笔记 CRUD | 已完成 |
| 实现 MetadataService：元数据索引 | 已完成 |
| 主窗口基础布局：左侧列表 + 右侧编辑器 | 已完成 |
| 集成 AvalonEdit 作为 Markdown 编辑器 | 已完成 |
| 笔记列表的增删操作 | 已完成 |

### Phase 2：Markdown 支持

| 任务 | 状态 |
|------|------|
| AvalonEdit Markdown 语法高亮 | 已完成 |
| Markdown 实时预览面板 | 已完成，使用 WebView2 渲染 |
| 基础 Markdown 工具栏 | 已完成 |

### Phase 3：便签窗口与磁贴

| 任务 | 状态 |
|------|------|
| 全局热键注册与监听 | 已完成，Ctrl+Space |
| 便签窗口热键呼出、编辑、保存关闭 | 已完成 |
| 新建/打开模式切换 | 已完成 |
| 钉住为磁贴 | 已完成 |
| 磁贴拖拽、缩放、内容编辑 | 已完成 |
| 置顶悬浮模式 | 已完成 |
| WorkerW 桌面嵌入模式 | 已完成 |
| 多便签并存管理 | 已完成 |
| 主窗口钉住笔记为磁贴 | 已完成 |

### Phase 4：系统集成

| 任务 | 状态 |
|------|------|
| 系统托盘图标 + 右键菜单 | 已完成 |
| 最小化到托盘 / 关闭到托盘 | 已完成 |
| 开机自启选项 | 已完成 |
| 应用配置管理（config.json） | 已完成 |

---

## 四、Tauri 重构任务清单

### Phase R0：重构准备

| 任务 | 状态 |
|------|------|
| 确认目标技术栈 | 已完成 |
| 确认前端设计稿目录 | 已完成 |
| 查看设计稿源码结构 | 已完成 |
| TypeScript 无输出检查 | 已完成 |
| 更新 `PLAN.md` | 已完成 |
| 更新 `PROGRESS.md` | 已完成 |
| 决定旧 WPF 目录处理方式 | 已完成：备份后用 Tauri 工程覆盖 `src/花笺/` |

### Phase R1：Tauri 基础骨架

| 任务 | 状态 |
|------|------|
| 建立正式 Tauri 2 工程结构 | 已完成：位于 `src/花笺/` |
| 修正产品名、identifier、窗口配置 | 已完成 |
| 迁移确认版设计稿主题和基础布局 | 已完成初版 |
| 配置 Tauri capabilities | 基础配置已迁入，后续随插件补齐 |
| 建立前端 API 调用封装 | 待开始 |

### Phase R2：Rust 数据核心

| 任务 | 状态 |
|------|------|
| 定义 Note / Metadata / Config 模型 | 待开始 |
| 实现笔记目录初始化 | 待开始 |
| 实现 `.md` CRUD | 待开始 |
| 实现 `metadata.json` 读写 | 待开始 |
| 实现 `config.json` 读写 | 待开始 |
| 添加 Rust 单元测试 | 待开始 |

### Phase R3：主窗口迁移

| 任务 | 状态 |
|------|------|
| 拆分生产级 MainWindow 组件 | 待开始 |
| 接入真实笔记列表 | 待开始 |
| 接入新建、选择、编辑、保存、删除 | 待开始 |
| 实现 Markdown 预览 | 待开始 |
| 实现搜索过滤 | 待开始 |
| 实现保存状态和自动保存 | 待开始 |

### Phase R4：便签与磁贴迁移

| 任务 | 状态 |
|------|------|
| 拆分生产级 NotePad 组件 | 待开始 |
| 设计多窗口 route / label 规则 | 待开始 |
| 实现快捷便签窗口创建 | 待开始 |
| 实现打开已有笔记模式 | 待开始 |
| 实现钉住为磁贴 | 待开始 |
| 实现多磁贴生命周期管理 | 待开始 |
| 实现拖拽、缩放、置顶 | 待开始 |

### Phase R5：Windows 系统集成

| 任务 | 状态 |
|------|------|
| 系统托盘 | 待开始 |
| 关闭到托盘 | 待开始 |
| 全局快捷键 | 待开始 |
| 开机自启 | 待开始 |
| WorkerW 桌面嵌入 | 待开始 |
| 多显示器 / DPI / Explorer 重启边界验证 | 待开始 |

### Phase R6：导入导出与设置

| 任务 | 状态 |
|------|------|
| 单文件 `.md` 导入 | 待开始 |
| 批量导入 | 待开始 |
| 当前笔记导出 | 待开始 |
| 批量导出 | 待开始 |
| 设置页 | 待开始 |
| 旧数据目录迁移识别 | 待开始 |

---

## 五、当前环境信息

旧 WPF 环境记录：

- .NET SDK：8.0.420
- 目标框架：net8.0-windows
- 开发环境：Windows 11

确认版设计稿依赖记录：

- `@tauri-apps/api`：`^2`
- `@tauri-apps/cli`：`^2`
- `@tauri-apps/plugin-opener`：`^2`
- React：`^19.1.0`
- TypeScript：`~5.8.3`
- Vite：`^7.0.4`
- Tailwind CSS：`^4.2.4`

注意：

- 本机 PowerShell 执行策略会拦截 `npm.ps1`，后续在 PowerShell 中运行 npm 命令优先使用 `npm.cmd`。
- 不得擅自升级 Node、Rust、Tauri CLI 或系统环境组件；遇到版本问题先记录并向用户确认。
- 临时测试数据和一次性脚本统一放到 `D:\Agent\Agent_temp`。

R1 验证记录：

- 已运行：`npm.cmd install --cache D:\Agent\Agent_temp\npm-cache`
- 结果：通过，0 个漏洞。
- 已运行：`npm.cmd run build`
- 结果：通过。
- 已运行：临时使用 `D:\Rust\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin` 加入本次进程 PATH 后执行 `cargo test`
- 结果：通过。当前系统 PATH 仍未包含 Cargo；未修改系统环境。
- 已运行：同样临时 PATH 下执行 `npm.cmd run tauri -- info`
- 结果：通过。Tauri 识别到 WebView2、MSVC、`rustc 1.94.1`、`cargo 1.94.1`；因未走 rustup 默认代理，`rustup/toolchain` 检测项有警告。
- 已调整：`package.json` 的 `tauri` 脚本在 npm 进程内临时加入 `D:\Rust\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin`，不修改系统 PATH。

---

## 六、变更日志

### 2026-04-28（Tauri R1 基础骨架启动）

- 按要求将 `huajian-demo-确认此方案/` 作为只读设计稿，不修改该目录。
- 将旧 WPF 项目源码备份到 `src/花笺-WPF备份-20260428-002118/`，备份时排除 `bin/obj`。
- 在 `src/花笺/` 内建立 Tauri 2 + React + TypeScript + Vite 工程骨架。
- 将确认版设计稿的主题与基础前端布局迁入正式目录，并去掉设计预览顶部切换壳。
- 修正 `package.json`、`Cargo.toml`、`tauri.conf.json` 中的临时项目名、identifier、窗口尺寸、构建命令和本机 Rust toolchain 调用路径。
- 删除 Vite/React/Tauri 示例图标资产，补充 Node/Tauri 构建产物 `.gitignore` 规则。
- 前端构建验证通过；Rust 侧使用 `D:\Rust` 下已有 stable toolchain 验证通过，未修改系统环境。
- 按要求将正式路径 `src/花笺/` 中旧 WPF 残留文件和根目录旧解决方案移除，仅保留 Tauri 项目结构；WPF 备份作为本机迁移备份忽略，不推送。

### 2026-04-27（Tauri 2 重构准备）

- 决定项目主线重构为 Tauri 2 + React + TypeScript + Rust。
- 确认前端设计稿使用 `huajian-demo-确认此方案/`。
- 查看确认版设计稿的主窗口、便签小窗、磁贴展示、主题样式和 Tauri 模板结构。
- 运行 `npm.cmd exec tsc -- --noEmit`，确认设计稿 TypeScript 检查通过。
- 更新 `PLAN.md` 为 Tauri 2 重构实施计划。
- 更新 `PROGRESS.md` 为重构准备进度。

### 2026-04-27（旧 WPF：系统集成）

- 旧 WPF 版本已完成托盘、关闭到托盘、开机自启、配置管理等 Phase 4 功能。
- 旧 WPF 版本中 WorkerW 桌面嵌入、便签窗口生命周期和热键逻辑后续作为 Rust/Tauri 迁移参考。

### 2026-04-26（旧 WPF：基础与 Markdown）

- 旧 WPF 版本完成 Phase 1 基础骨架。
- 完成 Markdown 语法高亮、WebView2 预览、工具栏和相关预览问题修复。

---

## 七、下一步

1. 运行 `npm.cmd run tauri -- dev` 验证桌面应用启动路径。
2. 建立前端 API 调用封装，为 R2 Rust 数据核心接入做准备。
3. 先实现 Rust 数据核心，再把主窗口从 mock 数据切换到真实笔记数据。
4. 每完成一个重构阶段，都更新本文件的任务状态和验证记录。
