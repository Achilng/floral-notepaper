# 花笺 开发进度

## 当前阶段：Phase 4 — 系统集成

### Phase 1 任务清单

| 任务 | 状态 |
|------|------|
| 搭建项目结构（.NET 8 + WPF） | 已完成 |
| 实现 NoteService：笔记 CRUD | 已完成 |
| 实现 MetadataService：元数据索引 | 已完成 |
| 主窗口基础布局：左侧列表 + 右侧编辑器 | 已完成 |
| 集成 AvalonEdit 作为 Markdown 编辑器 | 已完成 |
| 笔记列表的增删操作 | 已完成 |

### Phase 2 任务清单

| 任务 | 状态 |
|------|------|
| AvalonEdit 的 Markdown 语法高亮 | 已完成 |
| Markdown 实时预览面板（编辑/预览切换） | 已完成 — 使用 WebView2 渲染 |
| 基础 Markdown 工具栏 | 已完成 |

### Phase 3 任务清单

| 任务 | 状态 |
|------|------|
| 实现 HotkeyService：全局热键注册与监听 | 已完成 — Ctrl+Space 注册 |
| NotePadWindow 基础：热键呼出、编辑、保存关闭 | 已完成 |
| 新建/打开模式切换 | 已完成 — 新建/打开标签切换 |
| 钉住功能：窗口从编辑态切换为磁贴态 | 已完成 — 右键菜单"贴在一旁"，实体纸面背景 |
| 磁贴态：可缩放、可拖拽的无边框窗口 | 已完成 — 点阵握柄拖拽移动，Thumb 拖拽调整大小 |
| 磁贴态：内容可编辑 | 已完成 — 始终保持 TextBox 可编辑 |
| 置顶悬浮模式 | 已完成 — Topmost 绑定 |
| 桌面嵌入模式（WorkerW） | 已完成 — 右键菜单切换"贴在桌面"/"切回置顶悬浮" |
| 多便签并存管理 | 已完成 — NotePadManager 管理列表 |
| 主窗口中对笔记执行"钉住"操作 | 已完成 — 工具栏钉住按钮 |

### 环境信息

- .NET SDK：8.0.420
- 目标框架：net8.0-windows
- 开发环境：Windows 11

### 已安装 NuGet 包

- CommunityToolkit.Mvvm 8.4.2
- AvalonEdit 6.3.1.120
- Hardcodet.NotifyIcon.Wpf 2.0.1
- Markdig 1.1.3（Phase 2 新增）
- Microsoft.Web.WebView2 1.0.3912.50（Phase 2 新增）

### 项目文件结构

```
src/花笺/
├── App.xaml / App.xaml.cs         ← Phase 3 更新：接入 HotkeyService + NotePadManager
├── Models/
│   ├── Note.cs
│   ├── NoteMetadataStore.cs
│   └── AppConfig.cs
├── Services/
│   ├── MetadataService.cs
│   ├── NoteService.cs
│   ├── MarkdownService.cs
│   ├── HotkeyService.cs          ← Phase 3 新增：Win32 全局热键
│   ├── NotePadManager.cs         ← Phase 3 新增：便签窗口管理
│   └── DesktopEmbedService.cs    ← Phase 3 新增：WorkerW 桌面嵌入
├── ViewModels/
│   ├── MainViewModel.cs          ← Phase 3 更新：QuickNote/PinNote 命令
│   └── NotePadViewModel.cs       ← Phase 3 新增
├── Views/
│   ├── MainWindow.xaml/.cs       ← Phase 3 更新：快速记录按钮 + 钉住按钮
│   └── NotePadWindow.xaml/.cs    ← Phase 3 新增：便签窗口
├── Helpers/
│   ├── AvalonEditBehavior.cs
│   ├── NoteDisplayConverter.cs
│   ├── WebBrowserHelper.cs
│   └── Win32Interop.cs           ← Phase 3 新增：P/Invoke 声明
└── Resources/
    ├── Styles.xaml
    └── MarkdownHighlighting.xshd
```

---

## 变更日志

### 2026-04-27
- 修复：便签小窗口/磁贴拖拽区域被标题输入框覆盖的问题；新增左侧点阵拖拽握柄，使用 `Thumb.DragDelta` 直接移动窗口。
- 修复：磁贴钉住态背景从半透明色改为实体纸面色，降低透底感。
- 优化：便签窗口圆角、边框、阴影、分隔线、正文选区色和字号细节，整体更接近设计稿的轻量纸面质感。
- 验证：结构性回归检查通过；结束残留的 `花笺.exe` 进程后，常规 `dotnet build --no-restore` 通过（0 警告，0 错误）。
- Phase 3 进度：
  - HotkeyService：通过 Win32 RegisterHotKey 注册 Ctrl+Space 全局热键，HwndSource 消息钩子监听 WM_HOTKEY ✅
  - NotePadWindow：无边框透明窗口（AllowsTransparency=True），圆角 + DropShadowEffect，标题输入框、关闭按钮（鼠标悬停显示）、拖拽移动、Thumb 缩放 ✅
  - 新建/打开模式切换：顶部"新建"|"打开"标签，打开模式显示已有笔记列表供选择 ✅
  - 钉住功能：右键菜单"贴在一旁"将窗口切换为磁贴态（Topmost + 半透明背景），右键"取消钉住"恢复 ✅
  - NotePadManager：管理所有便签窗口生命周期，支持多窗口并存，错开定位避免重叠 ✅
  - MainWindow 标题栏新增"快速记录 Ctrl+Space"按钮 ✅
  - MainWindow 编辑器工具栏新增"钉为磁贴"按钮 ✅
  - 自动保存：关联已有笔记的便签 800ms 防抖自动保存；未关联的便签关闭时自动创建笔记 ✅
  - 便签保存到笔记库后自动刷新主窗口笔记列表 ✅

