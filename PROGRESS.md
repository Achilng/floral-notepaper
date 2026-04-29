# 花笺 开发进度

## 当前阶段：R7 — 打包、验证与打磨（进行中）

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
| 定义 Note / Metadata / Config 模型 | 已完成 |
| 实现笔记目录初始化 | 已完成 |
| 实现 `.md` CRUD | 已完成 |
| 实现 `metadata.json` 读写 | 已完成，包含损坏文件改名备份和重建 |
| 实现 `config.json` 读写 | 已完成 |
| 添加 Rust 单元测试 | 已完成，测试数据位于 `D:\Agent\Agent_temp\huajian-rust-tests` |

### Phase R3：主窗口迁移

| 任务 | 状态 |
|------|------|
| 拆分生产级 MainWindow 组件 | 已完成，抽出 notes API、notes utility、Markdown preview |
| 接入真实笔记列表 | 已完成，调用 Rust `notes_list` / `notes_get` |
| 接入新建、选择、编辑、保存、删除 | 已完成 |
| 实现 Markdown 预览 | 已完成 |
| 实现搜索过滤 | 已完成 |
| 实现保存状态和自动保存 | 已完成，编辑后自动延迟保存，也保留手动保存入口 |

### Phase R4：便签与磁贴迁移

| 任务 | 状态 |
|------|------|
| 拆分生产级 NotePad 组件 | 已完成 |
| 设计多窗口 route / label 规则 | 已完成，使用 `main`、`notepad-*`、`tile-*` |
| 实现快捷便签窗口创建 | 已完成，Rust command 动态创建或聚焦窗口 |
| 实现打开已有笔记模式 | 已完成 |
| 实现钉住为磁贴 | 已完成，主窗口和便签窗口均可打开磁贴 |
| 实现多磁贴生命周期管理 | 已完成，按 `tile-{noteId}` label 管理 |
| 实现拖拽、缩放、置顶 | 已完成，使用 Tauri window API；WorkerW 桌面嵌入已取消 |

### Phase R5：Windows 系统集成

| 任务 | 状态 |
|------|------|
| 系统托盘 | 已完成，托盘菜单包含打开主窗口、快速记录、关闭到托盘、开机自启动、退出 |
| 关闭到托盘 | 已完成，主窗口关闭时按配置隐藏到托盘 |
| 全局快捷键 | 已完成，启动时按 `config.json` 中 `globalShortcut` 注册，默认 `Ctrl+Space` 呼出便签 |
| 开机自启 | 已完成，接入 `tauri-plugin-autostart` 并与 `config.json` 同步 |
| WorkerW 桌面嵌入 | 已取消，当前不需要此功能 |
| 多显示器 / DPI / Explorer 重启边界验证 | 改为用户侧手动验收，不作为 R5 施工阻塞项 |

### Phase R6：导入导出与设置

| 任务 | 状态 |
|------|------|
| 单文件 `.md` 导入 | 已完成 |
| 批量导入 | 已取消，当前不做 |
| 当前笔记导出 | 已完成，入口为笔记列表右键菜单 |
| 批量导出 | 已取消，当前不做 |
| 设置页 | 已完成，包含便签自动保存开关 |
| 旧数据目录迁移识别 | 已取消，WPF 版本未发布，无需兼容 |

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

R2 验证记录：

- 已运行：`cargo test`
- 运行方式：本次进程临时设置 `PATH=D:\Rust\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;...`，并设置 `CARGO_HOME=D:\Agent\Agent_temp\cargo-home`。
- 结果：通过，3 个 Rust 单元测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。

R3 验证记录：

- 已运行：`npm.cmd test`
- 结果：通过，3 个前端 utility 测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`cargo test`
- 结果：通过，3 个 Rust 单元测试全部通过。

R4 验证记录：

