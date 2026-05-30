use super::{
    check::UpdateCheckService,
    download::UpdateDownloadService,
    errors,
    install::UpdateInstallService,
    types::{
        DownloadSourceUsed, UpdateCheckResult, UpdateDownloadResult, UpdateErrorDto,
        UpdateInstallResult, UpdateStateDto, UpdateStatus,
    },
    ActiveTaskGuard, InstallPrepareState, InstallPrepareWindowStatus, UpdatePaths, UpdateTaskKind,
    UpdaterState,
};
use crate::desktop;
use crate::services::notes::AppError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    thread,
    time::{Duration, Instant},
};
use tauri::{async_runtime, Emitter, Manager, State};

const INSTALL_PREPARE_EVENT: &str = "update://prepare-install";
const INSTALL_PREPARE_TIMEOUT: Duration = Duration::from_secs(10);
const INSTALL_PREPARE_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
pub fn update_mirror_cdk_set(state: State<'_, UpdaterState>, cdk: String) -> Result<(), AppError> {
    state.set_mirror_cdk(&cdk)
}

#[tauri::command]
pub fn update_mirror_cdk_clear(state: State<'_, UpdaterState>) -> Result<(), AppError> {
    state.clear_mirror_cdk()
}

#[tauri::command]
pub async fn update_check(
    app: tauri::AppHandle,
    state: State<'_, UpdaterState>,
    manual: bool,
) -> Result<UpdateCheckResult, AppError> {
    let (task, paths) = prepare_update_check(&app, &state)?;
    let result_paths = paths.clone();
    let current_version = state.current_version().to_string();
    let emit_version = current_version.clone();

    let result = async_runtime::spawn_blocking(move || {
        let _task = task;
        run_update_check_blocking(&paths, manual, &current_version)
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
    let current_state = state.load_state()?;
    let task = state.begin_task(UpdateTaskKind::Download)?;
    let cancel_flag = task
        .cancel_flag()
        .ok_or_else(|| errors::app_error("updateCancelUnavailable", "当前没有可取消的更新任务"))?;
    let paths = state.paths().clone();
    let result_paths = paths.clone();
    let app_handle = app.clone();

    let result = async_runtime::spawn_blocking(move || {
        let _task = task;
        let service = UpdateDownloadService::from_env();
        service.run(&paths, current_state, source, cancel_flag, |progress| {
            let _ = app_handle.emit("update://download-progress", &progress);
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
                let _ = app.emit("update://download-finished", &next_state);
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
            let _ = app.emit("update://error", &error_payload);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn update_install(
    app: tauri::AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<UpdateInstallResult, AppError> {
    let current_state = state.load_state()?;
    let task = state.begin_task(UpdateTaskKind::Install)?;
    let request_id = begin_install_prepare(&app, &state);
    if let Err(error) = wait_for_install_prepare(&state, &request_id).await {
        state.clear_install_prepare(&request_id);
        let error_payload = load_saved_error_payload(
            state.paths(),
            &error,
            "retryInstall",
            state.current_version(),
        );
        let _ = app.emit("update://error", &error_payload);
        return Err(error);
    }
    state.clear_install_prepare(&request_id);
    let paths = state.paths().clone();
    let result_paths = paths.clone();

    let result = async_runtime::spawn_blocking(move || {
        let _task = task;
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
        Ok(_install_result) => {
            if let Ok(next_state) = state.load_state() {
                let _ = app.emit("update://install-finished", &next_state);
            }
            desktop::mark_app_exiting(&app);
            force_terminate_self();
        }
        Err(error) => {
            let error_payload = load_saved_error_payload(
                &result_paths,
                &error,
                "retryInstall",
                state.current_version(),
            );
            let _ = app.emit("update://error", &error_payload);
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
    let result = run_update_check_blocking(&paths, false, state.current_version());
    finalize_update_check(&app, &paths, false, state.current_version(), result)
}

fn parse_download_source(source: &str) -> Result<DownloadSourceUsed, AppError> {
    match source.trim() {
        "mirror" => Ok(DownloadSourceUsed::Mirror),
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
        let _ = self.emit("update://checking", state);
    }

    fn emit_checked(&self, state: &UpdateStateDto) {
        let _ = self.emit("update://checked", state);
    }

    fn emit_error(&self, error: &UpdateErrorDto) {
        let _ = self.emit("update://error", error);
    }
}

fn prepare_update_check<E: UpdateCheckEmitter>(
    emitter: &E,
    state: &UpdaterState,
) -> Result<(ActiveTaskGuard, UpdatePaths), AppError> {
    let task = state.begin_task(UpdateTaskKind::Check)?;
    let mut checking_state = state.load_state().unwrap_or_default();
    checking_state.status = UpdateStatus::Checking;
    checking_state.checked_at = Some(Utc::now());
    checking_state.last_error = None;
    state.save_state(&checking_state)?;
    emitter.emit_checking(&checking_state);

    Ok((task, state.paths().clone()))
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

async fn wait_for_install_prepare(state: &UpdaterState, request_id: &str) -> Result<(), AppError> {
    let deadline = Instant::now() + INSTALL_PREPARE_TIMEOUT;

    loop {
        match state.poll_install_prepare(request_id) {
            InstallPrepareState::Ready => return Ok(()),
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
            InstallPrepareState::Pending { .. } => {
                if Instant::now() >= deadline {
                    return Err(errors::with_detail(
                        errors::app_error(
                            "updateInstallSaveTimedOut",
                            "等待窗口保存未保存内容超时，请稍后重试",
                        ),
                        "requestId",
                        request_id,
                    ));
                }
                thread::sleep(INSTALL_PREPARE_POLL_INTERVAL);
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

fn run_update_check_blocking(
    paths: &UpdatePaths,
    manual: bool,
    current_version: &str,
) -> Result<UpdateCheckResult, AppError> {
    let service = UpdateCheckService::from_env();
    service.run(paths, manual, current_version)
}

fn finalize_update_check<E: UpdateCheckEmitter>(
    emitter: &E,
    paths: &UpdatePaths,
    manual: bool,
    current_version: &str,
    result: Result<UpdateCheckResult, AppError>,
) -> Result<UpdateCheckResult, AppError> {
    if let Ok(next_state) = super::state::load_with_current_version(paths, current_version) {
        emitter.emit_checked(&next_state);
    }

    match result {
        Ok(check_result) => Ok(check_result),
        Err(error) => {
            let error_payload = super::state::load_with_current_version(paths, current_version)
                .ok()
                .and_then(|saved_state| saved_state.last_error)
                .unwrap_or_else(|| {
                    super::types::UpdateErrorDto::recoverable(
                        error.code.clone(),
                        error.message.clone(),
                        Some("retry".into()),
                    )
                });
            if manual {
                emitter.emit_error(&error_payload);
            }
            Err(error)
        }
    }
}

// std::process::exit runs atexit handlers which deadlock with WebView2 on Windows.
fn force_terminate_self() -> ! {
    #[cfg(target_os = "windows")]
    unsafe {
        windows_sys::Win32::System::Threading::ExitProcess(0);
    }
    #[cfg(not(target_os = "windows"))]
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
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
            env!("CARGO_PKG_VERSION"),
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
            env!("CARGO_PKG_VERSION"),
            Err(errors::github_rate_limited()),
        );

        assert_eq!(
            result.expect_err("automatic failure should bubble").code,
            "updateGithubRateLimited"
        );
        assert_eq!(emitter.checked.lock().expect("checked events").len(), 1);
        assert!(emitter.errors.lock().expect("error events").is_empty());
    }
}