### 2026-04-27（WorkerW 桌面嵌入）
- 新增 `DesktopEmbedService`：通过 `Progman` 触发 WorkerW 创建，枚举包含 `SHELLDLL_DefView` 的桌面窗口并找到相邻 `WorkerW`，使用 `SetParent` 将磁贴窗口嵌入桌面层。
- 新增磁贴模式切换：`NotePadViewModel.IsDesktopEmbedded` 和 `ToggleDesktopEmbedCommand` 管理"置顶悬浮 / 贴在桌面"状态；进入桌面嵌入时关闭 `Topmost`，切回悬浮时恢复置顶。
- 更新 `NotePadWindow` 右键菜单：磁贴态新增"贴在桌面"与"切回置顶悬浮"入口；窗口关闭或取消钉住时自动从桌面层还原。
- 验证：WorkerW 结构性回归检查先失败后通过；`dotnet build .\src\花笺\花笺.csproj --no-restore -o D:\Agent\Agent_temp\huajian-build-check-project` 通过（0 警告，0 错误）。
- 手动试用反馈：暂未发现明显问题；若后续出现桌面层级、多显示器或 Explorer 重启后的异常，归入 Phase 6 打磨项继续处理。

### 下一步
- Phase 3 计划项已经收口。下一轮进入 Phase 4：系统托盘图标、最小化/关闭到托盘、开机自启和 `config.json` 配置管理。
- 建议先做托盘常驻：接入 Hardcodet.NotifyIcon.Wpf，提供"快速记录"、"打开主窗口"、"退出"三个基础菜单项。
- 当前便签窗口已可用，但仍需要后续 UI/交互打磨；这些细节可放入 Phase 6 统一处理。

### 2026-04-26
- 完成 Phase 1 全部任务
- Phase 2 进度：
  - Markdown 语法高亮（.xshd 定义：标题/粗体/斜体/代码/链接/引用/列表）✅
  - 编辑/预览模式切换：从 IE WebBrowser 迁移至 WebView2（Edge Chromium），修复首次预览空白/未导航问题 ✅
  - 修复空笔记初次输入时 AvalonEdit 未同步到 ViewModel，导致预览一直显示"空内容"的问题 ✅
  - 修复普通换行在预览中被浏览器折叠，导致连续文本显示在同一行的问题 ✅
  - Markdown 工具栏（标题/粗体/斜体/代码/列表/引用/分割线，支持选中文本包裹）✅

## 踩坑记录

### Markdown 预览修复

- 旧版 WPF `WebBrowser` 走 IE 内核，Markdown 预览渲染能力和兼容性不足；本阶段改为 WebView2 + Markdig。
- WebView2 初始化是异步流程，不能假设点击"预览"时控件已经可导航；初始化和 `NavigateToString` 必须回到 UI Dispatcher，并等待布局进入可见状态后再执行。
- 预览导航不能只监听 `IsEditMode` 切换；`PreviewHtml` 生成也会触发属性变化，需要在预览模式下响应 `PreviewHtml` 更新，否则容易出现首次预览空白或旧内容。
- 空笔记场景暴露了 AvalonEdit 双向绑定问题：`BoundText` 附加属性默认值原本是 `string.Empty`，而空笔记初始内容也是 `string.Empty`，WPF 不触发属性变更回调，导致 `TextChanged` 事件没有挂上，用户输入无法同步到 `EditorContent`，预览始终拿到空字符串。
- 修复 AvalonEdit 绑定时，将 `BoundText` 默认值改为 `null`，并在 getter 中兜底为空字符串，确保空笔记第一次绑定也会完成初始化。
- 验证用例不能只覆盖"文件已有内容后切预览"；必须覆盖"新建空笔记 → 在编辑器输入 → 切到预览"的真实用户路径。前者会绕过 AvalonEdit 输入同步问题，容易误判修复完成。
- Markdig 默认遵循标准 Markdown 软换行语义：段落内普通换行不会生成 `<br>`，浏览器会把换行空白折叠成同一行。对笔记软件来说，用户在编辑器里按回车通常期待预览也换行，因此在 Markdown 管线中启用 `UseSoftlineBreakAsHardlineBreak()`，把软换行渲染为可见换行。
