# Floral MCP Add-on 接入说明

`floral_cli_backend.exe` 可以作为本地 stdio MCP Server 使用，让支持 MCP 的 AI 客户端在用户确认后读取和管理 Floral 笔记。Add-on 是可选组件，普通 Floral GUI 不依赖它，也不会将它打入安装包。

## 构建

在仓库根目录执行：

```powershell
cargo build --release `
  --manifest-path src-tauri/Cargo.toml `
  --bin floral_cli_backend `
  --features floral-ai-addon
```

生成文件：

```text
src-tauri/target/release/floral_cli_backend.exe
```

## 启动

使用 Floral 正常数据目录：

```powershell
floral_cli_backend.exe mcp
```

使用指定数据目录：

```powershell
floral_cli_backend.exe mcp --data-dir D:\FloralData
```

MCP 使用 stdio 通信。不要在终端中直接运行后等待交互，应由 MCP 客户端负责启动进程并交换协议消息。

## 客户端配置示例

不同 MCP 客户端的配置文件位置不同，但启动配置通常类似：

```json
{
  "mcpServers": {
    "floral": {
      "command": "D:\\FloralAddon\\floral_cli_backend.exe",
      "args": ["mcp"]
    }
  }
}
```

使用独立数据目录时：

```json
{
  "mcpServers": {
    "floral": {
      "command": "D:\\FloralAddon\\floral_cli_backend.exe",
      "args": ["mcp", "--data-dir", "D:\\FloralData"]
    }
  }
}
```

## 可用能力

MCP Tools：

- `floral_notes_list`
- `floral_notes_search`
- `floral_notes_get`
- `floral_notes_create`
- `floral_notes_update`
- `floral_notes_move`
- `floral_categories_list`
- `floral_categories_create`

Resources：

- `floral://notes`
- `floral://categories`
- `floral://notes/{id}`

Prompts：

- `summarize_note`
- `organize_note`
- `create_note_from_conversation`

MCP 不提供删除笔记、分类重命名、配置修改、文件导入导出和任意路径操作。

## 并发与安全

- GUI、CLI 和 MCP Add-on 的写操作共享 `.floral-notepaper.lock`，锁等待超过 5 秒时返回 `storeBusy`。
- `floral_notes_update` 和 `floral_notes_move` 必须携带读取笔记时获得的 `expectedUpdatedAt`。
- 笔记在读取后被其他进程修改时，写操作返回 `noteConflict`，不会覆盖新内容。
- MCP 客户端应在调用创建、更新、移动和创建分类等写入工具前征得用户确认。
- Add-on 只在本机通过 stdio 工作，不监听网络端口，不保存持久审计日志。

## 兼容的直接调用

旧 JSON 调用协议保持可用：

```powershell
'{}' | floral_cli_backend.exe notes.list
'{"query":"计划","offset":0,"limit":50}' | floral_cli_backend.exe notes.search
floral_cli_backend.exe --version
```

新增的安全直接调用操作包括：

- `protocol.version`
- `notes.search`
- `notes.updateIfUnchanged`
- `notes.moveIfUnchanged`

`notes.updateIfUnchanged` 和 `notes.moveIfUnchanged` 同样要求传入 `expectedUpdatedAt`，版本不一致时返回 `noteConflict`。

无参数运行仍会返回 `missingOperation`，不会进入 MCP 等待状态。