- 已运行：`npm.cmd test`
- 结果：通过，2 个测试文件、6 个前端测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`cargo test`
- 结果：通过，4 个 Rust 单元测试全部通过。
- 已运行：`npm.cmd run tauri -- info`
- 结果：通过，Tauri 识别到 WebView2、MSVC、`rustc 1.94.1`、`cargo 1.94.1`；仍提示 rustup/toolchain 检测警告，原因是本项目通过 `D:\Rust` 中已有 toolchain 直接注入 PATH。
- 已运行：`npm.cmd run tauri -- build --debug`
- 结果：通过，生成 `src-tauri\target\debug\huajian.exe`、`src-tauri\target\debug\bundle\msi\花笺_0.1.0_x64_zh-CN.msi`、`src-tauri\target\debug\bundle\nsis\花笺_0.1.0_x64-setup.exe`。
- 调试记录：首次 MSI 打包失败是 WiX 默认 `en-US`/code page 1252 不支持中文产品名；已在 `tauri.conf.json` 配置 WiX `language` 为 `zh-CN` 后验证通过。

R5 阶段验证记录（托盘 / 关闭到托盘 / 快捷键 / 自启动）：

- 已运行：`cargo test`
- 结果：通过，8 个 Rust 单元测试全部通过。
- 已运行：`npm.cmd test`
- 结果：通过，2 个测试文件、6 个前端测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`npm.cmd run tauri -- build --debug`
- 结果：通过，生成 `src-tauri\target\debug\huajian.exe`、`src-tauri\target\debug\bundle\msi\花笺_0.1.0_x64_zh-CN.msi`、`src-tauri\target\debug\bundle\nsis\花笺_0.1.0_x64-setup.exe`。
- 备注：Tauri 提示 identifier `com.huajian.app` 对 macOS bundle 不推荐；当前目标平台是 Windows，暂不调整。

磁贴只读交互验证记录：

- 已运行：`npm.cmd test -- src/components/TileShowcase.test.tsx`
- 结果：通过，磁贴组件渲染中不再包含 `input`、`textarea` 和“保存”按钮。
- 已运行：`npm.cmd test -- src/features/windows/api.test.ts`
- 结果：通过，窗口 API 打开便签和磁贴时会携带可选 bounds，用于同位转换。
- 已运行：`npm.cmd test`
- 结果：通过，4 个测试文件、9 个前端测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`cargo test`
- 结果：通过，8 个 Rust 单元测试全部通过。
- 已运行：`npm.cmd run tauri -- build --debug`
- 结果：通过，生成最新可运行文件 `src-tauri\target\debug\huajian.exe`，以及 MSI / NSIS debug 安装包。

磁贴同窗变形验证记录：

- 已运行：`npm.cmd test -- src/features/windows/surfaceMode.test.ts src/components/NotePad.test.tsx src/components/TileShowcase.test.tsx`
- 结果：通过，新增 surface mode 约束和组件渲染测试；磁贴初始态不再渲染输入框、正文编辑框或“保存”按钮，并保留圆角与边框纸面外观。
- 已运行：`npm.cmd test`
- 结果：通过，6 个测试文件、13 个前端测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`cargo test`
- 结果：通过，8 个 Rust 单元测试全部通过。
- 已运行：`npm.cmd run tauri -- build --debug`
- 结果：通过，生成最新可运行文件 `src-tauri\target\debug\huajian.exe`，以及 MSI / NSIS debug 安装包；Tauri 仍提示 Windows 文件占用导致 NSIS bundler type 标记写入失败，但命令退出码为 0，产物已输出。
- 已运行：`git diff --check`
- 结果：通过，仅有工作区 CRLF 提示，无空白错误。

磁贴尺寸与边框修正验证记录：

- 已运行：`npm.cmd test -- src/features/windows/surfaceMode.test.ts src/components/NotePad.test.tsx src/components/TileShowcase.test.tsx`
- 结果：通过，磁贴模式切换保持当前窗口位置和尺寸，磁贴渲染中不再出现外层 `p-2` 包边和可见 `border-paper-deep` 内边框。
- 已运行：`npm.cmd test`
- 结果：通过，6 个测试文件、13 个前端测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`cargo test`
- 结果：通过，8 个 Rust 单元测试全部通过。
- 已运行：`npm.cmd run tauri -- build --debug`
- 结果：通过，生成最新可运行文件 `src-tauri\target\debug\huajian.exe`，以及 MSI / NSIS debug 安装包；Tauri 仍提示 Windows 文件占用导致 NSIS bundler type 标记写入失败，但命令退出码为 0，产物已输出。

磁贴视觉方案迁移验证记录：

