use super::{
    check::UpdateCheckService,
    download::UpdateDownloadService,
    errors,
    install::UpdateInstallService,
    types::{
        DownloadSourceUsed, UpdateCheckResult, UpdateDownloadResult, UpdateErrorDto,
        UpdateInstallMode, UpdateInstallResult, UpdateStateDto, UpdateStatus,
    },
    ActiveTaskGuard, InstallPrepareState, InstallPrepareWindowStatus, UpdatePaths, UpdateTaskKind,
    UpdaterState,
};
use crate::desktop;
use crate::services::notes::AppError;
use chrono::Utc;

use super::debug_log;

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::{async_runtime, Emitter, Manager, State};

const INSTALL_PREPARE_EVENT: &str = "update://prepare-install";
const INSTALL_PREPARE_MIN_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_PREPARE_TIMEOUT_PER_PENDING_WINDOW: Duration = Duration::from_secs(3);
const INSTALL_PREPARE_MAX_TIMEOUT: Duration = Duration::from_secs(120);
const INSTALL_PREPARE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const INSTALL_TERMINATE_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallPrepareRequestPayload {
    request_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallPrepareReportStatus {
    Ready,
    Failed,
}

#[tauri::command]
pub fn update_status(state: State<'_, UpdaterState>) -> Result<UpdateStateDto, AppError> {
    state.load_state()
}

#[tauri::command]
pub fn update_settings_get(
    state: State<'_, UpdaterState>,
) -> Result<super::types::UpdateSettingsDto, AppError> {
    state.settings()
}

#[tauri::command]
pub fn update_settings_save(
    state: State<'_, UpdaterState>,
    settings: super::types::UpdateSettingsDto,
) -> Result<super::types::UpdateSettingsDto, AppError> {
    state.save_settings(settings)
}

#[tauri::command]
pub fn update_mirror_chyan_cdk_set(
    state: State<'_, UpdaterState>,
    cdk: String,
) -> Result<(), AppError> {
    state.set_mirror_chyan_cdk(&cdk)
}

#[tauri::command]
pub fn update_mirror_chyan_cdk_clear(state: State<'_, UpdaterState>) -> Result<(), AppError> {
    state.clear_mirror_chyan_cdk()
}

#[tauri::command]
pub fn update_mirror_chyan_cdk_get(state: State<'_, UpdaterState>) -> Option<String> {
    state.get_mirror_chyan_cdk()
}

#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    state: State<'_, UpdaterState>,
    manual: bool,
) -> Result<UpdateCheckResult, AppError> {
    debug_log!("check", "收到检查请求 manual={manual}");
    let task = state.begin_task(UpdateTaskKind::Check)?;
    let paths = state.paths().clone();
    let result_paths = paths.clone();
    let current_version = state.current_version().to_string();
    let emit_version = current_version.clone();
    let cdk = state.get_mirror_chyan_cdk();
    debug_log!("check", "CDK 已加载 has_cdk={}", cdk.is_some());

    let prepare_paths = paths.clone();
    let prepare_version = current_version.clone();
    let checking_state = async_runtime::spawn_blocking(move || {
        prepare_update_check_state(&prepare_paths, &prepare_version)
    })
    .await
    .map_err(|error| {
        errors::app_error(
            "updateCheckTaskJoinFailed",
            format!("准备检查更新任务失败：{error}"),
        )
    })??;
    debug_log!("check", "发出 update://checking");
    app.emit_checking(&checking_state);

    let result = async_runtime::spawn_blocking(move || {
        let _task = task;
        run_update_check_blocking(&paths, manual, &current_version, cdk)
    })
    .await
    .map_err(|error| {
        errors::app_error(
            "updateCheckTaskJoinFailed",
            format!("检查更新任务执行失败：{error}"),
        )
    })?;

    finalize_update_check(&app, &result_paths, manual, &emit_version, result)
}

