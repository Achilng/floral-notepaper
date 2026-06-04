# Floral Notepaper CLI 命令手册

本文档列出 `cli-anything-floral-notepaper` 当前支持的全部命令和参数。

## 基本用法

```powershell
cli-anything-floral-notepaper [全局参数] 命令组 子命令 [参数]
```

查看帮助：

```powershell
cli-anything-floral-notepaper --help
cli-anything-floral-notepaper notes --help
cli-anything-floral-notepaper notes create --help
```

不带任何子命令运行时，会进入交互式 REPL：

```powershell
cli-anything-floral-notepaper
```

在 REPL 中输入 `help` 查看帮助，输入 `exit` 或 `quit` 退出。

## 全局参数

全局参数必须放在 `notes`、`categories`、`config` 或 `session` 命令组之前。

| 参数              | 说明                                                                                      |
| ----------------- | ----------------------------------------------------------------------------------------- |
| `--data-dir PATH` | 指定 Floral 数据根目录。该目录包含 `config.json`、`metadata.json` 和默认的 `notes` 目录。 |
| `--json`          | 使用适合程序读取的 JSON 格式输出。                                                        |
| `--help`          | 显示帮助信息。                                                                            |

例如：

```powershell
cli-anything-floral-notepaper --data-dir D:\temp\floral-test --json notes list
```

不指定 `--data-dir` 时，CLI 默认操作 Floral Notepaper 的真实数据目录：

```text
Windows: %USERPROFILE%\Documents\花笺
macOS:   ~/Library/Application Support/花笺
其他:    当前工作目录下的 data
```

在测试和自动化脚本中，建议始终指定一个独立的 `--data-dir`。

## 笔记命令

### 列出笔记

```powershell
cli-anything-floral-notepaper notes list [--category TEXT] [--query TEXT]
```

| 参数              | 说明                                                   |
| ----------------- | ------------------------------------------------------ |
| `--category TEXT` | 仅列出指定分类中的笔记。使用空字符串可筛选未分类笔记。 |
| `--query TEXT`    | 在笔记标题和预览中搜索文本，不区分英文大小写。         |

示例：

```powershell
cli-anything-floral-notepaper --json notes list
cli-anything-floral-notepaper --json notes list --category "工作"
cli-anything-floral-notepaper --json notes list --query "会议"
```

列表结果中的 `id` 是其他笔记命令需要使用的 `NOTE_ID`。

### 读取笔记

```powershell
cli-anything-floral-notepaper notes get NOTE_ID
```

示例：

```powershell
cli-anything-floral-notepaper --json notes get 12345678-1234-1234-1234-123456789abc
```

### 创建笔记

```powershell
cli-anything-floral-notepaper notes create --title TEXT [--content TEXT] [--content-file FILE] [--category TEXT]
```

| 参数                  | 说明                                       |
| --------------------- | ------------------------------------------ |
| `--title TEXT`        | 笔记标题，必填。                           |
| `--content TEXT`      | 直接传入笔记内容。                         |
| `--content-file FILE` | 从 UTF-8 文件读取笔记内容。                |
| `--category TEXT`     | 创建到指定分类中，省略时创建为未分类笔记。 |

`--content` 和 `--content-file` 不能同时使用。

示例：