- 已运行：`npm.cmd test -- src/components/NotePad.test.tsx src/components/TileShowcase.test.tsx`
- 结果：通过，磁贴态渲染已覆盖 12px 圆角、极淡边框、四角 L 形装饰标记、正文 13px / 1.8 排版、空内容状态和无可见功能按钮。
- 已运行：`npm.cmd test -- src/components/Tile.test.tsx src/components/NotePad.test.tsx src/components/TileShowcase.test.tsx`
- 结果：通过，新增验证磁贴青色变体、磁贴态透明外层和四角装饰。
- 已运行：`cargo test makes_note_surfaces_transparent_for_rounded_tile_corners`
- 结果：通过，便签/磁贴动态窗口创建时启用透明窗口，用于保留可见圆角。
- 已运行：`npm.cmd test`
- 结果：通过，6 个测试文件、13 个前端测试全部通过。
- 已运行：`npm.cmd run build`
- 结果：通过，TypeScript 与 Vite 生产构建成功。
- 已运行：`git diff --check`
- 结果：通过，仅有工作区 CRLF 提示，无空白错误。

---

## 六、变更日志

### 2026-04-29（R7：便签底栏修复、磁贴滚动与设置优化）

- 修复便签小窗底栏侵入正文区的根因：`.noise-bg > *` CSS 规则对所有直接子元素设置 `position: relative`，覆盖了四角缩放手柄的 Tailwind `absolute` 定位，导致四个 32×32px 手柄以正常流排列，占去 128px 高度。改用 `isolation: isolate` + `z-index: -1` 方案，移除 `.noise-bg > *` 规则。
- textarea 最小高度从 `min-h-[96px]` 改为 `min-h-0`，允许编辑区在紧凑窗口中自由缩小。
- 右侧两角（右上靠近关闭按钮、右下靠近保存按钮）缩放热区从 32×32px 缩小到 20×20px，减少误触；左侧两角保持 32×32px。
- 磁贴只读态新增内容滚动能力，内容超出时可滚动查看全文；滚动条隐藏，保持视觉干净。
- 设置页磁贴颜色输入旁新增"默认"按钮，点击恢复为默认颜色 `#f6f3ec`。
- 验证记录：`npm.cmd test -- --run` 通过 16 个测试文件 / 36 个测试；`npm.cmd run build` 通过；`cargo test` 通过 15 个 Rust 测试；`tauri build --debug` 通过。

### 2026-04-29（R7：磁贴字号、缩放与自定义配色）

- 磁贴标题字号调整为 15px，正文调整为 14px，与便签小窗编辑态保持一致，并让标题明显大于正文。
- 移除磁贴鼠标悬停时的缩放动画，仅保留阴影变化，避免悬停造成卡片尺寸跳动。
- 磁贴缩放热区从右下角单点扩展为四角均可缩放，并将判定区域扩大到 32×32px。
- 设置页新增自定义磁贴颜色，提供十六进制颜色输入和系统颜色选择器；磁贴不再依赖预设色板，默认颜色为 `#f6f3ec`。
- 配置文件新增 `tileColor` 字段，Rust 侧兼容旧 `config.json`，缺失字段时自动使用默认磁贴颜色。
- 验证记录：`npm.cmd test -- --run` 通过 16 个测试文件 / 36 个测试；`npm.cmd run build` 通过；`cargo test` 通过 14 个 Rust 测试。

### 2026-04-29（R7：磁贴标题与便签窗口尺寸修正）

- 磁贴标题颜色恢复为此前的灰色弱化样式，同时保留 15px 字号，避免标题重新变得过小。
- 便签小窗补齐四角隐藏缩放热区，和磁贴保持一致，四角均可触发原生窗口缩放。
- 快捷唤起便签的默认窗口尺寸从 420×430 调整为 260×260，最小尺寸调整为 220×180。
- 验证记录：`npm.cmd test -- --run` 通过 16 个测试文件 / 36 个测试；`npm.cmd run build` 通过；`cargo test` 通过 15 个 Rust 测试。

### 2026-04-29（R7：便签正文区被底栏侵入修复尝试，未解决）

