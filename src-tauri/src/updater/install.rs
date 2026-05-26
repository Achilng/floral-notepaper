use super::{
    errors, helper,
    platform::{self, PlatformInfo},
    state,
    types::{UpdateErrorDto, UpdateInstallMode, UpdateInstallResult, UpdateStateDto, UpdateStatus},
    UpdatePaths,
};
use crate::services::notes::AppError;
use chrono::Utc;
use std::{env, path::PathBuf, process::Command};

const UPDATE_HELPER_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_HELPER_PATH";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelperLaunchRequest {
    helper_path: PathBuf,
    command: helper::UpdateHelperCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelperLaunchOutcome {
    exit_code: i32,
}

pub(crate) trait InstallExecutor: Clone + Send + Sync + 'static {
    fn execute(&self, request: &HelperLaunchRequest) -> Result<HelperLaunchOutcome, AppError>;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessInstallExecutor;

impl InstallExecutor for ProcessInstallExecutor {
    fn execute(&self, request: &HelperLaunchRequest) -> Result<HelperLaunchOutcome, AppError> {
        let status = Command::new(&request.helper_path)
            .args(request.command.to_args())
            .status()
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

        Ok(HelperLaunchOutcome {
            exit_code: status.code().unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UpdateInstallService<E = ProcessInstallExecutor>
where
    E: InstallExecutor,
{
    helper_path_override: Option<PathBuf>,
    platform_override: Option<PlatformInfo>,
    helper_mode: helper::UpdateHelperMode,
    executor: E,
}

impl UpdateInstallService<ProcessInstallExecutor> {
    pub(crate) fn from_env() -> Self {
        Self {
            helper_path_override: env::var(UPDATE_HELPER_PATH_ENV)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            platform_override: None,
            helper_mode: helper::UpdateHelperMode::DryRun,
            executor: ProcessInstallExecutor,
        }
    }
}

impl<E> UpdateInstallService<E>
where
    E: InstallExecutor,
{
    #[cfg(test)]
    fn with_executor(
        executor: E,
        helper_mode: helper::UpdateHelperMode,
        helper_path_override: Option<PathBuf>,
        platform_override: Option<PlatformInfo>,
    ) -> Self {
        Self {
            helper_path_override,
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
        let request = self.prepare_request(paths, &current_state)?;
        let log_path_text = request.command.log_path.to_string_lossy().to_string();
        let install_mode = install_mode(self.helper_mode);
        let started_at = Utc::now();
        state::save(
            paths,
            &installing_state(&current_state, &request, install_mode.clone(), started_at),
        )?;

        let execution_result = self.executor.execute(&request);
        match execution_result {
            Ok(outcome) if outcome.exit_code == helper::UpdateHelperExitCode::Success.as_i32() => {
                let scheduled_at = Utc::now();
                state::save(
                    paths,
                    &scheduled_state(
                        &current_state,
                        &request,
                        install_mode.clone(),
                        started_at,
                        scheduled_at,
                    ),
                )?;

                Ok(UpdateInstallResult {
                    status: UpdateStatus::InstallScheduled,
                    log_path: Some(log_path_text),
                    mode: install_mode,
                })
            }
            Ok(outcome) => {
                let error = map_helper_exit_code(outcome.exit_code, &request.command);
                state::save(
                    paths,
                    &failed_state(&current_state, &request, install_mode, started_at, &error),
                )?;
                Err(error)
            }
            Err(error) => {
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
            UpdateStatus::Downloaded | UpdateStatus::InstallScheduled
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

        let platform = self
            .platform_override
            .clone()
            .unwrap_or_else(platform::current_platform);
        if !platform.supports_update_assets() {
            return Err(errors::unsupported_platform());
        }

        let target_path = resolve_install_target(&platform)?;
        let helper_path = self.resolve_helper_path(&platform)?;
        let log_path = build_log_path(paths, target_version);

        Ok(HelperLaunchRequest {
            helper_path,
            command: helper::UpdateHelperCommand {
                mode: self.helper_mode,
                asset_path: PathBuf::from(asset_path),
                asset_sha256: asset_sha256.clone(),
                asset_size,
                target_path,
                log_path,
                current_version: current_state.current_version.clone(),
                target_version: target_version.clone(),
            },
        })
    }

    fn resolve_helper_path(&self, platform: &PlatformInfo) -> Result<PathBuf, AppError> {
        if let Some(path) = self.helper_path_override.as_ref() {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(errors::with_detail(
                errors::app_error("updateHelperNotFound", "找不到更新安装助手可执行文件"),
                "helperPath",
                path.display().to_string(),
            ));
        }

        for candidate in helper_candidates(platform) {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(errors::app_error(
            "updateHelperNotFound",
            "找不到更新安装助手可执行文件",
        ))
    }
}

fn resolve_install_target(platform: &PlatformInfo) -> Result<PathBuf, AppError> {
    match platform.install_kind {
        super::types::InstallKind::MacosAppBundle => platform
            .current_app_bundle
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(errors::unsupported_platform),
        super::types::InstallKind::WindowsNsis | super::types::InstallKind::WindowsPortable => {
            platform
                .current_exe
                .as_ref()
                .map(PathBuf::from)
                .ok_or_else(errors::unsupported_platform)
        }
        super::types::InstallKind::Unknown => Err(errors::unsupported_platform()),
    }
}

fn helper_candidates(platform: &PlatformInfo) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(bundle) = platform.current_app_bundle.as_ref() {
        candidates.push(
            PathBuf::from(bundle)
                .join("Contents")
                .join("MacOS")
                .join(helper::HELPER_BINARY_NAME),
        );
    }

    if let Some(current_exe) = platform.current_exe.as_ref() {
        let current_exe = PathBuf::from(current_exe);
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join(helper::HELPER_BINARY_NAME));
        }
    }

    candidates
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
        helper::UpdateHelperMode::DryRun => UpdateInstallMode::DryRun,
        helper::UpdateHelperMode::Test => UpdateInstallMode::Test,
    }
}

fn installing_state(
    current_state: &UpdateStateDto,
    request: &HelperLaunchRequest,
    install_mode: UpdateInstallMode,
    started_at: chrono::DateTime<Utc>,
) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::Installing,
        current_version: current_state.current_version.clone(),
        latest_version: current_state.latest_version.clone(),
        channel: current_state.channel.clone(),
        asset_name: current_state.asset_name.clone(),
        asset_path: current_state.asset_path.clone(),
        asset_sha256: current_state.asset_sha256.clone(),
        asset_size: current_state.asset_size,
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

fn scheduled_state(
    current_state: &UpdateStateDto,
    request: &HelperLaunchRequest,
    install_mode: UpdateInstallMode,
    started_at: chrono::DateTime<Utc>,
    scheduled_at: chrono::DateTime<Utc>,
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
        source: current_state.source.clone(),
        checked_at: current_state.checked_at,
        downloaded_at: current_state.downloaded_at,
        install_log_path: Some(request.command.log_path.to_string_lossy().to_string()),
        install_mode: Some(install_mode),
        install_started_at: Some(started_at),
        install_scheduled_at: Some(scheduled_at),
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

fn install_failure_action(code: &str) -> &'static str {
    match code {
        "updateInstallAssetMissing"
        | "updateInstallAssetSizeMismatch"
        | "updateInstallAssetHashMismatch" => "retryDownload",
        _ => "retryInstall",
    }
}

fn map_helper_exit_code(code: i32, command: &helper::UpdateHelperCommand) -> AppError {
    match code {
        value if value == helper::UpdateHelperExitCode::InvalidArguments.as_i32() => {
            errors::app_error(
                "updateInstallHelperInvalidArguments",
                "更新安装助手参数无效，当前不会执行安装",
            )
        }
        value if value == helper::UpdateHelperExitCode::AssetMissing.as_i32() => {
            errors::with_detail(
                errors::app_error("updateInstallAssetMissing", "更新包文件不存在或无法读取"),
                "assetPath",
                command.asset_path.display().to_string(),
            )
        }
        value if value == helper::UpdateHelperExitCode::AssetSizeMismatch.as_i32() => {
            errors::with_detail(
                errors::app_error("updateInstallAssetSizeMismatch", "更新包大小校验失败"),
                "assetPath",
                command.asset_path.display().to_string(),
            )
        }
        value if value == helper::UpdateHelperExitCode::AssetHashMismatch.as_i32() => {
            errors::with_detail(
                errors::app_error("updateInstallAssetHashMismatch", "更新包哈希校验失败"),
                "assetPath",
                command.asset_path.display().to_string(),
            )
        }
        value if value == helper::UpdateHelperExitCode::TargetMissing.as_i32() => {
            errors::with_detail(
                errors::app_error("updateInstallTargetMissing", "当前安装目标不存在，无法继续"),
                "targetPath",
                command.target_path.display().to_string(),
            )
        }
        value if value == helper::UpdateHelperExitCode::LogWriteFailed.as_i32() => {
            errors::with_detail(
                errors::app_error(
                    "updateInstallLogWriteFailed",
                    "无法写入安装日志，当前不会执行安装",
                ),
                "logPath",
                command.log_path.display().to_string(),
            )
        }
        _ => errors::app_error(
            "updateInstallHelperFailed",
            "更新安装助手执行失败，当前不会执行安装",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::types::{DownloadSourceUsed, UpdateChannel};
    use std::{fs, path::Path, sync::Arc};

    #[derive(Debug, Clone)]
    enum FakeExecutorResult {
        Success,
        Exit(i32),
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
                FakeExecutorResult::Success => Ok(HelperLaunchOutcome {
                    exit_code: helper::UpdateHelperExitCode::Success.as_i32(),
                }),
                FakeExecutorResult::Exit(code) => Ok(HelperLaunchOutcome { exit_code: *code }),
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
            current_version: "1.0.4".into(),
            latest_version: Some("1.0.5".into()),
            channel: UpdateChannel::Stable,
            asset_name: Some("asset.zip".into()),
            asset_path: Some(asset_path.to_string_lossy().to_string()),
            asset_sha256: Some("abc".repeat(21) + "a"),
            asset_size: Some(16),
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
            app_version: "1.0.4".into(),
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

    #[test]
    fn schedules_install_after_helper_dry_run() {
        let paths = test_paths("install-success");
        paths.ensure_dirs().expect("ensure dirs");
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        fs::create_dir_all(bundle.join("Contents").join("MacOS")).expect("create bundle");
        let helper_path = bundle
            .join("Contents")
            .join("MacOS")
            .join(helper::HELPER_BINARY_NAME);
        fs::write(&helper_path, b"helper").expect("write helper placeholder");

        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let service = UpdateInstallService::with_executor(
            executor.clone(),
            helper::UpdateHelperMode::DryRun,
            Some(helper_path.clone()),
            Some(platform_with_bundle(&bundle)),
        );
        let state = downloaded_state(&paths);

        let result = service
            .run(&paths, state.clone())
            .expect("install scheduling should succeed");

        let saved = state::load(&paths).expect("load saved state");
        assert_eq!(result.status, UpdateStatus::InstallScheduled);
        assert_eq!(result.mode, UpdateInstallMode::DryRun);
        assert_eq!(saved.status, UpdateStatus::InstallScheduled);
        assert_eq!(saved.latest_version, state.latest_version);
        assert!(saved.install_log_path.is_some());

        let calls = executor.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].helper_path, helper_path);
        assert_eq!(calls[0].command.mode, helper::UpdateHelperMode::DryRun);
    }

    #[test]
    fn maps_helper_hash_failure_into_failed_state() {
        let paths = test_paths("install-hash-failure");
        paths.ensure_dirs().expect("ensure dirs");
        let helper_path = paths.root_dir().join(helper::HELPER_BINARY_NAME);
        fs::write(&helper_path, b"helper").expect("write helper placeholder");
        let executor = FakeExecutor::new(FakeExecutorResult::Exit(
            helper::UpdateHelperExitCode::AssetHashMismatch.as_i32(),
        ));
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        fs::create_dir_all(bundle.join("Contents").join("MacOS")).expect("create bundle");
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::DryRun,
            Some(helper_path),
            Some(platform_with_bundle(&bundle)),
        );

        let error = service
            .run(&paths, downloaded_state(&paths))
            .expect_err("hash mismatch should fail");

        let saved = state::load(&paths).expect("load failed state");
        assert_eq!(error.code, "updateInstallAssetHashMismatch");
        assert_eq!(saved.status, UpdateStatus::Failed);
        assert_eq!(
            saved.last_error.expect("last error").action.as_deref(),
            Some("retryDownload")
        );
    }

    #[test]
    fn rejects_install_without_downloaded_asset() {
        let paths = test_paths("install-not-ready");
        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::Test,
            Some(paths.root_dir().join(helper::HELPER_BINARY_NAME)),
            None,
        );

        let error = service
            .run(&paths, UpdateStateDto::idle())
            .expect_err("idle state should fail");

        assert_eq!(error.code, "updateInstallNotReady");
    }

    #[test]
    fn helper_path_override_must_exist() {
        let paths = test_paths("install-helper-missing");
        let executor = FakeExecutor::new(FakeExecutorResult::Success);
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        fs::create_dir_all(bundle.join("Contents").join("MacOS")).expect("create bundle");
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::DryRun,
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
        let helper_path = paths.root_dir().join(helper::HELPER_BINARY_NAME);
        fs::write(&helper_path, b"helper").expect("write helper placeholder");
        let bundle = paths.root_dir().join("Floral Notepaper.app");
        fs::create_dir_all(bundle.join("Contents").join("MacOS")).expect("create bundle");
        let executor = FakeExecutor::new(FakeExecutorResult::Error(errors::app_error(
            "updateInstallSpawnFailed",
            "启动更新安装助手失败：boom",
        )));
        let service = UpdateInstallService::with_executor(
            executor,
            helper::UpdateHelperMode::DryRun,
            Some(helper_path),
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
    fn collects_default_helper_candidates() {
        let bundle = PathBuf::from("/Applications/Floral Notepaper.app");
        let platform = platform_with_bundle(&bundle);

        let candidates = helper_candidates(&platform);

        assert!(candidates.iter().any(|path| {
            path.ends_with(
                Path::new("Contents")
                    .join("MacOS")
                    .join(helper::HELPER_BINARY_NAME),
            )
        }));
    }
}