```powershell
cli-anything-floral-notepaper notes create --title "购物清单" --content "- 牛奶`n- 面包"
cli-anything-floral-notepaper notes create --title "项目说明" --content-file README.md --category "工作"
```

### 更新笔记

```powershell
cli-anything-floral-notepaper notes update NOTE_ID [--title TEXT] [--content TEXT] [--content-file FILE] [--category TEXT]
```

只修改明确传入的字段，未传入的标题、内容和分类会保持不变。

示例：

```powershell
cli-anything-floral-notepaper notes update NOTE_ID --title "新标题"
cli-anything-floral-notepaper notes update NOTE_ID --content-file updated.md
cli-anything-floral-notepaper notes update NOTE_ID --category "归档"
```

### 删除笔记

```powershell
cli-anything-floral-notepaper notes delete NOTE_ID [--yes]
```

| 参数    | 说明                                 |
| ------- | ------------------------------------ |
| `--yes` | 跳过交互式删除确认，适合自动化脚本。 |

示例：

```powershell
cli-anything-floral-notepaper notes delete NOTE_ID
cli-anything-floral-notepaper notes delete NOTE_ID --yes
```

### 移动笔记

```powershell
cli-anything-floral-notepaper notes move NOTE_ID --category TEXT
```

移动到未分类：

```powershell
cli-anything-floral-notepaper notes move NOTE_ID --category ""
```

移动到指定分类：

```powershell
cli-anything-floral-notepaper notes move NOTE_ID --category "工作"
```

目标分类不存在时，后端会自动创建对应目录。

### 导入 Markdown

```powershell
cli-anything-floral-notepaper notes import PATH [--category TEXT]
```

示例：

```powershell
cli-anything-floral-notepaper notes import D:\docs\meeting.md
cli-anything-floral-notepaper notes import D:\docs\meeting.md --category "会议"
```

### 导出 Markdown

```powershell
cli-anything-floral-notepaper notes export NOTE_ID PATH
```

示例：

```powershell
cli-anything-floral-notepaper notes export NOTE_ID D:\backup\note.md
```

## 分类命令

### 列出分类

```powershell
cli-anything-floral-notepaper categories list
```

### 创建分类

```powershell
cli-anything-floral-notepaper categories create NAME
```

示例：

```powershell
cli-anything-floral-notepaper categories create "工作"
```

### 重命名分类

```powershell
cli-anything-floral-notepaper categories rename OLD_NAME NEW_NAME
```

示例：

```powershell
cli-anything-floral-notepaper categories rename "工作" "项目"
```

分类内笔记的元数据会一并更新。

### 删除分类

```powershell
cli-anything-floral-notepaper categories delete NAME [--yes]
```

删除分类时，分类内的笔记会移动到未分类区域。

示例：

```powershell
cli-anything-floral-notepaper categories delete "临时"
cli-anything-floral-notepaper categories delete "临时" --yes
```

## 配置命令

### 查看配置

```powershell
cli-anything-floral-notepaper config show
```

推荐使用 JSON 输出：

```powershell
cli-anything-floral-notepaper --json config show
```

### 修改配置

```powershell
cli-anything-floral-notepaper config patch PATCH
```

`PATCH` 可以是 JSON 对象，也可以是包含 JSON 对象的文件路径。只会覆盖传入的顶层字段。

直接传入 JSON：

```powershell
cli-anything-floral-notepaper config patch '{"theme":"dark","fontSize":16}'
```

从文件读取：

```powershell
cli-anything-floral-notepaper config patch D:\config\floral-patch.json
```

PowerShell 中也可以先保存 JSON，再传入命令：

```powershell
$patch = '{"theme":"dark","fontSize":16}'
cli-anything-floral-notepaper config patch $patch
```

当前可修改的配置字段：

| 字段                       | 类型        | 说明                                     |
| -------------------------- | ----------- | ---------------------------------------- |
| `locale`                   | string      | 界面语言。                               |
| `notesDir`                 | string      | 实际笔记文件目录。保存时会进行安全检查。 |
| `globalShortcut`           | string      | 全局快捷键。                             |
| `closeToTray`              | boolean     | 关闭窗口时最小化到托盘。                 |
| `autostart`                | boolean     | 开机启动。                               |
| `defaultViewMode`          | string      | 默认视图模式。                           |
| `noteAutoSave`             | boolean     | 主窗口笔记自动保存。                     |
| `noteSurfaceAutoSave`      | boolean     | 独立笔记窗口自动保存。                   |
| `tileColor`                | string      | 磁贴颜色。                               |
| `tileColorMode`            | string      | 磁贴颜色模式。                           |
| `theme`                    | string      | 主题。                                   |
| `fontSize`                 | number      | 主窗口字体大小。                         |
| `surfaceFontSize`          | number      | 独立笔记窗口字体大小。                   |
| `tabIndentSize`            | number      | Tab 缩进宽度，保存时限制在 1 到 8。      |
| `externalFileAutoSave`     | boolean     | 外部文件自动保存。                       |
| `backgroundImagePath`      | string      | 背景图片路径。                           |
| `backgroundFit`            | string      | 背景图片适配方式。                       |
| `backgroundDim`            | number      | 背景图片暗度。                           |
| `backgroundBlur`           | number      | 背景图片模糊程度。                       |
| `backgroundScale`          | number      | 背景图片缩放。                           |
| `backgroundPositionX`      | number      | 背景图片横向位置。                       |
| `backgroundPositionY`      | number      | 背景图片纵向位置。                       |
| `rememberSurfaceSize`      | boolean     | 记住独立笔记窗口尺寸。                   |
| `tileCtrlClose`            | boolean     | 使用 Ctrl 操作磁贴关闭行为。             |
| `tileRenderMarkdown`       | boolean     | 磁贴渲染 Markdown。                      |
| `renderHtmlMarkdown`       | boolean     | Markdown 中渲染 HTML。                   |
| `surfaceWidth`             | number/null | 独立笔记窗口宽度。                       |
| `surfaceHeight`            | number/null | 独立笔记窗口高度。                       |
| `toggleVisibilityShortcut` | string      | 切换窗口可见性的快捷键。                 |
| `openAtCursor`             | boolean     | 在鼠标位置打开窗口。                     |

## 会话历史命令

CLI 在执行成功的修改操作前保存数据目录快照，可以撤销或重做由当前 CLI 执行的修改。

### 查看历史状态

```powershell
cli-anything-floral-notepaper session status
```

输出当前数据目录以及可撤销、可重做的操作数量。

### 撤销

```powershell
cli-anything-floral-notepaper session undo
```

### 重做

```powershell
cli-anything-floral-notepaper session redo
```

会话历史按数据目录隔离，默认保存在：

```text
~/.cli-anything/floral-notepaper/
```

可以通过环境变量指定其他历史目录：

```powershell
$env:CLI_ANYTHING_SESSION_DIR = "D:\floral-cli-history"
```

注意：撤销和重做使用整个 Floral 数据目录的快照。不要同时使用 GUI 和 CLI 修改同一数据目录。

## JSON 输出与自动化

将 `--json` 放在命令组之前：

```powershell
cli-anything-floral-notepaper --json notes list
```

PowerShell 中读取笔记列表：

```powershell
$notes = cli-anything-floral-notepaper --json notes list | ConvertFrom-Json
$notes | Select-Object id, title, category
```

通过标题查找并更新笔记：

```powershell
$notes = cli-anything-floral-notepaper --json notes list | ConvertFrom-Json
$note = $notes | Where-Object title -eq "每日计划" | Select-Object -First 1