- 问题现象：便签小窗在 260×260 紧凑尺寸下，正文编辑区域仍被底部状态/操作栏侵入或挤占，用户复测确认此前改动没有解决。
- 已尝试方案：小窗主体显式占满窗口高度，编辑区增加独立容器和正文最小高度；底栏改为贴住编辑容器底部；便签窗口最小高度从 180 调整为 220。
- 当前结论：自动化渲染测试和构建均通过，但未覆盖真实 WebView/Windows 窗口尺寸下的视觉布局问题；该问题仍需继续排查，不应视为已修复。
- 后续排查方向：需要在真实 Tauri 窗口中复现 260×260 小窗，检查 flex 高度分配、textarea 默认样式、底栏实际高度、窗口内高与 CSS `h-screen` 的差异，必要时重新设计小窗紧凑布局。
- 验证记录：此前尝试后 `npm.cmd test -- --run` 通过 16 个测试文件 / 36 个测试；`npm.cmd run build` 通过；`cargo test` 通过 15 个 Rust 测试，但用户复测失败。

### 2026-04-29（R7：应用图标与 release 构建）

- 使用 `lily_logo_true_transparent.png`（1254×1254 RGBA 百合花）作为应用图标，通过 `tauri icon` 生成全套图标文件（ico、icns、各尺寸 PNG、Appx Store Logo）。
- dev 模式下需 touch `build.rs` 强制 Cargo 重新执行构建脚本以嵌入新图标，已验证窗口和托盘图标均正确显示。
- 执行 release 构建（`tauri build`），产出 `huajian.exe`、`花笺_0.1.0_x64_zh-CN.msi`、`花笺_0.1.0_x64-setup.exe`，均位于 `src-tauri/target/release/` 下。
- `tauri.conf.json` 无需修改，原有 `bundle.icon` 配置已指向正确路径。

### 2026-04-29（R6 收尾：取消旧数据目录迁移识别）

- 经用户确认，旧 WPF 版本从未发布，不存在真实用户侧旧数据迁移需求。
- R6 中“旧数据目录迁移识别”取消，不再实现 WPF 旧配置或旧目录自动识别。
- R6 状态调整为完成，后续主线进入 R7：打包、验证与打磨。

### 2026-04-29（窗口与菜单出入场动画）

- 便签和磁贴窗口新增出入场动画：打开时 350ms 从透明缩小淡入到正常，关闭时 250ms 从正常缩小淡出后销毁窗口。
- 全局右键菜单（编辑区、磁贴）新增出入场动画：弹出时 200ms 上浮淡入，关闭时 150ms 下沉淡出后移除 DOM。
- 主窗口笔记列表右键菜单同样添加出入场动画，重新右键时取消正在进行的关闭动画直接显示新菜单。
- 退场动画期间附加 `pointer-events: none`，避免用户在淡出过程中误触。
- 新增 CSS keyframes：`window-enter`、`window-exit`、`menu-enter`、`menu-exit`，统一使用 `cubic-bezier(0.22, 1, 0.36, 1)` 缓动曲线。
- 验证记录：`npm.cmd test -- --run` 通过 15 个测试文件 / 34 个测试；`npm.cmd run build` 通过。

### 2026-04-29（磁贴右键菜单与便签自动保存开关）

- 磁贴右键菜单从单一“转为小窗”扩展为“复制 / 保存 / 转为小窗 / 关闭”，其中保存会写入当前小窗或磁贴内容，关闭会关闭当前窗口。
- 设置页新增“便签自动保存”开关，配置字段为 `noteSurfaceAutoSave`，默认开启以保持原有行为。
- 关闭便签自动保存后，转为磁贴不会自动落盘；只有小窗“保存”按钮或磁贴右键“保存”会写入笔记库，关闭窗口会丢弃未保存草稿。
- Rust 配置读取兼容旧 `config.json`：缺少 `noteSurfaceAutoSave` 时自动按 `true` 处理，避免旧配置升级失败。
- 验证记录：`npm.cmd test -- --run` 通过 15 个测试文件 / 34 个测试；`npm.cmd run build` 通过；`cargo test` 通过 14 个 Rust 测试。

### 2026-04-29（便签小窗外层边框修复）

- 修复便签小窗外层额外边框问题：便签编辑态不再保留 `p-1` 透明外留边，改为和磁贴态一致贴满窗口区域，仅保留便签主体自身的圆角、浅边框和纸面背景。
- 更新 `NotePad` 渲染测试，先复现小窗仍输出 `bg-transparent p-1` 的失败用例，再约束小窗输出 `bg-transparent p-0`，避免后续回退出额外外圈。

