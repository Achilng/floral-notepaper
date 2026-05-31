use crate::services::notes::{
    default_store, AppConfig, AppError, Note, NoteStore, SaveNoteRequest,
};
use chrono::{DateTime, Utc};
use reqwest::{
    header::{HeaderMap, ETAG, IF_MATCH, IF_NONE_MATCH},
    Method, StatusCode,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

const NOTES_CHANGED_EVENT: &str = "notes-changed";
pub const SYNC_STATUS_CHANGED_EVENT: &str = "sync-status-changed";
const SYNC_DEBOUNCE_SECONDS: u64 = 10;
const SYNC_MAX_DIRTY_SECONDS: u64 = 60;
const SYNC_CONNECT_TIMEOUT_SECONDS: u64 = 8;
const SYNC_HTTP_TIMEOUT_SECONDS: u64 = 30;
const SYNC_RETRY_COOLDOWN_SECONDS: u64 = 15;
const SYNC_AUTH_RETRY_COOLDOWN_SECONDS: u64 = 60;
const SYNC_CONFIG_REQUIRED_MESSAGE: &str = "\u{8bf7}\u{5148}\u{586b}\u{5199} WebDAV \u{5730}\u{5740}\u{3001}\u{8d26}\u{53f7}\u{548c}\u{5bc6}\u{7801}\u{3002}";
const DEFAULT_SYNC_ROOT_DIR: &str = "floral-sync";
const REMOTE_NOTES_DIR: &str = "notes";
const REMOTE_STATE_DIR: &str = "floral-sync-meta";
const REMOTE_ARCHIVE_DIR: &str = "floral-sync-meta/archive";
const REMOTE_MANIFEST_PATH: &str = "floral-sync-meta/manifest.json";

static SYNC_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

enum AutoSyncCommand {
    Startup,
    LocalChange,
    ConfigChanged,
}

pub struct AutoSyncController {
    sender: mpsc::UnboundedSender<AutoSyncCommand>,
    sync_lock: Mutex<()>,
}

impl AutoSyncController {
    fn new(sender: mpsc::UnboundedSender<AutoSyncCommand>) -> Self {
        Self {
            sender,
            sync_lock: Mutex::new(()),
        }
    }

    fn send(&self, command: AutoSyncCommand) {
        let _ = self.sender.send(command);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub enabled: bool,
    pub configured: bool,
    pub last_revision: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncedNoteState {
    pub content_hash: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TombstoneState {
    pub deleted_at: DateTime<Utc>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub version: u32,
    pub device_id: String,
    pub last_manifest_revision: String,
    #[serde(default)]
    pub notes: HashMap<String, SyncedNoteState>,
    #[serde(default)]
    pub tombstones: HashMap<String, TombstoneState>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub category_tombstones: HashMap<String, DateTime<Utc>>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            version: 1,
            device_id: Uuid::new_v4().to_string(),
            last_manifest_revision: String::new(),
            notes: HashMap::new(),
            tombstones: HashMap::new(),
            categories: Vec::new(),
            category_tombstones: HashMap::new(),
            last_sync_at: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RemoteManifest {
    version: u32,
    generated_at: DateTime<Utc>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    category_tombstones: Vec<CategoryTombstoneEntry>,
    #[serde(default)]
    entries: Vec<ManifestEntry>,
}

impl Default for RemoteManifest {
    fn default() -> Self {
        Self {
            version: 1,
            generated_at: DateTime::UNIX_EPOCH,
            categories: Vec::new(),
            category_tombstones: Vec::new(),
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct CategoryTombstoneEntry {
    name: String,
    deleted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    id: String,
    title: String,
    category: String,
    path: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    content_hash: String,
}

#[derive(Debug, Clone)]
struct RemoteSnapshot {
    manifest: RemoteManifest,
    revision: String,
    etag: Option<String>,
    exists: bool,
}

#[derive(Debug, Clone)]
enum LocalChange {
    Upsert {
        note: Note,
        content_hash: String,
    },
    Delete {
        id: String,
        deleted_at: DateTime<Utc>,
        content_hash: String,
    },
}

enum ManifestSaveOutcome {
    Saved(String),
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteMutationOutcome {
    Applied,
    Missing,
    PermissionDenied,
    Unsupported,
}

#[derive(Debug, Clone)]
struct RemoteCleanup {
    path: String,
}

pub fn setup_auto_sync(app: &AppHandle) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let controller = Arc::new(AutoSyncController::new(sender));
    app.manage(controller.clone());
    controller.send(AutoSyncCommand::Startup);
    spawn_auto_sync_worker(app.clone(), controller, receiver);
}

pub fn request_auto_sync(app: &AppHandle) {
    if let Some(controller) = app.try_state::<Arc<AutoSyncController>>() {
        controller.send(AutoSyncCommand::LocalChange);
    }
}

pub fn notify_auto_sync_config_changed(app: &AppHandle) {
    if let Some(controller) = app.try_state::<Arc<AutoSyncController>>() {
        controller.send(AutoSyncCommand::ConfigChanged);
    }
}

pub fn status(store: &NoteStore, config: &AppConfig) -> Result<SyncStatus, AppError> {
    let state = load_state(store)?;
    Ok(status_from_state(config, &state))
}

pub async fn test_connection(
    store: &NoteStore,
    config: &AppConfig,
) -> Result<SyncStatus, AppError> {
    let mut state = load_state(store)?;
    if !is_configured(config) {
        let error = AppError::new("syncConfigIncomplete", SYNC_CONFIG_REQUIRED_MESSAGE);
        state.last_error = Some(error.message.clone());
        save_state(store, &state)?;
        return Err(error);
    }

    match verify_webdav_connection(config).await {
        Ok(()) => {
            state.last_error = None;
            save_state(store, &state)?;
            Ok(status_from_state(config, &state))
        }
        Err(error) => {
            state.last_error = Some(error.message.clone());
            save_state(store, &state)?;
            Err(error)
        }
    }
}

pub async fn sync_now(store: &NoteStore, config: &AppConfig) -> Result<SyncStatus, AppError> {
    let mut state = load_state(store)?;
    if !is_configured(config) {
        let error = AppError::new("syncConfigIncomplete", SYNC_CONFIG_REQUIRED_MESSAGE);
        state.last_error = Some(error.message.clone());
        save_state(store, &state)?;
        return Err(error);
    }

    match sync_inner(store, config, &mut state).await {
        Ok(()) => {
            state.last_sync_at = Some(Utc::now());
            state.last_error = None;
            save_state(store, &state)?;
            Ok(status_from_state(config, &state))
        }
        Err(error) => {
            state.last_error = Some(error.message.clone());
            save_state(store, &state)?;
            Err(error)
        }
    }
}

pub async fn sync_now_for_app(app: &AppHandle) -> Result<SyncStatus, AppError> {
    if let Some(controller) = app.try_state::<Arc<AutoSyncController>>() {
        run_sync_operation(app, Some(controller.inner())).await
    } else {
        run_sync_operation(app, None).await
    }
}

pub async fn sync_before_app_exit(app: &AppHandle) -> Result<(), AppError> {
    let store = default_store()?;
    let config = store.load_config()?;
    if !should_sync_on_app_exit(&config) {
        return Ok(());
    }

    sync_now_for_app(app).await.map(|_| ())
}

fn spawn_auto_sync_worker(
    app: AppHandle,
    controller: Arc<AutoSyncController>,
    mut receiver: mpsc::UnboundedReceiver<AutoSyncCommand>,
) {
    tauri::async_runtime::spawn(async move {
        let mut startup_pending = true;
        let mut dirty_since: Option<Instant> = None;
        let mut last_local_change: Option<Instant> = None;
        let mut last_poll_at = Instant::now();
        let mut next_attempt_at = Instant::now();

        loop {
            tokio::select! {
                command = receiver.recv() => {
                    let Some(command) = command else {
                        break;
                    };
                    match command {
                        AutoSyncCommand::Startup => {
                            startup_pending = true;
                        }
                        AutoSyncCommand::LocalChange => {
                            let now = Instant::now();
                            dirty_since.get_or_insert(now);
                            last_local_change = Some(now);
                        }
                        AutoSyncCommand::ConfigChanged => {
                            startup_pending = true;
                            dirty_since = None;
                            last_local_change = None;
                            next_attempt_at = Instant::now();
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            }

            let Ok(store) = default_store() else {
                continue;
            };
            let Ok(config) = store.load_config() else {
                continue;
            };

            if !config.sync_enabled || !is_configured(&config) {
                startup_pending = false;
                dirty_since = None;
                last_local_change = None;
                continue;
            }

            let now = Instant::now();
            if now < next_attempt_at {
                continue;
            }
            let poll_interval = Duration::from_secs(config.sync_interval_seconds.max(30) as u64);
            let poll_due = now.duration_since(last_poll_at) >= poll_interval;
            let debounce_due = last_local_change
                .map(|changed_at| {
                    now.duration_since(changed_at) >= Duration::from_secs(SYNC_DEBOUNCE_SECONDS)
                })
                .unwrap_or(false);
            let max_due = dirty_since
                .map(|started_at| {
                    now.duration_since(started_at) >= Duration::from_secs(SYNC_MAX_DIRTY_SECONDS)
                })
                .unwrap_or(false);

            if !startup_pending && !poll_due && !debounce_due && !max_due {
                continue;
            }

            match run_sync_operation(&app, Some(&controller)).await {
                Ok(_) => {
                    startup_pending = false;
                    dirty_since = None;
                    last_local_change = None;
                    let completed_at = Instant::now();
                    last_poll_at = completed_at;
                    next_attempt_at = completed_at;
                }
                Err(error) => {
                    startup_pending = false;
                    let retry_at = Instant::now() + retry_cooldown(&error);
                    next_attempt_at = retry_at;
                    last_poll_at = Instant::now();
                    if dirty_since.is_some() {
                        last_local_change = Some(retry_at);
                    }
                }
            }
        }
    });
}

async fn run_sync_operation(
    app: &AppHandle,
    controller: Option<&AutoSyncController>,
) -> Result<SyncStatus, AppError> {
    let _guard = match controller {
        Some(runtime) => Some(runtime.sync_lock.lock().await),
        None => None,
    };

    let store = default_store()?;
    let config = store.load_config()?;
    match sync_now(&store, &config).await {
        Ok(status) => {
            emit_sync_status(app, &status);
            let _ = app.emit(NOTES_CHANGED_EVENT, ());
            Ok(status)
        }
        Err(error) => {
            if let Ok(status) = status(&store, &config) {
                emit_sync_status(app, &status);
            }
            Err(error)
        }
    }
}

fn emit_sync_status(app: &AppHandle, status: &SyncStatus) {
    let _ = app.emit(SYNC_STATUS_CHANGED_EVENT, status);
}

async fn sync_inner(
    store: &NoteStore,
    config: &AppConfig,
    state: &mut SyncState,
) -> Result<(), AppError> {
    with_sync_step(
        "\u{51c6}\u{5907}\u{540c}\u{6b65}\u{76ee}\u{5f55}",
        ensure_sync_root(config).await,
    )?;
    with_sync_step(
        "\u{51c6}\u{5907}\u{7b14}\u{8bb0}\u{76ee}\u{5f55}",
        ensure_remote_collection(config, REMOTE_NOTES_DIR).await,
    )?;
    with_sync_step(
        "\u{51c6}\u{5907}\u{540c}\u{6b65}\u{5143}\u{6570}\u{636e}\u{76ee}\u{5f55}",
        ensure_remote_collection(config, REMOTE_STATE_DIR).await,
    )?;
    with_sync_step(
        "\u{51c6}\u{5907}\u{5f52}\u{6863}\u{76ee}\u{5f55}",
        ensure_remote_collection(config, REMOTE_ARCHIVE_DIR).await,
    )?;

    for _attempt in 0..3 {
        let snapshot = with_sync_step(
            "\u{8bfb}\u{53d6}\u{8fdc}\u{7aef}\u{540c}\u{6b65}\u{6e05}\u{5355}",
            fetch_manifest(config).await,
        )?;
        with_sync_step(
            "\u{5e94}\u{7528}\u{8fdc}\u{7aef}\u{53d8}\u{66f4}",
            apply_remote_manifest(store, state, config, &snapshot.manifest).await,
        )?;

        let mut baseline_state = state.clone();
        rebuild_state_from_manifest(&mut baseline_state, &snapshot.manifest);

        let local_changes = collect_local_changes(store, &baseline_state)?;
        let local_categories = normalized_categories(store.list_categories()?);
        let local_category_tombstones =
            collect_local_category_tombstones(store, &baseline_state, &local_categories)?;

        if local_changes.is_empty()
            && categories_match(&snapshot.manifest.categories, &local_categories)
            && category_tombstones_match(&snapshot.manifest, &local_category_tombstones)
        {
            state.last_manifest_revision = snapshot.revision;
            state.categories = local_categories;
            state.category_tombstones = local_category_tombstones;
            state.notes = baseline_state.notes;
            state.tombstones = baseline_state.tombstones;
            return Ok(());
        }

        let mut next_manifest = snapshot.manifest.clone();
        let remote_cleanups = with_sync_step(
            "\u{4e0a}\u{4f20}\u{672c}\u{5730}\u{7b14}\u{8bb0}\u{53d8}\u{66f4}",
            push_local_changes(config, &mut next_manifest, &local_changes).await,
        )?;

        let removed_categories: Vec<String> = snapshot
            .manifest
            .categories
            .iter()
            .filter(|category| local_category_tombstones.contains_key(*category))
            .cloned()
            .collect();
        if !removed_categories.is_empty() {
            with_sync_step(
                "\u{6e05}\u{7406}\u{8fdc}\u{7aef}\u{5df2}\u{5220}\u{9664}\u{5206}\u{7c7b}\u{76ee}\u{5f55}",
                cleanup_remote_category_directories(config, &removed_categories).await,
            )?;
        }

        next_manifest.generated_at = Utc::now();
        next_manifest.categories = local_categories.clone();
        next_manifest.category_tombstones =
            category_tombstone_entries(&local_category_tombstones, &local_categories);

        match with_sync_step(
            "\u{5199}\u{5165}\u{8fdc}\u{7aef}\u{540c}\u{6b65}\u{6e05}\u{5355}",
            save_manifest(
                config,
                &next_manifest,
                &snapshot.revision,
                snapshot.etag.as_deref(),
                snapshot.exists,
            )
            .await,
        )? {
            ManifestSaveOutcome::Saved(revision) => {
                run_remote_cleanups(config, &remote_cleanups).await;
                rebuild_state_from_manifest(state, &next_manifest);
                state.categories = local_categories;
                state.category_tombstones = local_category_tombstones;
                state.last_manifest_revision = revision;
                return Ok(());
            }
            ManifestSaveOutcome::Conflict => continue,
        }
    }

    Err(AppError::new(
        "syncConflict",
        "\u{540c}\u{6b65}\u{8fc7}\u{7a0b}\u{4e2d}\u{8fdc}\u{7aef}\u{76ee}\u{5f55}\u{53d1}\u{751f}\u{4e86}\u{53d8}\u{5316}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{91cd}\u{8bd5}\u{3002}",
    ))
}

fn rebuild_state_from_manifest(state: &mut SyncState, manifest: &RemoteManifest) {
    state.notes.clear();
    state.tombstones.clear();
    state.categories = normalized_categories(manifest.categories.clone());
    state.category_tombstones = manifest_category_tombstones(manifest);
    for entry in &manifest.entries {
        if let Some(deleted_at) = entry.deleted_at {
            state.tombstones.insert(
                entry.id.clone(),
                TombstoneState {
                    deleted_at,
                    content_hash: entry.content_hash.clone(),
                },
            );
        } else {
            state.notes.insert(
                entry.id.clone(),
                SyncedNoteState {
                    content_hash: entry.content_hash.clone(),
                    updated_at: entry.updated_at,
                },
            );
        }
    }
}

async fn apply_remote_manifest(
    store: &NoteStore,
    state: &SyncState,
    config: &AppConfig,
    manifest: &RemoteManifest,
) -> Result<(), AppError> {
    for entry in &manifest.entries {
        apply_remote_entry(store, state, config, entry).await?;
    }

    let remote_category_tombstones = manifest_category_tombstones(manifest);
    apply_remote_categories(
        store,
        state,
        &manifest.categories,
        &remote_category_tombstones,
    )?;
    Ok(())
}

fn apply_remote_categories(
    store: &NoteStore,
    state: &SyncState,
    remote_categories: &[String],
    remote_category_tombstones: &HashMap<String, DateTime<Utc>>,
) -> Result<(), AppError> {
    let remote_categories = normalized_categories(remote_categories.to_vec());
    for category in &remote_categories {
        if locally_deleted_category(state, category) {
            continue;
        }
        store.create_category(category)?;
    }

    let notes = store.list_notes()?;
    let categories_in_use: HashSet<String> = notes
        .into_iter()
        .map(|note| note.category)
        .filter(|category| !category.is_empty())
        .collect();

    for local in store.list_categories()? {
        let remote_deleted = remote_category_tombstones.contains_key(&local);
        let previously_synced_active = state.categories.contains(&local);
        let previously_tombstoned = state.category_tombstones.contains_key(&local);
        let recreated_after_tombstone =
            remote_deleted && !previously_synced_active && previously_tombstoned;

        if !remote_categories.contains(&local)
            && !categories_in_use.contains(&local)
            && (remote_deleted
                || ((previously_synced_active || previously_tombstoned)
                    && !recreated_after_tombstone))
        {
            store.delete_category(&local)?;
        }
    }

    Ok(())
}

async fn apply_remote_entry(
    store: &NoteStore,
    state: &SyncState,
    config: &AppConfig,
    entry: &ManifestEntry,
) -> Result<(), AppError> {
    let local = match store.read_note(&entry.id) {
        Ok(note) => Some(note),
        Err(error) if error.code == "noteNotFound" => None,
        Err(error) => return Err(error),
    };
    let local_changed = local
        .as_ref()
        .map(|note| local_note_changed(note, state))
        .unwrap_or(false);

    if let Some(deleted_at) = entry.deleted_at {
        if let Some(local_note) = local {
            if local_changed && local_note.updated_at > deleted_at {
                return Ok(());
            }
            if local_changed {
                create_conflict_backup(store, &local_note)?;
            }
            store.delete_synced_note(&entry.id)?;
        }
        return Ok(());
    }

    // When the note no longer exists locally but was previously tracked, treat it as
    // an intentional local deletion and do not resurrect it from the remote copy.
    // The deletion will be pushed to the remote manifest later in this sync round.
    if local.is_none() && locally_deleted_note(state, &entry.id) {
        return Ok(());
    }

    if let Some(local_note) = &local {
        if local_changed && local_note.updated_at > entry.updated_at {
            return Ok(());
        }
    }

    let Some(content) = get_text(config, &entry.path).await? else {
        return Err(AppError::new(
            "syncManifestMissingFile",
            "\u{8fdc}\u{7aef}\u{540c}\u{6b65}\u{6e05}\u{5355}\u{5f15}\u{7528}\u{7684}\u{7b14}\u{8bb0}\u{6587}\u{4ef6}\u{4e0d}\u{5b58}\u{5728}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5} WebDAV \u{76ee}\u{5f55}\u{3002}",
        ));
    };
    if manifest_entry_hash(entry, &content) != entry.content_hash {
        return Err(AppError::new(
            "syncManifestMismatch",
            "远端同步清单和笔记内容不一致，请稍后重试或检查 WebDAV 文件是否被手动修改。",
        ));
    }

    apply_remote_entry_content(store, entry, &content, local, local_changed)
}

fn apply_remote_entry_content(
    store: &NoteStore,
    entry: &ManifestEntry,
    content: &str,
    local: Option<Note>,
    local_changed: bool,
) -> Result<(), AppError> {
    if let Some(local_note) = local {
        if local_changed && note_hash(&local_note) != entry.content_hash {
            create_conflict_backup(store, &local_note)?;
        }
    }

    store.upsert_synced_note(
        &entry.id,
        &entry.title,
        content,
        &entry.category,
        entry.created_at,
        entry.updated_at,
    )?;
    Ok(())
}

fn collect_local_changes(
    store: &NoteStore,
    state: &SyncState,
) -> Result<Vec<LocalChange>, AppError> {
    let notes = store.list_note_contents()?;
    let mut current_ids = HashSet::new();
    let mut changes = Vec::new();

    for note in notes {
        current_ids.insert(note.id.clone());
        let content_hash = note_hash(&note);
        let known = state.notes.get(&note.id);
        if known.map(|entry| entry.content_hash.as_str()) != Some(content_hash.as_str()) {
            changes.push(LocalChange::Upsert { note, content_hash });
        }
    }

    for (id, known) in &state.notes {
        if current_ids.contains(id) || state.tombstones.contains_key(id) {
            continue;
        }
        let deleted_at = Utc::now();
        changes.push(LocalChange::Delete {
            id: id.clone(),
            deleted_at,
            content_hash: deleted_hash(id, deleted_at),
        });
        let _ = known;
    }

    Ok(changes)
}

fn collect_local_category_tombstones(
    store: &NoteStore,
    state: &SyncState,
    local_categories: &[String],
) -> Result<HashMap<String, DateTime<Utc>>, AppError> {
    let notes = store.list_notes()?;
    let categories_in_use: HashSet<String> = notes
        .into_iter()
        .map(|note| note.category)
        .filter(|category| !category.is_empty())
        .collect();

    let mut tombstones = state.category_tombstones.clone();
    for category in local_categories {
        tombstones.remove(category);
    }

    for category in &state.categories {
        if !local_categories.contains(category) && !categories_in_use.contains(category) {
            tombstones.entry(category.clone()).or_insert_with(Utc::now);
        }
    }

    Ok(tombstones)
}

async fn push_local_changes(
    config: &AppConfig,
    manifest: &mut RemoteManifest,
    changes: &[LocalChange],
) -> Result<Vec<RemoteCleanup>, AppError> {
    let mut cleanups = Vec::new();
    for change in changes {
        match change {
            LocalChange::Upsert { note, content_hash } => {
                let path = with_sync_step(
                    &format!("写入远端笔记《{}》", sync_note_label(&note.title, &note.id)),
                    upload_remote_note(config, note, content_hash).await,
                )?;

                if let Some(previous) = manifest_entry(manifest, &note.id).cloned() {
                    if previous.deleted_at.is_none() && previous.path != path {
                        with_sync_step(
                            &format!(
                                "清理远端旧笔记备份《{}》",
                                sync_note_label(&previous.title, &previous.id)
                            ),
                            cleanup_replaced_remote_note(config, &previous.path).await,
                        )?;
                    }
                }

                if let Some(previous) = manifest_entry(manifest, &note.id).cloned() {
                    if previous.deleted_at.is_none() && previous.path != path {
                        cleanups.push(RemoteCleanup {
                            path: previous.path,
                        });
                    }
                }

                upsert_manifest_entry(
                    manifest,
                    ManifestEntry {
                        id: note.id.clone(),
                        title: note.title.clone(),
                        category: note.category.clone(),
                        path,
                        created_at: note.created_at,
                        updated_at: note.updated_at,
                        deleted_at: None,
                        content_hash: content_hash.clone(),
                    },
                );
            }
            LocalChange::Delete {
                id,
                deleted_at,
                content_hash,
            } => {
                let existing = manifest_entry(manifest, id).cloned();
                let archive_path = remote_archive_path(existing.as_ref(), id, *deleted_at);
                ensure_remote_parent(config, &archive_path).await?;

                if let Some(previous) = &existing {
                    if previous.deleted_at.is_none() {
                        with_sync_step(
                            &format!(
                                "归档远端已删除笔记《{}》",
                                sync_note_label(&previous.title, &previous.id)
                            ),
                            archive_deleted_remote_note(config, &previous.path, &archive_path)
                                .await,
                        )?;
                        if previous.path != archive_path {
                            cleanups.push(RemoteCleanup {
                                path: previous.path.clone(),
                            });
                        }
                    }
                }

                let (title, category, created_at, updated_at) = existing
                    .map(|entry| {
                        (
                            entry.title,
                            entry.category,
                            entry.created_at,
                            entry.updated_at,
                        )
                    })
                    .unwrap_or_else(|| (String::new(), String::new(), *deleted_at, *deleted_at));

                upsert_manifest_entry(
                    manifest,
                    ManifestEntry {
                        id: id.clone(),
                        title,
                        category,
                        path: archive_path,
                        created_at,
                        updated_at,
                        deleted_at: Some(*deleted_at),
                        content_hash: content_hash.clone(),
                    },
                );
            }
        }
    }

    manifest.categories = normalized_categories(manifest.categories.clone());
    Ok(cleanups)
}

fn manifest_entry<'a>(manifest: &'a RemoteManifest, id: &str) -> Option<&'a ManifestEntry> {
    manifest.entries.iter().find(|entry| entry.id == id)
}

fn upsert_manifest_entry(manifest: &mut RemoteManifest, entry: ManifestEntry) {
    if let Some(existing) = manifest
        .entries
        .iter_mut()
        .find(|candidate| candidate.id == entry.id)
    {
        *existing = entry;
    } else {
        manifest.entries.push(entry);
    }
}

async fn fetch_manifest(config: &AppConfig) -> Result<RemoteSnapshot, AppError> {
    let client = sync_http_client()?;
    let response = client
        .get(webdav_url(config, REMOTE_MANIFEST_PATH)?)
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .send()
        .await
        .map_err(sync_transport_error)?;

    if response.status() == StatusCode::NOT_FOUND {
        let manifest = RemoteManifest::default();
        return Ok(RemoteSnapshot {
            revision: manifest_revision(None, &manifest),
            manifest,
            etag: None,
            exists: false,
        });
    }

    let response = response.error_for_status().map_err(sync_transport_error)?;
    let etag = response_etag(response.headers());
    let revision = response_revision(response.headers(), None);
    let text = response.text().await.map_err(sync_transport_error)?;
    let manifest: RemoteManifest = serde_json::from_str(&text).map_err(|_| {
        AppError::new(
            "syncManifestInvalid",
            "WebDAV \u{4e2d}\u{7684}\u{540c}\u{6b65}\u{6e05}\u{5355}\u{635f}\u{574f}\u{4e86}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5} floral-sync-meta/manifest.json\u{3002}",
        )
    })?;
    Ok(RemoteSnapshot {
        revision: revision.unwrap_or_else(|| manifest_revision(etag.as_deref(), &manifest)),
        manifest,
        etag,
        exists: true,
    })
}

async fn save_manifest(
    config: &AppConfig,
    manifest: &RemoteManifest,
    expected_revision: &str,
    current_etag: Option<&str>,
    manifest_exists: bool,
) -> Result<ManifestSaveOutcome, AppError> {
    ensure_remote_parent(config, REMOTE_MANIFEST_PATH).await?;

    if current_etag.is_none()
        && !latest_manifest_revision_matches(config, expected_revision).await?
    {
        return Ok(ManifestSaveOutcome::Conflict);
    }

    let body = serde_json::to_vec_pretty(manifest)?;
    for content_type in put_retry_content_types("application/json; charset=utf-8") {
        let response = send_put_request(
            config,
            REMOTE_MANIFEST_PATH,
            &body,
            content_type,
            current_etag,
            !manifest_exists,
        )
        .await?;

        if response.status() == StatusCode::PRECONDITION_FAILED {
            return Ok(ManifestSaveOutcome::Conflict);
        }

        match response.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::NO_CONTENT => {
                let etag = response_etag(response.headers());
                let revision = response_revision(response.headers(), Some(manifest))
                    .unwrap_or_else(|| manifest_revision(etag.as_deref(), manifest));
                return Ok(ManifestSaveOutcome::Saved(revision));
            }
            status if should_retry_put_with_compatibility_headers(status) => continue,
            status => return Err(sync_status_error(status)),
        }
    }

    Err(sync_status_error(StatusCode::METHOD_NOT_ALLOWED))
}

async fn latest_manifest_revision_matches(
    config: &AppConfig,
    expected_revision: &str,
) -> Result<bool, AppError> {
    Ok(fetch_manifest(config).await?.revision == expected_revision)
}

async fn verify_webdav_connection(config: &AppConfig) -> Result<(), AppError> {
    probe_webdav_endpoint(config).await?;
    probe_webdav_write_access(config).await
}

async fn probe_webdav_write_access(config: &AppConfig) -> Result<(), AppError> {
    ensure_remote_collection(config, REMOTE_NOTES_DIR).await?;
    ensure_remote_collection(config, REMOTE_STATE_DIR).await?;

    let probe_id = Uuid::new_v4().simple().to_string();
    let note_probe_path = format!("{REMOTE_NOTES_DIR}/floral-write-probe-{probe_id}.md");
    let state_probe_path = format!("{REMOTE_STATE_DIR}/connection-test-{probe_id}.json");

    with_sync_step(
        "测试写入笔记目录",
        put_text(
            config,
            &note_probe_path,
            "# floral write probe\n",
            "text/markdown; charset=utf-8",
        )
        .await,
    )?;
    with_sync_step(
        "测试写入同步元数据目录",
        put_text(
            config,
            &state_probe_path,
            "{\"probe\":true}",
            "application/json; charset=utf-8",
        )
        .await,
    )?;

    let _ = delete_resource(config, &note_probe_path).await;
    let _ = delete_resource(config, &state_probe_path).await;
    Ok(())
}

async fn upload_remote_note(
    config: &AppConfig,
    note: &Note,
    content_hash: &str,
) -> Result<String, AppError> {
    let preferred_path = remote_note_path(note, content_hash);
    ensure_remote_parent(config, &preferred_path).await?;

    match put_text(
        config,
        &preferred_path,
        &note.content,
        "text/markdown; charset=utf-8",
    )
    .await
    {
        Ok(()) => Ok(preferred_path),
        Err(error) => {
            let fallback_path = compatibility_remote_note_path(note, content_hash);
            if !should_retry_note_upload_with_compat_path(&preferred_path, &fallback_path, &error) {
                return Err(error);
            }

            ensure_remote_parent(config, &fallback_path).await?;
            put_text(
                config,
                &fallback_path,
                &note.content,
                "text/markdown; charset=utf-8",
            )
            .await?;
            Ok(fallback_path)
        }
    }
}

async fn probe_webdav_endpoint(config: &AppConfig) -> Result<(), AppError> {
    let client = sync_http_client()?;
    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").expect("valid PROPFIND method"),
            webdav_base_url(config)?,
        )
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .header("Depth", "0")
        .send()
        .await
        .map_err(sync_transport_error)?;

    match response.status() {
        StatusCode::MULTI_STATUS | StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
        StatusCode::NOT_FOUND => ensure_sync_root(config).await,
        status => Err(sync_status_error(status)),
    }
}

async fn ensure_sync_root(config: &AppConfig) -> Result<(), AppError> {
    let client = sync_http_client()?;
    let current = webdav_base_url(config)?;
    match remote_collection_status(config, current.clone()).await? {
        StatusCode::MULTI_STATUS | StatusCode::OK | StatusCode::NO_CONTENT => return Ok(()),
        StatusCode::NOT_FOUND => {}
        status => return Err(sync_status_error(status)),
    }

    let response = client
        .request(
            Method::from_bytes(b"MKCOL").expect("valid MKCOL method"),
            current,
        )
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .send()
        .await
        .map_err(sync_transport_error)?;

    match response.status() {
        StatusCode::CREATED | StatusCode::METHOD_NOT_ALLOWED | StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
        StatusCode::FORBIDDEN | StatusCode::CONFLICT => Err(AppError::new(
            "syncRootCreateFailed",
            "\u{65e0}\u{6cd5}\u{81ea}\u{52a8}\u{521b}\u{5efa} floral-sync \u{6587}\u{4ef6}\u{5939}\u{ff0c}\u{8bf7}\u{5148}\u{5728} WebDAV \u{6839}\u{76ee}\u{5f55}\u{4e0b}\u{624b}\u{52a8}\u{521b}\u{5efa} floral-sync \u{6587}\u{4ef6}\u{5939}\u{5e76}\u{786e}\u{4fdd}\u{6709}\u{8bfb}\u{5199}\u{6743}\u{9650}\u{3002}",
        )),
        status => Err(sync_status_error(status)),
    }
}

async fn remote_collection_status(
    config: &AppConfig,
    url: reqwest::Url,
) -> Result<StatusCode, AppError> {
    let client = sync_http_client()?;
    let response = client
        .request(
            Method::from_bytes(b"PROPFIND").expect("valid PROPFIND method"),
            collection_url(url),
        )
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .header("Depth", "0")
        .send()
        .await
        .map_err(sync_transport_error)?;
    Ok(response.status())
}

async fn ensure_remote_collection(config: &AppConfig, relative_dir: &str) -> Result<(), AppError> {
    let normalized = relative_dir.trim_matches('/');
    if normalized.is_empty() {
        return Ok(());
    }

    let client = sync_http_client()?;
    let mut current = String::new();
    for segment in normalized.split('/') {
        if current.is_empty() {
            current.push_str(segment);
        } else {
            current.push('/');
            current.push_str(segment);
        }

        match remote_collection_status(config, webdav_url(config, &current)?).await? {
            StatusCode::MULTI_STATUS | StatusCode::OK | StatusCode::NO_CONTENT => continue,
            StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST => {}
            status => return Err(sync_status_error(status)),
        }

        let response = client
            .request(
                Method::from_bytes(b"MKCOL").expect("valid MKCOL method"),
                webdav_url(config, &current)?,
            )
            .basic_auth(
                config.sync_webdav_username.trim(),
                Some(config.sync_webdav_password.as_str()),
            )
            .send()
            .await
            .map_err(sync_transport_error)?;

        match response.status() {
            StatusCode::CREATED
            | StatusCode::METHOD_NOT_ALLOWED
            | StatusCode::OK
            | StatusCode::NO_CONTENT => {}
            StatusCode::FORBIDDEN | StatusCode::CONFLICT => {
                return Err(AppError::new(
                    "syncCollectionCreateFailed",
                    "\u{65e0}\u{6cd5}\u{81ea}\u{52a8}\u{521b}\u{5efa} WebDAV \u{5b50}\u{76ee}\u{5f55}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{76ee}\u{6807}\u{6839}\u{76ee}\u{5f55}\u{662f}\u{5426}\u{5b58}\u{5728}\u{5e76}\u{786e}\u{4fdd}\u{6709}\u{8bfb}\u{5199}\u{6743}\u{9650}\u{3002}",
                ))
            }
            status => return Err(sync_status_error(status)),
        }
    }

    Ok(())
}

async fn ensure_remote_parent(config: &AppConfig, relative_path: &str) -> Result<(), AppError> {
    let Some((parent, _)) = relative_path.rsplit_once('/') else {
        return Ok(());
    };
    ensure_remote_collection(config, parent).await
}

async fn put_text(
    config: &AppConfig,
    relative_path: &str,
    body: &str,
    content_type: &str,
) -> Result<(), AppError> {
    let bytes = body.as_bytes().to_vec();
    for candidate in put_retry_content_types(content_type) {
        let response =
            send_put_request(config, relative_path, &bytes, candidate, None, false).await?;
        match response.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::NO_CONTENT => return Ok(()),
            status if should_retry_put_with_compatibility_headers(status) => continue,
            status => return Err(sync_status_error(status)),
        }
    }

    Err(sync_status_error(StatusCode::METHOD_NOT_ALLOWED))
}

async fn send_put_request(
    config: &AppConfig,
    relative_path: &str,
    body: &[u8],
    content_type: Option<&str>,
    if_match: Option<&str>,
    if_none_match: bool,
) -> Result<reqwest::Response, AppError> {
    let client = sync_http_client()?;
    let mut request = client
        .put(webdav_url(config, relative_path)?)
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .body(body.to_vec());

    if let Some(content_type) = content_type {
        request = request.header("Content-Type", content_type);
    }
    if let Some(etag) = if_match {
        request = request.header(IF_MATCH, etag);
    }
    if if_none_match {
        request = request.header(IF_NONE_MATCH, "*");
    }

    request.send().await.map_err(sync_transport_error)
}

fn put_retry_content_types(content_type: &str) -> Vec<Option<&str>> {
    let mut candidates = vec![Some(content_type)];
    if content_type.starts_with("text/") && content_type != "text/plain; charset=utf-8" {
        candidates.push(Some("text/plain; charset=utf-8"));
    }
    if content_type != "application/octet-stream" {
        candidates.push(Some("application/octet-stream"));
    }
    candidates.push(None);
    candidates.dedup();
    candidates
}

fn should_retry_put_with_compatibility_headers(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::METHOD_NOT_ALLOWED | StatusCode::UNSUPPORTED_MEDIA_TYPE
    )
}

async fn get_text(config: &AppConfig, relative_path: &str) -> Result<Option<String>, AppError> {
    let client = sync_http_client()?;
    let response = client
        .get(webdav_url(config, relative_path)?)
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .send()
        .await
        .map_err(sync_transport_error)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let response = response.error_for_status().map_err(sync_transport_error)?;
    Ok(Some(response.text().await.map_err(sync_transport_error)?))
}

async fn archive_deleted_remote_note(
    config: &AppConfig,
    from: &str,
    archive_path: &str,
) -> Result<(), AppError> {
    copy_remote_resource(config, from, archive_path).await
}

async fn cleanup_replaced_remote_note(
    _config: &AppConfig,
    _previous_path: &str,
) -> Result<(), AppError> {
    // The actual DELETE is deferred until after the manifest compare-and-swap succeeds.
    // Leaving this step non-destructive prevents a losing client from removing the file
    // that the winning manifest revision still points at.
    Ok(())
}

async fn copy_remote_resource(config: &AppConfig, from: &str, to: &str) -> Result<(), AppError> {
    let Some(content) = get_text(config, from).await? else {
        return Ok(());
    };

    put_text(config, to, &content, "text/markdown; charset=utf-8").await?;
    Ok(())
}

async fn run_remote_cleanups(config: &AppConfig, cleanups: &[RemoteCleanup]) {
    for cleanup in cleanups {
        let _ = delete_resource(config, &cleanup.path).await;
    }
}

async fn delete_resource(
    config: &AppConfig,
    relative_path: &str,
) -> Result<RemoteMutationOutcome, AppError> {
    let client = sync_http_client()?;
    let response = client
        .delete(webdav_url(config, relative_path)?)
        .basic_auth(
            config.sync_webdav_username.trim(),
            Some(config.sync_webdav_password.as_str()),
        )
        .send()
        .await
        .map_err(sync_transport_error)?;

    match response.status() {
        StatusCode::OK | StatusCode::NO_CONTENT => Ok(RemoteMutationOutcome::Applied),
        StatusCode::NOT_FOUND => Ok(RemoteMutationOutcome::Missing),
        StatusCode::FORBIDDEN => Ok(RemoteMutationOutcome::PermissionDenied),
        StatusCode::METHOD_NOT_ALLOWED => Ok(RemoteMutationOutcome::Unsupported),
        status => Err(sync_status_error(status)),
    }
}

async fn cleanup_remote_category_directories(
    config: &AppConfig,
    categories: &[String],
) -> Result<(), AppError> {
    for category in categories {
        let dir = format!("{REMOTE_NOTES_DIR}/{}", sanitize_remote_segment(category));
        match delete_resource(config, &dir).await {
            Ok(RemoteMutationOutcome::Applied)
            | Ok(RemoteMutationOutcome::Missing)
            | Ok(RemoteMutationOutcome::PermissionDenied)
            | Ok(RemoteMutationOutcome::Unsupported) => {}
            Err(error) => {
                eprintln!("failed to clean up remote category directory {dir}: {error}");
            }
        }
    }
    Ok(())
}

fn sync_http_client() -> Result<&'static reqwest::Client, AppError> {
    if let Some(client) = SYNC_HTTP_CLIENT.get() {
        return Ok(client);
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(SYNC_CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(SYNC_HTTP_TIMEOUT_SECONDS))
        // A few consumer WebDAV providers respond inconsistently to MKCOL/PROPFIND over HTTP/2.
        // For sync traffic we prefer predictable HTTP/1.1 behavior over protocol negotiation.
        .http1_only()
        .build()
        .map_err(sync_transport_error)?;
    let _ = SYNC_HTTP_CLIENT.set(client);
    Ok(SYNC_HTTP_CLIENT
        .get()
        .expect("sync HTTP client should exist"))
}

fn load_state(store: &NoteStore) -> Result<SyncState, AppError> {
    let path = store.sync_state_path();
    if !path.exists() {
        return Ok(SyncState::default());
    }

    match serde_json::from_str::<SyncState>(&fs::read_to_string(&path)?) {
        Ok(mut state) => {
            if state.device_id.trim().is_empty() {
                state.device_id = Uuid::new_v4().to_string();
            }
            state.categories = normalized_categories(state.categories);
            state.category_tombstones.retain(|category, _| {
                !category.trim().is_empty() && !state.categories.contains(category)
            });
            Ok(state)
        }
        Err(_) => {
            archive_legacy_sync_state(&path)?;
            Ok(SyncState::default())
        }
    }
}

fn save_state(store: &NoteStore, state: &SyncState) -> Result<(), AppError> {
    write_json_atomic(&store.sync_state_path(), state)
}

fn archive_legacy_sync_state(path: &Path) -> Result<(), AppError> {
    let archived_name = format!(
        "sync_state.legacy-server-{}.json",
        Utc::now().format("%Y%m%d%H%M%S")
    );
    fs::rename(path, path.with_file_name(archived_name))?;
    Ok(())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serde_json::to_string_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)?;
    Ok(())
}

fn status_from_state(config: &AppConfig, state: &SyncState) -> SyncStatus {
    SyncStatus {
        enabled: config.sync_enabled,
        configured: is_configured(config),
        last_revision: state.last_manifest_revision.clone(),
        last_sync_at: state.last_sync_at,
        last_error: state.last_error.clone(),
    }
}

fn is_configured(config: &AppConfig) -> bool {
    !config.sync_webdav_url.trim().is_empty()
        && !config.sync_webdav_username.trim().is_empty()
        && !config.sync_webdav_password.trim().is_empty()
}

fn should_sync_on_app_exit(config: &AppConfig) -> bool {
    config.sync_enabled && is_configured(config)
}

fn sync_note_label(title: &str, id: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        id.to_string()
    } else {
        trimmed.to_string()
    }
}

fn remote_note_path(note: &Note, content_hash: &str) -> String {
    let file_name = format!(
        "{}__{}__{}.md",
        note.id,
        content_hash,
        slugify_title(&note.title)
    );
    remote_note_path_with_file_name(note, &file_name)
}

fn compatibility_remote_note_path(note: &Note, content_hash: &str) -> String {
    let file_name = format!(
        "{}__{}__{}.md",
        note.id,
        content_hash,
        ascii_slugify_title(&note.title)
    );
    format!("{REMOTE_NOTES_DIR}/{file_name}")
}

fn remote_note_path_with_file_name(note: &Note, file_name: &str) -> String {
    if note.category.trim().is_empty() {
        format!("{REMOTE_NOTES_DIR}/{file_name}")
    } else {
        format!(
            "{REMOTE_NOTES_DIR}/{}/{}",
            sanitize_remote_segment(&note.category),
            file_name
        )
    }
}

fn manifest_category_tombstones(manifest: &RemoteManifest) -> HashMap<String, DateTime<Utc>> {
    let active_categories: HashSet<String> = normalized_categories(manifest.categories.clone())
        .into_iter()
        .collect();
    let mut tombstones = HashMap::new();

    for entry in &manifest.category_tombstones {
        let name = entry.name.trim();
        if name.is_empty() || active_categories.contains(name) {
            continue;
        }

        tombstones
            .entry(name.to_string())
            .and_modify(|existing| {
                if *existing < entry.deleted_at {
                    *existing = entry.deleted_at;
                }
            })
            .or_insert(entry.deleted_at);
    }

    tombstones
}

fn category_tombstone_entries(
    tombstones: &HashMap<String, DateTime<Utc>>,
    active_categories: &[String],
) -> Vec<CategoryTombstoneEntry> {
    let active_categories: HashSet<&str> = active_categories.iter().map(String::as_str).collect();
    let mut entries: Vec<_> = tombstones
        .iter()
        .filter(|(name, _)| !name.trim().is_empty() && !active_categories.contains(name.as_str()))
        .map(|(name, deleted_at)| CategoryTombstoneEntry {
            name: name.clone(),
            deleted_at: *deleted_at,
        })
        .collect();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn category_tombstones_match(
    manifest: &RemoteManifest,
    local_category_tombstones: &HashMap<String, DateTime<Utc>>,
) -> bool {
    manifest_category_tombstones(manifest) == *local_category_tombstones
}

fn remote_archive_path(
    existing: Option<&ManifestEntry>,
    id: &str,
    deleted_at: DateTime<Utc>,
) -> String {
    let title = existing.map(|entry| entry.title.as_str()).unwrap_or("");
    let category = existing.map(|entry| entry.category.as_str()).unwrap_or("");
    let file_name = format!(
        "{}__{}__deleted-{}.md",
        id,
        slugify_title(title),
        deleted_at.format("%Y%m%d%H%M%S")
    );
    if category.trim().is_empty() {
        format!("{REMOTE_ARCHIVE_DIR}/{file_name}")
    } else {
        format!(
            "{REMOTE_ARCHIVE_DIR}/{}/{}",
            sanitize_remote_segment(category),
            file_name
        )
    }
}

fn categories_match(left: &[String], right: &[String]) -> bool {
    normalized_categories(left.to_vec()) == normalized_categories(right.to_vec())
}

fn normalized_categories(mut categories: Vec<String>) -> Vec<String> {
    categories.retain(|category| !category.trim().is_empty());
    categories.sort();
    categories.dedup();
    categories
}

fn local_note_changed(note: &Note, state: &SyncState) -> bool {
    state
        .notes
        .get(&note.id)
        .map(|known| known.content_hash != note_hash(note))
        .unwrap_or(true)
}

fn locally_deleted_note(state: &SyncState, id: &str) -> bool {
    state.notes.contains_key(id) || state.tombstones.contains_key(id)
}

fn locally_deleted_category(state: &SyncState, category: &str) -> bool {
    state.categories.iter().any(|c| c == category)
        || state.category_tombstones.contains_key(category)
}

fn create_conflict_backup(store: &NoteStore, note: &Note) -> Result<(), AppError> {
    let suffix = Utc::now().format("%Y-%m-%d %H-%M");
    store.create_note(SaveNoteRequest {
        title: format!(
            "{} \u{51b2}\u{7a81}\u{526f}\u{672c} {}",
            base_conflict_backup_title(&note.title),
            suffix
        )
        .trim()
        .to_string(),
        content: note.content.clone(),
        category: note.category.clone(),
    })?;
    Ok(())
}

fn base_conflict_backup_title(title: &str) -> String {
    let mut current = title.trim();
    while let Some(stripped) = strip_conflict_backup_suffix(current) {
        current = stripped;
    }
    current.to_string()
}

fn strip_conflict_backup_suffix(title: &str) -> Option<&str> {
    let (base, suffix) = title.rsplit_once(" \u{51b2}\u{7a81}\u{526f}\u{672c} ")?;
    is_conflict_backup_timestamp(suffix).then_some(base.trim_end())
}

fn is_conflict_backup_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b' '
        && bytes[13] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13) || byte.is_ascii_digit())
}
fn note_hash(note: &Note) -> String {
    let created_at = note.created_at.to_rfc3339();
    let updated_at = note.updated_at.to_rfc3339();
    stable_hash([
        note.id.as_str(),
        note.title.as_str(),
        note.content.as_str(),
        note.category.as_str(),
        created_at.as_str(),
        updated_at.as_str(),
    ])
}

fn manifest_entry_hash(entry: &ManifestEntry, content: &str) -> String {
    let created_at = entry.created_at.to_rfc3339();
    let updated_at = entry.updated_at.to_rfc3339();
    stable_hash([
        entry.id.as_str(),
        entry.title.as_str(),
        content,
        entry.category.as_str(),
        created_at.as_str(),
        updated_at.as_str(),
    ])
}

fn deleted_hash(id: &str, deleted_at: DateTime<Utc>) -> String {
    let deleted_at = deleted_at.to_rfc3339();
    stable_hash([id, "deleted", deleted_at.as_str()])
}

fn manifest_revision(etag: Option<&str>, manifest: &RemoteManifest) -> String {
    etag.map(ToOwned::to_owned).unwrap_or_else(|| {
        let serialized = serde_json::to_string(manifest).unwrap_or_default();
        stable_hash([serialized.as_str()])
    })
}

fn response_etag(headers: &HeaderMap) -> Option<String> {
    headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn response_revision(headers: &HeaderMap, manifest: Option<&RemoteManifest>) -> Option<String> {
    headers
        .get("X-File-Version")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .or_else(|| response_etag(headers))
        .or_else(|| manifest.map(|entry| manifest_revision(None, entry)))
}

fn stable_hash<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn slugify_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for ch in title.trim().chars() {
        let should_separate = ch.is_whitespace()
            || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            || ch.is_control();
        if should_separate {
            if !slug.is_empty() && !last_was_separator {
                slug.push('-');
                last_was_separator = true;
            }
            continue;
        }
        slug.push(ch);
        last_was_separator = false;
        if slug.chars().count() >= 48 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "note".to_string()
    } else {
        slug.to_string()
    }
}

fn ascii_slugify_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for ch in title.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if (ch.is_ascii_whitespace() || matches!(ch, '-' | '_'))
            && !slug.is_empty()
            && !last_was_separator
        {
            slug.push('-');
            last_was_separator = true;
        }

        if slug.len() >= 32 {
            break;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "note".to_string()
    } else {
        slug.to_string()
    }
}

