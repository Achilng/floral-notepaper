use super::{
    errors, helper,
    platform::{self, PlatformInfo},
    state,
    types::{UpdateErrorDto, UpdateInstallMode, UpdateInstallResult, UpdateStateDto, UpdateStatus},
    UpdatePaths,
};
use crate::services::notes::AppError;
use chrono::Utc;
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};

const UPDATE_HELPER_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_HELPER_PATH";
const UPDATE_HELPER_MODE_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_HELPER_MODE";

macro_rules! debug_log {
    ($($arg:tt)*) => { super::debug_log!("install", $($arg)*) };
}
const HELPER_READY_TIMEOUT: Duration = Duration::from_secs(30);
const HELPER_READY_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelperLaunchRequest {
    helper_path: PathBuf,
    command: helper::UpdateHelperCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelperLaunchOutcome;

pub(crate) trait InstallExecutor: Clone + Send + Sync + 'static {
    fn execute(&self, request: &HelperLaunchRequest) -> Result<HelperLaunchOutcome, AppError>;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessInstallExecutor;

impl InstallExecutor for ProcessInstallExecutor {
    fn execute(&self, request: &HelperLaunchRequest) -> Result<HelperLaunchOutcome, AppError> {
        if request.command.ready_path.exists() {
            let _ = fs::remove_file(&request.command.ready_path);
        }
        let relaunch_path = helper::relaunch_marker_path(&request.command);
        if relaunch_path.exists() {
            let _ = fs::remove_file(relaunch_path);
        }
        let completion_path = helper::completion_marker_path(&request.command);
        if completion_path.exists() {
            let _ = fs::remove_file(completion_path);
        }

        let mut child = Command::new(&request.helper_path)
            .args(helper_process_args(&request.command))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                errors::with_detail(
                    errors::app_error(
                        "updateInstallSpawnFailed",
                        format!("启动更新安装助手失败：{error}"),
                    ),
                    "helperPath",
                    request.helper_path.display().to_string(),
                )
            })?;

        wait_for_helper_ready(&mut child, request)?;
        if let Err(error) = spawn_helper_watchdog(request, child.id()) {
            let _ = child.kill();
            return Err(error);
        }

        Ok(HelperLaunchOutcome)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UpdateInstallService<E = ProcessInstallExecutor>
where
    E: InstallExecutor,
{
    helper_source_override: Option<PathBuf>,
    platform_override: Option<PlatformInfo>,
    helper_mode: helper::UpdateHelperMode,
    executor: E,
}

impl UpdateInstallService<ProcessInstallExecutor> {
    pub(crate) fn from_env() -> Self {
        Self {
            helper_source_override: env::var(UPDATE_HELPER_PATH_ENV)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            platform_override: None,
            helper_mode: helper_mode_from_env(),
            executor: ProcessInstallExecutor,
        }
    }
}

fn helper_mode_from_env() -> helper::UpdateHelperMode {
    env::var(UPDATE_HELPER_MODE_ENV)
        .ok()
        .as_deref()
        .and_then(helper::UpdateHelperMode::parse)
        .filter(|mode| {
            matches!(
                mode,
                helper::UpdateHelperMode::Apply | helper::UpdateHelperMode::Test
            )
        })
        .unwrap_or(helper::UpdateHelperMode::Apply)
}

impl<E> UpdateInstallService<E>
where
    E: InstallExecutor,
{
    #[cfg(test)]
    fn with_executor(
        executor: E,
        helper_mode: helper::UpdateHelperMode,
        helper_source_override: Option<PathBuf>,
        platform_override: Option<PlatformInfo>,
    ) -> Self {
        Self {
            helper_source_override,
            platform_override,
            helper_mode,
            executor,
        }
    }

    pub(crate) fn run(
        &self,
        paths: &UpdatePaths,
        current_state: UpdateStateDto,
    ) -> Result<UpdateInstallResult, AppError> {
        debug_log!(
            "开始安装 version={:?} mode={:?}",
            current_state.latest_version,
            self.helper_mode
        );
        let request = match self.prepare_request(paths, &current_state) {
            Ok(request) => {
                debug_log!(
                    "安装请求已准备 helper={} target={}",
                    request.helper_path.display(),
                    request.command.target_path.display()
                );
                request
            }
            Err(error) => {
                debug_log!("安装准备失败 code={}", error.code);
                state::save(paths, &failed_state_without_request(&current_state, &error))?;
                return Err(error);
            }
        };
        let log_path_text = request.command.log_path.to_string_lossy().to_string();
        let install_mode = install_mode(self.helper_mode);
        let started_at = Utc::now();
        if self.helper_mode == helper::UpdateHelperMode::Apply {
            state::save(
                paths,
                &scheduled_state(&current_state, &request, install_mode.clone(), started_at),
            )?;
        }
        debug_log!("启动 helper 进程 mode={:?}", self.helper_mode);
        match self.executor.execute(&request) {
            Ok(_) => {
                debug_log!(
                    "helper 已启动 mode={:?} log={}",
                    self.helper_mode,
                    log_path_text
                );
                if self.helper_mode == helper::UpdateHelperMode::Test {
                    state::save(
                        paths,
                        &validated_test_state(
                            &current_state,
                            &request,
                            install_mode.clone(),
                            started_at,
                        ),
                    )?;
                    return Ok(UpdateInstallResult {
                        status: UpdateStatus::Downloaded,
                        log_path: Some(log_path_text),
                        mode: install_mode,
                    });
                }

                if let Some(error) =
                    load_current_install_failure(paths, &current_state.current_version, &request)
                {
                    debug_log!("helper 已在退出前失败 code={}", error.code);
                    return Err(error);
                }
                debug_log!("安装已调度，等待退出");
                Ok(UpdateInstallResult {
                    status: UpdateStatus::InstallScheduled,
                    log_path: Some(log_path_text),
                    mode: install_mode,
                })
            }
            Err(error) => {
                debug_log!("安装执行失败 code={}", error.code);
                state::save(
                    paths,
                    &failed_state(&current_state, &request, install_mode, started_at, &error),
                )?;
                Err(error)
            }
        }
    }

    fn prepare_request(
        &self,
        paths: &UpdatePaths,
        current_state: &UpdateStateDto,
    ) -> Result<HelperLaunchRequest, AppError> {
        if !matches!(
            current_state.status,
            UpdateStatus::Downloaded | UpdateStatus::InstallScheduled | UpdateStatus::Failed
        ) {
            return Err(errors::app_error(
                "updateInstallNotReady",
                "当前没有可安装的更新包",
            ));
        }

        let asset_path = current_state
            .asset_path
            .as_ref()
            .ok_or_else(|| errors::app_error("updateInstallNotReady", "当前没有可安装的更新包"))?;
        let asset_sha256 = current_state
            .asset_sha256
            .as_ref()
            .ok_or_else(|| errors::app_error("updateInstallNotReady", "当前没有可安装的更新包"))?;
        let asset_size = current_state
            .asset_size
            .ok_or_else(|| errors::app_error("updateInstallNotReady", "当前没有可安装的更新包"))?;
        let target_version = current_state
            .latest_version
            .as_ref()
            .ok_or_else(|| errors::app_error("updateInstallNotReady", "当前没有可安装的更新包"))?;

        let platform = self.platform_override.clone().unwrap_or_else(|| {
            platform::current_platform_with_version(current_state.current_version.clone())
        });
        platform.ensure_in_app_updates_supported()?;

        let target_path = resolve_install_target(&platform)?;
        let helper_source_path = self.resolve_helper_source_path(&platform)?;
        let helper_path = stage_helper_copy(paths, &platform, &helper_source_path)?;
        let log_path = build_log_path(paths, target_version);
        let ready_path = build_ready_path(paths, target_version);

        Ok(HelperLaunchRequest {
            helper_path,
            command: helper::UpdateHelperCommand {
                mode: self.helper_mode,
                install_kind: platform.install_kind.clone(),
                wait_pid: std::process::id(),
                state_path: paths.state_path(),
                asset_path: PathBuf::from(asset_path),
                asset_sha256: asset_sha256.clone(),
                asset_size,
                target_path,
                log_path,
                ready_path,
                current_version: current_state.current_version.clone(),
                target_version: target_version.clone(),
            },
        })
    }

    fn resolve_helper_source_path(&self, platform: &PlatformInfo) -> Result<PathBuf, AppError> {
        if let Some(path) = self.helper_source_override.as_ref() {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(errors::with_detail(
                errors::app_error("updateHelperNotFound", "找不到更新安装助手可执行文件"),
                "helperPath",
                path.display().to_string(),
            ));
        }

        if platform.install_kind == super::types::InstallKind::MacosAppBundle {
            return resolve_macos_helper_source_path(platform).ok_or_else(|| {
                build_helper_not_found_error(platform, "找不到更新安装助手可执行文件")
            });
        }

        platform
            .current_exe
            .as_ref()
            .map(PathBuf::from)
            .filter(|path| path.exists())
            .ok_or_else(|| {
                errors::app_error("updateHelperNotFound", "找不到更新安装助手可执行文件")
            })
    }
}

fn wait_for_helper_ready(child: &mut Child, request: &HelperLaunchRequest) -> Result<(), AppError> {
    let deadline = std::time::Instant::now() + HELPER_READY_TIMEOUT;
    loop {
        if request.command.ready_path.exists() {
            return Ok(());
        }
        if let Some(status) = child.try_wait().map_err(|error| {
            errors::app_error(
                "updateInstallHelperHandshakeFailed",
                format!("更新安装助手在握手前退出：{error}"),
            )
        })? {
            return Err(errors::with_detail(
                errors::app_error(
                    "updateInstallHelperHandshakeFailed",
                    format!("更新安装助手在握手前退出，状态码：{:?}", status.code()),
                ),
                "helperPath",
                request.helper_path.display().to_string(),
            ));
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            return Err(errors::with_detail(
                errors::app_error(
                    "updateInstallHelperHandshakeFailed",
                    "等待更新安装助手就绪超时",
                ),
                "readyPath",
                request.command.ready_path.display().to_string(),
            ));
        }
        thread::sleep(HELPER_READY_POLL_INTERVAL);
    }
}

fn helper_process_args(command: &helper::UpdateHelperCommand) -> Vec<OsString> {
    let mut args = vec![OsString::from("--update-helper")];
    args.extend(command.to_args());
    args
}

fn spawn_helper_watchdog(request: &HelperLaunchRequest, helper_pid: u32) -> Result<(), AppError> {
    let mut watchdog_command = request.command.clone();
    watchdog_command.mode = helper::UpdateHelperMode::Watchdog;
    watchdog_command.wait_pid = helper_pid;

    Command::new(&request.helper_path)
        .args(helper_process_args(&watchdog_command))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            errors::with_detail(
                errors::app_error(
                    "updateInstallSpawnFailed",
                    format!("启动更新安装助手监控进程失败：{error}"),
                ),
                "helperPath",
                request.helper_path.display().to_string(),
            )
        })
}

