# SignPath 签名配置（花笺 MSIX / AArch64 主程序 / AArch64 安装器）

本文档给出花笺发布链在 SignPath 侧需要创建的三个 Artifact Configuration 的完整 XML，
以及现有 x64 配置的参考形态。配置在 SignPath 项目（与现有 Windows 签名同一项目）中创建：

SignPath → Projects → 选择项目 → Artifact Configurations → New，
将下方 XML 粘贴到编辑器中（编辑器支持 `http://signpath.io/artifact-configuration/v1` schema 校验），
保存后把 slug 写入对应 GitHub Variable。

## 通用约定

- 所有配置复用现有 `release-signing` 签名 Policy，**不需要新建 Policy**；
- `parameters` 中的 `version` 由 Workflow 每次提交时传入
  （GitHub Action 的 `parameters: version: <X.Y.Z>`），用于请求审计跟踪；
- 参数在 XML 属性中以 `${parameterName}` 语法引用；
- 上传到 SignPath 的 Artifact 均为**单个未签名文件**（`actions/upload-artifact` 的 `archive: false`），
  因此 root 元素使用单文件类型（`<pe-file>` / `<msix-file>`），不需要 `<zip-file>` 包装。

---

## 1. Windows AArch64 主程序（新增）

对应 GitHub Variable：`SIGNPATH_WINDOWS_BINARY_ARM64_ARTIFACT_CONFIGURATION_SLUG`

输入文件：`floral-notepaper_<version>_aarch64.exe`（未签名，由 `build-windows-binary-aarch64` 上传）。

```xml
<artifact-configuration
  xmlns="http://signpath.io/artifact-configuration/v1"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://signpath.io/artifact-configuration/v1 https://app.signpath.io/Web/artifact-configuration/v1.xsd">

  <parameters>
    <parameter name="version" required="true" />
  </parameters>

  <!-- 单个未签名 PE：使用默认 SHA-256 算法签名并附加时间戳 -->
  <pe-file>
    <authenticode-sign hash-algorithm="sha256" />
  </pe-file>
</artifact-configuration>
```

说明：

- 该配置与现有 x64 主程序配置的区别在于 root 元素直接是 `<pe-file>`（x64 是 `<zip-file>` 内含 PE）；
- 不添加 `product-name` / `product-version` 等 `peConstraints` 属性，避免对 PE 元数据产生意外约束；
- 签名后的文件保持文件名不变，Workflow 按 `floral-notepaper_<version>_aarch64.exe` 接收并验证。

---

## 2. MSIX 包（新增）

对应 GitHub Variable：`SIGNPATH_WINDOWS_MSIX_ARTIFACT_CONFIGURATION_SLUG`

输入文件：`floral-notepaper_<version>_x64.msix` 与 `floral-notepaper_<version>_aarch64.msix`
（未签名，由 `build-windows-msix` 矩阵 Job 按架构分别上传、分别签名）。

```xml
<artifact-configuration
  xmlns="http://signpath.io/artifact-configuration/v1"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://signpath.io/artifact-configuration/v1 https://app.signpath.io/Web/artifact-configuration/v1.xsd">

  <parameters>
    <parameter name="version" required="true" />
  </parameters>

  <!--
    MSIX/APPX 包签名：MSIX 只允许 SHA-256 及以上（schema 已排除 SHA-1 等旧算法）。
    包内 floral-notepaper.exe 已在主程序签名阶段单独签名，这里只做包级签名，
    不配置 deep signing，避免内嵌文件被重签导致哈希变化。
  -->
  <msix-file>
    <authenticode-sign hash-algorithm="sha256" />
  </msix-file>
</artifact-configuration>
```

说明：

- 一个配置即可服务 x64 与 AArch64 两个包（每次签名请求处理一个文件）；
- **不要**在 `<msix-file>` 内嵌套 `<pe-file>`（deep signing）：
  内嵌主程序已预先签名，deep signing 会重签并改变其哈希；
  Workflow 对两种情形都有验证逻辑，但保持"仅包级签名"最符合当前设计；
- SignPath 的 MSIX 签名会生成正确的包级签名（signature.p7x 块），
  `Add-AppxPackage` 与 Store 摄取均要求该签名使用 SHA-256；
