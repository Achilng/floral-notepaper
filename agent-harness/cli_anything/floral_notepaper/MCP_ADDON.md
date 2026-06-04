# Floral Backend 与 MCP Add-on 使用教程

本教程介绍如何在 `floral-notepaper.exe` 和 `floral_cli_backend.exe` 位于同一个目录时，使用 Backend 直接操作 Floral，以及将 Backend 接入支持 MCP 的 AI 客户端。

## 组件关系

推荐的发布目录结构：

```text
Floral/
├── floral-notepaper.exe
└── floral_cli_backend.exe
```

两个程序的职责不同：

| 程序                     | 用途                                                  |
| ------------------------ | ----------------------------------------------------- |
| `floral-notepaper.exe`   | Floral 图形界面客户端，供用户正常编辑和管理笔记。     |
| `floral_cli_backend.exe` | 可选 Add-on，提供直接 JSON 调用协议和本地 stdio MCP。 |

两个 exe 放在同一个目录只是为了方便分发和配置，它们不会因为位于同一目录就自动连接。

- 正常启动 `floral-notepaper.exe` 时，不会自动启动 Backend。
- 直接调用 Backend 时，每条命令会启动一次 Backend，完成操作后退出。
- 使用 MCP 时，由 MCP 客户端启动并持续管理 Backend 进程。
- 删除 `floral_cli_backend.exe` 不会影响 Floral 图形界面的正常使用。

## 数据目录

Backend 与 Floral GUI 通过读取同一个 Floral 数据目录共享笔记，而不是通过两个 exe 之间的进程通信共享笔记。

数据根目录通常包含：

```text
花笺/
├── .floral-notepaper.lock
├── config.json
├── metadata.json
├── images/
└── notes/
```

Windows 默认数据根目录：

```text
%USERPROFILE%\Documents\花笺
```

在默认情况下，GUI、直接 JSON 调用和 MCP 都会使用这个目录。可以通过 Backend 查询它实际使用的默认目录：

```powershell
Set-Location D:\Floral
'{}' | .\floral_cli_backend.exe store.info
```

成功时会返回类似结果：

```json
{
  "ok": true,
  "data": {
    "baseDir": "C:\\Users\\User\\Documents\\花笺"
  }
}
```

### 使用独立数据目录

测试或开发时，建议使用一个独立数据目录，避免修改真实笔记。

直接 JSON 调用通过环境变量指定数据目录：

```powershell
$env:FLORAL_NOTEPAPER_DATA_DIR = "D:\FloralTestData"
'{}' | .\floral_cli_backend.exe store.info
```

取消当前 PowerShell 会话中的环境变量：

```powershell
Remove-Item Env:FLORAL_NOTEPAPER_DATA_DIR
```

MCP 模式通过 `--data-dir` 指定数据目录：

```powershell
.\floral_cli_backend.exe mcp --data-dir D:\FloralTestData
```

注意：

- `--data-dir` 只适用于 MCP 模式。
- 直接 JSON 调用需要使用 `FLORAL_NOTEPAPER_DATA_DIR`。
- 想让 AI 操作 GUI 中显示的真实笔记时，不要指定独立测试目录。

## 检查 Backend

打开 PowerShell，进入两个 exe 所在目录：

```powershell
Set-Location D:\Floral
```

查看版本：

```powershell
.\floral_cli_backend.exe --version
```

正常输出：

```text
floral_cli_backend 1.0.4 protocol 1
```

不带参数运行时，Backend 会返回 `missingOperation` 并退出：

```powershell
.\floral_cli_backend.exe
```

这是正常行为。无参数运行不会自动进入 MCP 模式。

## 使用直接 JSON 调用

直接 JSON 调用适合 PowerShell 脚本、自动化程序和不支持 MCP 的 AI 工具。

### 配置 PowerShell UTF-8 编码

