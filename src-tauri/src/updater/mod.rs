pub mod cdk_store;
pub mod check;
pub mod commands;
pub mod download;
pub mod errors;
mod file_lock;
pub mod helper;
pub mod install;
pub mod manifest;
pub mod platform;
mod scheduler;
pub mod settings;
pub mod state;
pub mod types;
pub mod version;

macro_rules! debug_log {
    ($label:literal, $($arg:tt)*) => {{
        if cfg!(debug_assertions) {
            eprintln!("[update:{}] {}", $label, format!($($arg)*));
        }
    }};
}
pub(super) use debug_log;

use crate::services::notes::{default_config_dir, AppError};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::{Duration, SystemTime},
};
use uuid::Uuid;

pub const APP_ID: &str = "com.floral-notepaper.app";

pub(super) fn sha256_hex(path: &Path) -> Result<String, std::io::Error> {
    use sha2::{Digest, Sha256};
    use std::io::{BufReader, Read};

    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }

    let bytes = digest.finalize();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        hex.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap_or('0'));
    }
    Ok(hex)
}
pub use scheduler::start_auto_check_scheduler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateTaskKind {
    Check,
    Download,
    Install,
}

const DOWNLOAD_RETENTION: Duration = Duration::from_secs(3 * 24 * 60 * 60);
const MAX_INSTALL_LOGS: usize = 20;

impl UpdateTaskKind {
    fn is_cancelable(self) -> bool {
        matches!(self, Self::Download)
    }
}

#[derive(Debug)]
struct ActiveUpdateTask {
    id: u64,
    kind: UpdateTaskKind,
    cancel_flag: Option<Arc<AtomicBool>>,
}

#[derive(Debug, Clone)]
pub enum InstallPrepareWindowStatus {
    Ready,
    Failed(String),
}