fn resolve_install_target(platform: &PlatformInfo) -> Result<PathBuf, AppError> {
    match platform.install_kind {
        super::types::InstallKind::MacosAppBundle => {
            resolve_macos_bundle_path(platform).ok_or_else(errors::unsupported_platform)
        }
        super::types::InstallKind::WindowsNsis => platform
            .current_exe
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(errors::unsupported_platform),
        super::types::InstallKind::WindowsPortable => Err(errors::portable_manual_only()),
        super::types::InstallKind::Unknown => Err(errors::unsupported_platform()),
    }
}

fn resolve_macos_bundle_path(platform: &PlatformInfo) -> Option<PathBuf> {
    resolve_macos_bundle_path_in(platform, &macos_bundle_search_dirs())
}

fn resolve_macos_bundle_path_in(
    platform: &PlatformInfo,
    search_dirs: &[PathBuf],
) -> Option<PathBuf> {
    macos_bundle_candidates(platform, search_dirs)
        .into_iter()
        .find(|path| path.exists())
}

fn resolve_macos_helper_source_path(platform: &PlatformInfo) -> Option<PathBuf> {
    resolve_macos_helper_source_path_in(platform, &macos_bundle_search_dirs())
}

fn resolve_macos_helper_source_path_in(
    platform: &PlatformInfo,
    search_dirs: &[PathBuf],
) -> Option<PathBuf> {
    for bundle in macos_bundle_candidates(platform, search_dirs) {
        let helper_path = macos_helper_executable_path(&bundle);
        if helper_path.exists() {
            return Some(helper_path);
        }
    }
    resolve_macos_bundle_path_in(platform, search_dirs)
}

