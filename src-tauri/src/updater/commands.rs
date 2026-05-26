use super::{
    check::UpdateCheckService,
    download::UpdateDownloadService,
    errors,
    install::UpdateInstallService,
    types::{
        DownloadSourceUsed, UpdateCheckResult, UpdateDownloadResult, UpdateErrorDto,
        UpdateInstallResult, UpdateStateDto, UpdateStatus,
    },
    UpdateTaskKind, UpdaterState,
};
use crate::services::notes::AppError;
use chrono::Utc;
use tauri::{async_runtime, Emitter, State};

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
pub fn update_check(
    app: tauri::AppHandle,
    state: State<'_, UpdaterState>,
    manual: bool,
) -> Result<UpdateCheckResult, AppError> {
    let _guard = state.begin_task(UpdateTaskKind::Check)?;
    let mut checking_state = state.load_state().unwrap_or_default();
    checking_state.status = UpdateStatus::Checking;
    checking_state.checked_at = Some(Utc::now());
    checking_state.last_error = None;
    state.save_state(&checking_state)?;
    let _ = app.emit("update://checking", &checking_state);

    let service = UpdateCheckService::from_env();
    match service.run(state.paths(), manual) {
        Ok(result) => {
            if let Ok(next_state) = state.load_state() {
                let _ = app.emit("update://checked", &next_state);
            }
            Ok(result)
        }
        Err(error) => {
            let error_payload = state
                .load_state()
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
                let _ = app.emit("update://error", &error_payload);
            }
            Err(error)
        }
    }
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
            if let Ok(next_state) = super::state::load(&result_paths) {
                let _ = app.emit("update://download-finished", &next_state);
            }
            Ok(download_result)
        }
        Err(error) => {
            let error_payload = load_saved_error_payload(&result_paths, &error, "retryDownload");
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
    let paths = state.paths().clone();
    let result_paths = paths.clone();
    let app_handle = app.clone();

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
        Ok(install_result) => {
            if let Ok(next_state) = super::state::load(&result_paths) {
                let _ = app.emit("update://install-finished", &next_state);
            }
            Ok(install_result)
        }
        Err(error) => {
            let error_payload = load_saved_error_payload(&result_paths, &error, "retryInstall");
            let _ = app_handle.emit("update://error", &error_payload);
            Err(error)
        }
    }
}

#[tauri::command]
pub fn update_cancel(state: State<'_, UpdaterState>) -> Result<(), AppError> {
    state.request_cancel()
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
) -> UpdateErrorDto {
    super::state::load(paths)
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
