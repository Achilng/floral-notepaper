# Floral Notepaper CLI

完整命令、参数和自动化示例请参阅 [COMMANDS.md](COMMANDS.md)。

## 安装

在仓库根目录执行：

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --bin floral_cli_backend --features floral-ai-addon
python -m pip install -e agent-harness
```

第一次运行 CLI 时，如果找不到 backend，也会自动执行构建。

MCP Add-on 的构建和接入方法见 [MCP_ADDON.md](MCP_ADDON.md)。

## 使用

不带子命令会进入交互式 REPL：

```powershell
cli-anything-floral-notepaper
```

建议在自动化任务中显式指定独立数据目录：

```powershell
cli-anything-floral-notepaper --data-dir D:\temp\floral-cli notes create --title "计划" --content "# 今天"
cli-anything-floral-notepaper --data-dir D:\temp\floral-cli notes list
cli-anything-floral-notepaper --data-dir D:\temp\floral-cli --json notes list
```

### 笔记

```powershell
cli-anything-floral-notepaper notes get NOTE_ID
cli-anything-floral-notepaper notes update NOTE_ID --title "新标题" --content-file note.md
cli-anything-floral-notepaper notes move NOTE_ID --category 工作
cli-anything-floral-notepaper notes import README.md --category 资料
cli-anything-floral-notepaper notes export NOTE_ID exported.md
cli-anything-floral-notepaper notes delete NOTE_ID --yes
```

### 分类和配置

```powershell
cli-anything-floral-notepaper categories create 工作
cli-anything-floral-notepaper categories rename 工作 项目
cli-anything-floral-notepaper categories delete 项目 --yes
cli-anything-floral-notepaper config show
cli-anything-floral-notepaper config patch '{"theme":"dark","fontSize":16}'
```

### Undo / Redo

```powershell
cli-anything-floral-notepaper session status
cli-anything-floral-notepaper session undo
cli-anything-floral-notepaper session redo
```

会话历史按数据目录隔离，保存在用户目录下的 `.cli-anything/floral-notepaper/`。

## JSON 输出

在根命令后增加 `--json`：

```powershell
cli-anything-floral-notepaper --json notes list
```

成功时输出命令数据；失败时输出包含 `code`、`message` 和 `details` 的错误对象。