fn macos_bundle_search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home_dir) = dirs::home_dir() {
        dirs.push(home_dir.join("Applications"));
    }
    dirs
}

fn macos_bundle_candidates(platform: &PlatformInfo, search_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(bundle) = platform.current_app_bundle.as_ref().map(PathBuf::from) {
        push_unique_candidate(&mut candidates, bundle);
    }

    if let Some(bundle) = platform
        .current_exe
        .as_deref()
        .map(Path::new)
        .and_then(find_bundle_path_from_executable_path)
    {
        push_unique_candidate(&mut candidates, bundle);
    }

    if let Some(bundle_name) = current_bundle_name(platform) {
        for search_dir in search_dirs {
            push_unique_candidate(&mut candidates, search_dir.join(&bundle_name));
        }
    }

    candidates
}

fn push_unique_candidate(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn current_bundle_name(platform: &PlatformInfo) -> Option<std::ffi::OsString> {
    platform
        .current_app_bundle
        .as_deref()
        .map(Path::new)
        .and_then(Path::file_name)
        .map(|value| value.to_os_string())
        .or_else(|| {
            platform
                .current_exe
                .as_deref()
                .map(Path::new)
                .and_then(find_bundle_path_from_executable_path)
                .and_then(|path| path.file_name().map(|value| value.to_os_string()))
        })
}

fn find_bundle_path_from_executable_path(executable_path: &Path) -> Option<PathBuf> {
    platform::find_macos_app_bundle(executable_path)
}

fn build_helper_not_found_error(platform: &PlatformInfo, message: &str) -> AppError {
    let attempted_paths = macos_bundle_candidates(platform, &macos_bundle_search_dirs())
        .into_iter()
        .flat_map(|bundle_path| {
            [
                macos_helper_executable_path(&bundle_path)
                    .display()
                    .to_string(),
                bundle_path.display().to_string(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let mut error = errors::app_error("updateHelperNotFound", message);
    if !attempted_paths.is_empty() {
        error = errors::with_detail(error, "attemptedPaths", attempted_paths);
    }
    error
}

fn macos_helper_executable_path(bundle_path: &Path) -> PathBuf {
    bundle_path
        .join("Contents")
        .join("MacOS")
        .join(helper::HELPER_BINARY_NAME)
}

fn stage_helper_copy(
    paths: &UpdatePaths,
    platform: &PlatformInfo,
    source_path: &Path,
) -> Result<PathBuf, AppError> {
    if platform.install_kind == super::types::InstallKind::MacosAppBundle
        && source_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
    {
        return stage_helper_bundle_copy(platform, source_path, paths);
    }
    if platform.install_kind == super::types::InstallKind::WindowsNsis {
        return stage_windows_helper_copy(paths, source_path);
    }

    let extension = source_path.extension().and_then(|ext| ext.to_str());
    let staged_path = unique_helper_path(paths, extension);
    fs::copy(source_path, &staged_path).map_err(|error| {
        let error = errors::with_detail(
            errors::app_error(
                "updateInstallSpawnFailed",
                format!("准备更新安装助手失败：{error}"),
            ),
            "helperSourcePath",
            source_path.display().to_string(),
        );
        errors::with_detail(error, "helperPath", staged_path.display().to_string())
    })?;
    Ok(staged_path)
}

fn stage_windows_helper_copy(paths: &UpdatePaths, source_path: &Path) -> Result<PathBuf, AppError> {
    let staged_dir = unique_helper_path(paths, Some("dir"));
    fs::create_dir_all(&staged_dir).map_err(|error| {
        errors::with_detail(
            errors::app_error(
                "updateInstallSpawnFailed",
                format!("准备更新安装助手目录失败：{error}"),
            ),
            "helperPath",
            staged_dir.display().to_string(),
        )
    })?;

    let source_ext = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("exe");
    let staged_exe = staged_dir.join(format!("floral-update-helper.{source_ext}"));
    copy_helper_file(source_path, &staged_exe)?;

    if let Some(source_dir) = source_path.parent() {
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path == source_path || !is_windows_runtime_dependency(&path) {
                continue;
            }
            copy_helper_file(&path, &staged_dir.join(entry.file_name()))?;
        }
    }

    Ok(staged_exe)
}

fn copy_helper_file(source_path: &Path, staged_path: &Path) -> Result<(), AppError> {
    fs::copy(source_path, staged_path)
        .map(|_| ())
        .map_err(|error| {
            let error = errors::with_detail(
                errors::app_error(
                    "updateInstallSpawnFailed",
                    format!("准备更新安装助手失败：{error}"),
                ),
                "helperSourcePath",
                source_path.display().to_string(),
            );
            errors::with_detail(error, "helperPath", staged_path.display().to_string())
        })
}

fn is_windows_runtime_dependency(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
}

fn stage_helper_bundle_copy(
    platform: &PlatformInfo,
    source_bundle: &Path,
    paths: &UpdatePaths,
) -> Result<PathBuf, AppError> {
    let staged_bundle = unique_helper_path(paths, Some("app"));
    Command::new("/usr/bin/ditto")
        .arg(source_bundle)
        .arg(&staged_bundle)
        .status()
        .map_err(|error| {
            errors::with_detail(
                errors::app_error(
                    "updateInstallSpawnFailed",
                    format!("准备更新安装助手失败：{error}"),
                ),
                "helperSourcePath",
                source_bundle.display().to_string(),
            )
        })
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(errors::app_error(
                    "updateInstallSpawnFailed",
                    "复制更新安装助手应用包失败",
                ))
            }
        })?;

    let executable_name = platform
        .current_exe
        .as_deref()
        .and_then(|file| Path::new(file).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or(helper::HELPER_BINARY_NAME);
    let executable_path = staged_bundle
        .join("Contents")
        .join("MacOS")
        .join(executable_name);
    if executable_path.exists() {
        Ok(executable_path)
    } else {
        Err(errors::with_detail(
            errors::app_error("updateInstallSpawnFailed", "复制后的更新安装助手不可执行"),
            "helperPath",
            executable_path.display().to_string(),
        ))
    }
}

fn unique_helper_path(paths: &UpdatePaths, extension: Option<&str>) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut file_name = format!("embedded-update-helper-{unique}");
    if let Some(extension) = extension.filter(|value| !value.is_empty()) {
        file_name.push('.');
        file_name.push_str(extension);
    }
    paths.staging_dir().join(file_name)
}

fn build_ready_path(paths: &UpdatePaths, version: &str) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let version = sanitize_segment(version);
    paths
        .staging_dir()
        .join(format!("helper-ready-{version}-{timestamp}.marker"))
}

