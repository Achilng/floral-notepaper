[简体中文](../CONTRIBUTING.md) | **繁體中文** | [English](CONTRIBUTING_en-US.md)

# 貢獻指南

感謝你對花箋（Floral Notepaper）的關注！這份指南會幫助你了解如何參與專案開發。

## 開發環境配置

### 依賴

- [Node.js](https://nodejs.org/) - 22.12+（Mermaid 12 要求；建議使用 Node 24 LTS）
- [Rust](https://www.rust-lang.org/tools/install) 工具鏈
- 其他：

  | 平台        | 所需依賴                                                                                                                                                                                                             |
  | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | **macOS**   | Xcode Command Line Tools                                                                                                                                                                                             |
  | **Linux**   | `libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev`、`libsoup-3.0-dev`                                                                                                           |
  | **Windows** | [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)、[Microsoft Edge WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 11 或高版本 Windows 10 已內建） |

### 複製與啟動

```bash
git clone git@github.com:Achilng/floral-notepaper.git
cd floral-notepaper
npm install

# 啟動開發版
npm run tauri dev

# 構建正式版
npm run tauri build
```

## 分支與提交規範

本專案採用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/) 規範。

### Commit Message 格式

```
<type>: <簡短描述>
```

常用 type：

| type       | 說明                         |
| ---------- | ---------------------------- |
| `feat`     | 新功能                       |
| `fix`      | 修復 bug                     |
| `docs`     | 文件變更                     |
| `style`    | 格式調整（不影響程式碼邏輯） |
| `refactor` | 重構（既非新功能也非修復）   |
| `perf`     | 效能優化                     |
| `test`     | 新增或修改測試               |
| `chore`    | 構建、CI、依賴等雜項         |
| `ci`       | CI/CD 配置變更               |
| `revert`   | 回滾提交                     |

### 分支命名

分支名稱應與 commit type 對應：`feat/描述`、`fix/描述`、`refactor/描述` 等。

範例：

- `feat/note-tagging` → 新功能分支
- `fix/startup-crash` → 修復分支

## 程式碼風格與測試

提交 PR 前，請確保本機通過以下檢查。可以在每次 commit 前也跑一遍，避免 CI 掛掉後反覆修：

```bash
# 後端
cd src-tauri
cargo test -p floral-notepaper --lib        # 單元測試
cargo clippy --all-targets -- -D warnings   # 靜態檢查
cargo fmt --check                           # 格式檢查

# 前端
npm test                                    # 單元測試
npx oxlint                                  # 靜態檢查
npx oxfmt --check                           # 格式檢查
```

## Pull Request 流程

1. **先搜尋** [已有 PR](https://github.com/Achilng/floral-notepaper/pulls)，確認沒有雷同的提交，避免重複勞動
2. Fork 本倉庫，從 `main` 分支創建你的功能/修復分支
3. 開發完成後，確保本機檢查全部通過
4. 提交 PR 到 `main` 分支，**填寫 PR 模板**中的各項內容
5. CI 必須全部通過（構建 + lint + 測試）

   > 註：CI 檢出的是 PR 分支與 `main` 合併後的程式碼（`refs/pull/<編號>/merge`），而非你分支的原始提交。

6. 至少一位 maintainer **review 通過**後方可合併
7. Maintainer 使用 **squash merge** 合入，合入後自動刪除遠端分支

PR 標題同樣遵循 Conventional Commits 格式，例如 `feat: 添加筆記標籤功能`。

## 問題回饋

- 報告 bug：[提交 Issue](https://github.com/Achilng/floral-notepaper/issues/new?template=bug.yml)
- 功能請求：[提交 Issue](https://github.com/Achilng/floral-notepaper/issues/new?template=enhancement.yml)
- 一般討論：[GitHub Discussions](https://github.com/Achilng/floral-notepaper/discussions)