if ($note) {
    cli-anything-floral-notepaper notes update $note.id --content-file D:\notes\today.md
}
```

使用独立测试目录完成一次完整流程：

```powershell
$dataDir = "$env:TEMP\floral-cli-test"

$note = cli-anything-floral-notepaper --data-dir $dataDir --json notes create `
    --title "测试笔记" `
    --content "第一版内容" | ConvertFrom-Json

cli-anything-floral-notepaper --data-dir $dataDir categories create "测试分类"
cli-anything-floral-notepaper --data-dir $dataDir notes move $note.id --category "测试分类"
cli-anything-floral-notepaper --data-dir $dataDir --json notes get $note.id
cli-anything-floral-notepaper --data-dir $dataDir session undo
```

## Backend 环境变量

CLI 默认查找开发版本的 Rust backend：

```text
src-tauri/target/debug/floral_cli_backend.exe
```

使用 release backend 或已部署的 backend 时，通过 `FLORAL_CLI_BACKEND` 指定完整路径：

```powershell
$env:FLORAL_CLI_BACKEND = "D:\Floral\floral_cli_backend.exe"
cli-anything-floral-notepaper --json notes list
```

backend 每次运行时会收到 CLI 指定的数据根目录。通常不需要手动设置 `FLORAL_NOTEPAPER_DATA_DIR`。

## 完整命令索引

```text
cli-anything-floral-notepaper
├── notes
│   ├── list
│   ├── get NOTE_ID
│   ├── create
│   ├── update NOTE_ID
│   ├── delete NOTE_ID
│   ├── move NOTE_ID
│   ├── import PATH
│   └── export NOTE_ID PATH
├── categories
│   ├── list
│   ├── create NAME
│   ├── rename OLD_NAME NEW_NAME
│   └── delete NAME
├── config
│   ├── show
│   └── patch PATCH
└── session
    ├── status
    ├── undo
    └── redo
```