fn build_log_path(paths: &UpdatePaths, version: &str) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let version = sanitize_segment(version);
    paths
        .logs_dir()
        .join(format!("install-{version}-{timestamp}.log"))
}

fn sanitize_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".into()
    } else {
        sanitized
    }
}

fn install_mode(mode: helper::UpdateHelperMode) -> UpdateInstallMode {
    match mode {
        helper::UpdateHelperMode::Apply => UpdateInstallMode::Apply,
        helper::UpdateHelperMode::Test => UpdateInstallMode::Test,
        helper::UpdateHelperMode::Watchdog => UpdateInstallMode::Apply,
    }
}

fn scheduled_state(
    current_state: &UpdateStateDto,
    request: &HelperLaunchRequest,
    install_mode: UpdateInstallMode,
    started_at: chrono::DateTime<Utc>,
) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::InstallScheduled,
        current_version: current_state.current_version.clone(),
        latest_version: current_state.latest_version.clone(),
        channel: current_state.channel.clone(),
        asset_name: current_state.asset_name.clone(),
        asset_path: current_state.asset_path.clone(),
        asset_sha256: current_state.asset_sha256.clone(),
        asset_size: current_state.asset_size,
        asset_url: current_state.asset_url.clone(),
        source: current_state.source.clone(),
        checked_at: current_state.checked_at,
        downloaded_at: current_state.downloaded_at,
        install_log_path: Some(request.command.log_path.to_string_lossy().to_string()),
        install_mode: Some(install_mode),
        install_started_at: None,
        install_scheduled_at: Some(started_at),
        last_error: None,
    }
}