Backend 的 JSON 协议使用 UTF-8。PowerShell 7 默认可以正确传递 UTF-8；Windows 自带的 PowerShell 5.1 默认使用 ASCII 管道编码，在传递中文标题、正文或分类前必须先执行：

```powershell
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
```

可以将这两行放在自动化脚本开头。否则，包含中文的请求可能返回：

```text
stream did not contain valid UTF-8
```

调用格式：

```powershell
'JSON 参数' | .\floral_cli_backend.exe 操作名称
```

成功响应统一使用：

```json
{
  "ok": true,
  "data": {}
}
```

失败响应统一使用：

```json
{
  "ok": false,
  "error": {
    "code": "错误代码",
    "message": "错误说明",
    "details": {}
  }
}
```

### 查询协议版本

```powershell
'{}' | .\floral_cli_backend.exe protocol.version
```

### 列出笔记

```powershell
'{}' | .\floral_cli_backend.exe notes.list
```

在 PowerShell 中读取返回结果：

```powershell
$response = '{}' | .\floral_cli_backend.exe notes.list | ConvertFrom-Json
$response.data | Select-Object id, title, category, updatedAt
```

### 搜索笔记

搜索会匹配标题、预览和 Markdown 正文。

```powershell
$inputJson = @{
    query = "项目计划"
    offset = 0
    limit = 50
} | ConvertTo-Json

$inputJson | .\floral_cli_backend.exe notes.search
```

搜索结果默认最多返回 50 条，单次请求最多返回 200 条。

按分类搜索：

```powershell
$inputJson = @{
    query = "会议"
    category = "工作"
    offset = 0
    limit = 50
} | ConvertTo-Json

$inputJson | .\floral_cli_backend.exe notes.search
```

### 读取完整笔记

```powershell
$inputJson = @{
    id = "NOTE_ID"
} | ConvertTo-Json

$inputJson | .\floral_cli_backend.exe notes.get
```

### 创建笔记

```powershell
$inputJson = @{
    title = "Backend 测试笔记"
    content = "# 标题`n`n这是由 Backend 创建的笔记。"
    category = ""
} | ConvertTo-Json

$inputJson | .\floral_cli_backend.exe notes.create
```

创建到指定分类：

```powershell
$inputJson = @{
    title = "项目记录"
    content = "项目正文"
    category = "工作"
} | ConvertTo-Json

$inputJson | .\floral_cli_backend.exe notes.create
```

### 安全更新笔记

面向 AI 或自动化程序时，推荐使用 `notes.updateIfUnchanged`，避免覆盖用户在 GUI 中刚刚修改的内容。

先读取笔记并保存当前 `updatedAt`：

```powershell
$noteId = "NOTE_ID"
$getInput = @{ id = $noteId } | ConvertTo-Json
$note = ($getInput | .\floral_cli_backend.exe notes.get | ConvertFrom-Json).data
```

携带读取时获得的 `updatedAt` 更新：

```powershell
$updateInput = @{
    id = $note.id
    expectedUpdatedAt = $note.updatedAt
    request = @{
        title = $note.title
        content = "$($note.content)`n`n新增内容"
        category = $note.category
    }
} | ConvertTo-Json -Depth 4

$updateInput | .\floral_cli_backend.exe notes.updateIfUnchanged
```

如果笔记在读取后已被 GUI 或其他程序修改，Backend 会返回：

```json
{
  "ok": false,
  "error": {
    "code": "noteConflict",
    "message": "The note changed after it was read. Read the latest version before updating it.",
    "details": {
      "noteId": "NOTE_ID",
      "currentUpdatedAt": "..."
    }
  }
}
```

收到 `noteConflict` 后，应重新读取笔记，比较内容，再决定是否更新。

### 安全移动笔记

```powershell
$noteId = "NOTE_ID"
$getInput = @{ id = $noteId } | ConvertTo-Json
$note = ($getInput | .\floral_cli_backend.exe notes.get | ConvertFrom-Json).data

$moveInput = @{
    id = $note.id
    expectedUpdatedAt = $note.updatedAt
    category = "归档"
} | ConvertTo-Json

$moveInput | .\floral_cli_backend.exe notes.moveIfUnchanged
```