fn should_retry_note_upload_with_compat_path(
    preferred_path: &str,
    fallback_path: &str,
    error: &AppError,
) -> bool {
    preferred_path != fallback_path
        && !preferred_path.is_ascii()
        && error.code == "syncTransport"
        && (error.message.contains("WebDAV \u{62d2}\u{7edd}\u{4e86}\u{5f53}\u{524d}\u{64cd}\u{4f5c}")
            || error.message.contains("\u{5f53}\u{524d} WebDAV \u{670d}\u{52a1}\u{4e0d}\u{652f}\u{6301}\u{8fd9}\u{4e2a}\u{540c}\u{6b65}\u{64cd}\u{4f5c}"))
}

fn sanitize_remote_segment(value: &str) -> String {
    let mut segment = String::new();
    for ch in value.trim().chars() {
        if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || ch.is_control() {
            segment.push('_');
        } else {
            segment.push(ch);
        }
    }
    let trimmed = segment.trim_matches('_');
    if trimmed.is_empty() {
        "category".to_string()
    } else {
        trimmed.to_string()
    }
}

fn collection_url(mut url: reqwest::Url) -> reqwest::Url {
    let normalized_path = format!("{}/", url.path().trim_end_matches('/'));
    url.set_path(&normalized_path);
    url
}