- 清单（`AppxManifest.xml`）内容由 `scripts/build-msix.ps1` 在打包阶段写入，与本配置无关。

---

## 3. Windows AArch64 安装器（新增）

对应 GitHub Variable：`SIGNPATH_WINDOWS_INSTALLER_ARM64_ARTIFACT_CONFIGURATION_SLUG`

输入文件：`floral-notepaper_<version>_aarch64-setup.exe`（未签名，由 `build-windows-installer-aarch64` 上传）。

```xml
<artifact-configuration
  xmlns="http://signpath.io/artifact-configuration/v1"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://signpath.io/artifact-configuration/v1 https://app.signpath.io/Web/artifact-configuration/v1.xsd">

  <pe-file>
    <authenticode-sign hash-algorithm="sha256" />
  </pe-file>
</artifact-configuration>
```

说明：

- **不声明 `<parameters>`**：AArch64 安装器签名请求不传 `parameters`（与现有 x64 安装器一致）。
  若照抄带 `<parameter name="version" required="true" />` 的模板会导致签名请求校验失败；
- **不配置 deep signing**：安装器内嵌的主程序必须保持 SignPath 主程序签名阶段的产物，
  安装测试硬约束校验安装后主程序哈希与 portable 二进制一致，deep signing 会导致哈希变化而失败；
- NSIS 安装器本体是 x86-unicode stub，签名时按普通 PE 处理即可，与架构无关；
- 签名后的文件保持文件名不变，Workflow 按 `floral-notepaper_<version>_aarch64-setup.exe` 接收并验证。

---

## 4. 现有 x64 主程序（参考，无需改动）

对应 GitHub Variable：`SIGNPATH_WINDOWS_BINARY_ARTIFACT_CONFIGURATION_SLUG`

输入文件：`unsigned-windows-binaries.zip`（内含 `floral-notepaper_<version>.exe`）。
此配置已存在且正在生产使用，下方形态供对照新配置时参考：

```xml
<artifact-configuration
  xmlns="http://signpath.io/artifact-configuration/v1"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://signpath.io/artifact-configuration/v1 https://app.signpath.io/Web/artifact-configuration/v1.xsd">

  <parameters>
    <parameter name="version" required="true" />
  </parameters>

  <!-- x64 上传的是 zip 容器，因此 root 为 zip-file，内嵌 PE 按文件名匹配 -->
  <zip-file>
    <pe-file path="floral-notepaper_${version}.exe">
      <authenticode-sign hash-algorithm="sha256" />
    </pe-file>
  </zip-file>
</artifact-configuration>
```

---

## 创建后的核对清单

1. 四个 slug 均已填入 GitHub Variables：
   - `SIGNPATH_WINDOWS_BINARY_ARM64_ARTIFACT_CONFIGURATION_SLUG`
   - `SIGNPATH_WINDOWS_INSTALLER_ARM64_ARTIFACT_CONFIGURATION_SLUG`
   - `SIGNPATH_WINDOWS_MSIX_ARTIFACT_CONFIGURATION_SLUG`
   - （现有）`SIGNPATH_WINDOWS_BINARY_ARTIFACT_CONFIGURATION_SLUG`
2. 在 SignPath 中分别用测试 Tag 提交一次签名请求，确认：
   - `release-signing` Policy 的 Trusted Build System / Origin Verification 规则对新配置生效；
   - AArch64 主程序配置能匹配 `floral-notepaper_<version>_aarch64.exe`；
   - AArch64 安装器配置能匹配 `floral-notepaper_<version>_aarch64-setup.exe`，返回文件可被
     `Get-AuthenticodeSignature` 验证为 `Valid` 且带时间戳，内嵌主程序哈希与签名前一致；
   - MSIX 配置返回的包可被 `Get-AuthenticodeSignature` 验证为 `Valid` 且带时间戳；
   - 证书固定值（`SIGNPATH_CERTIFICATE_SUBJECT/ISSUER/SHA1`）与实际签名证书一致。
3. 四个配置均不需要新建 Policy；Policy 侧配置保持不变。
