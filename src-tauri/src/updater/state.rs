use super::{
    file_lock::acquire_update_state_lock,
    settings::{rename_corrupt_file, write_json_atomic},
    types::{UpdateErrorDto, UpdateStateDto, UpdateStatus},
    version, UpdatePaths,
};
use crate::services::notes::AppError;
use std::{fs, io, path::Path};

#[cfg(test)]
pub fn load(paths: &UpdatePaths) -> Result<UpdateStateDto, AppError> {
    load_with_current_version(paths, super::version::CURRENT_APP_VERSION)
}

pub fn load_with_current_version(
    paths: &UpdatePaths,
    current_version: &str,
) -> Result<UpdateStateDto, AppError> {
    let _lock = acquire_update_state_lock(&paths.state_path()).map_err(map_state_lock_error)?;
    load_with_current_version_unlocked(paths, current_version)
}

fn load_with_current_version_unlocked(
    paths: &UpdatePaths,
    current_version: &str,
) -> Result<UpdateStateDto, AppError> {
    let path = paths.state_path();
    if !path.exists() {
        let state = UpdateStateDto::idle_with_version(current_version);
        save_unlocked(paths, &state)?;
        return Ok(state);
    }

    match serde_json::from_str::<UpdateStateDto>(&fs::read_to_string(&path)?) {
        Ok(state) => {
            let normalized = normalize_state(state, current_version);
            save_unlocked(paths, &normalized)?;
            Ok(normalized)
        }
        Err(_error) => {
            rename_corrupt_file(&path, "state")?;
            let state = UpdateStateDto::failed_with_version(
                current_version,
                UpdateErrorDto::recoverable(
                    "updateStateCorrupted",
                    "更新状态文件已损坏，已重置为空闲状态",
                    Some("retry".into()),
                ),
            );
            save_unlocked(paths, &state)?;
            Ok(state)
        }
    }
}

fn normalize_state(mut state: UpdateStateDto, current_version: &str) -> UpdateStateDto {
    state.current_version = current_version.to_string();
    let asset_missing = state
        .asset_path
        .as_deref()
        .is_some_and(|asset_path| !Path::new(asset_path).exists());
    if !asset_missing {
        return state;
    }

    if matches!(
        state.status,
        UpdateStatus::Downloaded | UpdateStatus::InstallScheduled | UpdateStatus::Installing
    ) {
        state.status = UpdateStatus::Failed;
        state.last_error = Some(UpdateErrorDto::recoverable(
            "updateInstallAssetMissing",
            "更新包文件不存在或无法读取，请重新下载后再安装",
            Some("retryDownload".into()),
        ));
    }

    state
}

pub fn save(paths: &UpdatePaths, state: &UpdateStateDto) -> Result<(), AppError> {
    let _lock = acquire_update_state_lock(&paths.state_path()).map_err(map_state_lock_error)?;
    save_unlocked(paths, state)
}

fn save_unlocked(paths: &UpdatePaths, state: &UpdateStateDto) -> Result<(), AppError> {
    write_json_atomic(&paths.state_path(), state)
}

pub fn save_with_current_version(
    paths: &UpdatePaths,
    state: &UpdateStateDto,
    current_version: &str,
) -> Result<(), AppError> {
    let normalized = normalize_state(state.clone(), current_version);
    save(paths, &normalized)
}

#[cfg(test)]
pub fn recover(paths: &UpdatePaths) -> Result<UpdateStateDto, AppError> {
    recover_with_current_version(paths, super::version::CURRENT_APP_VERSION)
}

