# Floral Notepaper CLI-Anything Harness

这个 harness 为 Floral Notepaper 提供无需打开图形界面的自动化入口。它保留应用现有的数据模型和文件布局，实际读写由 Rust `NoteStore` 完成，Python 层负责命令解析、JSON 输出、REPL 和 undo/redo 会话历史。

相关文档：

- [COMMANDS.md](COMMANDS.md)：Python CLI 的完整命令、参数和自动化示例。
- [MCP_ADDON.md](MCP_ADDON.md)：Backend 直接 JSON 协议、MCP 接入和便携部署教程。

## 安装

在仓库根目录执行：

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --bin floral_cli_backend --features floral-ai-addon
python -m pip install -e agent-harness
```

第一次运行 CLI 时，如果找不到开发版本的 Backend，也会自动执行构建。

## 快速开始

不带子命令会进入交互式 REPL：

```powershell
cli-anything-floral-notepaper
```

建议在自动化和测试任务中显式指定独立数据目录：

```powershell
cli-anything-floral-notepaper --data-dir D:\temp\floral-cli notes create --title "计划" --content "# 今天"
cli-anything-floral-notepaper --data-dir D:\temp\floral-cli notes list
cli-anything-floral-notepaper --data-dir D:\temp\floral-cli --json notes list
```

常用命令：

```powershell
cli-anything-floral-notepaper notes get NOTE_ID
cli-anything-floral-notepaper notes update NOTE_ID --title "新标题" --content-file note.md
cli-anything-floral-notepaper notes move NOTE_ID --category 工作
cli-anything-floral-notepaper categories create 工作
cli-anything-floral-notepaper config show
cli-anything-floral-notepaper session undo
```

在根命令后增加 `--json` 可获得适合程序读取的输出：

```powershell
cli-anything-floral-notepaper --json notes list
```

成功时输出命令数据；失败时输出包含 `code`、`message` 和 `details` 的错误对象。

## 架构

```text
cli-anything-floral-notepaper
  -> Python Click CLI / REPL / JSON / session snapshots
  -> floral_cli_backend Rust binary
  -> Floral Notepaper OperationService
  -> Floral Notepaper NoteStore
  -> Markdown files + metadata.json + config.json
```

后端映射：

| CLI 命令                               | Rust 后端                            |
| -------------------------------------- | ------------------------------------ |
| `notes list/get/create/update/delete`  | `NoteStore` 笔记 CRUD                |
| `notes move`                           | `NoteStore::move_note_to_category`   |
| `notes import/export`                  | `NoteStore` Markdown 导入导出        |
| `categories list/create/rename/delete` | `NoteStore` 分类 API                 |
| `config show/patch`                    | `NoteStore::load_config/save_config` |
| `session undo/redo`                    | harness 数据目录快照                 |

## 安全边界

- 默认使用 Floral Notepaper 的正常数据目录。
- 自动化和测试时应使用 `--data-dir` 指向独立目录。
- `config patch` 会经过应用原有的 `save_config` 校验。
- undo/redo 只追踪通过当前 harness 执行的修改。
- GUI、CLI 和 MCP Add-on 的 NoteStore 操作共享数据目录中的 `.floral-notepaper.lock` 跨进程锁。
- MCP 更新和移动操作要求携带读取时获得的 `updatedAt`，发生版本冲突时不会覆盖较新的笔记。

Backend 和 MCP 的详细安全边界见 [MCP_ADDON.md](MCP_ADDON.md)。

## 测试

Python 单元测试覆盖：

- 数据目录快照
- undo / redo 状态变化
- CLI JSON 输出和过滤
- 修改成功后提交历史，失败时不留下无效快照

端到端测试通过已安装命令调用真实 Rust Backend，覆盖：

- 创建、读取、更新和列出笔记
- 创建分类和移动笔记
- undo / redo
- 配置读取

所有 Python 端到端测试使用临时 `FLORAL_NOTEPAPER_DATA_DIR`，不会读取或修改用户真实笔记。

运行测试：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --features floral-ai-addon
python -m pytest agent-harness/cli_anything/floral_notepaper/tests -q
```

## 使用已发布 Backend

CLI 默认查找：

```text
src-tauri/target/debug/floral_cli_backend.exe
```

使用 release Backend 或已部署的 Backend 时，通过 `FLORAL_CLI_BACKEND` 指定完整路径：

```powershell
$env:FLORAL_CLI_BACKEND = "D:\Floral\floral_cli_backend.exe"
cli-anything-floral-notepaper --json notes list
```

CLI 每次运行时会把所选数据根目录传给 Backend，通常不需要手动设置 `FLORAL_NOTEPAPER_DATA_DIR`。