### 2026-04-28（R6：设置页）

- 主窗口右上角设置入口已接入右侧设置面板，不新开窗口。
- 设置页支持笔记目录选择、全局快捷键选择、关闭到托盘、开机自启和默认视图模式。
- 配置继续通过 Rust `config_get` / `config_save` 读写 `config.json`，前端新增 settings API 封装。
- 保存配置时会同步运行时开机自启和全局快捷键；修改笔记目录后会刷新笔记列表。
- 验证记录：`npm.cmd test -- --run` 通过 12 个测试文件 / 29 个测试；`npm.cmd run build` 通过；`cargo test` 通过 13 个 Rust 测试；`git diff --check` 无空白错误，仅有既有 CRLF 提示。

### 2026-04-28（R6：取消批量导入导出）

- 按用户明确决定，R6 不再做批量导入和批量导出，保留已完成的单文件导入和当前笔记导出。
- R6 后续继续推进设置页。

### 2026-04-28（R6：笔记列表右键菜单与 Markdown 导出）

- 笔记列表项新增右键菜单，当前包含“导出 Markdown”和“删除笔记”两项。
- 删除笔记现在可以直接从左侧笔记列表右键触发，操作目标为被右键点中的那条笔记。
- Markdown 导出改为从笔记列表右键触发，弹出保存文件对话框，默认文件名使用笔记标题；标题为空时使用 `无标题笔记.md`。
- 导出内容完全使用笔记正文原文，不自动补标题、不改写 Markdown。
- 如果导出的是当前选中且有未保存修改的笔记，会先保存再导出，避免导出旧内容。
- Rust 数据层新增 `notes_export_markdown` command 和 `NoteStore::export_markdown_file`。
- 验证记录：`npm.cmd test -- --run` 通过 9 个测试文件 / 21 个测试；`npm.cmd run build` 通过；`cargo test` 通过 12 个 Rust 测试。

### 2026-04-28（R6：单文件 Markdown 导入）

- 新增单个 `.md` 文件导入能力：主窗口侧栏可打开 Markdown 文件选择器，导入成功后刷新列表并选中新笔记。
- 导入标题规则：优先使用 Markdown 原文第一行的 `# 标题`；如果第一行不是一级标题，则使用源文件名作为标题。
- 导入正文规则：无论标题来自第一行 `# 标题` 还是文件名，导入后的笔记正文都完整保留源文件内容，不删除第一行标题。
- Rust 数据层新增 `notes_import_markdown` command 和 `NoteStore::import_markdown_file`，继续由 Rust 统一写入笔记目录和 metadata。
- 接入 Tauri dialog 插件，仅开放打开文件对话框权限，用于选择 `.md` 文件。
- 验证记录：`npm.cmd test -- --run` 通过 8 个测试文件 / 18 个测试；`npm.cmd run build` 通过；`cargo test` 通过 11 个 Rust 测试。

### 2026-04-28（R5 范围收敛：取消 WorkerW 与 Agent 侧边界验证）

- 按当前产品需求取消 WorkerW 桌面嵌入能力，磁贴继续保留置顶悬浮模式，不再进入桌面 WorkerW 层。
- Explorer 重启、多显示器、DPI 缩放等边界项改为用户侧手动验收，不作为 Agent 继续施工的 R5 阻塞项。
- R5 状态调整为“功能完成，用户侧验收”；后续开发主线可进入 R6 导入导出与设置页。

### 2026-04-28（磁贴切屏后双边框修复）

- 修复磁贴在切屏或窗口重绘后偶发出现“边框套边框”的问题：磁贴态不再使用便签小窗的 4px 透明外留边，改为贴满窗口，仅保留磁贴自身的圆角和淡边框。
- 便签编辑态继续保留透明外留边，用于配合 Windows 原生圆角窗口区域；本次只收敛磁贴态，避免影响小窗圆角。
- 更新 `NotePad` 渲染测试，明确约束磁贴态输出 `bg-transparent p-0`，并防止重新回退到 `bg-transparent p-1`。
- 验证记录：`npm.cmd test -- src/components/NotePad.test.tsx --run` 先失败后通过；`npm.cmd test -- --run` 通过 7 个测试文件 / 16 个测试；`npm.cmd run build` 通过；`cargo test` 通过 9 个 Rust 测试。