#[tauri::command]
pub async fn update_download(
    app: tauri::AppHandle,
    state: State<'_, UpdaterState>,
    source: Option<String>,
) -> Result<UpdateDownloadResult, AppError> {
    let source = source.as_deref().map(parse_download_source).transpose()?;
    let task = state.begin_task(UpdateTaskKind::Download)?;
    let cancel_flag = task
        .cancel_flag()
        .ok_or_else(|| errors::app_error("updateCancelUnavailable", "当前没有可取消的更新任务"))?;
    let paths = state.paths().clone();
    let result_paths = paths.clone();
    let current_version = state.current_version().to_string();
    let app_handle = app.clone();
    let cdk = state.get_mirror_chyan_cdk();

    let result = async_runtime::spawn_blocking(move || {
        let _task = task;
        let current_state = super::state::load_with_current_version(&paths, &current_version)?;
        let service = UpdateDownloadService::from_env().with_cdk(cdk);
        service.run(&paths, current_state, source, cancel_flag, |progress| {
            if let Err(error) = app_handle.emit("update://download-progress", &progress) {
                eprintln!("failed to emit update://download-progress: {error}");
            }
        })
    })
    .await
    .map_err(|error| {
        errors::app_error(
            "updateDownloadTaskJoinFailed",
            format!("下载任务执行失败：{error}"),
        )
    })?;

    match result {
        Ok(download_result) => {
            if let Ok(next_state) = state.load_state() {
                if let Err(error) = app.emit("update://download-finished", &next_state) {
                    eprintln!("failed to emit update://download-finished: {error}");
                }
            }
            Ok(download_result)
        }
        Err(error) => {
            let error_payload = load_saved_error_payload(
                &result_paths,
                &error,
                "retryDownload",
                state.current_version(),
            );
            if let Err(emit_error) = app.emit("update://error", &error_payload) {
                eprintln!("failed to emit update://error: {emit_error}");
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<UpdateInstallResult, AppError> {
    let task = state.begin_task(UpdateTaskKind::Install)?;
    let request_id = begin_install_prepare(&app, &state);
    if let Err(error) = wait_for_install_prepare(&app, &state, &request_id).await {
        state.clear_install_prepare(&request_id);
        let error_payload = load_saved_error_payload(
            state.paths(),
            &error,
            "retryInstall",
            state.current_version(),
        );
        if let Err(emit_error) = app.emit("update://error", &error_payload) {
            eprintln!("failed to emit update://error: {emit_error}");
        }
        return Err(error);
    }
    state.clear_install_prepare(&request_id);
    let paths = state.paths().clone();
    let result_paths = paths.clone();
    let current_version = state.current_version().to_string();

    let result = async_runtime::spawn_blocking(move || {
        let _task = task;
        let current_state = super::state::load_with_current_version(&paths, &current_version)?;
        let service = UpdateInstallService::from_env();
        service.run(&paths, current_state)
    })
    .await
    .map_err(|error| {
        errors::app_error(
            "updateInstallTaskJoinFailed",
            format!("安装任务执行失败：{error}"),
        )
    })?;

    match result {
        Ok(install_result) => {
            let mut should_terminate = install_result.mode == UpdateInstallMode::Apply;
            if let Ok(next_state) = state.load_state() {
                if next_state.status == UpdateStatus::Failed {
                    should_terminate = false;
                }
                if let Err(error) = app.emit("update://install-finished", &next_state) {
                    eprintln!("failed to emit update://install-finished: {error}");
                }
            }
            if should_terminate {
                desktop::mark_app_exiting(&app);
                schedule_force_terminate_self();
            }
            Ok(install_result)
        }
        Err(error) => {
            let error_payload = load_saved_error_payload(
                &result_paths,
                &error,
                "retryInstall",
                state.current_version(),
            );
            if let Err(emit_error) = app.emit("update://error", &error_payload) {
                eprintln!("failed to emit update://error: {emit_error}");
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub fn update_install_prepare_report(
    state: State<'_, UpdaterState>,
    request_id: String,
    window_label: String,
    status: InstallPrepareReportStatus,
    message: Option<String>,
) -> Result<(), AppError> {
    let status = match status {
        InstallPrepareReportStatus::Ready => InstallPrepareWindowStatus::Ready,
        InstallPrepareReportStatus::Failed => InstallPrepareWindowStatus::Failed(
            message.unwrap_or_else(|| "窗口未能完成安装前保存".to_string()),
        ),
    };
    state.report_install_prepare(&request_id, &window_label, status);
    Ok(())
}

#[tauri::command]
pub fn update_cancel(state: State<'_, UpdaterState>) -> Result<(), AppError> {
    state.request_cancel()
}

pub(crate) fn run_automatic_update_check(
    app: tauri::AppHandle,
    state: &UpdaterState,
) -> Result<UpdateCheckResult, AppError> {
    let (task, paths) = prepare_update_check(&app, state)?;
    let _task = task;
    let cdk = state.get_mirror_chyan_cdk();
    let result = run_update_check_blocking(&paths, false, state.current_version(), cdk);
    finalize_update_check(&app, &paths, false, state.current_version(), result)
}

fn parse_download_source(source: &str) -> Result<DownloadSourceUsed, AppError> {
    match source.trim() {
        "mirrorChyan" => Ok(DownloadSourceUsed::MirrorChyan),
        "github" => Ok(DownloadSourceUsed::Github),
        _ => Err(errors::with_detail(
            errors::app_error("updateDownloadSourceInvalid", "无效的下载源参数"),
            "source",
            source,
        )),
    }
}

fn load_saved_error_payload(
    paths: &super::UpdatePaths,
    error: &AppError,
    fallback_action: &str,
    current_version: &str,
) -> UpdateErrorDto {
    super::state::load_with_current_version(paths, current_version)
        .ok()
        .and_then(|saved_state| saved_state.last_error)
        .unwrap_or_else(|| {
            UpdateErrorDto::recoverable(
                error.code.clone(),
                error.message.clone(),
                Some(fallback_action.into()),
            )
        })
}

trait UpdateCheckEmitter {
    fn emit_checking(&self, state: &UpdateStateDto);
    fn emit_checked(&self, state: &UpdateStateDto);
    fn emit_error(&self, error: &UpdateErrorDto);
}

impl UpdateCheckEmitter for tauri::AppHandle {
    fn emit_checking(&self, state: &UpdateStateDto) {
        debug_log!(
            "check",
            "emit update://checking current_version={}",
            state.current_version
        );
        if let Err(error) = self.emit("update://checking", state) {
            eprintln!("failed to emit update://checking: {error}");
        }
    }

    fn emit_checked(&self, state: &UpdateStateDto) {
        debug_log!(
            "check",
            "emit update://checked status={:?} latest_version={:?}",
            state.status,
            state.latest_version
        );
        if let Err(error) = self.emit("update://checked", state) {
            eprintln!("failed to emit update://checked: {error}");
        }
    }

    fn emit_error(&self, error: &UpdateErrorDto) {
        debug_log!(
            "check",
            "emit update://error code={} message={}",
            error.code,
            error.message
        );
        if let Err(emit_error) = self.emit("update://error", error) {
            eprintln!("failed to emit update://error: {emit_error}");
        }
    }
}

fn prepare_update_check<E: UpdateCheckEmitter>(
    emitter: &E,
    state: &UpdaterState,
) -> Result<(ActiveTaskGuard, UpdatePaths), AppError> {
    let task = state.begin_task(UpdateTaskKind::Check)?;
    let checking_state = prepare_update_check_state(state.paths(), state.current_version())?;
    emitter.emit_checking(&checking_state);

    Ok((task, state.paths().clone()))
}

fn prepare_update_check_state(
    paths: &UpdatePaths,
    current_version: &str,
) -> Result<UpdateStateDto, AppError> {
    let mut checking_state = super::state::load_with_current_version(paths, current_version)?;
    checking_state.status = UpdateStatus::Checking;
    checking_state.checked_at = Some(Utc::now());
    checking_state.last_error = None;
    super::state::save_with_current_version(paths, &checking_state, current_version)?;
    Ok(checking_state)
}

fn begin_install_prepare(app: &tauri::AppHandle, state: &UpdaterState) -> String {
    let windows = app.webview_windows().into_values().collect::<Vec<_>>();
    let request_id = state.begin_install_prepare(
        windows
            .iter()
            .map(|window| window.label().to_string())
            .collect::<Vec<_>>(),
    );
    let payload = InstallPrepareRequestPayload {
        request_id: request_id.clone(),
    };

    for window in windows {
        if let Err(error) = window.emit(INSTALL_PREPARE_EVENT, &payload) {
            state.report_install_prepare(
                &request_id,
                window.label(),
                InstallPrepareWindowStatus::Failed(format!("无法通知窗口保存未保存内容：{error}")),
            );
        }
    }

    request_id
}

async fn wait_for_install_prepare(
    app: &tauri::AppHandle,
    state: &UpdaterState,
    request_id: &str,
) -> Result<(), AppError> {
    let started_at = Instant::now();
    let mut deadline = started_at + install_prepare_timeout_for_pending_count(0);

    loop {
        sync_install_prepare_windows(app, state, request_id);
        match state.poll_install_prepare(request_id) {
            InstallPrepareState::Ready => {
                sync_install_prepare_windows(app, state, request_id);
                if matches!(
                    state.poll_install_prepare(request_id),
                    InstallPrepareState::Ready
                ) {
                    return Ok(());
                }
            }
            InstallPrepareState::Failed {
                window_label,
                message,
            } => {
                return Err(errors::with_detail(
                    errors::with_detail(
                        errors::app_error("updateInstallSaveFailed", message),
                        "requestId",
                        request_id,
                    ),
                    "windowLabel",
                    window_label,
                ));
            }
            InstallPrepareState::Pending { pending_labels } => {
                let pending_count = pending_labels.len();
                let scaled_deadline =
                    started_at + install_prepare_timeout_for_pending_count(pending_count);
                if scaled_deadline > deadline {
                    deadline = scaled_deadline;
                }

                if Instant::now() >= deadline {
                    let error = errors::with_detail(
                        errors::app_error(
                            "updateInstallSaveTimedOut",
                            "等待窗口保存未保存内容超时，请稍后重试",
                        ),
                        "requestId",
                        request_id,
                    );
                    let error =
                        errors::with_detail(error, "pendingWindows", pending_count.to_string());
                    let error = errors::with_detail(
                        error,
                        "timeoutSeconds",
                        deadline.duration_since(started_at).as_secs().to_string(),
                    );
                    return Err(error);
                }
                tokio::time::sleep(INSTALL_PREPARE_POLL_INTERVAL).await;
            }
            InstallPrepareState::Unknown => {
                return Err(errors::with_detail(
                    errors::app_error(
                        "updateInstallSaveFailed",
                        "安装前保存会话已失效，请重试安装",
                    ),
                    "requestId",
                    request_id,
                ));
            }
        }
    }
}

fn install_prepare_timeout_for_pending_count(pending_count: usize) -> Duration {
    let scaled = INSTALL_PREPARE_MIN_TIMEOUT.saturating_add(
        INSTALL_PREPARE_TIMEOUT_PER_PENDING_WINDOW.saturating_mul(pending_count as u32),
    );
    scaled.min(INSTALL_PREPARE_MAX_TIMEOUT)
}

fn sync_install_prepare_windows(app: &tauri::AppHandle, state: &UpdaterState, request_id: &str) {
    let windows = app.webview_windows();
    let added_labels = state.sync_install_prepare_labels(request_id, windows.keys().cloned());
    if added_labels.is_empty() {
        return;
    }

    let payload = InstallPrepareRequestPayload {
        request_id: request_id.to_string(),
    };
    for label in added_labels {
        let Some(window) = windows.get(&label) else {
            state.report_install_prepare(request_id, &label, InstallPrepareWindowStatus::Ready);
            continue;
        };
        if let Err(error) = window.emit(INSTALL_PREPARE_EVENT, &payload) {
            state.report_install_prepare(
                request_id,
                &label,
                InstallPrepareWindowStatus::Failed(format!("无法通知窗口保存未保存内容：{error}")),
            );
        }
    }
}

fn run_update_check_blocking(
    paths: &UpdatePaths,
    manual: bool,
    current_version: &str,
    cdk: Option<String>,
) -> Result<UpdateCheckResult, AppError> {
    let service = UpdateCheckService::from_env().with_cdk(cdk);
    service.run(paths, manual, current_version)
}

fn finalize_update_check<E: UpdateCheckEmitter>(
    emitter: &E,
    paths: &UpdatePaths,
    manual: bool,
    current_version: &str,
    result: Result<UpdateCheckResult, AppError>,
) -> Result<UpdateCheckResult, AppError> {
    let loaded_state = super::state::load_with_current_version(paths, current_version).ok();
    if let Some(next_state) = &loaded_state {
        debug_log!(
            "check",
            "发出 update://checked status={:?}",
            next_state.status
        );
        emitter.emit_checked(next_state);
    }

    match result {
        Ok(check_result) => Ok(check_result),
        Err(error) => {
            let error_payload = loaded_state
                .and_then(|saved_state| saved_state.last_error)
                .unwrap_or_else(|| {
                    super::types::UpdateErrorDto::recoverable(
                        error.code.clone(),
                        error.message.clone(),
                        Some("retry".into()),
                    )
                });
            if manual {
                debug_log!("check", "发出 update://error code={}", error_payload.code);
                emitter.emit_error(&error_payload);
            }
            Err(error)
        }
    }
}

fn schedule_force_terminate_self() {
    std::thread::spawn(|| {
        std::thread::sleep(INSTALL_TERMINATE_DELAY);
        force_terminate_self();
    });
}

// std::process::exit and ExitProcess both run DLL detach handlers that can
// deadlock with WebView2 on Windows.  TerminateProcess skips DLL cleanup
// entirely, which is safe here because the update helper will replace the
// binary anyway.
fn force_terminate_self() -> ! {
    #[cfg(target_os = "windows")]
    unsafe {
        windows_sys::Win32::System::Threading::TerminateProcess(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            0,
        );
        std::process::abort();
    }
    #[cfg(not(target_os = "windows"))]
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::super::version::CURRENT_APP_VERSION;
    use super::*;
    use std::{fs, sync::Mutex};

    #[derive(Default)]
    struct FakeEmitter {
        checking: Mutex<Vec<UpdateStateDto>>,
        checked: Mutex<Vec<UpdateStateDto>>,
        errors: Mutex<Vec<UpdateErrorDto>>,
    }

    impl UpdateCheckEmitter for FakeEmitter {
        fn emit_checking(&self, state: &UpdateStateDto) {
            self.checking
                .lock()
                .expect("lock checking events")
                .push(state.clone());
        }

        fn emit_checked(&self, state: &UpdateStateDto) {
            self.checked
                .lock()
                .expect("lock checked events")
                .push(state.clone());
        }

        fn emit_error(&self, error: &UpdateErrorDto) {
            self.errors
                .lock()
                .expect("lock error events")
                .push(error.clone());
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

    #[test]
    fn manual_failure_emits_checked_state_and_error_payload() {
        let paths = test_paths("commands-manual-failure");
        let failed_state = UpdateStateDto::failed(UpdateErrorDto::recoverable(
            "updateGithubApi",
            "GitHub API 请求失败",
            Some("retry".into()),
        ));
        super::super::state::save(&paths, &failed_state).expect("save failed state");
        let emitter = FakeEmitter::default();

        let result = finalize_update_check(
            &emitter,
            &paths,
            true,
            CURRENT_APP_VERSION,
            Err(errors::github_api_error("request failed")),
        );

        assert_eq!(
            result.expect_err("manual failure should bubble").code,
            "updateGithubApi"
        );
        let checked = emitter.checked.lock().expect("checked events");
        assert_eq!(checked.len(), 1);
        assert_eq!(checked[0].status, UpdateStatus::Failed);
        let errors = emitter.errors.lock().expect("error events");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "updateGithubApi");
    }

    #[test]
    fn automatic_failure_emits_checked_state_without_manual_error_event() {
        let paths = test_paths("commands-auto-failure");
        let failed_state = UpdateStateDto::failed(UpdateErrorDto::recoverable(
            "updateGithubRateLimited",
            "GitHub API 频率限制，请稍后重试",
            Some("retry".into()),
        ));
        super::super::state::save(&paths, &failed_state).expect("save failed state");
        let emitter = FakeEmitter::default();

        let result = finalize_update_check(
            &emitter,
            &paths,
            false,
            CURRENT_APP_VERSION,
            Err(errors::github_rate_limited()),
        );

        assert_eq!(
            result.expect_err("automatic failure should bubble").code,
            "updateGithubRateLimited"
        );
        assert_eq!(emitter.checked.lock().expect("checked events").len(), 1);
        assert!(emitter.errors.lock().expect("error events").is_empty());
    }

    #[test]
    fn install_prepare_timeout_scales_with_pending_windows() {
        assert_eq!(
            install_prepare_timeout_for_pending_count(0),
            INSTALL_PREPARE_MIN_TIMEOUT
        );
        assert_eq!(
            install_prepare_timeout_for_pending_count(15),
            Duration::from_secs(75)
        );
        assert_eq!(
            install_prepare_timeout_for_pending_count(100),
            INSTALL_PREPARE_MAX_TIMEOUT
        );
    }
}