### 分类操作

列出分类：

```powershell
'{}' | .\floral_cli_backend.exe categories.list
```

创建分类：

```powershell
'{"name":"工作"}' | .\floral_cli_backend.exe categories.create
```

### 旧协议中的高权限操作

为了兼容现有 CLI 和旧脚本，直接 JSON 协议还保留以下操作：

- `notes.update`
- `notes.move`
- `notes.delete`
- `notes.import`
- `notes.export`
- `categories.rename`
- `categories.delete`
- `config.get`
- `config.patch`

这些操作不属于 MCP 安全工具集合。尤其是无版本校验的更新、删除、配置修改和任意文件导入导出，不建议直接提供给 AI。

## 使用 MCP

MCP 模式让支持 MCP 的 AI 客户端通过标准协议调用 Backend。

MCP Add-on 的工作流程：

```text
用户打开 AI 客户端
  -> AI 客户端启动 floral_cli_backend.exe mcp
  -> Backend 通过 stdin/stdout 与 AI 客户端交换 MCP 消息
  -> Backend 直接读写 Floral 数据目录
  -> Floral GUI 重新读取数据后显示变化
```

### 不要手动常驻启动 MCP

在 PowerShell 中运行：

```powershell
.\floral_cli_backend.exe mcp
```

命令会一直等待且没有普通输出，这是正常行为，因为 Backend 正在等待 MCP 客户端通过 stdio 发送协议消息。

通常不需要手动执行这条命令。应将它配置到 MCP 客户端中，由客户端负责启动和关闭。

按 `Ctrl+C` 可以终止手动启动的 MCP 进程。

### 通用 MCP 配置

假设两个 exe 位于：

```text
D:\Floral\
```

通用配置：

```json
{
  "mcpServers": {
    "floral": {
      "command": "D:\\Floral\\floral_cli_backend.exe",
      "args": ["mcp"]
    }
  }
}
```

必须使用 `floral_cli_backend.exe` 的绝对路径。MCP 配置通常不会根据 `floral-notepaper.exe` 的位置自动寻找 Backend。

使用独立数据目录：

```json
{
  "mcpServers": {
    "floral-test": {
      "command": "D:\\Floral\\floral_cli_backend.exe",
      "args": ["mcp", "--data-dir", "D:\\FloralTestData"]
    }
  }
}
```

配置完成后，完全退出并重新启动 MCP 客户端。

### 验证 MCP 是否连接成功

连接成功后，MCP 客户端应能发现以下八个 Tools：

- `floral_notes_list`
- `floral_notes_search`
- `floral_notes_get`
- `floral_notes_create`
- `floral_notes_update`
- `floral_notes_move`
- `floral_categories_list`
- `floral_categories_create`

可以向 AI 客户端发送：

```text
请列出我最近的 10 条 Floral 笔记，只显示标题、分类和更新时间。
```

AI 应调用 `floral_notes_list`，而不是要求用户手动提供笔记内容。

## MCP Tools 参数

### `floral_notes_list`

列出笔记目录，不返回完整正文。

| 参数       | 是否必填 | 说明                              |
| ---------- | -------- | --------------------------------- |
| `category` | 否       | 只列出指定分类。                  |
| `offset`   | 否       | 分页起点，默认 `0`。              |
| `limit`    | 否       | 返回数量，默认 `50`，最大 `200`。 |

对话示例：

```text
列出“工作”分类下最近的 20 条 Floral 笔记。
```

### `floral_notes_search`

搜索标题、预览和完整正文，并返回匹配摘要。

| 参数       | 是否必填 | 说明                              |
| ---------- | -------- | --------------------------------- |
| `query`    | 是       | 搜索关键词。                      |
| `category` | 否       | 只搜索指定分类。                  |
| `offset`   | 否       | 分页起点，默认 `0`。              |
| `limit`    | 否       | 返回数量，默认 `50`，最大 `200`。 |