pub fn recover_with_current_version(
    paths: &UpdatePaths,
    current_version: &str,
) -> Result<UpdateStateDto, AppError> {
    let _lock = acquire_update_state_lock(&paths.state_path()).map_err(map_state_lock_error)?;
    let mut state = load_with_current_version_unlocked(paths, current_version)?;

    match state.status {
        UpdateStatus::Checking => {
            state.status = UpdateStatus::Idle;
            state.current_version = current_version.to_string();
            state.last_error = Some(UpdateErrorDto::recoverable(
                "updateCheckInterrupted",
                "上次检查更新被中断，已恢复为空闲状态",
                Some("retry".into()),
            ));
            save_unlocked(paths, &state)?;
        }
        UpdateStatus::Downloading => {
            state.status = UpdateStatus::Failed;
            state.last_error = Some(UpdateErrorDto::recoverable(
                "updateDownloadInterrupted",
                "上次下载被中断，已清理为可重试状态",
                Some("retryDownload".into()),
            ));
            save_unlocked(paths, &state)?;
        }
        UpdateStatus::Installing | UpdateStatus::InstallScheduled => {
            if state.status == UpdateStatus::Installing
                && installed_version_matches_target(&state, current_version)
            {
                let asset_path = state.asset_path.clone();
                state = verified_install_state(state, current_version);
                save_unlocked(paths, &state)?;
                remove_download_dir(asset_path);
                return Ok(state);
            }

            let asset_is_usable = state.asset_path.as_deref().is_some_and(|asset_path| {
                verify_asset(
                    Path::new(asset_path),
                    state.asset_size,
                    state.asset_sha256.as_deref(),
                )
            });
            let previous_status = state.status.clone();
            state.status = UpdateStatus::Failed;
            state.current_version = current_version.to_string();
            state.install_scheduled_at = None;
            state.last_error = Some(match (previous_status, asset_is_usable) {
                (UpdateStatus::Installing, true) => UpdateErrorDto::recoverable(
                    "updateInstallVersionMismatch",
                    "安装后重新打开的仍是旧版本，可直接重试安装",
                    Some("retryInstall".into()),
                ),
                (_, true) => UpdateErrorDto::recoverable(
                    "updateInstallInterrupted",
                    "上次安装未完成，可直接重试安装",
                    Some("retryInstall".into()),
                ),
                (_, false) => UpdateErrorDto::recoverable(
                    "updateInstallInterrupted",
                    "上次安装未完成，安装包已失效，请重新下载更新包",
                    Some("retryDownload".into()),
                ),
            });
            save_unlocked(paths, &state)?;
        }
        _ => {}
    }

    Ok(state)
}

fn map_state_lock_error(error: io::Error) -> AppError {
    if error.kind() == io::ErrorKind::TimedOut {
        return AppError {
            code: "updateStateLockTimeout".into(),
            message: "等待更新状态文件锁超时".into(),
            details: Default::default(),
        };
    }
    error.into()
}

fn installed_version_matches_target(state: &UpdateStateDto, current_version: &str) -> bool {
    let Some(target_version) = state.latest_version.as_deref() else {
        return false;
    };

    match (
        version::normalize_version(current_version),
        version::normalize_version(target_version),
    ) {
        (Ok(current), Ok(target)) => current == target,
        _ => current_version.trim() == target_version.trim(),
    }
}

fn verified_install_state(previous: UpdateStateDto, current_version: &str) -> UpdateStateDto {
    let mut next = UpdateStateDto::idle_with_version(current_version);
    next.channel = previous.channel;
    next.checked_at = Some(chrono::Utc::now());
    next
}

fn remove_download_dir(asset_path: Option<String>) {
    let Some(download_dir) = asset_path
        .as_deref()
        .map(Path::new)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
    else {
        return;
    };

    if download_dir.exists() {
        let _ = fs::remove_dir_all(download_dir);
    }
}

fn verify_asset(path: &Path, expected_size: Option<u64>, expected_hash: Option<&str>) -> bool {
    if expected_size.is_none() && expected_hash.is_none() {
        return false;
    }

    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return false,
    };
    if expected_size.is_some_and(|size| metadata.len() != size) {
        return false;
    }
    if let Some(expected_hash) = expected_hash {
        let expected_hash = expected_hash.trim();
        if !is_valid_sha256_hex(expected_hash) {
            return false;
        }
        let Ok(actual_hash) = sha256_hex(path) else {
            return false;
        };
        if actual_hash != expected_hash.to_ascii_lowercase() {
            return false;
        }
    }
    true
}

fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(path: &Path) -> Result<String, std::io::Error> {
    super::sha256_hex(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::types::UpdateChannel;

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
    fn creates_default_state_file() {
        let paths = test_paths("state-default");

        let state = load_with_current_version(&paths, "1.0.9").expect("load state");

        assert_eq!(state.status, UpdateStatus::Idle);
        assert_eq!(state.channel, UpdateChannel::Stable);
        assert_eq!(state.current_version, "1.0.9");
        assert!(paths.state_path().exists());
    }

    #[test]
    fn recovers_interrupted_download() {
        let paths = test_paths("state-recover-download");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::Downloading;
        save(&paths, &state).expect("save downloading state");

        let recovered = recover(&paths).expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Failed);
        assert_eq!(
            recovered.last_error.expect("last error").code,
            "updateDownloadInterrupted"
        );
    }

    #[test]
    fn recovers_interrupted_check() {
        let paths = test_paths("state-recover-check");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::Checking;
        save(&paths, &state).expect("save checking state");

        let recovered = recover_with_current_version(&paths, "1.0.3").expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Idle);
        assert_eq!(
            recovered.last_error.expect("last error").code,
            "updateCheckInterrupted"
        );
    }

    #[test]
    fn marks_version_mismatch_as_retry_install_when_asset_is_still_usable() {
        let paths = test_paths("state-recover-install-retry");
        let asset_path = paths.downloads_dir().join("1.0.5").join("asset.zip");
        fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("create asset dir");
        fs::write(&asset_path, b"installable asset").expect("write asset");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::Installing;
        state.latest_version = Some("1.0.5".into());
        state.asset_path = Some(asset_path.to_string_lossy().to_string());
        state.asset_size = Some(fs::metadata(&asset_path).expect("asset metadata").len());
        state.asset_sha256 = Some(sha256_hex(&asset_path).expect("asset hash"));
        save(&paths, &state).expect("save installing state");

        let recovered = recover_with_current_version(&paths, "1.0.3").expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Failed);
        let last_error = recovered.last_error.expect("last error");
        assert_eq!(last_error.code, "updateInstallVersionMismatch");
        assert_eq!(last_error.action, Some("retryInstall".into()));
        assert_eq!(recovered.install_log_path, None);
    }

    #[test]
    fn recovers_interrupted_scheduled_install_with_intact_asset_as_retry_install() {
        let paths = test_paths("state-recover-install-scheduled");
        let asset_path = paths.downloads_dir().join("1.0.5").join("asset.zip");
        fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("create asset dir");
        fs::write(&asset_path, b"installable asset").expect("write asset");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::InstallScheduled;
        state.latest_version = Some("1.0.5".into());
        state.asset_path = Some(asset_path.to_string_lossy().to_string());
        state.asset_size = Some(fs::metadata(&asset_path).expect("asset metadata").len());
        state.asset_sha256 = Some(sha256_hex(&asset_path).expect("asset hash"));
        save(&paths, &state).expect("save scheduled installing state");

        let recovered = recover_with_current_version(&paths, "1.0.3").expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Failed);
        let last_error = recovered.last_error.expect("last error");
        assert_eq!(last_error.code, "updateInstallInterrupted");
        assert_eq!(last_error.action, Some("retryInstall".into()));
    }

    #[test]
    fn recovers_interrupted_install_with_missing_asset_as_retry_download() {
        let paths = test_paths("state-recover-install-redownload");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::Installing;
        state.latest_version = Some("1.0.5".into());
        state.asset_path = Some(
            paths
                .downloads_dir()
                .join("1.0.5")
                .join("missing.zip")
                .to_string_lossy()
                .to_string(),
        );
        state.asset_size = Some(123);
        state.asset_sha256 =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        save(&paths, &state).expect("save installing state");

        let recovered = recover(&paths).expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Failed);
        assert_eq!(
            recovered.last_error.expect("last error").action,
            Some("retryDownload".into())
        );
    }

    #[test]
    fn rejects_invalid_expected_hash_when_recovering_install_asset() {
        let paths = test_paths("state-recover-install-invalid-hash");
        let asset_path = paths.downloads_dir().join("1.0.5").join("asset.zip");
        fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("create asset dir");
        fs::write(&asset_path, b"installable asset").expect("write asset");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::InstallScheduled;
        state.latest_version = Some("1.0.5".into());
        state.asset_path = Some(asset_path.to_string_lossy().to_string());
        state.asset_size = Some(fs::metadata(&asset_path).expect("asset metadata").len());
        state.asset_sha256 = Some("not-a-sha256".into());
        save(&paths, &state).expect("save scheduled state");

        let recovered = recover_with_current_version(&paths, "1.0.3").expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Failed);
        assert_eq!(
            recovered.last_error.expect("last error").action,
            Some("retryDownload".into())
        );
    }

    #[test]
    fn rejects_recovery_asset_without_size_or_hash() {
        let paths = test_paths("state-recover-install-missing-asset-metadata");
        let asset_path = paths.downloads_dir().join("1.0.5").join("asset.zip");
        fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("create asset dir");
        fs::write(&asset_path, b"installable asset").expect("write asset");
        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::InstallScheduled;
        state.latest_version = Some("1.0.5".into());
        state.asset_path = Some(asset_path.to_string_lossy().to_string());
        state.asset_size = None;
        state.asset_sha256 = None;
        save(&paths, &state).expect("save scheduled state");

        let recovered = recover_with_current_version(&paths, "1.0.3").expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Failed);
        assert_eq!(
            recovered.last_error.expect("last error").action,
            Some("retryDownload".into())
        );
    }

    #[test]
    fn finalizes_verified_install_when_runtime_version_matches_target() {
        let paths = test_paths("state-recover-install-success");
        let asset_path = paths.downloads_dir().join("1.0.5").join("asset.zip");
        fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("create asset dir");
        fs::write(&asset_path, b"installable asset").expect("write asset");

        let mut state = UpdateStateDto::idle();
        state.status = UpdateStatus::Installing;
        state.current_version = "1.0.3".into();
        state.latest_version = Some("1.0.5".into());
        state.channel = UpdateChannel::Beta;
        state.asset_path = Some(asset_path.to_string_lossy().to_string());
        state.asset_size = Some(fs::metadata(&asset_path).expect("asset metadata").len());
        state.asset_sha256 = Some(sha256_hex(&asset_path).expect("asset hash"));
        state.install_log_path = Some(
            paths
                .logs_dir()
                .join("install.log")
                .to_string_lossy()
                .to_string(),
        );
        save(&paths, &state).expect("save installing state");

        let recovered = recover_with_current_version(&paths, "1.0.5").expect("recover state");

        assert_eq!(recovered.status, UpdateStatus::Idle);
        assert_eq!(recovered.current_version, "1.0.5");
        assert_eq!(recovered.latest_version, None);
        assert_eq!(recovered.channel, UpdateChannel::Beta);
        assert!(recovered.asset_path.is_none());
        assert!(!paths.downloads_dir().join("1.0.5").exists());
    }

    #[test]
    fn resets_corrupt_state_file() {
        let paths = test_paths("state-corrupt");
        paths.ensure_dirs().expect("create dirs");
        fs::write(paths.state_path(), "{ broken").expect("write corrupt state");

        let state = load(&paths).expect("corrupt state should recover");

        assert_eq!(
            state.last_error.expect("last error").code,
            "updateStateCorrupted"
        );
        assert!(paths.state_path().exists());
    }
}