### 2026-04-28（磁贴视觉方案迁移到正式项目）

- 将 `D:\Agent\Agent_temp\tile-design\src\components\Tile.tsx` 直接迁移为正式项目 `src/components/Tile.tsx`，`NotePad` 磁贴态改为调用该组件。
- 磁贴卡片改为清新淡雅的 `bg-cloud` 表面、12px 圆角、极淡边框、轻阴影和悬停微缩放。
- 新增更深的青色磁贴变体，并将转为磁贴后的默认外观切换为该青色，以便和普通便签窗口区分。
- 修复磁贴圆角不可见的问题：磁贴态页面外层改为透明，便签/磁贴动态窗口创建时启用透明窗口。
- 新增四角纯装饰性 L 形角标（8px、stroke 0.8px、极淡灰色），用于框定内容区域。
- 移除磁贴态可见的置顶、关闭和缩放图标按钮，保留整体拖拽、右键转小窗和无视觉提示的右下角缩放热区。
- 正文排版同步为 13px、1.8 行高、墨色 75% 透明度；标题仅在存在时以 11px 极淡衬线字显示，空内容显示极简“空”状态。
- 更新 `Tile`、`NotePad` 与 `TileShowcase` 渲染测试，防止磁贴重新出现编辑控件、可见功能按钮或缺失四角装饰。

### 2026-04-28（磁贴视觉方案确认）