对话示例：

```text
在 Floral 中搜索提到“WebDAV”的笔记，并总结搜索结果。
```

### `floral_notes_get`

读取一篇完整笔记。

| 参数 | 是否必填 | 说明      |
| ---- | -------- | --------- |
| `id` | 是       | 笔记 ID。 |

### `floral_notes_create`

创建笔记。

| 参数       | 是否必填 | 说明                      |
| ---------- | -------- | ------------------------- |
| `title`    | 是       | 笔记标题。                |
| `content`  | 否       | Markdown 正文，默认为空。 |
| `category` | 否       | 分类名称，默认为未分类。  |

对话示例：

```text
把我们刚才讨论的部署步骤整理成 Markdown。在创建 Floral 笔记前先让我确认标题和正文。
```

### `floral_notes_update`

安全更新笔记。必须提供完整的新标题、正文、分类，以及读取笔记时获得的 `updatedAt`。

| 参数                | 是否必填 | 说明                               |
| ------------------- | -------- | ---------------------------------- |
| `id`                | 是       | 笔记 ID。                          |
| `expectedUpdatedAt` | 是       | 读取笔记时获得的 `updatedAt`。     |
| `title`             | 是       | 更新后的完整标题。                 |
| `content`           | 是       | 更新后的完整正文。                 |
| `category`          | 否       | 更新后的分类；省略时会变为未分类。 |

版本不匹配时返回 `noteConflict`，不会覆盖新内容。

虽然 `category` 在协议层是可选参数，但更新已有分类笔记时应始终传入原分类或目标分类。省略该参数会把笔记移动到未分类。

### `floral_notes_move`

安全移动笔记到其他分类。

| 参数                | 是否必填 | 说明                           |
| ------------------- | -------- | ------------------------------ |
| `id`                | 是       | 笔记 ID。                      |
| `expectedUpdatedAt` | 是       | 读取笔记时获得的 `updatedAt`。 |
| `category`          | 否       | 目标分类，空字符串表示未分类。 |

### `floral_categories_list`

列出所有分类，无参数。

### `floral_categories_create`

创建分类。

| 参数   | 是否必填 | 说明       |
| ------ | -------- | ---------- |
| `name` | 是       | 分类名称。 |

## MCP Resources

Resources 适合让 MCP 客户端直接读取 Floral 中的结构化信息。

| URI                   | 内容                                      |
| --------------------- | ----------------------------------------- |
| `floral://notes`      | 最近 200 条笔记目录，包含总数和是否截断。 |
| `floral://categories` | 分类目录。                                |
| `floral://notes/{id}` | 指定 ID 的完整笔记。                      |

`floral://notes` 最多返回最近 200 条笔记。如果 `truncated=true`，应使用 `floral_notes_list` 分页读取更多笔记。

## MCP Prompts

Prompts 为 AI 客户端提供预设工作流，本身不会修改数据。

### `summarize_note`

参数：

- `noteId`：必填，笔记 ID。
- `detailLevel`：可选，例如 `concise` 或 `detailed`。

用途：读取并总结指定笔记。

### `organize_note`

参数：

- `noteId`：必填，笔记 ID。
- `goal`：可选，例如“整理为项目计划”。

用途：生成结构优化建议。Prompt 会提醒 AI 在调用更新工具前取得用户确认。

### `create_note_from_conversation`

参数：

- `title`：可选，建议标题。
- `category`：可选，建议分类。

用途：把当前对话整理为待确认的 Floral 笔记。

## MCP 安全边界

MCP 只公开有限的安全工具，不公开：

- 删除笔记
- 分类重命名和删除
- Floral 配置修改
- 文件导入和导出
- 任意路径读写
- 网络服务和远程访问

写入工具包括：

- `floral_notes_create`
- `floral_notes_update`
- `floral_notes_move`
- `floral_categories_create`

