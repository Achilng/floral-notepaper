# MSIX 商店渠道更新 UI 设计

日期：2026-08-18
状态：已批准

## 背景与目标

花笺已接入 Microsoft Store 发布链路（`feat/msstore-and-msix` 分支）。MSIX 包由 Store 托管更新，应用内置的自研更新器（GitHub + Mirror酱 双源，`src-tauri/src/updater/`）在 MSIX 下不适用，后端已通过运行时检测禁用全部更新路径（自动检查调度器不启动、手动检查/下载/安装被 gate 拒绝）。

但前端 UI 呈现不完整：关于页更新区在 MSIX 下仅显示纯文本提示"此版本由 Microsoft Store 管理更新…"，无可点击的 Store 入口；设置页更新区显示同样提示而非完全隐藏；用户希望彻底隐藏内置自更新与 Mirror酱 相关 UI，并把关于页的更新入口改为链往 Microsoft Store 产品页。

目标：

1. 关于页（`mode="checkOnly"`）MSIX 下：仅显示"在 Microsoft Store 中查看更新"按钮，点击打开 `ms-windows-store://pdp/?productid=9NRCC0ZSG81R`
2. 设置页（`mode="settingsOnly"`）MSIX 下：更新设置区（含 Mirror酱 CDK、下载源选择等）完全隐藏
3. 关于页底部"您知道吗"tips 跑马灯：保留现状（不过滤 Mirror酱 推广）
4. 非 MSIX 渠道（NSIS/GitHub 版）行为零变化

## 已确认的设计决策

1. **渠道判定**：复用现有运行时检测（`platform.rs::has_package_identity()` → 前端 `status.installKind === "windowsMsix"`），无编译期改动。同一 exe 二进制同时进 NSIS 与 MSIX 包，无法（也不应）编译期裁剪。
2. **关于页形态**：去掉提示文字，仅保留链接按钮（用户选择）
3. **设置页形态**：完全隐藏——组件在 MSIX + `settingsOnly` 下返回 null（用户选择）
4. **tips 跑马灯**：不过滤 Mirror酱 推广（用户选择）
5. **Store 链接**：`ms-windows-store://pdp/?productid=9NRCC0ZSG81R`（product id 硬编码为模块级常量，导出供单测断言）
6. **打开方式**：`@tauri-apps/plugin-opener` 的 `openUrl`（Windows 上经 ShellExecute 链交给系统协议处理器拉起 Store 应用）
7. **后端**：零改动（gate/调度器已就位，保持作为安全网）

## 关键发现：opener 插件的 URL scope

`opener:default` 权限的 scope 仅允许 `mailto:*`、`tel:*`、`http://*`、`https://*`（`tauri-plugin-opener-2.5.4/permissions/allow-default-urls.toml`）；`open_url` 命令对不在 scope 内的 URL 返回 `ForbiddenUrl`（`commands.rs:36-39`）。因此**必须**在 `src-tauri/capabilities/default.json` 中为 `opener:allow-open-url` 追加 scope `ms-windows-store://*`（glob 匹配，非 path 模式，`*` 可跨 `/`），否则按钮点击静默失败。

## 改动文件

| 文件                                                 | 改动                                                                                                                                                 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/capabilities/default.json`                | 追加 `{ "identifier": "opener:allow-open-url", "allow": [{ "url": "ms-windows-store://*" }] }`                                                       |
| `src/features/update/UpdateSettingsSection.tsx`      | 新增导出常量 `MICROSOFT_STORE_PRODUCT_ID` / `MICROSOFT_STORE_PDP_URL`；`storeManaged` 分支按 mode 区分：`settingsOnly` → null，其余 → Store 链接按钮 |
| `src/locales/{zh-CN,zh-HK,en-US}/translation.json`   | 新增 `settings.update.openInStore`；删除已无引用的 `storeManagedNotice`                                                                              |
| `src/features/update/UpdateSettingsSection.test.tsx` | 更新 MSIX 用例断言；新增 checkOnly + MSIX、settingsOnly + MSIX 用例                                                                                  |

不改动：`AboutPanel.tsx`、`SettingsPanel.tsx`、`MainWindow.tsx`（徽标在 MSIX 下不显示）、`src-tauri/src/**`、tips 相关。

## 风险与缓解

| 风险                                            | 缓解                                                |
| ----------------------------------------------- | --------------------------------------------------- |
| opener scope 拒绝导致按钮静默失败（最高）       | capabilities 显式追加 scope；已验证插件源码确认机制 |
| MSIX 沙箱环境无 Store（Server SKU）点击无响应   | 与所有 `ms-windows-store://` 链接行为一致，可接受   |
| 无 @testing-library，click→openUrl 无自动化覆盖 | 常量单测 + Windows 手动验证（旁加载 MSIX）补齐      |
| i18n 漏加 key 回落中文                          | 三语言同步新增；`t()` 有 defaultValue 兜底          |

## 验证

- `npm test`（100/100 通过）、`npm run lint`、`npm run build`
- Windows 手动验证：旁加载 MSIX → 关于页仅见 Store 链接按钮（点击打开 Store 产品页）；设置页更新区完全消失；NSIS 版回归不变