#[derive(Debug)]
struct InstallPrepareSession {
    request_id: String,
    expected_labels: BTreeSet<String>,
    reports: BTreeMap<String, InstallPrepareWindowStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallPrepareState {
    Pending {
        pending_labels: Vec<String>,
    },
    Ready,
    Failed {
        window_label: String,
        message: String,
    },
    Unknown,
}

#[derive(Debug)]
pub struct ActiveTaskGuard {
    task_id: u64,
    active_task: Arc<Mutex<Option<ActiveUpdateTask>>>,
    cancel_flag: Option<Arc<AtomicBool>>,
}

impl ActiveTaskGuard {
    pub fn cancel_flag(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_flag.clone()
    }
}

impl Drop for ActiveTaskGuard {
    fn drop(&mut self) {
        let mut slot = match self.active_task.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if slot.as_ref().map(|task| task.id) == Some(self.task_id) {
            *slot = None;
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdatePaths {
    root_dir: PathBuf,
}

impl UpdatePaths {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root_dir.join("settings.json")
    }

    pub fn state_path(&self) -> PathBuf {
        self.root_dir.join("state.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root_dir.join("logs")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root_dir.join("downloads")
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root_dir.join("staging")
    }

    pub fn ensure_dirs(&self) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.root_dir)?;
        std::fs::create_dir_all(self.logs_dir())?;
        std::fs::create_dir_all(self.downloads_dir())?;
        std::fs::create_dir_all(self.staging_dir())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct UpdaterState {
    paths: UpdatePaths,
    current_version: String,
    cdk_store: cdk_store::CdkStore,
    active_task: Arc<Mutex<Option<ActiveUpdateTask>>>,
    install_prepare: Arc<Mutex<Option<InstallPrepareSession>>>,
    next_task_id: AtomicU64,
}

impl UpdaterState {
    pub fn new(current_version: impl Into<String>) -> Self {
        Self {
            paths: UpdatePaths::new(default_updates_dir()),
            current_version: current_version.into(),
            cdk_store: cdk_store::CdkStore::default(),
            active_task: Arc::new(Mutex::new(None)),
            install_prepare: Arc::new(Mutex::new(None)),
            next_task_id: AtomicU64::new(1),
        }
    }

    pub fn initialize(&self) -> Result<(), AppError> {
        self.paths.ensure_dirs()?;
        let _ = settings::load(&self.paths)?;
        let recovered_state =
            state::recover_with_current_version(&self.paths, &self.current_version)?;
        download::cleanup_partial_downloads(&self.paths)?;
        cleanup_update_artifacts(&self.paths, &recovered_state)?;
        helper::cleanup_stale_macos_mounts(&self.paths);
        Ok(())
    }

    pub fn paths(&self) -> &UpdatePaths {
        &self.paths
    }

    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    pub fn settings(&self) -> Result<types::UpdateSettingsDto, AppError> {
        let settings = settings::load(&self.paths)?;
        let (has_mirror_chyan_cdk, mirror_chyan_cdk_length) = self.cdk_settings_info();
        Ok(settings.into_dto(has_mirror_chyan_cdk, mirror_chyan_cdk_length))
    }

    pub fn save_settings(
        &self,
        settings: types::UpdateSettingsDto,
    ) -> Result<types::UpdateSettingsDto, AppError> {
        let existing = settings::load(&self.paths)?;
        let stored = settings::StoredUpdateSettings::from_user_settings(&existing, settings);
        settings::save(&self.paths, &stored)?;
        let (has_mirror_chyan_cdk, mirror_chyan_cdk_length) = self.cdk_settings_info();
        Ok(stored.into_dto(has_mirror_chyan_cdk, mirror_chyan_cdk_length))
    }

    pub fn set_mirror_chyan_cdk(&self, cdk: &str) -> Result<(), AppError> {
        self.cdk_store.set_cdk(cdk)
    }

    pub fn clear_mirror_chyan_cdk(&self) -> Result<(), AppError> {
        self.cdk_store.clear_cdk()
    }

    pub fn has_mirror_chyan_cdk(&self) -> Result<bool, AppError> {
        self.cdk_store.has_cdk()
    }

    pub fn get_mirror_chyan_cdk(&self) -> Option<String> {
        self.cdk_store.get_cdk()
    }

    fn cdk_settings_info(&self) -> (bool, Option<u32>) {
        match self.cdk_store.has_cdk() {
            Ok(true) => {
                let len = self
                    .cdk_store
                    .get_cdk()
                    .map(|cdk| cdk.len() as u32)
                    .filter(|n| *n > 0);
                (true, len)
            }
            Ok(false) => (false, None),
            Err(error) => {
                eprintln!("failed to read MirrorChyan CDK from secure storage: {error}");
                (false, None)
            }
        }
    }

    pub fn load_state(&self) -> Result<types::UpdateStateDto, AppError> {
        state::load_with_current_version(&self.paths, &self.current_version)
    }

    pub fn save_state(&self, update_state: &types::UpdateStateDto) -> Result<(), AppError> {
        state::save_with_current_version(&self.paths, update_state, &self.current_version)
    }

    pub fn begin_task(&self, kind: UpdateTaskKind) -> Result<ActiveTaskGuard, AppError> {
        let mut slot = recover_mutex_guard(&self.active_task);

        if slot.is_some() {
            return Err(errors::app_error(
                "updateAlreadyRunning",
                "已有更新任务正在运行",
            ));
        }

        let task_id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let cancel_flag = kind
            .is_cancelable()
            .then(|| Arc::new(AtomicBool::new(false)));
        *slot = Some(ActiveUpdateTask {
            id: task_id,
            kind,
            cancel_flag: cancel_flag.clone(),
        });

        Ok(ActiveTaskGuard {
            task_id,
            active_task: Arc::clone(&self.active_task),
            cancel_flag,
        })
    }

    pub fn request_cancel(&self) -> Result<(), AppError> {
        let slot = recover_mutex_guard(&self.active_task);

        match slot.as_ref() {
            Some(task) if task.kind == UpdateTaskKind::Download => {
                if let Some(cancel_flag) = &task.cancel_flag {
                    cancel_flag.store(true, Ordering::Relaxed);
                    return Ok(());
                }
            }
            _ => {}
        }

        Err(errors::app_error(
            "updateCancelUnavailable",
            "当前没有可取消的更新任务",
        ))
    }

    pub fn begin_install_prepare<I>(&self, expected_labels: I) -> String
    where
        I: IntoIterator<Item = String>,
    {
        let request_id = Uuid::new_v4().to_string();
        let session = InstallPrepareSession {
            request_id: request_id.clone(),
            expected_labels: expected_labels.into_iter().collect(),
            reports: BTreeMap::new(),
        };
        *recover_mutex_guard(&self.install_prepare) = Some(session);
        request_id
    }

    pub fn report_install_prepare(
        &self,
        request_id: &str,
        window_label: &str,
        status: InstallPrepareWindowStatus,
    ) {
        let mut session = recover_mutex_guard(&self.install_prepare);
        let Some(session) = session.as_mut() else {
            return;
        };
        if session.request_id != request_id || !session.expected_labels.contains(window_label) {
            return;
        }
        session.reports.insert(window_label.to_string(), status);
    }

    pub fn sync_install_prepare_labels<I>(&self, request_id: &str, labels: I) -> Vec<String>
    where
        I: IntoIterator<Item = String>,
    {
        let current_labels = labels.into_iter().collect::<BTreeSet<_>>();
        let mut session = recover_mutex_guard(&self.install_prepare);
        let Some(session) = session.as_mut() else {
            return Vec::new();
        };
        if session.request_id != request_id {
            return Vec::new();
        }

        session
            .expected_labels
            .retain(|label| current_labels.contains(label));
        session
            .reports
            .retain(|label, _| current_labels.contains(label));

        current_labels
            .into_iter()
            .filter(|label| session.expected_labels.insert(label.clone()))
            .collect()
    }

    pub fn poll_install_prepare(&self, request_id: &str) -> InstallPrepareState {
        let session = recover_mutex_guard(&self.install_prepare);
        let Some(session) = session.as_ref() else {
            return InstallPrepareState::Unknown;
        };
        if session.request_id != request_id {
            return InstallPrepareState::Unknown;
        }

        for (window_label, status) in &session.reports {
            if let InstallPrepareWindowStatus::Failed(message) = status {
                return InstallPrepareState::Failed {
                    window_label: window_label.clone(),
                    message: message.clone(),
                };
            }
        }

        let pending_labels = session
            .expected_labels
            .iter()
            .filter(|label| !session.reports.contains_key(*label))
            .cloned()
            .collect::<Vec<_>>();

        if pending_labels.is_empty() {
            InstallPrepareState::Ready
        } else {
            InstallPrepareState::Pending { pending_labels }
        }
    }

    pub fn clear_install_prepare(&self, request_id: &str) {
        let mut session = recover_mutex_guard(&self.install_prepare);
        if session
            .as_ref()
            .is_some_and(|current| current.request_id == request_id)
        {
            *session = None;
        }
    }
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self::new(version::CURRENT_APP_VERSION)
    }
}

#[cfg(test)]
impl UpdaterState {
    pub(crate) fn with_paths(paths: UpdatePaths) -> Self {
        Self::with_paths_and_version(paths, version::CURRENT_APP_VERSION)
    }

    pub(crate) fn with_paths_and_version(
        paths: UpdatePaths,
        current_version: impl Into<String>,
    ) -> Self {
        Self::with_paths_version_and_cdk_store(
            paths,
            current_version,
            cdk_store::CdkStore::default(),
        )
    }

    pub(crate) fn with_paths_version_and_cdk_store(
        paths: UpdatePaths,
        current_version: impl Into<String>,
        cdk_store: cdk_store::CdkStore,
    ) -> Self {
        Self {
            paths,
            current_version: current_version.into(),
            cdk_store,
            active_task: Arc::new(Mutex::new(None)),
            install_prepare: Arc::new(Mutex::new(None)),
            next_task_id: AtomicU64::new(1),
        }
    }
}

fn recover_mutex_guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn default_updates_dir() -> PathBuf {
    if let Ok(config_dir) = default_config_dir() {
        return config_dir.join("updates");
    }

    env::current_dir()
        .unwrap_or_else(|_| env::temp_dir())
        .join("floral-notepaper")
        .join("updates")
}

fn cleanup_update_artifacts(
    paths: &UpdatePaths,
    state: &types::UpdateStateDto,
) -> Result<(), AppError> {
    cleanup_staging_entries(paths)?;
    cleanup_download_entries(paths, state)?;
    cleanup_install_logs(paths)?;
    Ok(())
}

fn cleanup_staging_entries(paths: &UpdatePaths) -> Result<(), AppError> {
    prune_dir_entries(&paths.staging_dir(), Duration::ZERO, |_| true)
}

fn cleanup_download_entries(
    paths: &UpdatePaths,
    state: &types::UpdateStateDto,
) -> Result<(), AppError> {
    let preserved_dir = state
        .asset_path
        .as_deref()
        .map(PathBuf::from)
        .and_then(|path| path.parent().map(Path::to_path_buf));
    prune_dir_entries(&paths.downloads_dir(), DOWNLOAD_RETENTION, |entry_path| {
        preserved_dir
            .as_ref()
            .is_none_or(|preserved| preserved != entry_path)
    })
}

fn cleanup_install_logs(paths: &UpdatePaths) -> Result<(), AppError> {
    let logs_dir = paths.logs_dir();
    if !logs_dir.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(&logs_dir)?
        .flatten()
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((entry.path(), modified))
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.1));