MCP 客户端应在调用这些工具前向用户确认。

## GUI、Backend 与 MCP 同时运行

GUI、直接 JSON 调用和 MCP Add-on 可以同时访问同一个 Floral 数据目录。

所有 NoteStore 操作会使用数据根目录中的：

```text
.floral-notepaper.lock
```

跨进程锁等待上限为 5 秒。超时后返回：

```text
storeBusy
```

遇到 `storeBusy` 时，等待片刻后重试即可。

更新和移动工具还会使用 `updatedAt` 做版本检查。即使其他进程在锁释放后修改了笔记，旧版本写入也会收到 `noteConflict`，不会覆盖较新的内容。

## 推荐使用方式

### 普通用户

只运行：

```powershell
.\floral-notepaper.exe
```

不需要配置或启动 Backend。

### PowerShell 自动化

通过直接 JSON 协议调用：

```powershell
'{}' | .\floral_cli_backend.exe notes.list
```

优先使用：

- `notes.search`
- `notes.updateIfUnchanged`
- `notes.moveIfUnchanged`

### AI 客户端

在 MCP 客户端中配置：

```json
{
  "command": "D:\\Floral\\floral_cli_backend.exe",
  "args": ["mcp"]
}
```

不需要提前启动 Backend，也不需要保持 PowerShell 窗口打开。

## 常见问题

### 两个 exe 在同一个目录，为什么 AI 看不到笔记？

同目录不会自动启用 MCP。必须在 AI 客户端中配置 `floral_cli_backend.exe mcp`，并重启 AI 客户端。

### MCP 启动后没有窗口，也没有输出

这是正常行为。MCP 使用 stdin/stdout 协议通信，不提供图形界面，也不会在 stdout 输出普通日志。

### AI 读取到的笔记与 GUI 中不同

通常是 GUI 和 MCP 使用了不同的数据目录。

检查 Backend 默认数据目录：

```powershell
'{}' | .\floral_cli_backend.exe store.info
```

检查 MCP 配置中是否存在 `--data-dir`。如果希望操作真实 Floral 笔记，应移除测试用的 `--data-dir`。

### 配置相对路径后无法启动

请在 MCP 配置中使用 Backend 的绝对路径：

```json
"command": "D:\\Floral\\floral_cli_backend.exe"
```

### 返回 `noteConflict`

笔记在 AI 读取后被其他进程修改。让 AI 重新读取笔记并重新生成修改建议，不要绕过版本检查。

### 返回 `storeBusy`

其他 Floral 进程正在操作数据目录。等待几秒后重试。

### 返回 `noteNotFound`

笔记可能已被删除，或者 MCP 连接到了错误的数据目录。

### 返回 `missingOperation`

Backend 被无参数启动。直接调用时需要提供操作名称；MCP 客户端配置中需要添加 `"args": ["mcp"]`。

### PowerShell 中的中文内容出现乱码

推荐使用 PowerShell 7。使用 Windows PowerShell 5.1 时，在调用 Backend 前设置 UTF-8 管道编码：

```powershell
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
```

### Floral GUI 必须保持运行吗？

不需要。Backend 可以直接读写 Floral 数据目录。

GUI 未运行时，MCP 和直接 JSON 调用仍然可以操作笔记。下次启动 GUI 时会读取更新后的数据。

### Backend 是否监听网络端口？

不会。当前 MCP Add-on 只支持本机 stdio，不监听 HTTP 或其他网络端口，也不需要 Token。

## 构建 Backend

在仓库根目录执行：

```powershell
cargo build --release `
  --manifest-path src-tauri/Cargo.toml `
  --bin floral_cli_backend `
  --features floral-ai-addon
```

生成文件：

```text
src-tauri\target\release\floral_cli_backend.exe
```

将它与 `floral-notepaper.exe` 放入同一个发布目录即可。普通 Floral 安装包不依赖 Backend，Backend 也不需要 Python、Node.js 或 Rust 环境才能运行。
