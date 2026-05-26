use super::{
    settings::{rename_corrupt_file, write_json_atomic},
    types::{UpdateErrorDto, UpdateStateDto, UpdateStatus},
    UpdatePaths,
};
use crate::services::notes::AppError;
use std::fs;

pub fn load(paths: &UpdatePaths) -> Result<UpdateStateDto, AppError> {
    paths.ensure_dirs()?;
    let path = paths.state_path();
    if !path.exists() {
        let state = UpdateStateDto::idle();
        save(paths, &state)?;
        return Ok(state);
    }

    match serde_json::from_str::<UpdateStateDto>(&fs::read_to_string(&path)?) {
        Ok(state) => Ok(state),
        Err(_error) => {
            rename_corrupt_file(&path, "state")?;
            let state = UpdateStateDto::failed(UpdateErrorDto::recoverable(
                "updateStateCorrupted",
                "更新状态文件已损坏，已重置为空闲状态",
                Some("retry".into()),
            ));
            save(paths, &state)?;
            Ok(state)
        }
    }
}

pub fn save(paths: &UpdatePaths, state: &UpdateStateDto) -> Result<(), AppError> {
    paths.ensure_dirs()?;
    write_json_atomic(&paths.state_path(), state)
}

pub fn recover(paths: &UpdatePaths) -> Result<UpdateStateDto, AppError> {
    let mut state = load(paths)?;

    match state.status {
        UpdateStatus::Downloading => {
            state.status = UpdateStatus::Failed;
            state.last_error = Some(UpdateErrorDto::recoverable(
                "updateDownloadInterrupted",
                "上次下载被中断，已清理为可重试状态",
                Some("retryDownload".into()),
            ));
            save(paths, &state)?;
        }
        UpdateStatus::Installing | UpdateStatus::InstallScheduled => {
            state.status = UpdateStatus::Failed;
            state.last_error = Some(UpdateErrorDto::recoverable(
                "updateInstallInterrupted",
                "上次安装未完成，请重新检查更新",
                Some("retryCheck".into()),
            ));
            save(paths, &state)?;
        }
        _ => {}
    }

    Ok(state)
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

        let state = load(&paths).expect("load state");

        assert_eq!(state.status, UpdateStatus::Idle);
        assert_eq!(state.channel, UpdateChannel::Stable);
        assert_eq!(state.current_version, env!("CARGO_PKG_VERSION"));
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