- 确认磁贴视觉设计方向：清新淡雅，不使用拟物或纸质风格。
- 四角使用纯装饰性 L 形角标（8×8px、stroke 0.8px、极淡配色），无顶部渐变线。
- 移除原设计稿中所有功能性元素（拖拽握柄、关闭圆点、缩放角标）。
- 保留四种色彩变体（default / warm / green / mist）、圆角卡片、悬停微交互。
- 设计稿位于 `D:\Agent\Agent_temp\tile-design\`，使用与主项目一致的 React + TypeScript + Tailwind 4 技术栈。

### 2026-04-28（磁贴交互：保持小窗尺寸与去双边框）

- 磁贴模式切换不再把当前窗口缩到 280x220，改为保持切换前的小窗位置和尺寸，只改变内部表面样式。
- Rust 创建磁贴窗口的默认尺寸同步恢复为 420x430，与快捷便签小窗一致。
- 磁贴外层去掉 `p-2` 绿色包边背景，内部去掉可见纸面边框，避免出现边框包边框；保留圆角、纸面背景和内容内边距。
- 更新 surface mode 与 `NotePad` 渲染测试，防止磁贴再次缩小或出现嵌套边框。

### 2026-04-28（磁贴交互：同窗变形与紧凑纸面）

- 将便签小窗和磁贴合并为同一个 `NotePad` 表面状态，支持 `pad` / `tile` 两种模式；`TileShowcase` 现在只作为磁贴初始态入口，不再维护另一套编辑窗口。
- 小窗“转为磁贴”改为保存当前笔记后直接切换当前窗口状态，并通过 Tauri window API 对当前窗口尺寸和位置做短动画；不再先打开磁贴窗口再关闭小窗。
- 磁贴右键“转为小窗”改为发送当前窗口内的 surface mode 事件，切回小窗模式；不再调用 `openNotepadWindow` 和 `closeCurrentWindow`。
- 磁贴恢复纸面外观：当时默认目标尺寸为 280x220，保留圆角、边框、纸面背景和内边距，空内容时仅显示“空”；后续已改为保持小窗尺寸并去掉嵌套边框。
- 新增窗口 surface mode 工具与测试，覆盖模式白名单、目标尺寸居中计算，以及小窗/磁贴两种渲染形态。

### 2026-04-28（磁贴交互：只读展示与右键转小窗）

- 磁贴窗口改为只读展示，移除标题输入框、正文编辑框、自动保存逻辑和底部“保存”按钮。
- 磁贴进一步精简为只显示正文内容；移除标题、日期、字数、状态栏和按钮 tooltip 文案，正文为空时仅显示“空”。
- 磁贴整体区域可拖动窗口，置顶、关闭和右下角缩放控件弱化为悬停显示。
- 小窗“转为磁贴”和磁贴右键“转为小窗”都会读取当前窗口位置与尺寸，并将目标窗口创建到相同位置和尺寸，降低跳窗感。
- 去除小窗保存按钮和自定义右键菜单中的投影效果；Rust 创建便签/磁贴窗口时关闭窗口阴影。
- 新增 `TileShowcase` 渲染测试和窗口 API 测试，防止磁贴重新出现编辑控件、元信息 UI 或转换时丢失 bounds。

### 2026-04-28（Tauri R5：系统托盘、快捷键与自启动接入）

- 新增 Rust `desktop` 模块，集中管理主窗口、便签窗口、磁贴窗口、托盘菜单、关闭到托盘、全局快捷键和自启动逻辑。
- 接入 Tauri tray API，托盘菜单支持打开主窗口、快速记录、切换关闭到托盘、切换开机自启动和退出。
- 主窗口关闭事件改为读取 `config.json` 中的 `closeToTray`：开启时隐藏窗口，托盘退出时才真正退出应用。
- 接入 `tauri-plugin-global-shortcut`，启动时根据 `globalShortcut` 注册快捷键，默认 `Ctrl+Space` 打开快捷便签。
- 接入 `tauri-plugin-autostart`，托盘菜单切换时同步系统开机自启动状态与 `config.json`。
- 新增 Rust 单元测试覆盖托盘菜单动作映射、托盘菜单配置状态、快捷键配置解析和动态窗口 label。

### 2026-04-28（UI 修正：无边框窗口与交互统一）

- 所有窗口移除系统标题栏（`decorations: false`），主窗口新增自定义标题栏，含拖拽、双击最大化/还原、自定义最小化/最大化/关闭按钮。
- 删除设计稿遗留的仿 Mac 红黄绿圆点导航栏。
- 去除主窗口、便签、磁贴内容的卡片式外层包裹（圆角、边框、间距），内容铺满整个窗口。
- 便签"钉为磁贴"改为"转为磁贴"：转换后便签窗口自动关闭，不再两窗并存。
- 磁贴操作栏从 Mac 风格彩色圆形按钮改为与应用一致的图标按钮。
- 新增全局自定义右键菜单组件（剪切/复制/粘贴/全选），替代浏览器默认右键菜单，风格与应用统一。
- 新增窗口控制 API：`minimizeCurrentWindow`、`toggleMaximizeCurrentWindow`、`isCurrentWindowMaximized`。
- 更新 Tauri capabilities，增加 `minimize`、`toggle-maximize`、`is-maximized`、`show`、`hide` 权限。

### 2026-04-28（R4 补丁：修复便签窗口打开卡死）

- 将 `open_notepad_window` 和 `open_tile_window` 从同步命令改为 `async fn`，避免 `WebviewWindowBuilder::build()` 在同步命令中向主线程派发窗口创建时，与 Windows WebView2 IPC 处理产生死锁。
- 移除 `build()` 之后的 `set_focus()` 调用，新创建的窗口默认获得焦点，在窗口尚未完全初始化时立即调用 `set_focus()` 可能导致阻塞。
- 验证：`cargo test` 4 通过、`npm test` 6 通过、`tauri dev` 手动测试便签和磁贴窗口正常打开不再卡死。

### 2026-04-28（Tauri R4 便签与磁贴窗口迁移）

- 新增窗口路由模块，统一解析和生成 `main`、`notepad`、`tile` 视图及 `noteId` 参数。
- 新增 Rust 动态窗口 commands：`open_notepad_window`、`open_tile_window`，使用 `notepad-*`、`tile-*` label 创建或聚焦窗口。
- 更新 Tauri capabilities，允许便签和磁贴窗口调用关闭、置顶、设置尺寸、设置位置、拖拽和缩放相关窗口 API。
- 将便签窗口接入真实笔记数据，支持新建、保存、打开已有笔记、钉为磁贴和关闭窗口。
- 将磁贴窗口接入真实笔记数据，支持编辑保存、自动保存、关闭、置顶、拖拽和右下角缩放。
- 修复中文产品名 MSI 打包问题，将 WiX 语言配置为 `zh-CN`。

### 2026-04-28（Tauri R3 主窗口真实数据接入）

- 新增前端 notes API 封装，统一调用 `notes_list`、`notes_get`、`notes_create`、`notes_update`、`notes_delete`。
- 将主窗口从 mock 数据切换到 Rust 数据核心，支持真实笔记列表、新建、选择、编辑、保存和删除。
- 新增保存状态和延迟自动保存策略，并保留手动保存入口。
- 新增 Markdown preview 生产组件，保留确认版设计稿的纸面与墨色视觉方向。
- 新增前端 notes utility 和 Vitest 测试，覆盖标题展示、预览生成、字数统计和搜索过滤。

### 2026-04-28（Tauri R2 Rust 数据核心）

- 新增 Rust 数据核心服务，统一管理笔记目录、`.md` 原文、`metadata.json` 和 `config.json`。
- 新增 `Note`、`NoteMetadata`、`AppConfig`、`SaveNoteRequest`、`AppError` 等模型和 DTO。
- 新增 Tauri commands：`notes_list`、`notes_get`、`notes_create`、`notes_update`、`notes_delete`、`config_get`、`config_save`。
- 实现 metadata 损坏恢复：损坏文件会改名为 `metadata.corrupt-*.json`，再从现有 `.md` 文件重建索引。
- 新增 Rust 单元测试覆盖笔记 CRUD、metadata 损坏重建、config 读写。

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
- 旧 WPF 版本中便签窗口生命周期和热键逻辑后续作为 Rust/Tauri 迁移参考；WorkerW 桌面嵌入当前不迁移。

### 2026-04-26（旧 WPF：基础与 Markdown）

- 旧 WPF 版本完成 Phase 1 基础骨架。
- 完成 Markdown 语法高亮、WebView2 预览、工具栏和相关预览问题修复。

---

## 七、下一步

1. 继续推进 **R7** 剩余项：视觉回归检查、异常处理和日志、清理旧 WPF 项目。
2. 继续保持每完成一个阶段就更新 `PLAN.md`、`PROGRESS.md`，提交并推送一次。

---

## 八、当前暂停记录

### 2026-04-28（磁贴视觉方案确认）

- 磁贴视觉方案已确认，设计稿位于 `D:\Agent\Agent_temp\tile-design\`。
- 设计风格：不使用拟物风格和纸质风格，贯彻清新淡雅的整体设计语言。
- 装饰元素：四角使用纯装饰性 L 形角标（8px，极淡），像印刷品对位标记一样框定内容区域；不使用顶部渐变线。
- 去掉的功能性元素：原设计稿中的拖拽握柄、关闭圆点、缩放三角点全部移除；用户默认知晓拖动、缩放和右键转小窗操作，不需要显式 UI 提示。
- 保留的特征：圆角卡片（12px）、四种色彩变体（default / warm / green / mist）、悬停阴影加深与微缩放、可选标题（11px 极淡衬线字）。
- 内容区仅显示用户正文；正文为空时显示极简的”空”状态。
- 设计稿使用与主项目一致的技术栈（React + TypeScript + Tailwind 4），组件可直接迁移。
- 之前三种被否决的方向（清新竹纸、素雅宣纸、清爽便签）因风格不贴合软件整体风格而弃用。
- 迁移状态：已迁移到正式项目 `src/花笺/` 中的 `TileShowcase` / `NotePad` 磁贴态。

### 2026-04-28（便签/磁贴圆角修复）

- 便签小窗和磁贴窗口的外层统一改为透明背景并保留 4px 内边距，便签编辑态主体恢复 `rounded-xl` 圆角与轻边框。
- `notepad-*` / `tile-*` 动态窗口在创建、复用和尺寸变化时都会重新应用 Windows 原生圆角窗口区域，避免只靠 CSS 圆角但系统窗口仍是矩形。
- 磁贴态继续使用更深的青色变体，用于和普通便签小窗区分。
- 验证记录：`npm.cmd test -- --run` 通过 7 个测试文件 / 16 个测试；`npm.cmd run build` 通过；`cargo test` 通过 9 个 Rust 测试；`git diff --check` 无空白错误，仅保留既有 CRLF 提示。
- 运行记录：已重启 Tauri dev，当前便签窗口由新构建的 `huajian.exe` 启动；Win32 `GetWindowRgn` 返回 `3`，确认窗口已应用原生非矩形区域。
