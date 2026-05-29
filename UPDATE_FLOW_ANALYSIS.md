# "重启并安装"更新流程分析文档

> 面向开发者，分析用户点击"安装并重启"按钮后，软件在不同平台上的完整逻辑链路，指出薄弱点及潜在风险。
>
> **已验证代码版本**: `feat/auto-upgrade` 分支，基于实际源码（`src-tauri/src/updater/`）分析。

---

## 目录

1. [完整调用链路](#一完整调用链路)
2. [Helper 进程详解](#二helper-进程详解)
3. [状态恢复机制](#三状态恢复机制)
4. [macOS 分析](#四macos-分析)
5. [Windows 分析](#五windows-分析)
6. [跨平台通用问题](#六跨平台通用问题)
7. [优先级汇总](#七优先级汇总)

---

## 一、完整调用链路

```
用户点击"安装并重启"按钮
  │
  ├─ UpdateSettingsSection.tsx           handleInstall()
  │   └─ installUpdate() → invoke("update_install")
  │
  ├─ commands.rs:138                     update_install command
  │   │
  │   ├─ state.load_state()             从 state.json 加载当前状态
  │   ├─ begin_task(Install)            获取互斥锁 ActiveTaskGuard
  │   │
  │   ├─ begin_install_prepare()        通知所有窗口保存未保存内容 (≤10s timeout)
  │   │   ├─ 生成 UUID request_id
  │   │   ├─ 向所有窗口 emit "update://prepare-install"
  │   │   └─ 等待各窗口调用 update_install_prepare_report(Ready/Failed)
  │   │       - 10s 超时 → 返回错误 (app 不退出)
  │   │       - 任意窗口 Failed → 返回错误 (app 不退出)
  │   │       - 全部 Ready → 继续
  │   │
  │   ├─ spawn_blocking → UpdateInstallService::from_env().run()
  │   │   │
  │   │   ├─ prepare_request()                        [install.rs:149]
  │   │   │   ├─ 校验 status ∈ {Downloaded, InstallScheduled, Failed}
  │   │   │   ├─ 读取 asset_path / asset_sha256 / asset_size / latest_version
  │   │   │   ├─ 检测平台 → InstallKind / target_path
  │   │   │   ├─ resolve_helper_source_path()         确定 helper 源
  │   │   │   │   ├─ macOS AppBundle → current_app_bundle (.app)
  │   │   │   │   └─ 其他 → current_exe (裸二进制)
  │   │   │   ├─ stage_helper_copy()                  复制 helper 到 staging
  │   │   │   │   ├─ macOS AppBundle → ditto 复制整个 .app bundle
  │   │   │   │   └─ 其他 → fs::copy 单文件复制
  │   │   │   └─ 构建 HelperLaunchRequest { helper_path, command }
  │   │   │
  │   │   └─ ProcessInstallExecutor::execute()        [install.rs:41]
  │   │       ├─ 删除残留 ready_path marker
  │   │       ├─ Command::new(helper_path)            启动 helper 子进程
  │   │       │   .args(["--update-helper", "--mode", "apply", ...])
  │   │       │   .stdin(null).stdout(null).stderr(null)
  │   │       │   .spawn()
  │   │       └─ wait_for_helper_ready()              ≤5s, 100ms 轮询
  │   │           └─ ready_path marker 存在 → 成功
  │   │           └─ helper 提前退出 → handshake failed
  │   │           └─ 超时 → kill helper → handshake failed
  │   │
  │   ├─ [executor 成功] state::save(InstallScheduled) [install.rs:129]
  │   │   ├─ emit "update://install-finished"
  │   │   ├─ desktop::mark_app_exiting(&app)
  │   │   └─ app.exit(0)                              ← 主进程终止点
  │   │
  │   └─ [executor 失败] state::save(Failed)           [install.rs:140]
  │       └─ emit "update://error"
  │
  └─ ============ 主进程已退出 ============
  │
  └─ helper 进程（独立运行，由 spawn 创建，主进程退出后继续）
       │
       ├─ helper.rs:130    run_cli() → parse_args() → execute()
       │
       ├─ execute()                                    [helper.rs:239]
       │   ├─ open_log()                              创建/追加日志
       │   ├─ validate_request()                      校验资产完整性
       │   │   ├─ target_path 必须存在
       │   │   ├─ asset_path metadata 可读 + 大小匹配
       │   │   └─ SHA256(asset) == expected           全文件哈希
       │   ├─ ensure_sufficient_disk_space()           需要 2×asset_size
       │   ├─ write_ready_marker()                     ← 主进程 poll 检测此文件
       │   │
       │   └─ [mode==Test] 只校验，不安装，直接成功退出
       │
       └─ [mode==Apply] execute_apply()               [helper.rs:261]
            ├─ wait_for_process_exit(wait_pid)         最多 30s, 500ms 轮询
            │   └─ 超时 → WaitTimedOut → 走失败路径
            │
            ├─ persist_installing_state()              state.json = Installing
            │
            ├─ apply_update()  按 install_kind 分发
            │   ├─ MacosAppBundle   → install_macos_bundle
            │   ├─ WindowsPortable  → 直接返回错误 (不支持)
            │   └─ WindowsNsis      → install_windows_installer
            │
            ├─ [成功] persist_success_state()          state.json = Idle, version 更新
            │   └─ cleanup_after_install()             删除 ready marker + 整个下载目录
            │       └─ relaunch_target()               启动新版本
            │
            └─ [失败] persist_failed_state()           state.json = Failed
                └─ cleanup_after_install()             同样删除整个下载目录！
                    └─ relaunch_existing_target()      尝试启动旧版本
```

### 状态转换概览

```
Idle → Checking → Available → Downloading → Downloaded
                                                  │
                                         用户点击"安装并重启"
                                                  │
                                                  ▼
                                          InstallScheduled  ← 主进程写，然后 app.exit(0)
                                                  │
                                                  ▼
                                            Installing      ← helper 写
                                                 ╱ ╲
                                               成功  失败
                                                ↓     ↓
                                              Idle  Failed
```

---

## 二、Helper 进程详解

### 2.1 设计理念

Helper 与主程序是**同一个可执行文件**的双模式设计。启动时通过 `--update-helper` 标志区分：

- **正常模式**：启动 Tauri 窗口应用
- **Helper 模式**：执行更新安装 CLI 逻辑后退出

### 2.2 双进程协作机制

主进程启动 helper 后，通过 **ready marker 文件** 进行握手：

```
主进程                            helper 进程
  │                                  │
  ├─ spawn helper ──────────────────→├─ 启动
  │                                  ├─ open_log()
  │                                  ├─ validate_request()  ← SHA256 在这里计算!
  │                                  ├─ disk_space_check()
  │                                  ├─ write_ready_marker() → 创建 ready_path 文件
  │                                  │
  ├─ poll ready_path (100ms间隔) ──→│
  ├─ ready_path 存在! ──────────────→│
  ├─ save InstallScheduled           │
  ├─ app.exit(0) ───────────────────→├─ wait_for_process_exit()  等主进程退出
  │                                  ├─ persist_installing_state()
  │                                  ├─ apply_update()
  │                                  └─ relaunch / cleanup
```

**关键时序**: helper 在写 ready marker 之前需要完成 SHA256 全文件校验，大文件时可能超过主进程的 5s 超时。

### 2.3 命令行参数

```
--update-helper
--mode apply|test
--install-kind macos-app-bundle|windows-portable|windows-nsis|unknown
--wait-pid <主进程PID>
--state-path <updates_dir>/state.json
--asset-path <下载的 .dmg/.zip/.exe/.msi>
--asset-sha256 <64位hex>
--asset-size <字节数>
--target-path <当前安装路径>
--log-path <日志文件路径>
--ready-path <握手marker路径>
--current-version <旧版本号>
--target-version <新版本号>
```

### 2.4 退出码映射

| 退出码 | 枚举                   | 含义                   | 建议 action   |
| ------ | ---------------------- | ---------------------- | ------------- |
| 0      | Success                | 安装成功               | -             |
| 2      | InvalidArguments       | 参数无效               | retryInstall  |
| 3      | AssetMissing           | 更新包文件不存在       | retryDownload |
| 4      | AssetSizeMismatch      | 更新包大小校验失败     | retryDownload |
| 5      | AssetHashMismatch      | 更新包哈希校验失败     | retryDownload |
| 6      | TargetMissing          | 安装目标不存在         | retryInstall  |
| 7      | LogWriteFailed         | 无法写入日志           | retryInstall  |
| 8      | WaitTimedOut           | 等待进程退出超时       | retryInstall  |
| 9      | UnsupportedInstallKind | 不支持的安装类型       | retryInstall  |
| 10     | AssetExtractFailed     | 无法解包               | retryDownload |
| 11     | ReplacementFailed      | 替换文件失败           | retryInstall  |
| 12     | RelaunchFailed         | 重新启动失败           | retryInstall  |
| 13     | StateWriteFailed       | 状态写入失败           | retryInstall  |
| 14     | InstallerFailed        | 安装程序执行失败       | retryInstall  |
| 15     | InsufficientSpace      | 磁盘空间不足           | retryInstall  |
| 16     | InstallerTimedOut      | 安装程序执行超时       | retryInstall  |
| 17     | InstallerCancelled     | 安装已被取消           | retryInstall  |
| 18     | InstallerBusy          | 另一个安装程序正在运行 | retryInstall  |
| 19     | InstallerFatal         | 安装程序返回致命错误   | retryInstall  |

### 2.5 状态文件操作

Helper 通过 read-modify-write 模式操作 `state.json`：

```rust
// 读取（失败静默回退）
fn load_state_snapshot(path) {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(...)   // 解析失败 → 空状态 + 编译时版本号
}

// 写入（通过 write_json_atomic: write .tmp → rename）
fn write_state_snapshot(path, state) {
    write_json_atomic(path, state)
}
```

**注意**：`load_state_snapshot` 在解析失败时静默回退为 default 状态，版本号来自编译时 `env!("CARGO_PKG_VERSION")`。如果 helper 是从旧版本 staging 复制出来的，这里存在版本不一致风险。

---

## 三、状态恢复机制

### 3.1 启动时恢复 (state::recover)

应用启动时调用 `state::recover()` (`state.rs:75-116`) 处理中断状态：

| 中断状态           | 恢复策略                                                                             |
| ------------------ | ------------------------------------------------------------------------------------ |
| `Downloading`      | → `Failed`, action=`retryDownload`                                                   |
| `InstallScheduled` | → `Failed`, 检查 asset 是否完好 (存在+size+hash) → `retryInstall` 或 `retryDownload` |
| `Installing`       | 同上                                                                                 |

### 3.2 Asset 完整性检查

```rust
// state.rs:118-135
fn verify_asset(path, expected_size, expected_hash) -> bool {
    metadata.len() == expected_size   // 大小匹配
    && sha256(path) == expected_hash  // SHA256 匹配（重新计算！）
}
```

恢复时重新计算 SHA256 意味着：如果 asset 文件很大（几百 MB），应用启动时会阻塞在哈希计算上。但这是确保文件未被损坏的必要代价。

### 3.3 加载时规范化 (normalize_missing_asset)

每次 `load()` 都会检查 asset_path 指向的文件是否存在（`state.rs:46-68`）。如果状态是 `Downloaded`/`InstallScheduled`/`Installing` 但 asset 文件已不存在 → 转为 `Failed`，action=`retryDownload`。

### 3.4 恢复机制的局限性

恢复逻辑无法处理以下中间状态：

- **swap 已完成但 relaunch 未执行**：此时新版在 target_path，asset 完好，recover 建议 `retryInstall`。重试安装会对已更新的版本再次执行 swap（macOS 上再次交换，可能变回旧版）。
- **Windows NSIS 安装器被强杀**：部分文件已写入，旧版和新版文件混合。recover 只看 asset 是否完好，不了解文件系统的实际状态。
- **helper 写入 Installing 前崩溃**：状态停留在 `InstallScheduled`，recover 按此处理。但关键问题是 **此时 helper 是否已经在执行安装**——如果 recover 和 helper 并发读写 state.json，存在 lost update 风险。

---

## 四、macOS 分析

### 4.1 DMG/ZIP 安装流程

```
install_macos_bundle()                                 [helper.rs:412]
  │
  ├─ 创建 stage_root 临时目录
  │
  ├─ 格式分发
  │   ├─ .zip → extract_app_bundle_from_zip()
  │   │   └─ ditto -x -k <zip> <extract_root>
  │   │       └─ select_app_bundle() BFS 搜索 .app
  │   │
  │   └─ .dmg → stage_app_bundle_from_dmg()
  │       ├─ hdiutil attach -nobrowse -readonly -mountpoint <mp> <dmg>
  │       ├─ select_app_bundle() 在挂载点 BFS 搜索 .app
  │       ├─ ditto <mounted.app> <stage_root>/<name>.app
  │       ├─ hdiutil detach <mount_point>         ← let _ = 忽略失败!
  │       └─ rm -rf <mount_point>                 ← let _ = 忽略失败!
  │
  ├─ verify_macos_bundle(staged_bundle)
  │   ├─ codesign --verify --deep --strict         必须通过
  │   └─ spctl --assess --type execute             必须通过 (若 spctl 存在)
  │
  └─ swap_macos_bundles(target, staged)
      └─ renamex_np(staged, target, RENAME_SWAP)  原子交换 ✓
          交换后: target = 新版, staged = 旧版
```

### 4.2 重启动机制

```rust
// 新版是 .app bundle
/usr/bin/open <target_path>

// 新版是裸可执行文件
Command::new(target_path).spawn()
```

### 4.3 select_app_bundle 逻辑

```rust
// helper.rs:1118-1150
fn find_app_bundles(root) -> Vec<PathBuf> {
    // 如果 root 本身就是 .app → 直接返回
    // 否则 BFS 遍历，收集所有 .app bundle
    // 结果排序（不确定顺序）后返回
}

fn select_app_bundle(root, expected_name, log) -> Result<PathBuf> {
    let bundles = find_app_bundles(root);
    // 优先按 expected_name 匹配
    // 其次只有 1 个 bundle 时直接返回
    // 多个 bundle 且都不匹配 → 错误
}
```

### 4.4 弱点

#### [P0] swap 后 relaunch 失败 → 旧版被永久删除

**代码**: `helper.rs:412-457` + `983-999`

```
swap_macos_bundles 成功后:
  - target_path (如 /Applications/Floral Notepaper.app) = 新版
  - staged_bundle (临时目录) = 旧版

然后 helper.rs:447-448:
  fs::remove_dir_all(&staged_bundle);   ← 删除旧版!
  fs::remove_dir_all(&stage_root);

最后 relaunch_target → /usr/bin/open target_path → 可能失败:
  - 新版签名无效 (虽然 verify_macos_bundle 检查了，但可能不完整)
  - 新版需要新版 macOS (用户系统太旧)
  - open 命令本身不可用 (无 GUI 会话)
  - 新版二进制 crash on startup

结果: 旧版已删除，新版无法启动 → 应用彻底损坏
```

`renamex_np(RENAME_SWAP)` 本身是原子的，但问题在于 swap **之后**的处理：旧版被立即删除，无任何回滚机制。

#### [P0] DMG 挂载泄漏

**代码**: `helper.rs:555-628`

```
hdiutil detach → let _ = 静默忽略失败
remove_dir_all → let _ = 静默忽略失败
```

如果 ditto 复制途中出错（磁盘满），DMG 仍在被占用：

- `hdiutil detach` 失败 → 挂载点残留
- `remove_dir_all` 失败 → 临时目录残留
- helper 进程被 SIGKILL → 挂载永久泄漏直到重启

`cleanup_stale_macos_mounts` 在下次启动时清理，但匹配的前缀是 `.floral-notepaper-mounted-dmg-*`，而 `unique_temp_path` 实际生成的前缀是 `.floral-notepaper-mount-*`——**可能存在模式不匹配**。

#### [P1] find_app_bundle BFS 歧义

如果 DMG/ZIP 中包含多个 `.app`（如主应用 + 卸载程序），BFS 返回的第一个不一定正确。虽然有 `expected_name` 匹配，但如果两个 bundle 名称不同（如 "Floral Notepaper.app" 和 "Uninstall Floral Notepaper.app"），`expected_name` 参数来自 `target_path.file_name()`，只能匹配精确同名。

#### [P1] verify_macos_bundle 的双重验证可能过于严格

`codesign --verify --deep --strict` 和 `spctl --assess` 都要求通过。在以下场景可能误杀合法更新：

- 自签名开发者构建（codesign 可能显示 "not signed with Apple Developer ID"）
- spctl 在某些 macOS 配置下可能不存在或被禁用
- 企业内部分发证书（非 Mac App Store / Developer ID）

**但这两个检查都在 swap 之前执行**，所以如果验证失败，swap 不会发生，旧版安全。

#### [P2] ditto 复制整个 .app bundle 无进度反馈

低端 Mac 或机械硬盘上，对于包含大量资源的 bundle，ditto 复制到 staging 可能需要数十秒，用户看到的是无响应的界面。

---

## 五、Windows 分析

### 5.1 安装类型检测

```rust
// platform.rs:82-110
fn detect_install_kind(os, current_exe) {
    // macOS → MacosAppBundle
    // Windows:
    //   1. 查注册表 (4个根路径) 是否有卸载条目匹配当前 exe
    //   2. 路径包含 \program files\ 或 \appdata\local\programs\ → NSIS
    //   3. 其他 → Portable
}
```

### 5.2 NSIS/MSI 安装流程

```
install_windows_installer()                           [helper.rs:468]
  │
  ├─ .msi → msiexec.exe /i <msi> /passive /norestart
  │   └─ Command::new("msiexec.exe").spawn()
  │       └─ wait_for_installer_completion(≤15min)
  │
  ├─ .exe (NSIS) → <exe> /S
  │   └─ Command::new(&asset_path).arg("/S").spawn()
  │       └─ wait_for_installer_completion(≤15min)
  │
  ├─ 检查 installer exit code:
  │   ├─ 1602 → InstallerCancelled
  │   ├─ 1603 → InstallerFatal
  │   ├─ 1618 → InstallerBusy
  │   └─ 其他非0 → InstallerFailed
  │
  ├─ resolve_windows_launch_target(target_path)
  │   ├─ target_path 还存在? → 直接用
  │   └─ 不存在? → 查 4 个注册表根路径:
  │       reg query HKCU\Software\...\Uninstall /s /f <exe_name>
  │       reg query HKLM\Software\...\Uninstall /s /f <exe_name>
  │       reg query HKLM\...\WOW6432Node\...\Uninstall /s /f <exe_name>
  │       reg query HKCU\...\WOW6432Node\...\Uninstall /s /f <exe_name>
  │       └─ 解析 REG_SZ 值, 匹配 exe 文件名或目录+exe
  │
  └─ wait_for_target_to_exist(launch_target, ≤30s)
      └─ relaunch_target(launch_target)
```

### 5.3 进程存活检测

```rust
// helper.rs:815-841
// Windows: OpenProcess + WaitForSingleObject(0)
// Unix:    kill -0 <pid>
```

### 5.4 Portable 流程

Portable 在 `prepare_request()` 阶段直接返回 `portable_manual_only()` 错误，不进入安装流程。

### 5.5 弱点

#### [P0] MSI/NSIS 安装器静默返回成功但实际未安装

**代码**: `helper.rs:498-514`

```
installer exit code == 0 → 认为成功 → persist_success_state
  → current_version 更新为目标版本

但实际上:
  - MSI 同版本已安装 → 走修复模式, 不报错, 不实际安装
  - NSIS /S 非标 → 安装器不识别, 弹出 GUI, 用户点取消, 进程返回 0
  - 自定义 Action 内部失败但主安装器返回 0
```

结果：状态文件标记为新版本（`Idle`, `current_version = target_version`），但磁盘上的 exe 还是旧版。应用永远无法再次检测到此更新（因为版本号已经"更新"了）。

**无安装后版本验证**——没有读取新 exe 的文件版本信息 (`GetFileVersionInfoW`) 与 `target_version` 比对。

#### [P0] helper 脱离原安装目录运行，可能缺少 DLL 依赖

**代码**: `install.rs:323-325` (非 macOS 分支)

`stage_helper_copy` 对非 macOS 平台使用 `fs::copy` 只复制单个 exe 文件到 staging 目录：

```
Windows 上:
  current_exe = C:\Program Files\花笺\floral-notepaper.exe
  helper 被复制到: staging\embedded-update-helper-<nanos>.exe

  但 staging 目录中没有:
  - WebView2Loader.dll
  - 其他伴随 DLL
  - 运行时资源文件
```

如果二进制不是纯静态链接，helper 启动即崩溃。主进程 5s 超时后 report handshake failed。

#### [P0] /passive 模式下 UAC 行为不确定

`msiexec /passive` 需要管理员权限时：

- 桌面用户看到 UAC 弹窗 → 可以点"否" → 安装失败
- 锁屏/远程桌面 → UAC 弹窗无法交互 → 安装器挂起直到 15 分钟超时
- NSIS `/S` 同理

锁屏定时自动更新场景下，安装器会挂满 15 分钟然后被强杀。

#### [P1] 注册表查找匹配到旧版本或其他程序

**代码**: `helper.rs:720-774`

`resolve_windows_launch_target` 只按 exe 文件名（不检查 Publisher/DisplayName）搜索 4 个注册表路径：

- 用户安装多个版本（v1.0 在 Program Files，v1.1 在 AppData）→ 可能启动旧版
- 其他程序恰好有同名 exe → 启动错误程序
- 找到多个候选时取第一个（顺序取决于 reg query 输出）→ 不确定性

#### [P1] reg.exe 使用相对路径和路径注入风险

**代码**: `helper.rs:730` (`Command::new("reg")`)

- 未使用绝对路径 `C:\Windows\System32\reg.exe`
- 如果当前工作目录有恶意 `reg.exe`，会被优先执行
- `reg query` 输出解析依赖系统语言（中文版输出"类型" vs 英文版"Type"）

#### [P1] 15 分钟超时强杀安装器 → 文件系统不一致

**代码**: `helper.rs:643` (`child.kill()`)

NSIS 无事务支持，强杀后：

- 新 exe 可能已写入但 DLL 缺失 → 新旧混合 → 应用损坏
- MSI 有事务回滚，但回滚本身可能因磁盘满/权限问题失败

超时后走失败路径 → `cleanup_after_install` 删除 asset → 无法重试。

#### [P1] 安装类型误判

**代码**: `platform.rs:82-110`

NSIS 安装时用户选择自定义路径（如 `D:\MyTools\`），路径不包含 `\program files\` 等特征 → 被误判为 Portable。后续检查/下载流程中会被拒绝（`portable_manual_only`），但体验很差——没有引导用户去哪里手动下载。

#### [P2] NSIS /S 并非真静默

`/S` 仅在 NSIS 脚本使用 `SilentInstall silent` 时才真正静默。脚本可能包含：

- `MUI_PAGE_*` 自定义页面（未跳过）
- 卸载旧版本确认对话框
- 关闭应用提示
- 第三方嵌入包（VC++ Redist 等）

任何一个都会弹出阻塞对话框，导致 15 分钟超时。

---

## 六、跨平台通用问题

### 6.1 [P0] 主进程退出后 helper 崩溃 → 无兜底机制

**代码**: `commands.rs:169-175`

```
主进程:
  1. executor.execute() 成功 (ready marker 检测到)
  2. save InstallScheduled
  3. app.exit(0)   ← 主进程消失

helper:
  4. 在 ready marker 之后的任何阶段崩溃:
     - SHA256 校验完成后系统 OOM
     - 被杀毒软件拦截
     - 系统休眠/断电
     - ditto 复制失败

结果: 没有任何 watch dog 进程来重启原应用
下次启动: state::recover() 检测中断状态 → 转为 Failed
```

### 6.2 [P0] app.exit(0) 跳过析构 → 数据丢失风险

**代码**: `commands.rs:174`

`std::process::exit(0)` 不会运行栈上对象的 `Drop` 实现。`install_prepare` 协议只是通知前端 JavaScript 保存（且只有 10s 超时），不保证：

1. 前端 JS 主线程未被阻塞（模态对话框、同步 I/O）
2. `invoke("update_install_prepare_report")` 成功到达
3. Rust 侧 buffer 已 fsync 落盘

如果前端保存未完成，未保存的笔记内容直接丢失。

### 6.3 [P0] 失败路径删除 asset 下载包

**代码**: `helper.rs:261-288` + `983-999`

```
execute_apply:
  成功路径: persist_success_state → cleanup_after_install (删除 asset) → relaunch
  失败路径: persist_failed_state  → cleanup_after_install (删除 asset) → relaunch_existing
```

无论成功还是失败，`cleanup_after_install` 都执行 `fs::remove_dir_all(asset_path.parent())`。安装失败（如磁盘满、权限不足、解包失败）后 asset 被删除，用户必须重新下载。对于几百 MB 的安装包和网络受限用户，这意味着可能需要数小时的重试周期。

### 6.4 [P0] 握手超时 5s 过短 + SHA256 阻塞

**代码**: `install.rs:24` (`HELPER_READY_TIMEOUT`)

Helper 在写 ready marker 之前需要：

1. 打开日志文件
2. **计算整个 asset 文件的 SHA256**（`sha256_hex` 全文件读取）
3. 检查磁盘空间

对于 200MB+ 的 installer 在机械硬盘上，仅 SHA256 计算就可能 2-5 秒。加上日志初始化和磁盘空间检查，总时间很容易超过 5 秒。超时后主进程 `child.kill()` 杀 helper，但此时 helper 可能已在写文件。

### 6.5 [P1] 30s wait 超时导致双实例风险

**代码**: `helper.rs:660-672`

```
helper: wait_for_process_exit(wait_pid, 30s)
  超时 → WaitTimedOut → 失败路径 → relaunch_existing_target

如果主进程退出被阻塞:
  - macOS: iCloud 同步、网络请求挂起
  - 磁盘 flush 延迟

30s 后 helper 判定超时，启动旧版 app
但主进程可能还活着 → 两个实例同时操作 notesDir → 数据竞争
```

Tauri 的单实例锁能防止 `app.exit(0)` 之前的新实例启动，但 `relaunch_existing_target` 是通过 `Command::new(target_path).spawn()` 直接 spawn，可能绕过 Tauri 的锁检查。

### 6.6 [P1] state.json 双进程 read-modify-write 竞态

**代码**: `helper.rs:1002-1025`

主进程和 helper 子进程共享同一个 `state.json`，但没有任何进程间锁：

```
时间线:
  T1: 主进程 write_json_atomic(InstallScheduled)
  T2: helper load_state_snapshot()     ← 读到 InstallScheduled
  T3: helper persist_installing_state() ← 写 Installing

如果在 T2 和 T3 之间:
  - 用户快速重启 app → 新进程读 state.json → 修改 → 写回
  - helper 在 T3 写 Installing 时覆盖了新进程的修改 (lost update)
```

`write_json_atomic` 保证单文件写入原子性，但不能防止 read-modify-write 的丢失更新。

### 6.7 [P1] Mutex 中毒永久死锁

**代码**: `mod.rs:182` (`begin_task`)

```rust
let mut slot = self.active_task.lock()
    .map_err(|_| errors::app_error(...))?;
```

`std::sync::Mutex::lock()` 在持有者线程 panic 时返回 `PoisonError`。代码直接 `map_err` 转为错误返回，不调用 `into_inner()` 恢复锁。

一旦中毒，所有更新操作（检查、下载、安装）**永久不可用**，直到应用重启。

### 6.8 [P1] 无安装后版本验证

**代码**: `helper.rs:498-514` (Windows) / `helper.rs:449-456` (macOS)

Windows 上安装器完成后，仅检查 `launch_target.exists()` 文件存在即认为成功。没有：

- 读取文件版本信息 (Windows: `GetFileVersionInfoW`)
- 读取 Info.plist 的 `CFBundleShortVersionString` (macOS)
- 与 `target_version` 比对

macOS 上有 `verify_macos_bundle` (codesign + spctl)，但只验证签名不验证版本。

### 6.9 [P2] 安装进程无用户可见的进度反馈

安装阶段 helper 独立运行，主进程已退出。用户看到的是：点击按钮 → 界面关闭 → 等待 → 新应用启动（或没启动）。中间没有任何进度指示。

### 6.10 [P2] 前后端 canInstall 校验不一致

**前端** `UpdateSettingsSection.tsx`:

```ts
Boolean(status?.latestVersion && status?.assetPath);
```

**后端** `install.rs:167-176`:

```rust
asset_path?         // 必需 — 前端检查
asset_sha256?       // 必需 — 前端不检查!
asset_size?         // 必需 — 前端不检查!
latest_version?     // 必需 — 前端检查
```

状态文件部分损坏（`assetPath` 有值但 `assetSha256` 为空）时，前端显示按钮可点击，后端返回 `updateInstallNotReady`。

### 6.11 [P2] Test 模式已实现但主流程未使用

Helper 支持 `Test` 模式（只校验不替换），但 `UpdateInstallService::from_env()` 硬编码为 `Apply`。没有调用路径做安装前预检。

### 6.12 [P2] select_app_bundle 不确定顺序

`find_app_bundles` 收集结果后 `results.sort()`，但路径排序不保证语义正确性。多个 `.app` 时 "取第一个" 的行为依赖系统排序实现。

---

## 七、优先级汇总

### P0 — 必须修复（可能导致数据丢失或应用不可用）

| #   | 问题                                                            | 平台    | 关键位置            |
| --- | --------------------------------------------------------------- | ------- | ------------------- |
| 1   | swap 后 relaunch 失败 → 旧版被 `remove_dir_all` 删除，无回滚    | macOS   | helper.rs:447-448   |
| 2   | MSI/NSIS 返回 0 但未实际安装 → 状态标记为新版，永远无法再次更新 | Windows | helper.rs:498-514   |
| 3   | helper 脱离原目录运行 (fs::copy 单文件) → DLL 缺失启动崩溃      | Windows | install.rs:323-325  |
| 4   | 主进程 exit 后 helper 崩溃 → 无 watch dog                       | 全平台  | commands.rs:169-175 |
| 5   | `app.exit(0)` 跳过析构 → 未保存笔记丢失                         | 全平台  | commands.rs:174     |
| 6   | 失败路径删除 asset → 重试需重新下载                             | 全平台  | helper.rs:983-999   |

### P1 — 应该修复（可能导致功能不可用或体验严重受损）

| #   | 问题                                     | 平台    | 关键位置            |
| --- | ---------------------------------------- | ------- | ------------------- |
| 7   | DMG 挂载泄漏（detach 失败被静默忽略）    | macOS   | helper.rs:622-627   |
| 8   | 注册表查找匹配旧版/同名 exe              | Windows | helper.rs:720-774   |
| 9   | reg.exe 路径注入 + 输出语言依赖          | Windows | helper.rs:730       |
| 10  | UAC 导致安装器无限挂起 (15min 超时)      | Windows | helper.rs:468-496   |
| 11  | 30s wait 超时 → 双实例同时运行           | 全平台  | helper.rs:660-672   |
| 12  | state.json 双进程 read-modify-write 竞态 | 全平台  | helper.rs:1002-1025 |
| 13  | Mutex 中毒永久死锁                       | 全平台  | mod.rs:182          |
| 14  | 无安装后版本验证                         | 全平台  | helper.rs:498/449   |
| 15  | handshake 5s 超时 + SHA256 阻塞          | 全平台  | install.rs:24       |
| 16  | NSIS 自定义路径被误判为 Portable         | Windows | platform.rs:82-110  |
| 17  | 15min 超时强杀安装器 → NSIS 文件混合     | Windows | helper.rs:643       |

### P2 — 建议修复（边界情况或用户体验问题）

| #   | 问题                                           | 平台    | 关键位置                  |
| --- | ---------------------------------------------- | ------- | ------------------------- |
| 18  | 安装过程无用户可见进度反馈                     | 全平台  | 整体架构                  |
| 19  | find_app_bundle 多 .app 时的歧义               | macOS   | helper.rs:1085-1116       |
| 20  | 前后端 canInstall 校验不一致（缺 sha256/size） | 全平台  | UpdateSettingsSection.tsx |
| 21  | NSIS /S 非真正静默（自定义页面/弹窗）          | Windows | helper.rs:491-492         |
| 22  | Test 模式已实现但主流程未使用                  | 全平台  | install.rs:93             |
| 23  | ditto 复制大 bundle 无进度                     | macOS   | install.rs:345            |
| 24  | 4 次 reg query 调用性能 + 无超时控制           | Windows | helper.rs:722-727         |
| 25  | verify_macos_bundle 可能在非标准证书环境下误杀 | macOS   | helper.rs:1152-1180       |

---

## 附录 A：当前代码已验证的改进（相比早期版本）

以下问题在早期分析中被标记但当前代码已修复或已有缓解措施：

| 原标记 | 问题                                   | 当前状态                                                           |
| ------ | -------------------------------------- | ------------------------------------------------------------------ |
| 原 P0  | .app bundle 替换非原子 (两条 rename)   | **已修复**: 使用 `renamex_np(RENAME_SWAP)` 原子交换                |
| 原 P1  | 替换后无代码签名/Gatekeeper 验证       | **已修复**: `verify_macos_bundle` 执行 codesign + spctl            |
| 原 P1  | 无磁盘空间预检                         | **已修复**: `ensure_sufficient_disk_space` 要求 2×asset_size       |
| 原 P1  | Installing 恢复不保留资产              | **已修复**: `recover()` 检查 asset 完整性，完好则 `retryInstall`   |
| 原 P1  | macOS stage_helper_copy 只复制裸二进制 | **已修复**: macOS AppBundle 走 `ditto` 复制整个 .app               |
| 原 P2  | Windows 安装器无超时                   | **已修复**: 15 分钟 `INSTALLER_TIMEOUT`                            |
| 原 P2  | PowerShell 进程检测                    | **已修复**: Windows 用 `OpenProcess` API, Unix 用 `kill -0`        |
| 原 P1  | 下载/日志从不清理                      | **已修复**: `cleanup_after_install` + `prune_dir_entries` 定期清理 |

## 附录 B：关键文件索引

| 文件                                            | 职责                                                |
| ----------------------------------------------- | --------------------------------------------------- |
| `src-tauri/src/updater/commands.rs`             | Tauri 命令入口，install_prepare 窗口协调，事件发射  |
| `src-tauri/src/updater/install.rs`              | 安装服务：准备请求、暂存 helper、spawn 子进程、握手 |
| `src-tauri/src/updater/helper.rs`               | CLI 模式入口：校验、替换、回滚、重启、退出码映射    |
| `src-tauri/src/updater/download.rs`             | 下载服务：HTTP 下载、重试、进度、校验               |
| `src-tauri/src/updater/state.rs`                | 状态文件读/写、中断恢复、asset 完整性验证           |
| `src-tauri/src/updater/settings.rs`             | 设置文件读写、原子写工具 (write_json_atomic)        |
| `src-tauri/src/updater/mod.rs`                  | 路径管理、任务互斥锁、UpdaterState、artifact 清理   |
| `src-tauri/src/updater/platform.rs`             | 平台检测、安装类型推断、Asset 文件名解析            |
| `src-tauri/src/updater/types.rs`                | DTO 类型定义、状态/错误模型                         |
| `src/features/update/UpdateSettingsSection.tsx` | 前端 UI：按钮渲染、状态展示、事件监听               |
| `src/features/update/api.ts`                    | 前端 API 封装：invoke 调用                          |
