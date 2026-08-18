# Microsoft Store 发布指南（花笺 MSIX）

本文档面向花笺维护者，说明 MSIX 版本发布到 Microsoft Store 所需的**一次性设置**、**配置项**、**发布行为**与**失败处理**。

发布自动化本身集成在 [Release Workflow](../.github/workflows/release.yml) 中（`publish-msix-store` Job），本文档只覆盖 Store 侧的准备与运维。

## 发布链路概览

```
Tag Push → 构建 x64/AArch64 未签名 EXE → SignPath 签名 → 构建 MSIX（x64 + AArch64）
  → SignPath 签名 MSIX → 安装/启动/卸载测试 → msstore publish（.msixupload 合并双架构包）
  → Partner Center 认证 → 上架
```

关键文件：

- 清单模板：`src-tauri/msix/AppxManifest.template.xml`
- 打包脚本：`scripts/build-msix.ps1`、`scripts/build-msixupload.ps1`
- 发布 Job：`.github/workflows/release.yml` 中的 `publish-msix-store`

## 一次性设置（Partner Center / Entra ID）

以下步骤只在首次发布前执行一次，不包含在本仓库代码中。

### 1. 开发者账户

- 在 [Partner Center](https://partner.microsoft.com/dashboard) 注册 Windows 应用开发者账户（个人或公司）。
- 确认账户中已关联一个 Microsoft Entra ID（Azure AD）租户；没有则新建。

### 2. 创建产品并保留名称

- Partner Center → Apps and games → 新建产品，**产品类型选择 MSIX**。
- 保留一个唯一的应用名称（例如 `FloralNotepaper`）。该名称将成为 MSIX 清单的 `Identity Name`，写入仓库变量 `MSIX_IDENTITY_NAME`。
- 记录 Partner Center 产品 ID（形如 `9PXXXXXXXXXX`），写入仓库变量 `MSSTORE_APP_ID`。

### 3. 首次提交（人工）

msstore CLI **不支持创建首个 submission**（源码中对首次提交直接拒绝）。首次发布必须在 Partner Center 网页完成：

1. 从 GitHub Draft Release 下载任一架构的已签名 MSIX（或等待首次 Tag 运行产出）；
2. 在产品的 Submissions 中创建第一个 submission；
3. 上传 MSIX 包（双架构可以分两次上传，或使用 `.msixupload`）；
4. 完成上架资料：描述、截图（至少 1366×768）、年龄分级、隐私政策 URL 等；
5. 提交认证。

首个 submission 通过后，后续版本更新即可由 `publish-msix-store` 全自动提交。

### 4. 发布者身份（不可随意变更）

- Partner Center 账户的 Publisher（显示名与证书 Subject CN）必须与 MSIX 清单一致。
- 仓库变量 `MSIX_PUBLISHER_CN` 必须等于账户 Publisher 的证书 CN（形如 `CN=...`），`MSIX_PUBLISHER_DISPLAY_NAME` 为显示名。
- **变更 Publisher 会改变 Package Family Name，导致现有用户收不到更新**；Publisher 与保留名称确定后应冻结。

### 5. Entra ID 应用注册（msstore CLI 凭据）

1. Entra ID 中新建应用注册，记录 Client ID；
2. 创建 Client Secret（证书方式亦可，本仓库使用 Secret）；
3. 将应用与 Partner Center 账户关联（Partner Center → 账户设置 → 用户管理 → Azure AD 应用程序），赋予 **Manager** 角色；
4. 将以下值写入仓库级（全局）Secrets：
   - `PARTNER_CENTER_TENANT_ID`（Entra ID 租户 ID）
   - `PARTNER_CENTER_SELLER_ID`（Partner Center Seller ID，账户设置中可见）
   - `PARTNER_CENTER_CLIENT_ID`
   - `PARTNER_CENTER_CLIENT_SECRET`

### 6. SignPath artifact configurations

在 SignPath 项目（与现有 Windows 签名同一项目）中新建两个 artifact configuration，
完整 XML 见 [signpath-configurations.md](signpath-configurations.md)：

- **MSIX 包配置**：root 元素 `<msix-file>`，`<authenticode-sign hash-algorithm="sha256" />`（仅包级签名，不做 deep signing），slug 写入仓库级 Variable `SIGNPATH_WINDOWS_MSIX_ARTIFACT_CONFIGURATION_SLUG`；
- **AArch64 主程序配置**：root 元素 `<pe-file>`，默认 SHA-256 签名，slug 写入 `SIGNPATH_WINDOWS_BINARY_ARM64_ARTIFACT_CONFIGURATION_SLUG`。

复用现有 `release-signing` Policy，无需新建 Policy。

## 配置项汇总

| 位置 | 类型     | 名称                                                                                                                                                                               | 说明                                                                                                       |
| ---- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 仓库 | Variable | `MSIX_IDENTITY_NAME`                                                                                                                                                               | 清单 Identity Name（= 保留名称）                                                                           |
| 仓库 | Variable | `MSIX_PUBLISHER_CN`                                                                                                                                                                | 清单 Publisher（证书 CN）                                                                                  |
| 仓库 | Variable | `MSIX_PUBLISHER_DISPLAY_NAME`                                                                                                                                                      | 清单 PublisherDisplayName                                                                                  |
| 仓库 | Variable | `MSSTORE_APP_ID`                                                                                                                                                                   | Partner Center 产品 ID                                                                                     |
| 仓库 | Variable | `MSSTORE_PUBLISH_ENABLED`                                                                                                                                                          | 置 `false` 暂停 Store 发布                                                                                 |
| 仓库 | Variable | `MSIX_ARM64_RUNTIME_TEST`                                                                                                                                                          | AArch64 运行时测试总开关；置 `false` 跳过 AArch64 MSIX 与 AArch64 NSIS 的运行时测试（签名/结构验证仍执行） |
| 仓库 | Secret   | `PARTNER_CENTER_TENANT_ID` / `PARTNER_CENTER_SELLER_ID` / `PARTNER_CENTER_CLIENT_ID` / `PARTNER_CENTER_CLIENT_SECRET`                                                              | msstore CLI 凭据                                                                                           |
| 仓库 | Variable | `SIGNPATH_WINDOWS_MSIX_ARTIFACT_CONFIGURATION_SLUG` / `SIGNPATH_WINDOWS_BINARY_ARM64_ARTIFACT_CONFIGURATION_SLUG` / `SIGNPATH_WINDOWS_INSTALLER_ARM64_ARTIFACT_CONFIGURATION_SLUG` | SignPath 配置                                                                                              |

发布不再依赖 Environment 审批；是否发布仅由 `MSSTORE_PUBLISH_ENABLED` 控制（置 `false` 或暂不推送 Tag 可暂停发布）。如需人工把关，可在提交前手动暂停变量或依赖 GitHub 人工流程。

## 发布行为

- `publish-msix-store` 的执行条件取决于触发方式：
  - **Tag Push**：每次 `vX.Y.Z` Tag 推送都会执行（除非 `MSSTORE_PUBLISH_ENABLED=false`）；
  - **手动触发（workflow_dispatch）**：由必选的 `publish-to-store` 输入决定（`true` 发布 / `false` 跳过），与 `MSSTORE_PUBLISH_ENABLED` 无关；
- Job 会合并两个已签名 MSIX 为 `.msixupload`（zip 容器，内含 x64 与 AArch64 包，容器本身不签名）并提交**同一个 submission**；
- 提交后 `msstore submission status` 打印状态；Store 认证通常需要数小时到数天；
- 免费产品限制：msstore CLI 仅支持免费产品的更新操作；
- msstore CLI 当前为 preview 状态，行为变化以官方文档为准（<https://aka.ms/msstoredevcli/docs>）。

## 版本与降级

- Store 拒绝低于或等于已提交版本的 submission；
- 发布前确认 Tag 版本高于 Store 中已提交版本；
- 不要通过移动 Tag 重发；需要修复时发布新的补丁版本。

## 失败处理

| 现象                                    | 处理                                                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `msstore reconfigure` 失败              | 检查 Entra ID 凭据与 Partner Center 关联（应用角色需为 Manager）                                 |
| `msstore publish` 提示首次提交不支持    | 首个 submission 必须在 Partner Center 人工创建                                                   |
| `msstore publish` 版本冲突              | 确认新版本高于 Store 已提交版本                                                                  |
| 提交成功但认证失败                      | Partner Center 查看认证报告，修复后发布新版本                                                    |
| 认证期间发现问题                        | 认证完成前在 Partner Center 取消 submission                                                      |
| 不想发布当前版本（Tag Push）            | `MSSTORE_PUBLISH_ENABLED=false` 重跑                                                             |
| 不想发布当前版本（手动触发）            | 手动触发时 `publish-to-store` 选 `false`                                                         |
| 包被 Store 摄取拒绝（.msixupload 结构） | 对照 Visual Studio 生成的 .msixupload（zip 内含各架构 .msix）检查 `scripts/build-msixupload.ps1` |

## 人工核验清单（每次发布）

1. `verify-windows-msix-install` 摘要：x64 与 AArch64 安装/启动/卸载通过；
2. GitHub Draft Release 含 `floral-notepaper_X.Y.Z_x64.msix` 与 `_arm64.msix`，SHA256SUMS 一致；
3. Partner Center 中该版本 submission 的包列表包含 x64 与 AArch64；
4. Store 上架资料（截图、描述、分级）无需更新（如版本无重大变化可沿用）；
5. 认证完成后抽查 Store 页面版本号与本次 Tag 一致。
