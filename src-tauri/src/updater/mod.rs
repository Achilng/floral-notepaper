pub mod cdk_store;
pub mod check;
pub mod commands;
pub mod download;
pub mod errors;
pub mod helper;
pub mod install;
pub mod manifest;
pub mod platform;
pub mod settings;
pub mod state;
pub mod types;
pub mod version;

use crate::services::notes::{default_store, AppError};
use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

pub const APP_ID: &str = "com.floral-notepaper.app";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateTaskKind {
    Check,
    Download,
    Install,
}

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
        if let Ok(mut slot) = self.active_task.lock() {
            if slot.as_ref().map(|task| task.id) == Some(self.task_id) {
                *slot = None;
            }
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
    cdk_store: cdk_store::CdkStore,
    active_task: Arc<Mutex<Option<ActiveUpdateTask>>>,
    next_task_id: AtomicU64,
}

impl UpdaterState {
    pub fn new() -> Self {
        Self {
            paths: UpdatePaths::new(default_updates_dir()),
            cdk_store: cdk_store::CdkStore::default(),
            active_task: Arc::new(Mutex::new(None)),
            next_task_id: AtomicU64::new(1),
        }
    }

    pub fn initialize(&self) -> Result<(), AppError> {
        self.paths.ensure_dirs()?;
        let _ = settings::load(&self.paths)?;
        let _ = state::recover(&self.paths)?;
        download::cleanup_partial_downloads(&self.paths)?;
        Ok(())
    }

    pub fn paths(&self) -> &UpdatePaths {
        &self.paths
    }

    pub fn settings(&self) -> Result<types::UpdateSettingsDto, AppError> {
        let settings = settings::load(&self.paths)?;
        let has_mirror_cdk = self.cdk_store.has_cdk().unwrap_or(false);
        Ok(settings.into_dto(has_mirror_cdk))
    }

    pub fn save_settings(
        &self,
        settings: types::UpdateSettingsDto,
    ) -> Result<types::UpdateSettingsDto, AppError> {
        let stored = settings::StoredUpdateSettings::from_dto(settings);
        settings::save(&self.paths, &stored)?;
        let has_mirror_cdk = self.cdk_store.has_cdk().unwrap_or(false);
        Ok(stored.into_dto(has_mirror_cdk))
    }

    pub fn set_mirror_cdk(&self, cdk: &str) -> Result<(), AppError> {
        self.cdk_store.set_cdk(cdk)
    }

    pub fn clear_mirror_cdk(&self) -> Result<(), AppError> {
        self.cdk_store.clear_cdk()
    }

    pub fn has_mirror_cdk(&self) -> bool {
        self.cdk_store.has_cdk().unwrap_or(false)
    }

    pub fn load_state(&self) -> Result<types::UpdateStateDto, AppError> {
        state::load(&self.paths)
    }

    pub fn save_state(&self, update_state: &types::UpdateStateDto) -> Result<(), AppError> {
        state::save(&self.paths, update_state)
    }

    pub fn begin_task(&self, kind: UpdateTaskKind) -> Result<ActiveTaskGuard, AppError> {
        let mut slot = self
            .active_task
            .lock()
            .map_err(|_| errors::app_error("updateStateCorrupted", "更新任务锁状态异常"))?;

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
        let slot = self
            .active_task
            .lock()
            .map_err(|_| errors::app_error("updateStateCorrupted", "更新任务锁状态异常"))?;

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
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self::new()
    }
}

fn default_updates_dir() -> PathBuf {
    if let Ok(store) = default_store() {
        return store.base_dir().join("updates");
    }

    env::current_dir()
        .unwrap_or_else(|_| env::temp_dir())
        .join("floral-notepaper")
        .join("updates")
}