fn webdav_base_url(config: &AppConfig) -> Result<reqwest::Url, AppError> {
    let mut base = configured_webdav_url(config)?;
    let already_pointing_to_sync_root = configured_path_segments(config)?
        .last()
        .is_some_and(|segment| segment == DEFAULT_SYNC_ROOT_DIR);

    {
        let mut segments = base.path_segments_mut().map_err(|_| {
            AppError::new(
                "syncConfig",
                "WebDAV \u{5730}\u{5740}\u{683c}\u{5f0f}\u{4e0d}\u{6b63}\u{786e}\u{ff0c}\u{8bf7}\u{586b}\u{5199}\u{5b8c}\u{6574}\u{5730}\u{5740}\u{ff0c}\u{4f8b}\u{5982} https://dav.example.com/floral/\u{3002}",
            )
        })?;
        segments.pop_if_empty();
        if !already_pointing_to_sync_root {
            segments.push(DEFAULT_SYNC_ROOT_DIR);
        }
    }

    Ok(collection_url(base))
}

fn configured_webdav_url(config: &AppConfig) -> Result<reqwest::Url, AppError> {
    let raw = config.sync_webdav_url.trim();
    let normalized = if raw.ends_with('/') {
        raw.to_string()
    } else {
        format!("{raw}/")
    };
    reqwest::Url::parse(&normalized).map_err(|_| {
        AppError::new(
            "syncConfig",
            "WebDAV \u{5730}\u{5740}\u{683c}\u{5f0f}\u{4e0d}\u{6b63}\u{786e}\u{ff0c}\u{8bf7}\u{586b}\u{5199}\u{5b8c}\u{6574}\u{5730}\u{5740}\u{ff0c}\u{4f8b}\u{5982} https://dav.example.com/floral/\u{3002}",
        )
    })
}