    for (path, _) in entries.into_iter().skip(MAX_INSTALL_LOGS) {
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

fn prune_dir_entries<F>(dir: &Path, retention: Duration, should_remove: F) -> Result<(), AppError>
where
    F: Fn(&Path) -> bool,
{
    if !dir.exists() {
        return Ok(());
    }

    let now = SystemTime::now();
    for entry in fs::read_dir(dir)? {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !should_remove(&path) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age < retention {
            continue;
        }
        if metadata.is_dir() {
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::types::{
        CheckSourcePreference, DownloadSourcePreference, UpdateChannel, UpdateSettingsDto,
        UpdateStateDto, UpdateStatus,
    };
    use chrono::Utc;
    use std::fs;

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
    fn save_settings_preserves_last_auto_check_at_from_disk() {
        let paths = test_paths("updater-save-settings-preserves-auto-check-at");
        let state = UpdaterState::with_paths(paths.clone());
        state.initialize().expect("initialize updater state");
        let saved_at = Utc::now();
        settings::save(
            &paths,
            &settings::StoredUpdateSettings {
                last_auto_check_at: Some(saved_at),
                ..settings::StoredUpdateSettings::default()
            },
        )
        .expect("seed settings");

        let saved = state
            .save_settings(UpdateSettingsDto {
                auto_check: false,
                auto_download: true,
                check_interval_hours: 168,
                check_source_preference: CheckSourcePreference::MirrorChyanFirst,
                download_source_preference: DownloadSourcePreference::GithubFirst,
                channel: UpdateChannel::Beta,
                allow_prerelease: true,
                last_auto_check_at: None,
                has_mirror_chyan_cdk: false,
                mirror_chyan_cdk_length: None,
            })
            .expect("save settings");

        assert_eq!(saved.last_auto_check_at, Some(saved_at));
        assert_eq!(
            settings::load(&paths)
                .expect("reload persisted settings")
                .last_auto_check_at,
            Some(saved_at)
        );
    }

    #[test]
    fn save_state_normalizes_runtime_version_and_missing_asset() {
        let paths = test_paths("updater-save-state-normalizes");
        let updater = UpdaterState::with_paths_and_version(paths.clone(), "1.0.9");
        let mut update_state = UpdateStateDto::idle_with_version("0.0.0");
        update_state.status = UpdateStatus::Downloaded;
        update_state.latest_version = Some("1.1.0".into());
        update_state.asset_path = Some(
            paths
                .downloads_dir()
                .join("missing.zip")
                .to_string_lossy()
                .to_string(),
        );

        updater
            .save_state(&update_state)
            .expect("save normalized update state");
        let saved = state::load_with_current_version(&paths, "1.0.9").expect("load saved state");

        assert_eq!(saved.current_version, "1.0.9");
        assert_eq!(saved.status, UpdateStatus::Failed);
        assert_eq!(
            saved
                .last_error
                .as_ref()
                .and_then(|error| error.action.as_deref()),
            Some("retryDownload")
        );
    }

    #[test]
    fn settings_tolerate_unavailable_secure_store() {
        let paths = test_paths("updater-settings-keyring-unavailable");
        let state = UpdaterState::with_paths_version_and_cdk_store(
            paths,
            version::CURRENT_APP_VERSION,
            cdk_store::CdkStore::invalid_for_tests(),
        );

        let settings = state.settings().expect("settings should still load");

        assert!(!settings.has_mirror_chyan_cdk);
    }

    #[test]
    fn has_mirror_chyan_cdk_propagates_unavailable_secure_store() {
        let state = UpdaterState::with_paths_version_and_cdk_store(
            test_paths("updater-has-cdk-keyring-unavailable"),
            version::CURRENT_APP_VERSION,
            cdk_store::CdkStore::invalid_for_tests(),
        );

        let error = state
            .has_mirror_chyan_cdk()
            .expect_err("direct CDK status should propagate secure store failures");

        assert_eq!(error.code, "updateSecureStoreUnavailable");
    }

    #[test]
    fn install_prepare_session_tracks_pending_ready_and_failed_windows() {
        let state = UpdaterState::with_paths(test_paths("updater-install-prepare"));
        let request_id =
            state.begin_install_prepare(vec!["main".to_string(), "notepad-1".to_string()]);

        assert_eq!(
            state.poll_install_prepare(&request_id),
            InstallPrepareState::Pending {
                pending_labels: vec!["main".into(), "notepad-1".into()]
            }
        );

        state.report_install_prepare(&request_id, "main", InstallPrepareWindowStatus::Ready);
        assert_eq!(
            state.poll_install_prepare(&request_id),
            InstallPrepareState::Pending {
                pending_labels: vec!["notepad-1".into()]
            }
        );

        state.report_install_prepare(
            &request_id,
            "notepad-1",
            InstallPrepareWindowStatus::Failed("save failed".into()),
        );
        assert_eq!(
            state.poll_install_prepare(&request_id),
            InstallPrepareState::Failed {
                window_label: "notepad-1".into(),
                message: "save failed".into()
            }
        );
    }

    #[test]
    fn install_prepare_session_syncs_new_and_closed_windows() {
        let state = UpdaterState::with_paths(test_paths("updater-install-prepare-sync"));
        let request_id =
            state.begin_install_prepare(vec!["main".to_string(), "notepad-1".to_string()]);
        state.report_install_prepare(&request_id, "main", InstallPrepareWindowStatus::Ready);

        let added = state.sync_install_prepare_labels(
            &request_id,
            vec!["main".to_string(), "tile-1".to_string()],
        );

        assert_eq!(added, vec!["tile-1".to_string()]);
        assert_eq!(
            state.poll_install_prepare(&request_id),
            InstallPrepareState::Pending {
                pending_labels: vec!["tile-1".into()]
            }
        );

        state.report_install_prepare(&request_id, "tile-1", InstallPrepareWindowStatus::Ready);
        assert_eq!(
            state.poll_install_prepare(&request_id),
            InstallPrepareState::Ready
        );
    }
}
