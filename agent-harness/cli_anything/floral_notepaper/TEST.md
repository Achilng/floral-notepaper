# 测试计划

## 单元测试

`test_core.py` 覆盖：

- 数据目录快照
- undo / redo 状态变化
- CLI JSON 输出和过滤
- mutation 成功后提交历史，失败时不留下无效快照

## 端到端测试

`test_full_e2e.py` 通过已安装命令调用真实 Rust backend，覆盖：

- 创建、读取、更新和列出笔记
- 创建分类和移动笔记
- undo / redo
- 配置读取

所有端到端测试使用临时 `FLORAL_NOTEPAPER_DATA_DIR`，不会读取或修改用户真实笔记。

## 运行

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --features floral-ai-addon --bin floral_cli_backend
cargo test --manifest-path src-tauri/Cargo.toml --features floral-ai-addon --test floral_cli_backend_mcp
python -m pytest agent-harness/cli_anything/floral_notepaper/tests -q
```
