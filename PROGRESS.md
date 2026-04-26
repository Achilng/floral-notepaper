# 花笺 开发进度

## 当前阶段：Phase 1 — 基础骨架

### Phase 1 任务清单

| 任务 | 状态 | 备注 |
|------|------|------|
| 搭建项目结构（.NET 8 + WPF） | 已完成 | 解决方案 + 项目 + 目录结构 |
| 实现 NoteService：笔记 CRUD | 已完成 | 创建、读取、保存、删除、改名 |
| 实现 MetadataService：元数据索引 | 已完成 | JSON 格式元数据读写 |
| 主窗口基础布局：左侧列表 + 右侧编辑器 | 已完成 | 参照设计稿配色方案 |
| 集成 AvalonEdit 作为 Markdown 编辑器 | 已完成 | 含双向绑定辅助类 |
| 笔记列表的增删操作 | 已完成 | 新建 + 删除确认 |

### 环境信息

- .NET SDK：8.0.420
- 目标框架：net8.0-windows
- 开发环境：Windows 11

### 已安装 NuGet 包

- CommunityToolkit.Mvvm 8.4.2（MVVM 框架）
- AvalonEdit 6.3.1.120（编辑器控件）
- Hardcodet.NotifyIcon.Wpf 2.0.1（系统托盘，Phase 4 使用）

### 项目文件结构

```
src/花笺/
├── App.xaml / App.xaml.cs          # 应用入口，服务初始化
├── Models/
│   ├── Note.cs                     # 笔记数据模型
│   ├── NoteMetadataStore.cs        # 元数据容器
│   └── AppConfig.cs                # 应用配置模型
├── Services/
│   ├── MetadataService.cs          # metadata.json 读写
│   └── NoteService.cs              # 笔记文件 CRUD
├── ViewModels/
│   └── MainViewModel.cs            # 主窗口 ViewModel
├── Views/
│   └── MainWindow.xaml/.cs         # 主窗口界面
├── Helpers/
│   ├── AvalonEditBehavior.cs       # AvalonEdit 双向绑定
│   └── NoteDisplayConverter.cs     # 显示转换器集合
└── Resources/
    └── Styles.xaml                 # 全局样式和配色
```

---

## 变更日志

### 2026-04-26
- 创建 PROGRESS.md
- 安装 .NET 8 SDK (8.0.420)
- 完成 Phase 1 全部任务：
  - 搭建完整项目结构
  - 实现 NoteService + MetadataService
  - 主窗口布局（侧边栏 + 编辑器 + 状态栏）
  - AvalonEdit 集成（含绑定辅助）
  - 笔记增删功能（含删除确认）
  - 自动保存（800ms 防抖）
  - 字数统计（中英文）
  - 参照设计稿实现配色方案