fn validated_test_state(
    current_state: &UpdateStateDto,
    request: &HelperLaunchRequest,
    install_mode: UpdateInstallMode,
    started_at: chrono::DateTime<Utc>,
) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::Downloaded,
        current_version: current_state.current_version.clone(),
        latest_version: current_state.latest_version.clone(),
        channel: current_state.channel.clone(),
        asset_name: current_state.asset_name.clone(),
        asset_path: current_state.asset_path.clone(),
        asset_sha256: current_state.asset_sha256.clone(),
        asset_size: current_state.asset_size,
        asset_url: current_state.asset_url.clone(),
        source: current_state.source.clone(),
        checked_at: current_state.checked_at,
        downloaded_at: current_state.downloaded_at,
        install_log_path: Some(request.command.log_path.to_string_lossy().to_string()),
        install_mode: Some(install_mode),
        install_started_at: Some(started_at),
        install_scheduled_at: None,
        last_error: None,
    }
}

fn failed_state(
    current_state: &UpdateStateDto,
    request: &HelperLaunchRequest,
    install_mode: UpdateInstallMode,
    started_at: chrono::DateTime<Utc>,
    error: &AppError,
) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::Failed,
        current_version: current_state.current_version.clone(),
        latest_version: current_state.latest_version.clone(),
        channel: current_state.channel.clone(),
        asset_name: current_state.asset_name.clone(),
        asset_path: current_state.asset_path.clone(),
        asset_sha256: current_state.asset_sha256.clone(),
        asset_size: current_state.asset_size,
        asset_url: current_state.asset_url.clone(),
        source: current_state.source.clone(),
        checked_at: current_state.checked_at,
        downloaded_at: current_state.downloaded_at,
        install_log_path: Some(request.command.log_path.to_string_lossy().to_string()),
        install_mode: Some(install_mode),
        install_started_at: Some(started_at),
        install_scheduled_at: None,
        last_error: Some(UpdateErrorDto::recoverable(
            error.code.clone(),
            error.message.clone(),
            Some(install_failure_action(&error.code).into()),
        )),
    }
}

fn failed_state_without_request(
    current_state: &UpdateStateDto,
    error: &AppError,
) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::Failed,
        current_version: current_state.current_version.clone(),
        latest_version: current_state.latest_version.clone(),
        channel: current_state.channel.clone(),
        asset_name: current_state.asset_name.clone(),
        asset_path: current_state.asset_path.clone(),
        asset_sha256: current_state.asset_sha256.clone(),
        asset_size: current_state.asset_size,
        asset_url: current_state.asset_url.clone(),
        source: current_state.source.clone(),
        checked_at: current_state.checked_at,
        downloaded_at: current_state.downloaded_at,
        install_log_path: None,
        install_mode: None,
        install_started_at: None,
        install_scheduled_at: None,
        last_error: Some(UpdateErrorDto::recoverable(
            error.code.clone(),
            error.message.clone(),
            Some(install_failure_action(&error.code).into()),
        )),
    }
}