fn configured_path_segments(config: &AppConfig) -> Result<Vec<String>, AppError> {
    let configured = configured_webdav_url(config)?;
    Ok(configured
        .path_segments()
        .map(|parts| {
            parts
                .filter(|segment| !segment.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

fn webdav_url(config: &AppConfig, relative_path: &str) -> Result<reqwest::Url, AppError> {
    let mut url = webdav_base_url(config)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            AppError::new(
                "syncConfig",
                "WebDAV \u{5730}\u{5740}\u{683c}\u{5f0f}\u{4e0d}\u{6b63}\u{786e}\u{ff0c}\u{8bf7}\u{586b}\u{5199}\u{5b8c}\u{6574}\u{5730}\u{5740}\u{ff0c}\u{4f8b}\u{5982} https://dav.example.com/floral/\u{3002}",
            )
        })?;
        segments.pop_if_empty();
        for segment in relative_path.trim_matches('/').split('/') {
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }
    Ok(url)
}

fn sync_transport_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        return AppError::new(
            "syncTransport",
            "\u{8fde}\u{63a5} WebDAV \u{670d}\u{52a1}\u{5668}\u{8d85}\u{65f6}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{7f51}\u{7edc}\u{6216}\u{670d}\u{52a1}\u{72b6}\u{6001}\u{3002}",
        );
    }
    if error.is_connect() {
        return AppError::new(
            "syncTransport",
            "\u{65e0}\u{6cd5}\u{8fde}\u{63a5}\u{5230} WebDAV \u{670d}\u{52a1}\u{5668}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{5730}\u{5740}\u{3001}\u{7aef}\u{53e3}\u{548c}\u{7f51}\u{7edc}\u{3002}",
        );
    }
    if error.is_decode() {
        return AppError::new(
            "syncTransport",
            "WebDAV \u{670d}\u{52a1}\u{5668}\u{8fd4}\u{56de}\u{4e86}\u{65e0}\u{6cd5}\u{8bc6}\u{522b}\u{7684}\u{540c}\u{6b65}\u{6570}\u{636e}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{8fdc}\u{7aef}\u{6587}\u{4ef6}\u{662f}\u{5426}\u{5b8c}\u{6574}\u{3002}",
        );
    }
    if error.is_builder() {
        return AppError::new(
            "syncConfig",
            "WebDAV \u{5730}\u{5740}\u{683c}\u{5f0f}\u{4e0d}\u{6b63}\u{786e}\u{ff0c}\u{8bf7}\u{586b}\u{5199}\u{5b8c}\u{6574}\u{5730}\u{5740}\u{ff0c}\u{4f8b}\u{5982} https://dav.example.com/floral/\u{3002}",
        );
    }
    if let Some(status) = error.status() {
        return match status {
            StatusCode::UNAUTHORIZED => AppError::new(
                "syncTransport",
                "WebDAV \u{8ba4}\u{8bc1}\u{5931}\u{8d25}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{8d26}\u{53f7}\u{548c}\u{5bc6}\u{7801}\u{662f}\u{5426}\u{6b63}\u{786e}\u{3002}",
            ),
            StatusCode::FORBIDDEN => AppError::new(
                "syncTransport",
                "WebDAV \u{62d2}\u{7edd}\u{4e86}\u{5f53}\u{524d}\u{64cd}\u{4f5c}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{76ee}\u{5f55}\u{6743}\u{9650}\u{ff0c}\u{6216}\u{7a0d}\u{540e}\u{518d}\u{8bd5}\u{3002}",
            ),
            _ => sync_status_error(status),
        };
    }
    AppError::new("syncTransport", "\u{540c}\u{6b65}\u{8bf7}\u{6c42}\u{5931}\u{8d25}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{91cd}\u{8bd5}\u{3002}")
}

fn with_sync_step<T>(step: &str, result: Result<T, AppError>) -> Result<T, AppError> {
    result.map_err(|mut error| {
        error.message = format!("{step}\u{5931}\u{8d25}\u{ff1a}{}", error.message);
        error
    })
}

fn sync_status_error(status: StatusCode) -> AppError {
    let message = match status.as_u16() {
        401 => "WebDAV \u{8ba4}\u{8bc1}\u{5931}\u{8d25}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{8d26}\u{53f7}\u{548c}\u{5bc6}\u{7801}\u{662f}\u{5426}\u{6b63}\u{786e}\u{3002}",
        403 => "WebDAV \u{62d2}\u{7edd}\u{4e86}\u{5f53}\u{524d}\u{64cd}\u{4f5c}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{76ee}\u{5f55}\u{6743}\u{9650}\u{6216} WebDAV \u{5199}\u{5165}\u{6743}\u{9650}\u{3002}",
        404 => "WebDAV \u{8def}\u{5f84}\u{4e0d}\u{5b58}\u{5728}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{5730}\u{5740}\u{662f}\u{5426}\u{6307}\u{5411}\u{53ef}\u{7528}\u{76ee}\u{5f55}\u{3002}",
        405 => "\u{5f53}\u{524d} WebDAV \u{670d}\u{52a1}\u{4e0d}\u{652f}\u{6301}\u{8fd9}\u{4e2a}\u{540c}\u{6b65}\u{64cd}\u{4f5c}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{670d}\u{52a1}\u{7aef}\u{517c}\u{5bb9}\u{6027}\u{3002}",
        409 => "WebDAV \u{76ee}\u{5f55}\u{7ed3}\u{6784}\u{4e0d}\u{5b8c}\u{6574}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{8fdc}\u{7aef}\u{8def}\u{5f84}\u{6743}\u{9650}\u{3002}",
        500..=599 => "WebDAV \u{670d}\u{52a1}\u{6682}\u{65f6}\u{4e0d}\u{53ef}\u{7528}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{518d}\u{8bd5}\u{3002}",
        code => return AppError::new("syncTransport", format!("WebDAV \u{8fd4}\u{56de}\u{4e86}\u{9519}\u{8bef}\u{ff08}HTTP {code}\u{ff09}\u{3002}")),
    };
    AppError::new("syncTransport", message)
}

fn retry_cooldown(error: &AppError) -> Duration {
    let auth_failure = error.code == "syncTransport"
        && error
            .message
            .contains("WebDAV \u{8ba4}\u{8bc1}\u{5931}\u{8d25}");
    if auth_failure {
        Duration::from_secs(SYNC_AUTH_RETRY_COOLDOWN_SECONDS)
    } else {
        Duration::from_secs(SYNC_RETRY_COOLDOWN_SECONDS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        thread,
    };

    fn test_root(name: &str) -> PathBuf {
        let base = std::env::var_os("FLORAL_NOTEPAPER_TEST_TEMP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("floral-notepaper-sync-tests"));
        let root = base.join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale sync test root");
        }
        fs::create_dir_all(&root).expect("create sync test root");
        root
    }

    fn test_config(sync_webdav_url: &str) -> AppConfig {
        AppConfig {
            locale: "zh-CN".into(),
            notes_dir: "D:\\notes".into(),
            global_shortcut: "Ctrl+Space".into(),
            close_to_tray: false,
            autostart: false,
            default_view_mode: "split".into(),
            note_auto_save: true,
            note_surface_auto_save: true,
            tile_color: "#f6f3ec".into(),
            tile_color_mode: "system".into(),
            theme: "system".into(),
            font_size: 14,
            surface_font_size: 14,
            tab_indent_size: 2,
            external_file_auto_save: true,
            background_image_path: String::new(),
            background_fit: "cover".into(),
            background_dim: 0.0,
            background_blur: 0.0,
            background_scale: 1.0,
            background_position_x: 50.0,
            background_position_y: 50.0,
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            render_html_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: String::new(),
            open_at_cursor: true,
            sync_enabled: true,
            sync_webdav_url: sync_webdav_url.into(),
            sync_webdav_username: "writer".into(),
            sync_webdav_password: "secret".into(),
            sync_interval_seconds: 300,
        }
    }

    #[test]
    fn collects_note_updates_and_deletes_from_local_state() {
        let store = NoteStore::new(test_root("collect-local-changes"));
        let note = store
            .create_note(SaveNoteRequest {
                title: "Local".into(),
                content: "body".into(),
                category: String::new(),
            })
            .expect("create local note");

        let mut state = SyncState::default();
        state.notes.insert(
            "deleted-note".into(),
            SyncedNoteState {
                content_hash: "old".into(),
                updated_at: parse_time("2026-05-20T08:00:00Z"),
            },
        );

        let changes = collect_local_changes(&store, &state).expect("collect changes");
        assert!(changes.iter().any(|change| matches!(change, LocalChange::Upsert { note: candidate, .. } if candidate.id == note.id)));
        assert!(changes.iter().any(
            |change| matches!(change, LocalChange::Delete { id, .. } if id == "deleted-note")
        ));
    }

    #[test]
    fn applies_remote_updates_and_keeps_conflict_copy() {
        let store = NoteStore::new(test_root("remote-conflict"));
        let local = store
            .upsert_synced_note(
                "conflict-note",
                "Local title",
                "local body",
                "",
                parse_time("2026-05-20T08:00:00Z"),
                parse_time("2026-05-20T08:05:00Z"),
            )
            .expect("seed local note");
        let mut state = SyncState::default();
        state.notes.insert(
            local.id.clone(),
            SyncedNoteState {
                content_hash: "previous-hash".into(),
                updated_at: parse_time("2026-05-20T08:00:00Z"),
            },
        );

        let entry = ManifestEntry {
            id: local.id.clone(),
            title: "Remote title".into(),
            category: String::new(),
            path: "notes/conflict-note__Remote-title.md".into(),
            created_at: parse_time("2026-05-20T08:00:00Z"),
            updated_at: parse_time("2026-05-20T08:20:00Z"),
            deleted_at: None,
            content_hash: stable_hash(["remote body"]),
        };

        apply_remote_entry_content(&store, &entry, "remote body", Some(local.clone()), true)
            .expect("apply remote entry");

        let synced = store.read_note(&local.id).expect("read synced note");
        assert_eq!(synced.title, "Remote title");
        assert_eq!(synced.content, "remote body");
        assert!(store
            .list_notes()
            .expect("list notes")
            .iter()
            .any(|note| note.title.contains("\u{51b2}\u{7a81}\u{526f}\u{672c}")));
    }

    #[test]
    fn archives_legacy_sync_state_files() {
        let store = NoteStore::new(test_root("legacy-sync-state"));
        fs::write(store.sync_state_path(), "{\"lastRevision\":2}").expect("write legacy state");

        let state = load_state(&store).expect("load migrated state");
        assert!(state.last_manifest_revision.is_empty());
        assert!(store
            .base_dir()
            .read_dir()
            .expect("read dir")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("sync_state.legacy-server-")));
    }

    #[test]
    fn keeps_ipv6_webdav_urls_intact() {
        let config = test_config(" http://[240e:abcd::1]:18787/floral ");

        let url = webdav_base_url(&config).expect("parse url");
        assert_eq!(
            url.as_str(),
            "http://[240e:abcd::1]:18787/floral/floral-sync/"
        );
    }

    #[test]
    fn appends_default_sync_root_when_webdav_url_has_no_path() {
        let config = test_config("http://192.168.1.6:5005");

        let url = webdav_base_url(&config).expect("resolve sync root");
        assert_eq!(url.as_str(), "http://192.168.1.6:5005/floral-sync/");
    }

    #[test]
    fn appends_sync_root_under_existing_provider_path() {
        let config = test_config("https://dav.jianguoyun.com/dav/");

        let url = webdav_base_url(&config).expect("resolve provider path");
        assert_eq!(url.as_str(), "https://dav.jianguoyun.com/dav/floral-sync/");
    }

    #[test]
    fn keeps_explicit_sync_root_path_unchanged() {
        let config = test_config("https://dav.jianguoyun.com/dav/floral-sync/");

        let url = webdav_base_url(&config).expect("keep explicit sync root");
        assert_eq!(url.as_str(), "https://dav.jianguoyun.com/dav/floral-sync/");
    }

    #[test]
    fn appends_relative_paths_without_double_slashes() {
        let config = test_config("https://dav.example.com/root/");

        let url = webdav_url(&config, "notes/test.md").expect("build note url");
        assert_eq!(
            url.as_str(),
            "https://dav.example.com/root/floral-sync/notes/test.md"
        );
    }

    #[test]
    fn strips_repeated_conflict_suffixes_before_creating_new_backup_title() {
        assert_eq!(
            base_conflict_backup_title("123 \u{51b2}\u{7a81}\u{526f}\u{672c} 2026-05-22 06-00 \u{51b2}\u{7a81}\u{526f}\u{672c} 2026-05-22 07-34"),
            "123"
        );
        assert_eq!(base_conflict_backup_title("normal title"), "normal title");
    }

    #[test]
    fn builds_ascii_fallback_note_paths_for_non_ascii_titles() {
        let note = Note {
            id: "note-1".into(),
            title: "\u{51b2}\u{7a81}\u{526f}\u{672c}".into(),
            file_name: "note-1.md".into(),
            category: "\u{5206}\u{7c7b}".into(),
            created_at: parse_time("2026-05-22T08:00:00Z"),
            updated_at: parse_time("2026-05-22T08:00:00Z"),
            word_count: 0,
            content: String::new(),
        };

        let content_hash = note_hash(&note);
        assert_eq!(
            compatibility_remote_note_path(&note, &content_hash),
            format!("notes/note-1__{content_hash}__note.md")
        );
    }

    #[test]
    fn remote_note_paths_include_content_hash() {
        let note = Note {
            id: "note-1".into(),
            title: "Daily note".into(),
            file_name: "note-1.md".into(),
            category: "Work".into(),
            created_at: parse_time("2026-05-22T08:00:00Z"),
            updated_at: parse_time("2026-05-22T08:30:00Z"),
            word_count: 4,
            content: "body".into(),
        };
        let content_hash = note_hash(&note);

        assert!(
            remote_note_path(&note, &content_hash).contains(&content_hash),
            "note uploads must not overwrite another client's pending revision before manifest save"
        );
    }

    #[tokio::test]
    async fn put_text_retries_with_compatibility_content_types() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        put_text(
            &config,
            "notes/test.md",
            "hello",
            "text/markdown; charset=utf-8",
        )
        .await
        .expect("fallback put should succeed");

        let requests = server.join().expect("join server thread");
        assert!(requests[0].contains("content-type: text/markdown; charset=utf-8"));
        assert!(requests[1].contains("content-type: text/plain; charset=utf-8"));
    }

    #[tokio::test]
    async fn upload_remote_note_retries_with_ascii_compatible_path() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        let note = Note {
            id: "note-1".into(),
            title: "\u{51b2}\u{7a81}\u{526f}\u{672c}".into(),
            file_name: "note-1.md".into(),
            category: String::new(),
            created_at: parse_time("2026-05-22T09:00:00Z"),
            updated_at: parse_time("2026-05-22T09:00:00Z"),
            word_count: 1,
            content: "body".into(),
        };

        let content_hash = note_hash(&note);
        let path = upload_remote_note(&config, &note, &content_hash)
            .await
            .expect("ascii fallback should upload note");

        assert_eq!(path, format!("notes/note-1__{content_hash}__note.md"));

        let requests = server.join().expect("join server thread");
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.contains("PUT ")
                    && request.contains("/dav/floral-sync/notes/"))
                .count(),
            2
        );
        assert!(requests.iter().any(|request| request.contains(&format!(
            "/dav/floral-sync/notes/note-1__{content_hash}__note.md"
        ))));
    }

    #[tokio::test]
    async fn ensure_sync_root_keeps_existing_provider_root() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).expect("read request");
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let response =
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            request
        });

        let config = test_config(&format!("http://{address}/dav/"));
        ensure_sync_root(&config)
            .await
            .expect("existing provider root should be accepted");

        let request = server.join().expect("join server thread");
        assert!(request.starts_with("PROPFIND /dav/floral-sync/ HTTP/1.1"));
        assert!(!request.contains("MKCOL /dav HTTP/1.1"));
    }

    #[tokio::test]
    async fn ensure_sync_root_creates_missing_sync_root_directory() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        ensure_sync_root(&config)
            .await
            .expect("missing sync root should be created");

        let requests = server.join().expect("join server thread");
        assert!(requests[0].starts_with("PROPFIND /dav/floral-sync/ HTTP/1.1"));
        assert!(requests[1].starts_with("MKCOL /dav/floral-sync/ HTTP/1.1"));
    }

    #[tokio::test]
    async fn ensure_remote_collection_checks_before_creating_subdirectories() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        ensure_remote_collection(&config, REMOTE_NOTES_DIR)
            .await
            .expect("missing notes directory should be created");

        let requests = server.join().expect("join server thread");
        assert!(requests[0].starts_with("PROPFIND "));
        assert!(requests[1].starts_with("MKCOL "));
    }

    #[tokio::test]
    async fn ensure_remote_collection_treats_bad_request_as_missing_directory() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        ensure_remote_collection(&config, REMOTE_NOTES_DIR)
            .await
            .expect("bad request probe should fall back to MKCOL");

        let requests = server.join().expect("join server thread");
        assert!(requests[0].starts_with("PROPFIND "));
        assert!(requests[1].starts_with("MKCOL "));
    }

    #[tokio::test]
    async fn ensure_sync_root_reports_manual_creation_when_creation_is_forbidden() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        let error = ensure_sync_root(&config)
            .await
            .expect_err("forbidden creation should ask for manual setup");

        let requests = server.join().expect("join server thread");
        assert!(requests[0].starts_with("PROPFIND /dav/floral-sync/ HTTP/1.1"));
        assert!(requests[1].starts_with("MKCOL /dav/floral-sync/ HTTP/1.1"));
        assert_eq!(
            error.message,
            "\u{65e0}\u{6cd5}\u{81ea}\u{52a8}\u{521b}\u{5efa} floral-sync \u{6587}\u{4ef6}\u{5939}\u{ff0c}\u{8bf7}\u{5148}\u{5728} WebDAV \u{6839}\u{76ee}\u{5f55}\u{4e0b}\u{624b}\u{52a8}\u{521b}\u{5efa} floral-sync \u{6587}\u{4ef6}\u{5939}\u{5e76}\u{786e}\u{4fdd}\u{6709}\u{8bfb}\u{5199}\u{6743}\u{9650}\u{3002}"
        );
    }

    #[tokio::test]
    async fn save_manifest_returns_conflict_when_revision_changes_without_etag() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let address = listener.local_addr().expect("listener address");

        let remote_manifest = RemoteManifest {
            version: 1,
            generated_at: parse_time("2026-05-22T12:00:00Z"),
            categories: vec!["Remote".into()],
            category_tombstones: Vec::new(),
            entries: Vec::new(),
        };
        let remote_body = serde_json::to_string(&remote_manifest).expect("serialize manifest");
        let expected_revision = manifest_revision(None, &RemoteManifest::default());

        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            let responses = [
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    remote_body.len(),
                    remote_body
                ),
            ];

            let mut index = 0;
            let started_at = Instant::now();
            while index < responses.len() && started_at.elapsed() < Duration::from_secs(2) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).ok();
                        let mut buffer = [0_u8; 4096];
                        let read = stream.read(&mut buffer).expect("read request");
                        requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                        stream
                            .write_all(responses[index].as_bytes())
                            .expect("write response");
                        index += 1;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept request: {error}"),
                }
            }

            requests
        });

        let config = test_config(&format!("http://{address}/"));
        let outcome = save_manifest(
            &config,
            &RemoteManifest::default(),
            &expected_revision,
            None,
            true,
        )
        .await
        .expect("save manifest");

        assert!(matches!(outcome, ManifestSaveOutcome::Conflict));

        let requests = server.join().expect("join server thread");
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("PROPFIND /floral-sync/floral-sync-meta/ HTTP/1.1"));
        assert!(requests[1].starts_with("GET /floral-sync/floral-sync-meta/manifest.json HTTP/1.1"));
    }

    #[tokio::test]
    async fn save_manifest_uses_if_none_match_when_creating_missing_manifest() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let address = listener.local_addr().expect("listener address");
        let expected_revision = manifest_revision(None, &RemoteManifest::default());

        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            let responses = [
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nETag: \"manifest-1\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ];

            let mut index = 0;
            let started_at = Instant::now();
            while index < responses.len() && started_at.elapsed() < Duration::from_secs(2) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).ok();
                        let mut buffer = [0_u8; 4096];
                        let read = stream.read(&mut buffer).expect("read request");
                        requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                        stream
                            .write_all(responses[index].as_bytes())
                            .expect("write response");
                        index += 1;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept request: {error}"),
                }
            }

            requests
        });

        let config = test_config(&format!("http://{address}/"));
        let outcome = save_manifest(
            &config,
            &RemoteManifest::default(),
            &expected_revision,
            None,
            false,
        )
        .await
        .expect("save manifest");

        assert!(matches!(outcome, ManifestSaveOutcome::Saved(_)));

        let requests = server.join().expect("join server thread");
        assert_eq!(requests.len(), 3);
        assert!(requests[2].starts_with("PUT /floral-sync/floral-sync-meta/manifest.json HTTP/1.1"));
        assert!(requests[2].contains("if-none-match: *"));
    }

    #[tokio::test]
    async fn test_connection_checks_write_access_for_notes_and_state_dirs() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let store = NoteStore::new(test_root("test-connection-write-probe"));
        let config = test_config(&format!("http://{address}/dav/"));
        let error = test_connection(&store, &config)
            .await
            .expect_err("write probe should fail on forbidden notes put");

        assert!(error.message.contains("测试写入笔记目录失败"));

        let requests = server.join().expect("join server thread");
        assert!(requests
            .iter()
            .any(|request| request.contains("PROPFIND ") && request.contains("/dav/floral-sync/")));
        assert!(
            requests.iter().any(|request| request.contains("PROPFIND ")
                && request.contains("/dav/floral-sync/notes/")),
            "requests: {requests:#?}"
        );
        assert!(
            requests.iter().any(|request| request.contains("PROPFIND ")
                && request.contains("/dav/floral-sync/floral-sync-meta/")),
            "requests: {requests:#?}"
        );
        assert!(requests.iter().any(|request| request.contains("PUT ")
            && request.contains("/dav/floral-sync/notes/floral-write-probe-")));
    }

    #[tokio::test]
    async fn push_local_changes_ignores_forbidden_cleanup_after_replacing_note_path() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        let note = Note {
            id: "note-1".into(),
            title: "Renamed note".into(),
            file_name: "note-1_Renamed-note.md".into(),
            category: String::new(),
            created_at: parse_time("2026-05-22T09:00:00Z"),
            updated_at: parse_time("2026-05-22T09:05:00Z"),
            word_count: 2,
            content: "new body".into(),
        };
        let content_hash = note_hash(&note);
        let expected_path = format!("notes/note-1__{content_hash}__Renamed-note.md");
        let mut manifest = RemoteManifest {
            version: 1,
            generated_at: parse_time("2026-05-22T09:00:00Z"),
            categories: Vec::new(),
            category_tombstones: Vec::new(),
            entries: vec![ManifestEntry {
                id: note.id.clone(),
                title: "Old title".into(),
                category: String::new(),
                path: "notes/note-1__Old-title.md".into(),
                created_at: note.created_at,
                updated_at: note.created_at,
                deleted_at: None,
                content_hash: "old-hash".into(),
            }],
        };

        push_local_changes(
            &config,
            &mut manifest,
            &[LocalChange::Upsert {
                note,
                content_hash: content_hash.clone(),
            }],
        )
        .await
        .expect("forbidden cleanup should not block upload");

        let updated = manifest_entry(&manifest, "note-1").expect("updated manifest entry");
        assert_eq!(updated.path, expected_path);
        assert_eq!(updated.title, "Renamed note");

        let requests = server.join().expect("join server thread");
        assert!(
            requests.iter().any(|request| request.contains("PROPFIND ")
                && request.contains("/dav/floral-sync/notes/")),
            "requests: {requests:#?}"
        );
        assert!(requests.iter().any(|request| request.contains("PUT ")
            && request.contains(&format!(
                "/dav/floral-sync/notes/note-1__{content_hash}__Renamed-note.md"
            ))));
        assert!(!requests.iter().any(|request| request.contains("DELETE ")));
    }

    #[tokio::test]
    async fn archives_deleted_notes_by_copying_without_removing_source() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            let body = "# old body\n";
            let put_response =
                format!("HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let get_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            for response in [
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
                "HTTP/1.1 207 Multi-Status\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string(),
                get_response,
                put_response,
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).expect("read request");
                requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
            requests
        });

        let config = test_config(&format!("http://{address}/dav/"));
        let deleted_at = parse_time("2026-05-22T11:00:00Z");
        let mut manifest = RemoteManifest {
            version: 1,
            generated_at: deleted_at,
            categories: Vec::new(),
            category_tombstones: Vec::new(),
            entries: vec![ManifestEntry {
                id: "note-1".into(),
                title: "Old title".into(),
                category: String::new(),
                path: "notes/note-1__Old-title.md".into(),
                created_at: deleted_at,
                updated_at: deleted_at,
                deleted_at: None,
                content_hash: "old-hash".into(),
            }],
        };

        push_local_changes(
            &config,
            &mut manifest,
            &[LocalChange::Delete {
                id: "note-1".into(),
                deleted_at,
                content_hash: deleted_hash("note-1", deleted_at),
            }],
        )
        .await
        .expect("copy should archive deleted note without removing the source");

        let updated = manifest_entry(&manifest, "note-1").expect("deleted manifest entry");
        assert_eq!(updated.deleted_at, Some(deleted_at));
        assert!(updated.path.starts_with("floral-sync-meta/archive/"));

        let requests = server.join().expect("join server thread");
        assert!(
            requests.iter().any(|request| request.contains("PROPFIND ")
                && request.contains("/dav/floral-sync/floral-sync-meta/")),
            "requests: {requests:#?}"
        );
        assert!(requests.iter().any(|request| request.contains("PROPFIND ")
            && request.contains("/dav/floral-sync/floral-sync-meta/archive/")));
        assert!(requests.iter().any(|request| request.contains("GET ")
            && request.contains("/dav/floral-sync/notes/note-1__Old-title.md")));
        assert!(requests.iter().any(|request| request.contains("PUT ")
            && request.contains("/dav/floral-sync/floral-sync-meta/archive/")));
        assert!(!requests.iter().any(|request| request.contains("DELETE ")));
    }

    #[test]
    fn returns_friendly_error_for_invalid_webdav_url() {
        let config = AppConfig {
            locale: "zh-CN".into(),
            notes_dir: "D:\\notes".into(),
            global_shortcut: "Ctrl+Space".into(),
            close_to_tray: false,
            autostart: false,
            default_view_mode: "split".into(),
            note_auto_save: true,
            note_surface_auto_save: true,
            tile_color: "#f6f3ec".into(),
            tile_color_mode: "system".into(),
            theme: "system".into(),
            font_size: 14,
            surface_font_size: 14,
            tab_indent_size: 2,
            external_file_auto_save: true,
            background_image_path: String::new(),
            background_fit: "cover".into(),
            background_dim: 0.0,
            background_blur: 0.0,
            background_scale: 1.0,
            background_position_x: 50.0,
            background_position_y: 50.0,
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            render_html_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: String::new(),
            open_at_cursor: true,
            sync_enabled: true,
            sync_webdav_url: "dav.example.com/floral".into(),
            sync_webdav_username: "writer".into(),
            sync_webdav_password: "secret".into(),
            sync_interval_seconds: 300,
        };

        let error = webdav_base_url(&config).expect_err("invalid url should fail");
        assert_eq!(
            error.message,
            "WebDAV \u{5730}\u{5740}\u{683c}\u{5f0f}\u{4e0d}\u{6b63}\u{786e}\u{ff0c}\u{8bf7}\u{586b}\u{5199}\u{5b8c}\u{6574}\u{5730}\u{5740}\u{ff0c}\u{4f8b}\u{5982} https://dav.example.com/floral/\u{3002}"
        );
    }
    #[test]
    fn keeps_new_local_empty_categories_until_they_are_pushed() {
        let store = NoteStore::new(test_root("local-empty-category"));
        store
            .create_category("Local Draft")
            .expect("create local category");

        let state = SyncState::default();
        let remote_tombstones = HashMap::new();
        apply_remote_categories(&store, &state, &[], &remote_tombstones)
            .expect("apply empty remote categories");

        assert!(store
            .list_categories()
            .expect("list categories")
            .contains(&"Local Draft".to_string()));
    }

    #[test]
    fn removes_empty_categories_that_were_deleted_on_remote() {
        let store = NoteStore::new(test_root("remote-category-delete"));
        store.create_category("Shared").expect("create category");

        let mut state = SyncState::default();
        state.categories = vec!["Shared".into()];

        let mut remote_tombstones = HashMap::new();
        remote_tombstones.insert("Shared".to_string(), parse_time("2026-05-22T10:00:00Z"));

        apply_remote_categories(&store, &state, &[], &remote_tombstones)
            .expect("apply remote category deletion");

        assert_eq!(
            store.list_categories().expect("list categories"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn preserves_remote_category_tombstones_after_note_moves_between_categories() {
        let store = NoteStore::new(test_root("remote-category-move"));
        let local = store
            .upsert_synced_note(
                "moved-note",
                "Shared note",
                "body",
                "Old",
                parse_time("2026-05-22T08:00:00Z"),
                parse_time("2026-05-22T08:00:00Z"),
            )
            .expect("seed old note");

        let entry = ManifestEntry {
            id: local.id.clone(),
            title: local.title.clone(),
            category: "New".into(),
            path: "notes/New/moved-note__Shared-note.md".into(),
            created_at: local.created_at,
            updated_at: parse_time("2026-05-22T08:30:00Z"),
            deleted_at: None,
            content_hash: note_hash(&local),
        };
        let local_content = local.content.clone();

        apply_remote_entry_content(&store, &entry, &local_content, Some(local), false)
            .expect("move note to new category");

        let mut previous_state = SyncState::default();
        previous_state.categories = vec!["Old".into()];

        let mut remote_tombstones = HashMap::new();
        remote_tombstones.insert("Old".to_string(), parse_time("2026-05-22T09:00:00Z"));

        apply_remote_categories(&store, &previous_state, &["New".into()], &remote_tombstones)
            .expect("apply remote categories");

        assert_eq!(
            store.list_categories().expect("list categories"),
            vec!["New".to_string()]
        );

        let manifest = RemoteManifest {
            version: 1,
            generated_at: parse_time("2026-05-22T09:00:00Z"),
            categories: vec!["New".into()],
            category_tombstones: vec![CategoryTombstoneEntry {
                name: "Old".into(),
                deleted_at: parse_time("2026-05-22T09:00:00Z"),
            }],
            entries: vec![entry],
        };
        let mut baseline_state = previous_state.clone();
        rebuild_state_from_manifest(&mut baseline_state, &manifest);

        let local_categories =
            normalized_categories(store.list_categories().expect("list categories"));
        let tombstones =
            collect_local_category_tombstones(&store, &baseline_state, &local_categories)
                .expect("collect local category tombstones");

        assert!(tombstones.contains_key("Old"));
    }

    #[test]
    fn syncs_before_exit_only_when_auto_sync_is_enabled_and_configured() {
        let configured = test_config("https://dav.example.com/root/");
        let disabled = AppConfig {
            sync_enabled: false,
            ..configured.clone()
        };
        let incomplete = AppConfig {
            sync_webdav_password: String::new(),
            ..configured.clone()
        };

        assert!(should_sync_on_app_exit(&configured));
        assert!(!should_sync_on_app_exit(&disabled));
        assert!(!should_sync_on_app_exit(&incomplete));
    }

    #[tokio::test]
    async fn remote_does_not_resurrect_locally_deleted_note() {
        let store = NoteStore::new(test_root("local-note-delete-no-restore"));
        let mut state = SyncState::default();
        state.notes.insert(
            "deleted-note".into(),
            SyncedNoteState {
                content_hash: "known-hash".into(),
                updated_at: parse_time("2026-05-22T08:00:00Z"),
            },
        );

        let entry = ManifestEntry {
            id: "deleted-note".into(),
            title: "Remote note".into(),
            category: String::new(),
            path: "notes/deleted-note__Remote-note.md".into(),
            created_at: parse_time("2026-05-22T08:00:00Z"),
            updated_at: parse_time("2026-05-22T09:00:00Z"),
            deleted_at: None,
            content_hash: "remote-hash".into(),
        };

        // The note is in state.notes but not on disk → locally deleted.
        // apply_remote_entry should skip it and not recreate it.
        let config = test_config("https://dav.example.com/root/");
        apply_remote_entry(&store, &state, &config, &entry)
            .await
            .expect("locally deleted note should be skipped");

        assert!(store.read_note("deleted-note").is_err());
    }

    #[tokio::test]
    async fn remote_does_not_resurrect_locally_tombstoned_note() {
        let store = NoteStore::new(test_root("local-note-tombstone-no-restore"));
        let mut state = SyncState::default();
        state.tombstones.insert(
            "deleted-note".into(),
            TombstoneState {
                deleted_at: parse_time("2026-05-22T10:00:00Z"),
                content_hash: "known-hash".into(),
            },
        );

        let entry = ManifestEntry {
            id: "deleted-note".into(),
            title: "Remote note".into(),
            category: String::new(),
            path: "notes/deleted-note__Remote-note.md".into(),
            created_at: parse_time("2026-05-22T08:00:00Z"),
            updated_at: parse_time("2026-05-22T09:00:00Z"),
            deleted_at: None,
            content_hash: "remote-hash".into(),
        };

        let config = test_config("https://dav.example.com/root/");
        apply_remote_entry(&store, &state, &config, &entry)
            .await
            .expect("locally tombstoned note should be skipped");

        assert!(store.read_note("deleted-note").is_err());
    }

    #[tokio::test]
    async fn local_read_errors_are_not_treated_as_local_deletions() {
        let store = NoteStore::new(test_root("local-read-error-not-delete"));
        let local = store
            .create_note(SaveNoteRequest {
                title: "Tracked note".into(),
                content: "local body".into(),
                category: String::new(),
            })
            .expect("create local note");
        fs::remove_file(store.base_dir().join("notes").join(&local.file_name))
            .expect("remove backing markdown file");

        let mut state = SyncState::default();
        state.notes.insert(
            local.id.clone(),
            SyncedNoteState {
                content_hash: note_hash(&local),
                updated_at: local.updated_at,
            },
        );
        let entry = ManifestEntry {
            id: local.id.clone(),
            title: local.title.clone(),
            category: String::new(),
            path: "notes/tracked.md".into(),
            created_at: local.created_at,
            updated_at: local.updated_at,
            deleted_at: None,
            content_hash: note_hash(&local),
        };
        let config = test_config("https://dav.example.com/root/");

        let error = apply_remote_entry(&store, &state, &config, &entry)
            .await
            .expect_err("read failure should not become a tombstone");

        assert_eq!(error.code, "io");
    }

    #[tokio::test]
    async fn rejects_remote_note_when_manifest_hash_does_not_match_downloaded_content() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let address = listener.local_addr().expect("listener address");

        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            let response =
                "HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\ntampered body";
            let started_at = Instant::now();
            while requests.is_empty() && started_at.elapsed() < Duration::from_secs(2) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).ok();
                        let mut buffer = [0_u8; 4096];
                        let read = stream.read(&mut buffer).expect("read request");
                        requests.push(String::from_utf8_lossy(&buffer[..read]).to_string());
                        stream
                            .write_all(response.as_bytes())
                            .expect("write response");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept request: {error}"),
                }
            }

            requests
        });

        let store = NoteStore::new(test_root("remote-hash-mismatch"));
        let state = SyncState::default();
        let config = test_config(&format!("http://{address}/"));
        let entry = ManifestEntry {
            id: "note-1".into(),
            title: "Remote note".into(),
            category: String::new(),
            path: "notes/note-1__Remote-note.md".into(),
            created_at: parse_time("2026-05-22T08:00:00Z"),
            updated_at: parse_time("2026-05-22T09:00:00Z"),
            deleted_at: None,
            content_hash: "expected-hash".into(),
        };

        let error = apply_remote_entry(&store, &state, &config, &entry)
            .await
            .expect_err("hash mismatch should stop the remote note");

        assert_eq!(error.code, "syncManifestMismatch");
        assert!(store.read_note("note-1").is_err());
        let requests = server.join().expect("join server thread");
        assert_eq!(requests.len(), 1);
    }

    #[test]
    fn remote_does_not_resurrect_locally_deleted_category() {
        let store = NoteStore::new(test_root("local-category-delete-no-restore"));
        let mut state = SyncState::default();
        state.categories = vec!["Shared".into()];

        // "Shared" was previously synced but is now gone locally.
        // Remote still lists it — we should not recreate it.
        apply_remote_categories(&store, &state, &["Shared".into()], &HashMap::new())
            .expect("locally deleted category should be skipped");

        assert!(!store
            .list_categories()
            .expect("list categories")
            .contains(&"Shared".to_string()));
    }

    #[test]
    fn remote_does_not_resurrect_locally_tombstoned_category() {
        let store = NoteStore::new(test_root("local-category-tombstone-no-restore"));
        let mut state = SyncState::default();
        state
            .category_tombstones
            .insert("Shared".into(), parse_time("2026-05-22T10:00:00Z"));

        apply_remote_categories(&store, &state, &["Shared".into()], &HashMap::new())
            .expect("locally tombstoned category should be skipped");

        assert!(!store
            .list_categories()
            .expect("list categories")
            .contains(&"Shared".to_string()));
    }

    #[test]
    fn records_category_tombstones_for_local_deletions() {
        let store = NoteStore::new(test_root("local-category-delete"));

        let mut state = SyncState::default();
        state.categories = vec!["Shared".into()];

        let local_categories =
            normalized_categories(store.list_categories().expect("list categories"));
        let tombstones = collect_local_category_tombstones(&store, &state, &local_categories)
            .expect("collect category tombstones");

        assert!(tombstones.contains_key("Shared"));
    }

    #[test]
    fn uses_longer_retry_cooldown_for_auth_failures() {
        let auth_error = AppError::new(
            "syncTransport",
            "WebDAV \u{8ba4}\u{8bc1}\u{5931}\u{8d25}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{8d26}\u{53f7}\u{548c}\u{5bc6}\u{7801}\u{662f}\u{5426}\u{6b63}\u{786e}\u{3002}",
        );
        let generic_error = AppError::new("syncTransport", "\u{540c}\u{6b65}\u{8bf7}\u{6c42}\u{5931}\u{8d25}\u{ff0c}\u{8bf7}\u{7a0d}\u{540e}\u{91cd}\u{8bd5}\u{3002}");
        let forbidden_error = AppError::new(
            "syncTransport",
            "WebDAV \u{62d2}\u{7edd}\u{4e86}\u{5f53}\u{524d}\u{64cd}\u{4f5c}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{76ee}\u{5f55}\u{6743}\u{9650}\u{ff0c}\u{6216}\u{7a0d}\u{540e}\u{518d}\u{8bd5}\u{3002}",
        );

        assert_eq!(
            retry_cooldown(&auth_error),
            Duration::from_secs(SYNC_AUTH_RETRY_COOLDOWN_SECONDS)
        );
        assert_eq!(
            retry_cooldown(&generic_error),
            Duration::from_secs(SYNC_RETRY_COOLDOWN_SECONDS)
        );
        assert_eq!(
            retry_cooldown(&forbidden_error),
            Duration::from_secs(SYNC_RETRY_COOLDOWN_SECONDS)
        );
    }

    #[test]
    fn prefixes_sync_step_name_in_error_messages() {
        let error = with_sync_step(
            "\u{51c6}\u{5907}\u{540c}\u{6b65}\u{76ee}\u{5f55}",
            Err::<(), _>(AppError::new(
                "syncTransport",
                "WebDAV \u{62d2}\u{7edd}\u{4e86}\u{5f53}\u{524d}\u{64cd}\u{4f5c}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{76ee}\u{5f55}\u{6743}\u{9650}\u{ff0c}\u{6216}\u{7a0d}\u{540e}\u{518d}\u{8bd5}\u{3002}",
            )),
        )
        .expect_err("step wrapper should keep the error");

        assert_eq!(
            error.message,
            "\u{51c6}\u{5907}\u{540c}\u{6b65}\u{76ee}\u{5f55}\u{5931}\u{8d25}\u{ff1a}WebDAV \u{62d2}\u{7edd}\u{4e86}\u{5f53}\u{524d}\u{64cd}\u{4f5c}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{76ee}\u{5f55}\u{6743}\u{9650}\u{ff0c}\u{6216}\u{7a0d}\u{540e}\u{518d}\u{8bd5}\u{3002}"
        );
    }

    fn parse_time(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid time")
            .with_timezone(&Utc)
    }
}
