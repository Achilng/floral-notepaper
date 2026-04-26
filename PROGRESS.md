# 花笺 开发进度

## 当前阶段：Phase 2 — Markdown 支持

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

### 环境信息

- .NET SDK：8.0.420
- 目标框架：net8.0-windows
- 开发环境：Windows 11

### 已安装 NuGet 包

- CommunityToolkit.Mvvm 8.4.2
- AvalonEdit 6.3.1.120
- Hardcodet.NotifyIcon.Wpf 2.0.1
- Markdig 1.1.3（Phase 2 新增）

### 项目文件结构

```
src/花笺/
├── App.xaml / App.xaml.cs
├── Models/
│   ├── Note.cs
│   ├── NoteMetadataStore.cs
│   └── AppConfig.cs
├── Services/
│   ├── MetadataService.cs
│   ├── NoteService.cs
│   └── MarkdownService.cs          ← Phase 2 新增
├── ViewModels/
│   └── MainViewModel.cs            ← Phase 2 更新：编辑/预览切换 + 工具栏命令
├── Views/
│   └── MainWindow.xaml/.cs         ← Phase 2 更新：预览面板 + 工具栏 + 语法高亮加载
├── Helpers/
│   ├── AvalonEditBehavior.cs
│   └── NoteDisplayConverter.cs
└── Resources/
    ├── Styles.xaml
    └── MarkdownHighlighting.xshd   ← Phase 2 新增
```

---

## 变更日志

### 2026-04-26
- 完成 Phase 1 全部任务
- Phase 2 进度：
  - Markdown 语法高亮（.xshd 定义：标题/粗体/斜体/代码/链接/引用/列表）✅
  - 编辑/预览模式切换：从 IE WebBrowser 迁移至 WebView2（Edge Chromium），修复首次预览空白/未导航问题 ✅
  - 修复空笔记初次输入时 AvalonEdit 未同步到 ViewModel，导致预览一直显示“空内容”的问题 ✅
  - Markdown 工具栏（标题/粗体/斜体/代码/列表/引用/分割线，支持选中文本包裹）✅

## 踩坑记录

### Markdown 预览修复

- 旧版 WPF `WebBrowser` 走 IE 内核，Markdown 预览渲染能力和兼容性不足；本阶段改为 WebView2 + Markdig。
- WebView2 初始化是异步流程，不能假设点击“预览”时控件已经可导航；初始化和 `NavigateToString` 必须回到 UI Dispatcher，并等待布局进入可见状态后再执行。
- 预览导航不能只监听 `IsEditMode` 切换；`PreviewHtml` 生成也会触发属性变化，需要在预览模式下响应 `PreviewHtml` 更新，否则容易出现首次预览空白或旧内容。
- 空笔记场景暴露了 AvalonEdit 双向绑定问题：`BoundText` 附加属性默认值原本是 `string.Empty`，而空笔记初始内容也是 `string.Empty`，WPF 不触发属性变更回调，导致 `TextChanged` 事件没有挂上，用户输入无法同步到 `EditorContent`，预览始终拿到空字符串。
- 修复 AvalonEdit 绑定时，将 `BoundText` 默认值改为 `null`，并在 getter 中兜底为空字符串，确保空笔记第一次绑定也会完成初始化。
- 验证用例不能只覆盖“文件已有内容后切预览”；必须覆盖“新建空笔记 → 在编辑器输入 → 切到预览”的真实用户路径。前者会绕过 AvalonEdit 输入同步问题，容易误判修复完成。