fn install_failure_action(code: &str) -> &'static str {
    match code {
        "updateInstallAssetMissing"
        | "updateInstallAssetSizeMismatch"
        | "updateInstallAssetHashMismatch"
        | "updateInstallAssetExtractFailed" => "retryDownload",
        "updatePlatformUnsupported" | "updatePortableManualOnly" => "useSupportedInstall",
        _ => "retryInstall",
    }
}

fn load_current_install_failure(
    paths: &UpdatePaths,
    current_version: &str,
    request: &HelperLaunchRequest,
) -> Option<AppError> {
    let log_path = request.command.log_path.to_string_lossy().to_string();
    let saved_state = state::load_with_current_version(paths, current_version).ok()?;
    if saved_state.status != UpdateStatus::Failed {
        return None;
    }
    if saved_state.install_log_path.as_deref() != Some(log_path.as_str()) {
        return None;
    }
    let error = saved_state.last_error?;
    Some(errors::app_error(error.code, error.message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::settings::write_json_atomic;
    use crate::updater::types::{DownloadSourceUsed, UpdateChannel};
    use std::{fs, sync::Arc};

    #[derive(Debug, Clone)]
    enum FakeExecutorResult {
        Success,
        PersistFailedStateThenSuccess,
        Error(AppError),
    }

    #[derive(Debug, Clone)]
    struct FakeExecutor {
        result: FakeExecutorResult,
        calls: Arc<std::sync::Mutex<Vec<HelperLaunchRequest>>>,
    }

    impl FakeExecutor {
        fn new(result: FakeExecutorResult) -> Self {
            Self {
                result,
                calls: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn calls(&self) -> Vec<HelperLaunchRequest> {
            self.calls.lock().expect("calls lock").clone()
        }
    }

    impl InstallExecutor for FakeExecutor {
        fn execute(&self, request: &HelperLaunchRequest) -> Result<HelperLaunchOutcome, AppError> {
            self.calls.lock().expect("calls lock").push(request.clone());
            match &self.result {
                FakeExecutorResult::Success => Ok(HelperLaunchOutcome),
                FakeExecutorResult::PersistFailedStateThenSuccess => {
                    let failed_state = UpdateStateDto {
                        status: UpdateStatus::Failed,
                        current_version: request.command.current_version.clone(),
                        latest_version: Some(request.command.target_version.clone()),
                        install_log_path: Some(
                            request.command.log_path.to_string_lossy().to_string(),
                        ),
                        install_mode: Some(UpdateInstallMode::Apply),
                        last_error: Some(UpdateErrorDto::recoverable(
                            "updateInstallInstallerFailed",
                            "更新安装程序执行失败",
                            Some("retryInstall".into()),
                        )),
                        ..UpdateStateDto::idle_with_version(request.command.current_version.clone())
                    };
                    write_json_atomic(&request.command.state_path, &failed_state)
                        .expect("persist failed helper state");
                    Ok(HelperLaunchOutcome)
                }
                FakeExecutorResult::Error(error) => Err(error.clone()),
            }
        }
    }

    fn test_paths(name: &str) -> UpdatePaths {
        let root = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale test dir");
        }
        UpdatePaths::new(root)
    }

    fn downloaded_state(paths: &UpdatePaths) -> UpdateStateDto {
        let asset_path = paths.downloads_dir().join("1.0.5").join("asset.zip");
        fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("create asset dir");
        fs::write(&asset_path, b"downloaded asset").expect("write asset");

        UpdateStateDto {
            status: UpdateStatus::Downloaded,
            current_version: "1.0.3".into(),
            latest_version: Some("1.0.5".into()),
            channel: UpdateChannel::Stable,
            asset_name: Some("asset.zip".into()),
            asset_path: Some(asset_path.to_string_lossy().to_string()),
            asset_sha256: Some("abc".repeat(21) + "a"),
            asset_size: Some(16),
            asset_url: None,
            source: Some(DownloadSourceUsed::Github),
            checked_at: Some(Utc::now()),
            downloaded_at: Some(Utc::now()),
            install_log_path: None,
            install_mode: None,
            install_started_at: None,
            install_scheduled_at: None,
            last_error: None,
        }
    }

    fn platform_with_bundle(bundle: &Path) -> PlatformInfo {
        PlatformInfo {
            os: platform::Os::Macos,
            arch: platform::Arch::Aarch64,
            app_version: "1.0.3".into(),
            app_id: super::super::APP_ID.into(),
            install_kind: super::super::types::InstallKind::MacosAppBundle,
            current_exe: Some(
                bundle
                    .join("Contents")
                    .join("MacOS")
                    .join("floral-notepaper")
                    .to_string_lossy()
                    .to_string(),
            ),
            current_app_bundle: Some(bundle.to_string_lossy().to_string()),
        }
    }

    fn write_bundle_with_helper(bundle: &Path) {
        let app_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join("floral-notepaper");
        let helper_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join(helper::HELPER_BINARY_NAME);
        fs::create_dir_all(app_binary.parent().expect("app binary parent")).expect("create bundle");
        fs::write(&app_binary, b"main app placeholder").expect("write app placeholder");
        fs::write(&helper_binary, b"helper placeholder").expect("write helper placeholder");
    }

    #[test]
    fn launches_real_install_helper_after_staging_copy() {
        let paths = test_paths("install-success");
        paths.ensure_dirs().expect("ensure dirs");
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        write_bundle_with_helper(&bundle);
        let app_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join("floral-notepaper");

        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let service = UpdateInstallService::with_executor(
            executor.clone(),
            helper::UpdateHelperMode::Apply,
            Some(app_binary.clone()),
            Some(platform_with_bundle(&bundle)),
        );
        let state = downloaded_state(&paths);

        let result = service
            .run(&paths, state.clone())
            .expect("install launch should succeed");

        let saved = state::load(&paths).expect("load saved state");
        assert_eq!(result.status, UpdateStatus::InstallScheduled);
        assert_eq!(result.mode, UpdateInstallMode::Apply);
        assert_eq!(saved.status, UpdateStatus::InstallScheduled);
        assert_eq!(saved.latest_version, state.latest_version);
        assert!(saved.install_log_path.is_some());

        let calls = executor.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].command.mode, helper::UpdateHelperMode::Apply);
        assert_eq!(
            calls[0].command.install_kind,
            super::super::types::InstallKind::MacosAppBundle
        );
        assert_eq!(calls[0].command.state_path, paths.state_path());
        assert!(calls[0].command.ready_path.starts_with(paths.staging_dir()));
        assert!(calls[0].helper_path.exists());
        assert_ne!(calls[0].helper_path, app_binary);
    }

    #[test]
    fn test_mode_keeps_downloaded_state_after_validation() {
        let paths = test_paths("install-test-mode");
        paths.ensure_dirs().expect("ensure dirs");
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        write_bundle_with_helper(&bundle);
        let app_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join("floral-notepaper");

        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let service = UpdateInstallService::with_executor(
            executor.clone(),
            helper::UpdateHelperMode::Test,
            Some(app_binary),
            Some(platform_with_bundle(&bundle)),
        );

        let result = service
            .run(&paths, downloaded_state(&paths))
            .expect("test validation should succeed");

        let saved = state::load(&paths).expect("load saved state");
        assert_eq!(result.status, UpdateStatus::Downloaded);
        assert_eq!(result.mode, UpdateInstallMode::Test);
        assert_eq!(saved.status, UpdateStatus::Downloaded);
        assert_eq!(saved.install_mode, Some(UpdateInstallMode::Test));
        assert!(saved.install_scheduled_at.is_none());
    }

    #[test]
    fn resolves_macos_helper_and_target_from_fallback_search_dir() {
        let paths = test_paths("install-macos-fallback-bundle");
        let fallback_dir = paths.root_dir().join("Applications");
        let actual_bundle = fallback_dir.join("花笺.app");
        write_bundle_with_helper(&actual_bundle);

        let stale_bundle = paths.root_dir().join("gone").join("花笺.app");
        let platform = platform_with_bundle(&stale_bundle);

        let resolved_bundle =
            resolve_macos_bundle_path_in(&platform, std::slice::from_ref(&fallback_dir))
                .expect("resolve fallback bundle");
        let resolved_helper =
            resolve_macos_helper_source_path_in(&platform, std::slice::from_ref(&fallback_dir))
                .expect("resolve fallback helper");

        assert_eq!(resolved_bundle, actual_bundle);
        assert_eq!(
            resolved_helper,
            actual_bundle
                .join("Contents")
                .join("MacOS")
                .join(helper::HELPER_BINARY_NAME)
        );
    }

    #[test]
    fn rejects_install_without_downloaded_asset() {
        let paths = test_paths("install-not-ready");
        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::Test,
            Some(paths.root_dir().join("helper")),
            None,
        );

        let error = service
            .run(&paths, UpdateStateDto::idle())
            .expect_err("idle state should fail");

        assert_eq!(error.code, "updateInstallNotReady");
    }

    #[test]
    fn helper_source_override_must_exist() {
        let paths = test_paths("install-helper-missing");
        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        fs::create_dir_all(bundle.join("Contents").join("MacOS")).expect("create bundle");
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::Apply,
            Some(paths.root_dir().join("missing-helper")),
            Some(platform_with_bundle(&bundle)),
        );

        let error = service
            .run(&paths, downloaded_state(&paths))
            .expect_err("missing helper should fail");

        assert_eq!(error.code, "updateHelperNotFound");
    }

    #[test]
    fn preserves_spawn_failures_as_failed_state() {
        let paths = test_paths("install-spawn-failure");
        paths.ensure_dirs().expect("ensure dirs");
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        write_bundle_with_helper(&bundle);
        let app_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join("floral-notepaper");
        let executor = FakeExecutor::new(FakeExecutorResult::Error(errors::app_error(
            "updateInstallSpawnFailed",
            "启动更新安装助手失败：boom",
        )));
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::Apply,
            Some(app_binary),
            Some(platform_with_bundle(&bundle)),
        );

        let error = service
            .run(&paths, downloaded_state(&paths))
            .expect_err("spawn failure should surface");

        let saved = state::load(&paths).expect("load failed state");
        assert_eq!(error.code, "updateInstallSpawnFailed");
        assert_eq!(saved.status, UpdateStatus::Failed);
    }

    #[test]
    fn surfaces_helper_failure_written_before_handoff_completes() {
        let paths = test_paths("install-helper-prehandoff-failure");
        paths.ensure_dirs().expect("ensure dirs");
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        write_bundle_with_helper(&bundle);
        let app_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join("floral-notepaper");
        let executor = FakeExecutor::new(FakeExecutorResult::PersistFailedStateThenSuccess);
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::Apply,
            Some(app_binary),
            Some(platform_with_bundle(&bundle)),
        );

        let error = service
            .run(&paths, downloaded_state(&paths))
            .expect_err("persisted helper failure should surface");

        let saved = state::load(&paths).expect("load failed state");
        assert_eq!(error.code, "updateInstallInstallerFailed");
        assert_eq!(saved.status, UpdateStatus::Failed);
        assert_eq!(
            saved.last_error.as_ref().map(|error| error.code.as_str()),
            Some("updateInstallInstallerFailed")
        );
    }

    #[test]
    fn stages_helper_copy_in_updater_staging_directory() {
        let paths = test_paths("install-stage-helper");
        paths.ensure_dirs().expect("ensure dirs");
        let source = paths.root_dir().join("current-helper");
        fs::write(&source, b"helper copy source").expect("write helper source");

        let staged = stage_helper_copy(
            &paths,
            &PlatformInfo {
                os: platform::Os::Unsupported,
                arch: platform::Arch::Unsupported,
                app_version: "1.0.3".into(),
                app_id: super::super::APP_ID.into(),
                install_kind: super::super::types::InstallKind::Unknown,
                current_exe: Some(source.to_string_lossy().to_string()),
                current_app_bundle: None,
            },
            &source,
        )
        .expect("stage helper");

        assert!(staged.starts_with(paths.staging_dir()));
        assert_eq!(
            fs::read(&staged).expect("read staged helper"),
            b"helper copy source"
        );
    }

    #[test]
    fn stages_windows_helper_with_adjacent_dll_dependencies() {
        let paths = test_paths("install-stage-windows-helper");
        paths.ensure_dirs().expect("ensure dirs");
        let source_dir = paths.root_dir().join("installed");
        fs::create_dir_all(&source_dir).expect("create source dir");
        let source = source_dir.join("floral-notepaper.exe");
        let dll = source_dir.join("WebView2Loader.dll");
        let ignored = source_dir.join("notes.txt");
        fs::write(&source, b"helper exe").expect("write helper source");
        fs::write(&dll, b"dll dependency").expect("write dll");
        fs::write(&ignored, b"not needed").expect("write ignored file");

        let staged = stage_helper_copy(
            &paths,
            &PlatformInfo {
                os: platform::Os::Windows,
                arch: platform::Arch::X86_64,
                app_version: "1.0.3".into(),
                app_id: super::super::APP_ID.into(),
                install_kind: super::super::types::InstallKind::WindowsNsis,
                current_exe: Some(source.to_string_lossy().to_string()),
                current_app_bundle: None,
            },
            &source,
        )
        .expect("stage helper");

        let staged_dir = staged.parent().expect("staged helper parent");
        assert_eq!(
            staged.file_name().and_then(|n| n.to_str()),
            Some("floral-update-helper.exe")
        );
        assert_eq!(fs::read(&staged).expect("read staged exe"), b"helper exe");
        assert_eq!(
            fs::read(staged_dir.join("WebView2Loader.dll")).expect("read staged dll"),
            b"dll dependency"
        );
        assert!(!staged_dir.join("notes.txt").exists());
    }
}
