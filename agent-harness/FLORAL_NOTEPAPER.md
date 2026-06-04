# Floral Notepaper CLI-Anything Harness

## 目标

这个 harness 为 Floral Notepaper 提供无需打开图形界面的自动化入口。它保留应用现有的数据模型和文件布局，实际读写由 Rust `NoteStore` 完成，Python 层只负责命令解析、JSON 输出、REPL 和 undo/redo 会话历史。

## 后端映射

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
- `config patch` 会经过应用原有 `save_config` 校验。
- undo/redo 只追踪通过当前 harness 执行的修改。
- GUI 与 CLI 不应同时修改同一条笔记；二者没有跨进程写锁。

## 架构

```text
cli-anything-floral-notepaper
  -> Python Click CLI / REPL / JSON / session snapshots
  -> floral_cli_backend Rust binary
  -> Floral Notepaper NoteStore
  -> Markdown files + metadata.json + config.json
```
