**简体中文** | [繁體中文](Docs/CONTRIBUTING_zh-HK.md) | [English](Docs/CONTRIBUTING_en-US.md)

# 贡献指南

感谢你对花笺（Floral Notepaper）的关注！这份指南会帮助你了解如何参与项目开发。

## 开发环境配置

### 依赖

- [Node.js](https://nodejs.org/) - 20.19+ 或 22.12+
- [Rust](https://www.rust-lang.org/tools/install) 工具链
- 其他：

  | 平台        | 所需依赖                                                                                                                                                                                                             |
  | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | **macOS**   | Xcode Command Line Tools                                                                                                                                                                                             |
  | **Linux**   | `libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev`、`libsoup-3.0-dev`                                                                                                           |
  | **Windows** | [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)、[Microsoft Edge WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 11 或高版本 Windows 10 已内置） |

### 克隆与启动

```bash
git clone git@github.com:Achilng/floral-notepaper.git
cd floral-notepaper
npm install

# 启动开发版
npm run tauri dev

# 构建正式版
npm run tauri build
```

## 分支与提交规范

本项目采用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/) 规范。

### Commit Message 格式

```
<type>: <简短描述>
```

常用 type：

| type       | 说明                       |
| ---------- | -------------------------- |
| `feat`     | 新功能                     |
| `fix`      | 修复 bug                   |
| `docs`     | 文档变更                   |
| `style`    | 格式调整（不影响代码逻辑） |
| `refactor` | 重构（既非新功能也非修复） |
| `perf`     | 性能优化                   |
| `test`     | 添加或修改测试             |
| `chore`    | 构建、CI、依赖等杂项       |
| `ci`       | CI/CD 配置变更             |
| `revert`   | 回滚提交                   |

### 分支命名

分支名应与 commit type 对应：`feat/描述`、`fix/描述`、`refactor/描述` 等。

示例：

- `feat/note-tagging` → 新功能分支
- `fix/startup-crash` → 修复分支

## 代码风格与测试

提交 PR 前，请确保本地通过以下检查。可以在每次 commit 前也跑一遍，避免 CI 挂掉后反复修：

```bash
# 后端
cd src-tauri
cargo test -p floral-notepaper --lib        # 单元测试
cargo clippy --all-targets -- -D warnings   # 静态检查
cargo fmt --check                           # 格式检查

# 前端
npm test                                    # 单元测试
npx oxlint                                  # 静态检查
npx oxfmt --check                           # 格式检查
```

## Pull Request 流程

1. **先搜索** [已有 PR](https://github.com/Achilng/floral-notepaper/pulls)，确认没有雷同的提交，避免重复劳动
2. Fork 本仓库，从 `main` 分支创建你的功能/修复分支
3. 开发完成后，确保本地检查全部通过
4. 提交 PR 到 `main` 分支，**填写 PR 模板**中的各项内容
5. CI 必须全部通过（构建 + lint + 测试）

   > 注：CI 检出的是 PR 分支与 `main` 合并后的代码（`refs/pull/<编号>/merge`），而非你分支的原始提交。

6. 至少一位 maintainer **review 通过**后方可合并
7. Maintainer 使用 **squash merge** 合入，合入后自动删除远程分支

PR 标题同样遵循 Conventional Commits 格式，例如 `feat: 添加笔记标签功能`。

## 发布流程（维护者）

正式版本的 Tag、跨平台构建、Windows SignPath 签名、安装验证和 Draft Release 操作，请参考 [Release 发布流程](Docs/RELEASE_WORKFLOW.md)。

## 问题反馈

- 报告 bug：[提交 Issue](https://github.com/Achilng/floral-notepaper/issues/new?template=bug.yml)
- 功能请求：[提交 Issue](https://github.com/Achilng/floral-notepaper/issues/new?template=enhancement.yml)
- 一般讨论：[GitHub Discussions](https://github.com/Achilng/floral-notepaper/discussions)
