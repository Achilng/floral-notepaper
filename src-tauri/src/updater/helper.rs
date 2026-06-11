#[cfg(target_os = "windows")]
use super::platform::normalize_windows_path;
use super::{
    file_lock::acquire_update_state_lock,
    settings::write_json_atomic,
    types::{InstallKind, UpdateErrorDto, UpdateInstallMode, UpdateStateDto, UpdateStatus},
    UpdatePaths,
};
use chrono::Utc;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(all(test, not(target_os = "windows")))]
fn normalize_windows_path(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
pub const HELPER_BINARY_NAME: &str = "floral-notepaper-update-helper.exe";
#[cfg(not(target_os = "windows"))]
pub const HELPER_BINARY_NAME: &str = "floral-notepaper-update-helper";

// Give the GUI process enough time to flush notes, WebView state, and filesystem buffers before
// the helper treats the handoff as failed. A premature timeout leaves the update recoverable but
// forces the user through another install attempt.
const WAIT_FOR_EXIT_TIMEOUT: Duration = Duration::from_secs(120);
const WAIT_FOR_EXIT_GRACE_PERIOD: Duration = Duration::from_secs(5);
const WAIT_FOR_REPLACEMENT_TIMEOUT: Duration = Duration::from_secs(30);
const WATCHDOG_HELPER_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const UPDATE_STAGE_PREFIX: &str = ".floral-notepaper-update-stage-";
#[cfg(target_os = "macos")]
const MOUNT_STAGE_PREFIX: &str = ".floral-notepaper-mounted-dmg-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateHelperMode {
    Apply,
    Test,
    Watchdog,
}

impl UpdateHelperMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Test => "test",
            Self::Watchdog => "watchdog",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "apply" => Some(Self::Apply),
            "test" => Some(Self::Test),
            "watchdog" => Some(Self::Watchdog),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateHelperCommand {
    pub mode: UpdateHelperMode,
    pub install_kind: InstallKind,
    pub wait_pid: u32,
    pub state_path: PathBuf,
    pub asset_path: PathBuf,
    pub asset_sha256: String,
    pub asset_size: u64,
    pub target_path: PathBuf,
    pub log_path: PathBuf,
    pub ready_path: PathBuf,
    pub current_version: String,
    pub target_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppliedUpdate {
    launch_target: PathBuf,
    rollback: Option<InstallRollbackPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MacosRollbackPlan {
    target_path: PathBuf,
    backup_path: PathBuf,
    stage_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InstallRollbackPlan {
    Macos(MacosRollbackPlan),
    Windows(WindowsRollbackPlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsRollbackPlan {
    target_path: PathBuf,
    install_dir: PathBuf,
    backup_dir: PathBuf,
    registry_backup: Option<WindowsRegistryRollbackPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedWindowsInstaller {
    installer_path: PathBuf,
    temp_link: Option<PathBuf>,
}

impl PreparedWindowsInstaller {
    fn cleanup(self) {
        if let Some(link) = self.temp_link {
            let _ = fs::remove_file(link);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsRegistryRollbackPlan {
    key_path: String,
    backup_path: PathBuf,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsRegistryInstallRecord {
    key_path: String,
    launch_target: Option<PathBuf>,
    install_dir: Option<PathBuf>,
    quiet_uninstall_command: Option<String>,
    uninstall_command: Option<String>,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsRegistryEntry {
    key_path: String,
    values: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiskSpaceRequirement {
    probe_path: PathBuf,
    required_bytes: u64,
    reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AggregatedDiskSpaceRequirement {
    probe_path: PathBuf,
    required_bytes: u64,
    reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchdogPostExitAction {
    Noop,
    MarkFailedAndRelaunch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsInstallerFamily {
    Msi,
    Nsis,
}

impl UpdateHelperCommand {
    pub fn to_args(&self) -> Vec<OsString> {
        vec![
            OsString::from("--mode"),
            OsString::from(self.mode.as_str()),
            OsString::from("--install-kind"),
            OsString::from(install_kind_as_str(&self.install_kind)),
            OsString::from("--wait-pid"),
            OsString::from(self.wait_pid.to_string()),
            OsString::from("--state-path"),
            self.state_path.clone().into_os_string(),
            OsString::from("--asset-path"),
            self.asset_path.clone().into_os_string(),
            OsString::from("--asset-sha256"),
            OsString::from(self.asset_sha256.clone()),
            OsString::from("--asset-size"),
            OsString::from(self.asset_size.to_string()),
            OsString::from("--target-path"),
            self.target_path.clone().into_os_string(),
            OsString::from("--log-path"),
            self.log_path.clone().into_os_string(),
            OsString::from("--ready-path"),
            self.ready_path.clone().into_os_string(),
            OsString::from("--current-version"),
            OsString::from(self.current_version.clone()),
            OsString::from("--target-version"),
            OsString::from(self.target_version.clone()),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum UpdateHelperExitCode {
    Success = 0,
    InvalidArguments = 2,
    AssetMissing = 3,
    AssetUnreadable = 22,
    AssetSizeMismatch = 4,
    AssetHashMismatch = 5,
    TargetMissing = 6,
    LogWriteFailed = 7,
    WaitTimedOut = 8,
    UnsupportedInstallKind = 9,
    AssetExtractFailed = 10,
    ReplacementFailed = 11,
    RelaunchFailed = 12,
    StateWriteFailed = 13,
    InstallerFailed = 14,
    InsufficientSpace = 15,
    InstallerTimedOut = 16,
    InstallerCancelled = 17,
    InstallerBusy = 18,
    InstallerFatal = 19,
    InstallerVersionMismatch = 20,
    CleanupFailed = 21,
}

impl UpdateHelperExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

pub fn run_cli<I, S>(args: I) -> UpdateHelperExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let command = match parse_args(args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{message}");
            return UpdateHelperExitCode::InvalidArguments;
        }
    };

    match execute(&command) {
        Ok(()) => UpdateHelperExitCode::Success,
        Err(code) => code,
    }
}

pub fn parse_args<I, S>(args: I) -> Result<UpdateHelperCommand, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut values = BTreeMap::new();
    let mut iter = args.into_iter().map(Into::into);

    while let Some(flag) = iter.next() {
        let flag = flag
            .into_string()
            .map_err(|_| "helper arguments must be valid UTF-8".to_string())?;
        if !flag.starts_with("--") {
            return Err(format!("unexpected positional argument: {flag}"));
        }
        if values.contains_key(&flag) {
            return Err(format!("duplicate argument: {flag}"));
        }

        let value = iter
            .next()
            .ok_or_else(|| format!("missing value for argument: {flag}"))?
            .into_string()
            .map_err(|_| format!("argument value for {flag} must be valid UTF-8"))?;
        values.insert(flag, value);
    }

    let mode = UpdateHelperMode::parse(required_arg(&values, "--mode")?)
        .ok_or_else(|| "invalid value for --mode".to_string())?;
    let install_kind = parse_install_kind(required_arg(&values, "--install-kind")?)
        .ok_or_else(|| "invalid value for --install-kind".to_string())?;
    let wait_pid = required_arg(&values, "--wait-pid")?
        .trim()
        .parse::<u32>()
        .map_err(|_| "invalid value for --wait-pid".to_string())?;
    let state_path = PathBuf::from(required_arg(&values, "--state-path")?);
    let asset_path = PathBuf::from(required_arg(&values, "--asset-path")?);
    let asset_sha256 = required_arg(&values, "--asset-sha256")?
        .trim()
        .to_lowercase();
    if asset_sha256.len() != 64 || !asset_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid value for --asset-sha256".to_string());
    }

    let asset_size = required_arg(&values, "--asset-size")?
        .trim()
        .parse::<u64>()
        .map_err(|_| "invalid value for --asset-size".to_string())?;
    let target_path = PathBuf::from(required_arg(&values, "--target-path")?);
    let log_path = PathBuf::from(required_arg(&values, "--log-path")?);
    let ready_path = PathBuf::from(required_arg(&values, "--ready-path")?);
    let current_version = require_text(values.get("--current-version"), "--current-version")?;
    let target_version = require_text(values.get("--target-version"), "--target-version")?;

    for key in values.keys() {
        if !matches!(
            key.as_str(),
            "--mode"
                | "--install-kind"
                | "--wait-pid"
                | "--state-path"
                | "--asset-path"
                | "--asset-sha256"
                | "--asset-size"
                | "--target-path"
                | "--log-path"
                | "--ready-path"
                | "--current-version"
                | "--target-version"
        ) {
            return Err(format!("unknown argument: {key}"));
        }
    }

    Ok(UpdateHelperCommand {
        mode,
        install_kind,
        wait_pid,
        state_path,
        asset_path,
        asset_sha256,
        asset_size,
        target_path,
        log_path,
        ready_path,
        current_version,
        target_version,
    })
}

pub fn execute(command: &UpdateHelperCommand) -> Result<(), UpdateHelperExitCode> {
    let mut log = open_log(&command.log_path)?;
    write_log_header(&mut log, command)?;
    if command.mode == UpdateHelperMode::Watchdog {
        return execute_watchdog(command, &mut log);
    }

    validate_request(command, &mut log)?;
    ensure_sufficient_disk_space(command, &mut log)?;
    write_ready_marker(command, &mut log)?;

    match command.mode {
        UpdateHelperMode::Test => {
            write_log_line(
                &mut log,
                &format!(
                    "validated test request from {} to {}",
                    command.current_version, command.target_version
                ),
            )?;
            Ok(())
        }
        UpdateHelperMode::Apply => execute_apply(command, &mut log),
        UpdateHelperMode::Watchdog => unreachable!("watchdog mode is handled before validation"),
    }
}

fn execute_apply(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    write_log_line(
        log,
        &format!(
            "waiting for process {} to exit before applying update",
            command.wait_pid
        ),
    )?;
    if let Err(code) = wait_for_process_exit(command.wait_pid, &command.target_path, log) {
        let persist_result = persist_failed_state(command, code, log);
        let completion_result = write_completion_marker(command, log);
        let cleanup_result = cleanup_after_install(command, log);
        persist_result?;
        completion_result?;
        cleanup_result?;
        return Err(code);
    }
    let applied_update = match apply_update(command, log) {
        Ok(update) => update,
        Err(code) => {
            let persist_result = persist_failed_state(command, code, log);
            let relaunch_result = relaunch_existing_target(command, log);
            let completion_result = if relaunch_result.is_ok() {
                Some(write_completion_marker(command, log))
            } else {
                None
            };
            let cleanup_result = cleanup_after_install(command, log);
            persist_result?;
            if let Some(result) = completion_result {
                result?;
            }
            cleanup_result?;
            relaunch_result?;
            return Err(code);
        }
    };

    persist_pending_verification_state(command, log)?;
    cleanup_after_install(command, log)?;
    if let Err(code) = relaunch_target(&applied_update.launch_target, log) {
        let failure_code = if let Some(rollback) = applied_update.rollback.as_ref() {
            rollback_applied_update(rollback, log).err().unwrap_or(code)
        } else {
            code
        };
        persist_failed_state(command, failure_code, log)?;
        return Err(failure_code);
    }
    if let Err(error) = write_relaunch_marker(command, log) {
        let _ = write_log_line(
            log,
            &format!(
                "failed to persist relaunch handoff marker {} ({error:?})",
                relaunch_marker_path(command).display()
            ),
        );
    }
    cleanup_applied_update(&applied_update, log)?;
    write_completion_marker(command, log)
}

fn execute_watchdog(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    write_log_line(
        log,
        &format!("watching update helper process {}", command.wait_pid),
    )?;
    wait_for_process_exit_with_timeout(command.wait_pid, WATCHDOG_HELPER_TIMEOUT, None, log)?;

    match watchdog_post_exit_action(command, log)? {
        WatchdogPostExitAction::Noop => Ok(()),
        WatchdogPostExitAction::MarkFailedAndRelaunch => {
            let persist_result =
                persist_failed_state(command, UpdateHelperExitCode::InstallerFailed, log);
            let relaunch_result = relaunch_existing_target(command, log);
            persist_result?;
            relaunch_result
        }
    }
}

fn watchdog_post_exit_action(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<WatchdogPostExitAction, UpdateHelperExitCode> {
    let completion_path = completion_marker_path(command);
    if completion_path.exists() {
        write_log_line(
            log,
            &format!(
                "update helper completed; watchdog marker found at {}",
                completion_path.display()
            ),
        )?;
        return Ok(WatchdogPostExitAction::Noop);
    }

    let relaunch_path = relaunch_marker_path(command);
    if relaunch_path.exists() {
        write_log_line(
            log,
            &format!(
                "update helper reached relaunch handoff without completion marker; treating handoff as successful to avoid duplicate relaunch ({})",
                relaunch_path.display()
            ),
        )?;
        return Ok(WatchdogPostExitAction::Noop);
    }

    write_log_line(
        log,
        "update helper exited without completion marker; persisting failure and relaunching existing target",
    )?;
    Ok(WatchdogPostExitAction::MarkFailedAndRelaunch)
}

fn validate_request(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    if !command.target_path.exists() {
        write_log_line(
            log,
            &format!("target missing: {}", command.target_path.display()),
        )?;
        return Err(UpdateHelperExitCode::TargetMissing);
    }

    let metadata = match fs::metadata(&command.asset_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            write_log_line(
                log,
                &format!("asset missing: {} ({error})", command.asset_path.display()),
            )?;
            return Err(UpdateHelperExitCode::AssetMissing);
        }
    };

    if metadata.len() != command.asset_size {
        write_log_line(
            log,
            &format!(
                "asset size mismatch: expected {}, actual {}",
                command.asset_size,
                metadata.len()
            ),
        )?;
        return Err(UpdateHelperExitCode::AssetSizeMismatch);
    }

    let actual_hash = match sha256_hex(&command.asset_path) {
        Ok(hash) => hash,
        Err(error) => {
            write_log_line(
                log,
                &format!(
                    "asset unreadable: failed to read {} ({error})",
                    command.asset_path.display()
                ),
            )?;
            return Err(UpdateHelperExitCode::AssetUnreadable);
        }
    };

    if actual_hash != command.asset_sha256 {
        write_log_line(
            log,
            &format!(
                "asset hash mismatch: expected {}, actual {}",
                command.asset_sha256, actual_hash
            ),
        )?;
        return Err(UpdateHelperExitCode::AssetHashMismatch);
    }

    Ok(())
}

fn write_ready_marker(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    if let Some(parent) = command.ready_path.parent() {
        fs::create_dir_all(parent).map_err(|_| UpdateHelperExitCode::StateWriteFailed)?;
    }
    write_log_line(
        log,
        &format!(
            "writing helper ready marker {}",
            command.ready_path.display()
        ),
    )?;
    write_marker_file(
        &command.ready_path,
        format!("ready {}\n", Utc::now().to_rfc3339()),
    )?;
    Ok(())
}

pub(crate) fn completion_marker_path(command: &UpdateHelperCommand) -> PathBuf {
    let mut path = command.ready_path.clone();
    path.set_extension("done");
    path
}

pub(crate) fn relaunch_marker_path(command: &UpdateHelperCommand) -> PathBuf {
    let mut path = command.ready_path.clone();
    path.set_extension("relaunching");
    path
}

fn write_completion_marker(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let completion_path = completion_marker_path(command);
    if let Some(parent) = completion_path.parent() {
        fs::create_dir_all(parent).map_err(|_| UpdateHelperExitCode::StateWriteFailed)?;
    }
    write_log_line(
        log,
        &format!(
            "writing helper completion marker {}",
            completion_path.display()
        ),
    )?;
    write_marker_file(
        &completion_path,
        format!("completed {}\n", Utc::now().to_rfc3339()),
    )?;
    Ok(())
}

fn write_relaunch_marker(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let relaunch_path = relaunch_marker_path(command);
    if let Some(parent) = relaunch_path.parent() {
        fs::create_dir_all(parent).map_err(|_| UpdateHelperExitCode::StateWriteFailed)?;
    }
    write_log_line(
        log,
        &format!("writing helper relaunch marker {}", relaunch_path.display()),
    )?;
    write_marker_file(
        &relaunch_path,
        format!("relaunching {}\n", Utc::now().to_rfc3339()),
    )?;
    Ok(())
}

fn write_marker_file(path: &Path, contents: String) -> Result<(), UpdateHelperExitCode> {
    let parent = path
        .parent()
        .ok_or(UpdateHelperExitCode::StateWriteFailed)?;
    let temp_path = unique_temp_path(parent, "marker", Some("tmp"));
    fs::write(&temp_path, contents).map_err(|_| UpdateHelperExitCode::StateWriteFailed)?;
    fs::rename(&temp_path, path).map_err(|_| {
        let _ = fs::remove_file(&temp_path);
        UpdateHelperExitCode::StateWriteFailed
    })
}

fn remove_file_if_exists(path: &Path) -> Result<(), std::io::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn ensure_sufficient_disk_space(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let requirements = aggregated_disk_space_requirements(command, log)?;
    for requirement in requirements.values() {
        let Some(available_bytes) = available_disk_space(&requirement.probe_path) else {
            continue;
        };
        if available_bytes >= requirement.required_bytes {
            continue;
        }

        write_log_line(
            log,
            &format!(
                "insufficient disk space at {}: required {} bytes, available {} bytes ({})",
                requirement.probe_path.display(),
                requirement.required_bytes,
                available_bytes,
                requirement.reasons.join(" + "),
            ),
        )?;
        return Err(UpdateHelperExitCode::InsufficientSpace);
    }

    Ok(())
}

fn aggregated_disk_space_requirements(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<BTreeMap<String, AggregatedDiskSpaceRequirement>, UpdateHelperExitCode> {
    let mut aggregated = BTreeMap::<String, AggregatedDiskSpaceRequirement>::new();

    for requirement in disk_space_requirements(command, log)? {
        let bucket = storage_bucket_key(&requirement.probe_path)
            .unwrap_or_else(|| fallback_bucket_key(&requirement.probe_path));
        let entry = aggregated
            .entry(bucket)
            .or_insert_with(|| AggregatedDiskSpaceRequirement {
                probe_path: requirement.probe_path.clone(),
                required_bytes: 0,
                reasons: Vec::new(),
            });
        entry.required_bytes = entry
            .required_bytes
            .saturating_add(requirement.required_bytes);
        if !entry.reasons.contains(&requirement.reason) {
            entry.reasons.push(requirement.reason);
        }
    }

    Ok(aggregated)
}

fn disk_space_requirements(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<Vec<DiskSpaceRequirement>, UpdateHelperExitCode> {
    let mut requirements = Vec::new();

    if let Some(target_dir) = command.target_path.parent() {
        requirements.push(DiskSpaceRequirement {
            probe_path: existing_probe_path(target_dir),
            required_bytes: command.asset_size.saturating_mul(2),
            reason: "installer workspace",
        });
    }

    if command.install_kind == InstallKind::WindowsNsis {
        let Some(install_dir) = command.target_path.parent() else {
            return Ok(requirements);
        };
        let backup_parent = windows_rollback_storage_parent(command, install_dir)?;
        let backup_bytes = directory_size_bytes(install_dir).map_err(|error| {
            let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);
            let _ = write_log_line(
                log,
                &format!(
                    "failed to measure Windows install directory for rollback sizing {} ({error})",
                    install_dir.display()
                ),
            );
            code
        })?;
        requirements.push(DiskSpaceRequirement {
            probe_path: existing_probe_path(&backup_parent),
            required_bytes: backup_bytes,
            reason: "Windows rollback backup",
        });
    }

    Ok(requirements)
}

fn apply_update(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<AppliedUpdate, UpdateHelperExitCode> {
    match command.install_kind {
        InstallKind::MacosAppBundle => install_macos_bundle(command, log),
        InstallKind::WindowsPortable => install_windows_portable(command, log),
        InstallKind::WindowsNsis => install_windows_installer(command, log),
        InstallKind::Unknown => {
            write_log_line(log, "unsupported install kind")?;
            Err(UpdateHelperExitCode::UnsupportedInstallKind)
        }
    }
}

fn install_macos_bundle(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<AppliedUpdate, UpdateHelperExitCode> {
    let target_parent = command
        .target_path
        .parent()
        .ok_or(UpdateHelperExitCode::ReplacementFailed)?;
    cleanup_stale_macos_stage_dirs(target_parent, log)?;
    let stage_root = unique_temp_path(target_parent, "update-stage", None);
    fs::create_dir_all(&stage_root).map_err(|_| UpdateHelperExitCode::ReplacementFailed)?;

    let extension = command
        .asset_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let result = (|| {
        let staged_bundle = if extension == "zip" {
            extract_app_bundle_from_zip(
                &command.asset_path,
                &stage_root,
                &command.target_path,
                log,
            )?
        } else if extension == "dmg" {
            stage_app_bundle_from_dmg(&command.asset_path, &stage_root, &command.target_path, log)?
        } else {
            write_log_line(
                log,
                &format!(
                    "unsupported macOS asset format for install: {}",
                    command.asset_path.display()
                ),
            )?;
            return Err(UpdateHelperExitCode::AssetExtractFailed);
        };

        verify_macos_bundle(&staged_bundle, log)?;
        swap_macos_bundles(&command.target_path, &staged_bundle, log)?;
        Ok(MacosRollbackPlan {
            target_path: command.target_path.clone(),
            backup_path: staged_bundle,
            stage_root: stage_root.clone(),
        })
    })();
    let rollback = match result {
        Ok(rollback) => rollback,
        Err(code) => {
            cleanup_stage_root(&stage_root, log)?;
            return Err(code);
        }
    };
    write_log_line(
        log,
        &format!(
            "replaced macOS app bundle with version {}",
            command.target_version
        ),
    )?;
    Ok(AppliedUpdate {
        launch_target: command.target_path.clone(),
        rollback: Some(InstallRollbackPlan::Macos(rollback)),
    })
}

fn install_windows_portable(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<AppliedUpdate, UpdateHelperExitCode> {
    write_log_line(log, "windows portable install is manual-only")?;
    let _ = command;
    Err(UpdateHelperExitCode::UnsupportedInstallKind)
}

fn install_windows_installer(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<AppliedUpdate, UpdateHelperExitCode> {
    let rollback = prepare_windows_rollback_plan(command, log)?;
    let prepared_installer = prepare_windows_installer_asset(command, log)?;
    let effective_extension = prepared_installer
        .installer_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let result = (|| {
        match effective_extension.as_str() {
            "msi" => {
                write_log_line(log, "launching Windows MSI installer")?;
                let mut cmd = Command::new("msiexec.exe");
                cmd.args([
                    "/i",
                    &prepared_installer.installer_path.to_string_lossy(),
                    "/passive",
                    "/norestart",
                ]);
                #[cfg(target_os = "windows")]
                cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
                let mut child = cmd
                    .spawn()
                    .map_err(|_| UpdateHelperExitCode::InstallerFailed)?;

                let status = wait_for_installer_completion(&mut child, log)?;

                if !status.success() {
                    write_log_line(
                        log,
                        &format!("installer exited with status {:?}", status.code()),
                    )?;
                    return Err(map_installer_exit(
                        WindowsInstallerFamily::Msi,
                        status.code().map(|code| code as u32),
                    ));
                }
            }
            "exe" => {
                write_log_line(log, "launching Windows NSIS installer")?;
                let exit_code =
                    shell_execute_installer(&prepared_installer.installer_path, "/S", log)?;
                if exit_code != 0 {
                    write_log_line(log, &format!("installer exited with code {exit_code}"))?;
                    return Err(map_installer_exit(
                        WindowsInstallerFamily::Nsis,
                        Some(exit_code),
                    ));
                }
            }
            _ => {
                write_log_line(
                    log,
                    &format!(
                        "unsupported Windows installer asset format: {}",
                        prepared_installer.installer_path.display()
                    ),
                )?;
                return Err(UpdateHelperExitCode::AssetExtractFailed);
            }
        }
        let launch_target = resolve_windows_launch_target(&command.target_path, log)?;
        wait_for_target_to_exist(&launch_target, log)?;
        verify_windows_installed_version(&launch_target, &command.target_version, log)?;
        Ok(launch_target)
    })();

    prepared_installer.cleanup();
    let launch_target = match result {
        Ok(launch_target) => launch_target,
        Err(code) => {
            rollback_windows_update(&rollback, log)?;
            return Err(code);
        }
    };
    write_log_line(
        log,
        &format!("installer completed for version {}", command.target_version),
    )?;
    Ok(AppliedUpdate {
        launch_target,
        rollback: Some(InstallRollbackPlan::Windows(rollback)),
    })
}

fn prepare_windows_installer_asset(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<PreparedWindowsInstaller, UpdateHelperExitCode> {
    let extension = command
        .asset_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !extension.is_empty() && matches!(extension.as_str(), "exe" | "msi") {
        return Ok(PreparedWindowsInstaller {
            installer_path: command.asset_path.clone(),
            temp_link: None,
        });
    }

    let link_path = command.asset_path.with_extension("exe");
    let link_result = fs::hard_link(&command.asset_path, &link_path).or_else(|link_error| {
        fs::copy(&command.asset_path, &link_path)
            .map(|_| ())
            .map_err(|copy_error| (link_error, copy_error))
    });
    if let Err((link_error, copy_error)) = link_result {
        let code = map_copy_error_code(&copy_error, UpdateHelperExitCode::AssetExtractFailed);
        write_log_line(
            log,
            &format!(
                "failed to create .exe link for extensionless asset: hard_link={link_error}; copy={copy_error}"
            ),
        )?;
        return Err(code);
    }
    write_log_line(
        log,
        &format!(
            "created .exe link for extensionless asset: {}",
            link_path.display()
        ),
    )?;
    Ok(PreparedWindowsInstaller {
        installer_path: link_path.clone(),
        temp_link: Some(link_path),
    })
}

fn prepare_windows_rollback_plan(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<WindowsRollbackPlan, UpdateHelperExitCode> {
    let install_dir = command
        .target_path
        .parent()
        .ok_or(UpdateHelperExitCode::ReplacementFailed)?
        .to_path_buf();
    let backup_parent = windows_rollback_storage_parent(command, &install_dir)?;
    if let Some(ready_parent) = command.ready_path.parent() {
        if backup_parent != ready_parent {
            write_log_line(
                log,
                &format!(
                    "Windows rollback storage moved outside install directory: {} -> {}",
                    ready_parent.display(),
                    backup_parent.display()
                ),
            )?;
        }
    }
    fs::create_dir_all(&backup_parent).map_err(|error| {
        let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);
        let _ = write_log_line(
            log,
            &format!(
                "failed to create Windows rollback storage parent {} ({error})",
                backup_parent.display()
            ),
        );
        code
    })?;
    let registry_backup = backup_windows_registry_state(command, &backup_parent, log)?;
    let backup_dir = unique_temp_path(&backup_parent, "windows-rollback", Some("dir"));
    fs::create_dir_all(&backup_dir).map_err(|error| {
        let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);
        let _ = write_log_line(
            log,
            &format!(
                "failed to create Windows rollback backup directory {} ({error})",
                backup_dir.display()
            ),
        );
        code
    })?;
    copy_dir_recursive(&install_dir, &backup_dir).map_err(|error| {
        let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);
        let _ = write_log_line(
            log,
            &format!(
                "failed to stage Windows rollback backup {} -> {} ({error})",
                install_dir.display(),
                backup_dir.display()
            ),
        );
        let _ = fs::remove_dir_all(&backup_dir);
        code
    })?;
    write_log_line(
        log,
        &format!("staged Windows rollback backup {}", backup_dir.display()),
    )?;
    Ok(WindowsRollbackPlan {
        target_path: command.target_path.clone(),
        install_dir,
        backup_dir,
        registry_backup,
    })
}

#[cfg(target_os = "windows")]
fn shell_execute_installer(
    path: &Path,
    args: &str,
    log: &mut File,
) -> Result<u32, UpdateHelperExitCode> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{GetExitCodeProcess, WaitForSingleObject},
        UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        },
    };

    let file_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params_wide: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();

    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    sei.lpFile = file_wide.as_ptr();
    sei.lpParameters = params_wide.as_ptr();

    if unsafe { ShellExecuteExW(&mut sei) } == 0 {
        let error = std::io::Error::last_os_error();
        let _ = write_log_line(log, &format!("ShellExecuteExW failed: {error}"));
        return Err(UpdateHelperExitCode::InstallerFailed);
    }

    let handle = sei.hProcess;
    if handle.is_null() {
        let _ = write_log_line(log, "no process handle from ShellExecuteExW");
        return Err(UpdateHelperExitCode::InstallerFailed);
    }

    let timeout_ms = INSTALLER_TIMEOUT.as_millis() as u32;
    let wait_result = unsafe { WaitForSingleObject(handle, timeout_ms) };

    if wait_result == WAIT_FAILED {
        let error = std::io::Error::last_os_error();
        unsafe { CloseHandle(handle) };
        write_log_line(log, &format!("WaitForSingleObject failed: {error}"))?;
        return Err(UpdateHelperExitCode::InstallerFailed);
    }

    if wait_result == WAIT_TIMEOUT {
        unsafe { CloseHandle(handle) };
        write_log_line(log, "installer timed out before completion")?;
        return Err(UpdateHelperExitCode::InstallerTimedOut);
    }

    if wait_result != WAIT_OBJECT_0 {
        unsafe { CloseHandle(handle) };
        write_log_line(
            log,
            &format!("installer wait returned unexpected status {wait_result:#x}"),
        )?;
        return Err(UpdateHelperExitCode::InstallerFailed);
    }

    let mut exit_code: u32 = 0;
    let read_exit_code = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
    unsafe {
        CloseHandle(handle);
    }
    if read_exit_code == 0 {
        let error = std::io::Error::last_os_error();
        write_log_line(log, &format!("GetExitCodeProcess failed: {error}"))?;
        return Err(UpdateHelperExitCode::InstallerFailed);
    }

    Ok(exit_code)
}

#[cfg(not(target_os = "windows"))]
fn shell_execute_installer(
    _path: &Path,
    _args: &str,
    _log: &mut File,
) -> Result<u32, UpdateHelperExitCode> {
    Err(UpdateHelperExitCode::UnsupportedInstallKind)
}

fn extract_app_bundle_from_zip(
    asset_path: &Path,
    stage_root: &Path,
    target_path: &Path,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    let extract_root = stage_root.join("unzipped");
    fs::create_dir_all(&extract_root).map_err(|_| UpdateHelperExitCode::AssetExtractFailed)?;
    write_log_line(
        log,
        &format!(
            "extracting app zip {} to {}",
            asset_path.display(),
            extract_root.display()
        ),
    )?;
    let status = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(asset_path)
        .arg(&extract_root)
        .status()
        .map_err(|_| UpdateHelperExitCode::AssetExtractFailed)?;

    if !status.success() {
        write_log_line(
            log,
            &format!("ditto extract exited with status {:?}", status.code()),
        )?;
        return Err(UpdateHelperExitCode::AssetExtractFailed);
    }

    select_app_bundle(
        &extract_root,
        target_path.file_name().and_then(|value| value.to_str()),
        log,
    )
}

fn stage_app_bundle_from_dmg(
    asset_path: &Path,
    stage_root: &Path,
    target_path: &Path,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    let mount_point = unique_temp_path(&std::env::temp_dir(), "mounted-dmg", None);
    fs::create_dir_all(&mount_point).map_err(|_| UpdateHelperExitCode::AssetExtractFailed)?;
    write_log_line(
        log,
        &format!(
            "mounting dmg {} at {}",
            asset_path.display(),
            mount_point.display()
        ),
    )?;
    let attach_status = Command::new("/usr/bin/hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount_point)
        .arg(asset_path)
        .status()
        .map_err(|error| {
            let _ = write_log_line(log, &format!("failed to run hdiutil attach: {error}"));
            let _ = fs::remove_dir_all(&mount_point);
            UpdateHelperExitCode::AssetExtractFailed
        })?;

    if !attach_status.success() {
        write_log_line(
            log,
            &format!(
                "hdiutil attach exited with status {:?}",
                attach_status.code()
            ),
        )?;
        let _ = fs::remove_dir_all(&mount_point);
        return Err(UpdateHelperExitCode::AssetExtractFailed);
    }

    let result = (|| {
        let mounted_bundle = select_app_bundle_from_mounted_dmg(
            &mount_point,
            target_path.file_name().and_then(|value| value.to_str()),
            log,
        )?;
        let bundle_name = mounted_bundle
            .file_name()
            .ok_or(UpdateHelperExitCode::AssetExtractFailed)?;
        let staged_bundle = stage_root.join(bundle_name);
        write_log_line(
            log,
            &format!(
                "copying mounted app bundle {} to {}",
                mounted_bundle.display(),
                staged_bundle.display()
            ),
        )?;
        let status = Command::new("/usr/bin/ditto")
            .arg(&mounted_bundle)
            .arg(&staged_bundle)
            .status()
            .map_err(|_| UpdateHelperExitCode::AssetExtractFailed)?;
        if !status.success() {
            write_log_line(
                log,
                &format!("ditto copy exited with status {:?}", status.code()),
            )?;
            return Err(UpdateHelperExitCode::AssetExtractFailed);
        }
        Ok(staged_bundle)
    })();

    cleanup_macos_mount(&mount_point, log)?;
    result
}

fn cleanup_macos_mount(mount_point: &Path, log: &mut File) -> Result<(), UpdateHelperExitCode> {
    let detach_status = Command::new("/usr/bin/hdiutil")
        .args(["detach"])
        .arg(mount_point)
        .status();
    match detach_status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            write_log_line(
                log,
                &format!(
                    "hdiutil detach exited with status {:?}; retrying with -force",
                    status.code()
                ),
            )?;
            let force_status = Command::new("/usr/bin/hdiutil")
                .args(["detach", "-force"])
                .arg(mount_point)
                .status()
                .map_err(|_| UpdateHelperExitCode::AssetExtractFailed)?;
            if !force_status.success() {
                write_log_line(
                    log,
                    &format!(
                        "hdiutil detach -force exited with status {:?}",
                        force_status.code()
                    ),
                )?;
                return Err(UpdateHelperExitCode::AssetExtractFailed);
            }
        }
        Err(error) => {
            write_log_line(log, &format!("failed to run hdiutil detach: {error}"))?;
            return Err(UpdateHelperExitCode::AssetExtractFailed);
        }
    }

    match fs::remove_dir_all(mount_point) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            write_log_line(
                log,
                &format!(
                    "failed to remove macOS dmg mount directory {} ({error})",
                    mount_point.display()
                ),
            )?;
            Err(UpdateHelperExitCode::AssetExtractFailed)
        }
    }
}

fn wait_for_installer_completion(
    child: &mut std::process::Child,
    log: &mut File,
) -> Result<std::process::ExitStatus, UpdateHelperExitCode> {
    let deadline = Instant::now() + INSTALLER_TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| UpdateHelperExitCode::InstallerFailed)?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            write_log_line(log, "installer timed out before completion")?;
            return Err(UpdateHelperExitCode::InstallerTimedOut);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn map_installer_exit(family: WindowsInstallerFamily, code: Option<u32>) -> UpdateHelperExitCode {
    match (family, code) {
        (WindowsInstallerFamily::Nsis, Some(1)) => UpdateHelperExitCode::InstallerCancelled,
        (WindowsInstallerFamily::Msi, Some(1602)) => UpdateHelperExitCode::InstallerCancelled,
        (WindowsInstallerFamily::Msi, Some(1603)) => UpdateHelperExitCode::InstallerFatal,
        (WindowsInstallerFamily::Msi, Some(1618)) => UpdateHelperExitCode::InstallerBusy,
        _ => UpdateHelperExitCode::InstallerFailed,
    }
}

fn wait_for_process_exit(
    pid: u32,
    expected_target_path: &Path,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let result = wait_for_process_exit_with_timeout(
        pid,
        WAIT_FOR_EXIT_GRACE_PERIOD,
        Some(expected_target_path),
        log,
    );
    if result.is_ok() {
        return result;
    }
    write_log_line(
        log,
        &format!("grace period elapsed, force-terminating process {pid}"),
    )?;
    if force_terminate_process(pid, log) {
        write_log_line(log, &format!("sent terminate signal to process {pid}"))?;
        wait_for_process_exit_with_timeout(
            pid,
            WAIT_FOR_EXIT_TIMEOUT,
            Some(expected_target_path),
            log,
        )
    } else {
        write_log_line(log, &format!("failed to terminate process {pid}"))?;
        Err(UpdateHelperExitCode::WaitTimedOut)
    }
}

fn wait_for_process_exit_with_timeout(
    pid: u32,
    timeout: Duration,
    expected_target_path: Option<&Path>,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process_is_running(pid, expected_target_path) {
            write_log_line(log, "application process has exited")?;
            return Ok(());
        }
        thread::sleep(POLL_INTERVAL);
    }

    write_log_line(log, &format!("timed out waiting for process {pid} to exit"))?;
    Err(UpdateHelperExitCode::WaitTimedOut)
}

fn wait_for_target_to_exist(
    target_path: &Path,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let deadline = Instant::now() + WAIT_FOR_REPLACEMENT_TIMEOUT;
    while Instant::now() < deadline {
        if target_path.exists() {
            return Ok(());
        }
        thread::sleep(POLL_INTERVAL);
    }

    write_log_line(
        log,
        &format!(
            "timed out waiting for install target to appear: {}",
            target_path.display()
        ),
    )?;
    Err(UpdateHelperExitCode::InstallerFailed)
}

#[cfg(target_os = "windows")]
fn verify_windows_installed_version(
    target_path: &Path,
    target_version: &str,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let actual_versions = windows_file_versions(target_path).ok_or_else(|| {
        let _ = write_log_line(
            log,
            &format!(
                "unable to read installed Windows file version from {}",
                target_path.display()
            ),
        );
        UpdateHelperExitCode::InstallerVersionMismatch
    })?;

    if actual_versions
        .iter()
        .any(|actual_version| installed_version_matches_target(actual_version, target_version))
    {
        write_log_line(
            log,
            &format!(
                "verified installed Windows version {}",
                actual_versions.join(", ")
            ),
        )?;
        return Ok(());
    }

    write_log_line(
        log,
        &format!(
            "installed Windows version mismatch: actual {}, expected {target_version}",
            actual_versions.join(", ")
        ),
    )?;
    Err(UpdateHelperExitCode::InstallerVersionMismatch)
}

#[cfg(not(target_os = "windows"))]
fn verify_windows_installed_version(
    _target_path: &Path,
    _target_version: &str,
    _log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_file_versions(path: &Path) -> Option<Vec<String>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    };

    let mut wide_path = path.as_os_str().encode_wide().collect::<Vec<_>>();
    wide_path.push(0);

    let mut handle = 0u32;
    let size = unsafe { GetFileVersionInfoSizeW(wide_path.as_ptr(), &mut handle) };
    if size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; size as usize];
    let ok = unsafe {
        GetFileVersionInfoW(
            wide_path.as_ptr(),
            0,
            size,
            buffer.as_mut_ptr().cast::<std::ffi::c_void>(),
        )
    };
    if ok == 0 {
        return None;
    }

    let sub_block = ['\\' as u16, 0];
    let mut fixed_info_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut fixed_info_len = 0u32;
    let ok = unsafe {
        VerQueryValueW(
            buffer.as_ptr().cast::<std::ffi::c_void>(),
            sub_block.as_ptr(),
            &mut fixed_info_ptr,
            &mut fixed_info_len,
        )
    };
    if ok == 0
        || fixed_info_ptr.is_null()
        || fixed_info_len < std::mem::size_of::<VS_FIXEDFILEINFO>() as u32
    {
        return None;
    }

    let fixed_info = unsafe { &*(fixed_info_ptr.cast::<VS_FIXEDFILEINFO>()) };
    if fixed_info.dwSignature != 0xFEEF04BD {
        return None;
    }

    let product =
        fixed_file_version_text(fixed_info.dwProductVersionMS, fixed_info.dwProductVersionLS);
    let file = fixed_file_version_text(fixed_info.dwFileVersionMS, fixed_info.dwFileVersionLS);
    let mut versions = vec![product];
    if versions.first() != Some(&file) {
        versions.push(file);
    }
    Some(versions)
}

#[cfg(target_os = "windows")]
fn fixed_file_version_text(ms: u32, ls: u32) -> String {
    let major = ms >> 16;
    let minor = ms & 0xffff;
    let patch = ls >> 16;
    let build = ls & 0xffff;
    format!("{major}.{minor}.{patch}.{build}")
}

#[cfg(any(target_os = "windows", test))]
fn installed_version_matches_target(actual_version: &str, target_version: &str) -> bool {
    let Some(actual) = numeric_version_prefix(actual_version) else {
        return false;
    };
    let Some(target) = numeric_version_prefix(target_version) else {
        return false;
    };
    if target.is_empty() {
        return false;
    }

    target
        .iter()
        .enumerate()
        .all(|(index, expected)| actual.get(index).copied().unwrap_or_default().eq(expected))
}

#[cfg(any(target_os = "windows", test))]
fn numeric_version_prefix(version: &str) -> Option<Vec<u64>> {
    let mut components = Vec::new();
    let normalized = version.trim().trim_start_matches(['v', 'V']);
    for segment in normalized.split(['.', '-', '+']) {
        if segment.is_empty() {
            break;
        }
        if !segment.bytes().all(|byte| byte.is_ascii_digit()) {
            break;
        }
        components.push(segment.parse::<u64>().ok()?);
    }
    (!components.is_empty()).then_some(components)
}

#[cfg(target_os = "windows")]
fn resolve_windows_launch_target(
    target_path: &Path,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    if target_path.exists() {
        return Ok(target_path.to_path_buf());
    }

    let Some(exe_name) = target_path.file_name().and_then(|value| value.to_str()) else {
        write_log_line(
            log,
            &format!(
                "unable to resolve Windows launch target for missing path {}",
                target_path.display()
            ),
        )?;
        return Err(UpdateHelperExitCode::InstallerVersionMismatch);
    };
    let resolved = query_windows_install_record_from_registry(exe_name, Some(target_path))
        .and_then(|record| record.launch_target);
    if let Some(path) = resolved.as_ref() {
        write_log_line(
            log,
            &format!(
                "resolved Windows install target from registry: {}",
                path.display()
            ),
        )?;
        return Ok(path.clone());
    }
    write_log_line(
        log,
        &format!(
            "install target missing and registry did not resolve a replacement path for {}",
            target_path.display()
        ),
    )?;
    Err(UpdateHelperExitCode::InstallerVersionMismatch)
}

#[cfg(not(target_os = "windows"))]
fn resolve_windows_launch_target(
    target_path: &Path,
    _log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    Ok(target_path.to_path_buf())
}

#[cfg(target_os = "windows")]
fn backup_windows_registry_state(
    command: &UpdateHelperCommand,
    backup_parent: &Path,
    log: &mut File,
) -> Result<Option<WindowsRegistryRollbackPlan>, UpdateHelperExitCode> {
    let Some(exe_name) = command
        .target_path
        .file_name()
        .and_then(|value| value.to_str())
    else {
        return Ok(None);
    };
    let Some(record) =
        query_windows_install_record_from_registry(exe_name, Some(&command.target_path))
    else {
        write_log_line(
            log,
            &format!(
                "no Windows uninstall registry entry matched {}; file rollback only",
                command.target_path.display()
            ),
        )?;
        return Ok(None);
    };

    let backup_path = unique_temp_path(backup_parent, "windows-registry-rollback", Some("reg"));
    let status = Command::new(reg_exe_path())
        .args([
            "export",
            &record.key_path,
            &backup_path.to_string_lossy(),
            "/y",
        ])
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
        .status()
        .map_err(|error| {
            let _ = write_log_line(
                log,
                &format!(
                    "failed to export Windows uninstall registry key {} to {} ({error})",
                    record.key_path,
                    backup_path.display()
                ),
            );
            UpdateHelperExitCode::ReplacementFailed
        })?;
    if !status.success() {
        write_log_line(
            log,
            &format!(
                "reg export failed for Windows uninstall registry key {} with status {:?}",
                record.key_path,
                status.code()
            ),
        )?;
        return Err(UpdateHelperExitCode::ReplacementFailed);
    }
    write_log_line(
        log,
        &format!(
            "exported Windows uninstall registry key {} to {}",
            record.key_path,
            backup_path.display()
        ),
    )?;
    Ok(Some(WindowsRegistryRollbackPlan {
        key_path: record.key_path,
        backup_path,
    }))
}

#[cfg(not(target_os = "windows"))]
fn backup_windows_registry_state(
    _command: &UpdateHelperCommand,
    _backup_parent: &Path,
    _log: &mut File,
) -> Result<Option<WindowsRegistryRollbackPlan>, UpdateHelperExitCode> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn query_windows_install_record_from_registry(
    exe_name: &str,
    expected_target_path: Option<&Path>,
) -> Option<WindowsRegistryInstallRecord> {
    const ROOTS: [&str; 4] = [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    ROOTS.iter().find_map(|root| {
        let key_path = Command::new(reg_exe_path())
            .args(["query", root, "/s", "/f", exe_name])
            .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|output| {
                parse_windows_install_registry_key_from_search_output(
                    &output,
                    exe_name,
                    expected_target_path,
                )
            })?;
        query_windows_install_registry_record(&key_path, exe_name, expected_target_path)
    })
}

#[cfg(target_os = "windows")]
fn reg_exe_path() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32").join("reg.exe"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\System32\reg.exe"))
}

#[cfg(target_os = "windows")]
fn query_windows_install_registry_record(
    key_path: &str,
    exe_name: &str,
    expected_target_path: Option<&Path>,
) -> Option<WindowsRegistryInstallRecord> {
    Command::new(reg_exe_path())
        .args(["query", key_path])
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|output| {
            parse_windows_install_registry_record_output(&output, exe_name, expected_target_path)
        })
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_install_registry_key_from_search_output(
    output: &str,
    exe_name: &str,
    expected_target_path: Option<&Path>,
) -> Option<String> {
    parse_windows_registry_entries(output)
        .into_iter()
        .find(|entry| registry_entry_matches_installation(entry, exe_name, expected_target_path))
        .map(|entry| entry.key_path)
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_install_registry_record_output(
    output: &str,
    exe_name: &str,
    expected_target_path: Option<&Path>,
) -> Option<WindowsRegistryInstallRecord> {
    let entry = parse_windows_registry_entries(output).into_iter().next()?;
    let mut launch_target = None;
    let mut install_dir = None;
    let mut quiet_uninstall_command = None;
    let mut uninstall_command = None;

    for (name, value) in &entry.values {
        match name.to_ascii_lowercase().as_str() {
            "displayicon" | "installlocation" | "installdir" => {
                if launch_target.is_none() {
                    launch_target = registry_candidate_launch_target(value, exe_name);
                }
                if install_dir.is_none() {
                    install_dir = registry_candidate_install_dir(value, exe_name);
                }
            }
            "quietuninstallstring" => quiet_uninstall_command = Some(value.clone()),
            "uninstallstring" => uninstall_command = Some(value.clone()),
            _ => {}
        }
    }
    if launch_target.is_none() {
        launch_target = install_dir.as_ref().map(|dir| dir.join(exe_name));
    }

    let record = WindowsRegistryInstallRecord {
        key_path: entry.key_path,
        launch_target,
        install_dir,
        quiet_uninstall_command,
        uninstall_command,
    };
    registry_record_matches_installation(&record, expected_target_path).then_some(record)
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_registry_entries(output: &str) -> Vec<WindowsRegistryEntry> {
    let mut entries = Vec::new();
    let mut current_key = None::<String>;
    let mut current_values = Vec::<(String, String)>::new();

    let flush = |entries: &mut Vec<WindowsRegistryEntry>,
                 current_key: &mut Option<String>,
                 current_values: &mut Vec<(String, String)>| {
        if let Some(key_path) = current_key.take() {
            entries.push(WindowsRegistryEntry {
                key_path,
                values: std::mem::take(current_values),
            });
        }
    };

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("HKEY_") {
            flush(&mut entries, &mut current_key, &mut current_values);
            current_key = Some(trimmed.to_string());
            continue;
        }
        if current_key.is_some() {
            if let Some(entry) = registry_value_entry_from_line(trimmed) {
                current_values.push(entry);
            }
        }
    }
    flush(&mut entries, &mut current_key, &mut current_values);
    entries
}

#[cfg(any(target_os = "windows", test))]
fn registry_entry_matches_installation(
    entry: &WindowsRegistryEntry,
    exe_name: &str,
    expected_target_path: Option<&Path>,
) -> bool {
    let record = WindowsRegistryInstallRecord {
        key_path: entry.key_path.clone(),
        launch_target: entry
            .values
            .iter()
            .find_map(|(_, value)| registry_candidate_launch_target(value, exe_name)),
        install_dir: entry
            .values
            .iter()
            .find_map(|(_, value)| registry_candidate_install_dir(value, exe_name)),
        quiet_uninstall_command: entry
            .values
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("QuietUninstallString"))
            .map(|(_, value)| value.clone()),
        uninstall_command: entry
            .values
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("UninstallString"))
            .map(|(_, value)| value.clone()),
    };
    registry_record_matches_installation(&record, expected_target_path)
}

#[cfg(any(target_os = "windows", test))]
fn registry_record_matches_installation(
    record: &WindowsRegistryInstallRecord,
    expected_target_path: Option<&Path>,
) -> bool {
    let Some(expected_target_path) = expected_target_path else {
        return record.launch_target.is_some() || record.install_dir.is_some();
    };
    let expected_path = normalize_windows_path(&expected_target_path.to_string_lossy());
    let expected_dir = expected_target_path
        .parent()
        .map(|path| normalize_windows_path(&path.to_string_lossy()));

    if record
        .launch_target
        .as_ref()
        .is_some_and(|path| normalize_windows_path(&path.to_string_lossy()) == expected_path)
    {
        return true;
    }
    record.install_dir.as_ref().is_some_and(|path| {
        expected_dir.as_ref().is_some_and(|expected_dir| {
            normalize_windows_path(&path.to_string_lossy()) == *expected_dir
        })
    })
}

#[cfg(any(target_os = "windows", test))]
fn registry_value_entry_from_line(line: &str) -> Option<(String, String)> {
    let (name, value) = line
        .split_once("REG_EXPAND_SZ")
        .or_else(|| line.split_once("REG_SZ"))
        .map(|(name, value)| (name.trim(), value.trim()))
        .filter(|(_, value)| !value.is_empty())?;
    let name = name
        .split_whitespace()
        .last()
        .filter(|value| !value.is_empty())?
        .to_string();
    Some((name, expand_windows_env_vars(value)))
}

#[cfg(any(target_os = "windows", test))]
fn registry_candidate_launch_target(value: &str, exe_name: &str) -> Option<PathBuf> {
    let candidate = registry_path_text_candidate(value);
    if windows_path_basename(&candidate).is_some_and(|name| name.eq_ignore_ascii_case(exe_name)) {
        return Some(PathBuf::from(candidate));
    }
    if !candidate.is_empty() {
        return Some(PathBuf::from(windows_join_path(&candidate, exe_name)));
    }
    None
}

#[cfg(any(target_os = "windows", test))]
fn registry_candidate_install_dir(value: &str, exe_name: &str) -> Option<PathBuf> {
    let candidate = registry_path_text_candidate(value);
    if windows_path_basename(&candidate).is_some_and(|name| name.eq_ignore_ascii_case(exe_name)) {
        return windows_path_dirname(&candidate).map(PathBuf::from);
    }
    Some(PathBuf::from(candidate))
}

#[cfg(any(target_os = "windows", test))]
fn registry_path_text_candidate(value: &str) -> String {
    let normalized = value.trim().trim_matches('"');
    let lower = normalized.to_ascii_lowercase();
    let trimmed = if let Some(index) = lower.find(".exe,") {
        &normalized[..index + 4]
    } else {
        normalized
    };
    trimmed.trim_matches('"').to_string()
}

#[cfg(any(target_os = "windows", test))]
fn windows_path_basename(value: &str) -> Option<&str> {
    value
        .trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .filter(|segment| !segment.is_empty())
}

#[cfg(any(target_os = "windows", test))]
fn windows_path_dirname(value: &str) -> Option<String> {
    let trimmed = value.trim_end_matches(['\\', '/']);
    let index = trimmed.rfind(['\\', '/'])?;
    Some(trimmed[..index].to_string())
}

#[cfg(any(target_os = "windows", test))]
fn windows_join_path(dir: &str, file_name: &str) -> String {
    let trimmed = dir.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        file_name.to_string()
    } else {
        format!(r"{trimmed}\{file_name}")
    }
}

#[cfg(any(target_os = "windows", test))]
fn expand_windows_env_vars(value: &str) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        expanded.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('%') else {
            expanded.push('%');
            expanded.push_str(after_start);
            return expanded;
        };
        let key = &after_start[..end];
        if let Ok(replacement) = std::env::var(key) {
            expanded.push_str(&replacement);
        } else {
            expanded.push('%');
            expanded.push_str(key);
            expanded.push('%');
        }
        rest = &after_start[end + 1..];
    }
    expanded.push_str(rest);
    expanded
}

#[cfg(any(target_os = "windows", test))]
fn build_silent_windows_uninstall_command(record: &WindowsRegistryInstallRecord) -> Option<String> {
    let quiet_command = record
        .quiet_uninstall_command
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if quiet_command.is_some() {
        return quiet_command.map(ToOwned::to_owned);
    }

    let command = record
        .uninstall_command
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let normalized = command.to_ascii_lowercase();
    if normalized.contains("msiexec") {
        if let Some(product_code) = extract_windows_msi_product_code(command) {
            return Some(format!("msiexec.exe /x {product_code} /qn /norestart"));
        }
        if contains_windows_command_flag(&normalized, "/quiet")
            || contains_windows_command_flag(&normalized, "/qn")
            || contains_windows_command_flag(&normalized, "/passive")
        {
            return Some(command.to_string());
        }
        return Some(format!("{command} /qn /norestart"));
    }
    if contains_windows_command_flag(&normalized, "/s") {
        return Some(command.to_string());
    }
    Some(format!("{command} /S"))
}

#[cfg(any(target_os = "windows", test))]
fn extract_windows_msi_product_code(command: &str) -> Option<&str> {
    let start = command.find('{')?;
    let rest = &command[start..];
    let end = rest.find('}')?;
    let product_code = &rest[..=end];
    product_code
        .chars()
        .all(|ch| ch.is_ascii_hexdigit() || matches!(ch, '{' | '}' | '-'))
        .then_some(product_code)
}

#[cfg(any(target_os = "windows", test))]
fn contains_windows_command_flag(command: &str, flag: &str) -> bool {
    command
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case(flag))
}

#[cfg(target_os = "windows")]
fn available_disk_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let mut wide_path = path.as_os_str().encode_wide().collect::<Vec<_>>();
    wide_path.push(0);
    let mut available = 0u64;
    let success = unsafe {
        GetDiskFreeSpaceExW(
            wide_path.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (success != 0).then_some(available)
}

#[cfg(unix)]
fn available_disk_space(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    let stat = unsafe { stat.assume_init() };
    Some((stat.f_bavail as u64).saturating_mul(stat.f_frsize))
}

#[cfg(not(any(unix, target_os = "windows")))]
fn available_disk_space(_path: &Path) -> Option<u64> {
    None
}

#[cfg(target_os = "windows")]
fn process_is_running(pid: u32, expected_target_path: Option<&Path>) -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut exit_code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut exit_code);
        let path_matches = expected_target_path.is_none_or(|expected| {
            query_windows_process_image_path(handle)
                .map(|actual| windows_paths_match(&actual, expected))
                .unwrap_or(true)
        });
        CloseHandle(handle);
        ok != 0 && exit_code == STILL_ACTIVE && path_matches
    }
}

#[cfg(target_os = "windows")]
fn query_windows_process_image_path(
    handle: windows_sys::Win32::Foundation::HANDLE,
) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Threading::QueryFullProcessImageNameW;

    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) };
    if ok == 0 || length == 0 {
        return None;
    }
    buffer.truncate(length as usize);
    Some(PathBuf::from(OsString::from_wide(&buffer)))
}

#[cfg(target_os = "windows")]
fn windows_paths_match(actual: &Path, expected: &Path) -> bool {
    let actual = fs::canonicalize(actual).unwrap_or_else(|_| actual.to_path_buf());
    let expected = fs::canonicalize(expected).unwrap_or_else(|_| expected.to_path_buf());
    normalize_windows_path(&actual.to_string_lossy())
        == normalize_windows_path(&expected.to_string_lossy())
}

#[cfg(target_os = "windows")]
fn force_terminate_process(pid: u32, log: &mut File) -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    };

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            let _ = write_log_line(
                log,
                &format!("failed to open process {pid} for termination"),
            );
            return false;
        }
        let terminated = TerminateProcess(handle, 1);
        let close_result = CloseHandle(handle);
        if terminated == 0 {
            let error = std::io::Error::last_os_error();
            let _ = write_log_line(log, &format!("TerminateProcess failed for {pid}: {error}"));
            return false;
        }
        if close_result == 0 {
            let error = std::io::Error::last_os_error();
            let _ = write_log_line(
                log,
                &format!("CloseHandle failed after terminating process {pid}: {error}"),
            );
        }
        true
    }
}

#[cfg(not(target_os = "windows"))]
fn force_terminate_process(pid: u32, _log: &mut File) -> bool {
    Command::new("/bin/kill")
        .args(["-9", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
use std::process::Stdio;

#[cfg(not(target_os = "windows"))]
fn process_is_running(pid: u32, expected_target_path: Option<&Path>) -> bool {
    let signal_ok = Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !signal_ok {
        return false;
    }

    match expected_target_path {
        Some(expected) => unix_pid_matches_expected_target(pid, expected),
        None => true,
    }
}

#[cfg(not(target_os = "windows"))]
fn unix_pid_matches_expected_target(pid: u32, expected_target_path: &Path) -> bool {
    let Some(expected_name) = expected_process_name(expected_target_path) else {
        return true;
    };
    match unix_process_command_name(pid) {
        Some(actual_name) => {
            actual_name == expected_name || actual_name.ends_with(&format!("/{expected_name}"))
        }
        None => true,
    }
}

#[cfg(not(target_os = "windows"))]
fn expected_process_name(path: &Path) -> Option<String> {
    let executable_path = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
    {
        let macos_dir = path.join("Contents").join("MacOS");
        if let Some(executable_name) = single_child_file_name(&macos_dir) {
            return Some(executable_name);
        }
        path.join("Contents").join("MacOS").join(path.file_stem()?)
    } else {
        path.to_path_buf()
    };
    executable_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

#[cfg(not(target_os = "windows"))]
fn single_child_file_name(path: &Path) -> Option<String> {
    let mut names = fs::read_dir(path)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }
            entry.file_name().to_str().map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    names.sort();
    (names.len() == 1).then(|| names.remove(0))
}

#[cfg(not(target_os = "windows"))]
fn unix_process_command_name(pid: u32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn relaunch_existing_target(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    if command.target_path.exists() {
        relaunch_target(&command.target_path, log)?;
    }
    Ok(())
}

fn relaunch_target(target_path: &Path, log: &mut File) -> Result<(), UpdateHelperExitCode> {
    if target_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
    {
        write_log_line(
            log,
            &format!("relaunching app bundle {}", target_path.display()),
        )?;
        let status = Command::new("/usr/bin/open")
            .arg(target_path)
            .status()
            .map_err(|_| UpdateHelperExitCode::RelaunchFailed)?;
        if !status.success() {
            write_log_line(log, &format!("open exited with status {:?}", status.code()))?;
            return Err(UpdateHelperExitCode::RelaunchFailed);
        }
        return Ok(());
    }

    write_log_line(
        log,
        &format!("relaunching executable {}", target_path.display()),
    )?;
    Command::new(target_path)
        .spawn()
        .map(|_| ())
        .map_err(|_| UpdateHelperExitCode::RelaunchFailed)
}

fn persist_pending_verification_state(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    mutate_state_snapshot(command, log, "pending verification", |state| {
        state.status = UpdateStatus::Installing;
        state.current_version = command.current_version.clone();
        if state.latest_version.is_none() {
            state.latest_version = Some(command.target_version.clone());
        }
        state.install_log_path = Some(command.log_path.to_string_lossy().to_string());
        state.install_mode = Some(UpdateInstallMode::Apply);
        state.install_started_at = Some(Utc::now());
        state.install_scheduled_at = None;
        state.last_error = None;
    })?;
    write_log_line(log, "persisted install state pending relaunch verification")?;
    Ok(())
}

fn persist_failed_state(
    command: &UpdateHelperCommand,
    code: UpdateHelperExitCode,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    mutate_state_snapshot(command, log, "failed install", |state| {
        state.status = UpdateStatus::Failed;
        state.current_version = command.current_version.clone();
        if state.latest_version.is_none() {
            state.latest_version = Some(command.target_version.clone());
        }
        state.install_log_path = Some(command.log_path.to_string_lossy().to_string());
        state.install_mode = Some(UpdateInstallMode::Apply);
        state.install_started_at = Some(Utc::now());
        state.install_scheduled_at = None;
        state.last_error = Some(UpdateErrorDto::recoverable(
            install_error_code(code),
            install_error_message(code),
            Some(install_error_action(code).into()),
        ));
    })?;
    write_log_line(log, "persisted failed install state")?;
    Ok(())
}

fn cleanup_after_install(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    remove_file_if_exists(&command.ready_path).map_err(|error| {
        let _ = write_log_line(
            log,
            &format!(
                "failed to remove helper ready marker {} ({error})",
                command.ready_path.display()
            ),
        );
        UpdateHelperExitCode::CleanupFailed
    })
}

fn cleanup_stage_root(stage_root: &Path, log: &mut File) -> Result<(), UpdateHelperExitCode> {
    if !stage_root.exists() {
        return Ok(());
    }
    if let Err(error) = fs::remove_dir_all(stage_root) {
        write_log_line(
            log,
            &format!(
                "failed to remove temporary update stage {} ({error})",
                stage_root.display()
            ),
        )?;
    }
    Ok(())
}

fn cleanup_applied_update(
    applied_update: &AppliedUpdate,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    if let Some(rollback) = applied_update.rollback.as_ref() {
        match rollback {
            InstallRollbackPlan::Macos(rollback) => {
                write_log_line(
                    log,
                    &format!(
                        "retaining macOS rollback backup after relaunch: {}",
                        rollback.stage_root.display()
                    ),
                )?;
            }
            InstallRollbackPlan::Windows(rollback) => {
                fs::remove_dir_all(&rollback.backup_dir).map_err(|error| {
                    let _ = write_log_line(
                        log,
                        &format!(
                            "failed to remove Windows rollback backup {} ({error})",
                            rollback.backup_dir.display()
                        ),
                    );
                    UpdateHelperExitCode::CleanupFailed
                })?;
                if let Some(registry_backup) = rollback.registry_backup.as_ref() {
                    remove_file_if_exists(&registry_backup.backup_path).map_err(|error| {
                        let _ = write_log_line(
                            log,
                            &format!(
                                "failed to remove Windows registry rollback backup {} ({error})",
                                registry_backup.backup_path.display()
                            ),
                        );
                        UpdateHelperExitCode::CleanupFailed
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn rollback_applied_update(
    rollback: &InstallRollbackPlan,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    match rollback {
        InstallRollbackPlan::Macos(rollback) => rollback_macos_update(rollback, log),
        InstallRollbackPlan::Windows(rollback) => rollback_windows_update(rollback, log),
    }
}

fn rollback_macos_update(
    rollback: &MacosRollbackPlan,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    write_log_line(
        log,
        &format!(
            "relaunch failed after macOS bundle swap; rolling back {} from {}",
            rollback.target_path.display(),
            rollback.backup_path.display()
        ),
    )?;
    swap_macos_bundles(&rollback.target_path, &rollback.backup_path, log)?;
    cleanup_stage_root(&rollback.stage_root, log)?;
    Ok(())
}

fn rollback_windows_update(
    rollback: &WindowsRollbackPlan,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    write_log_line(
        log,
        &format!(
            "restoring Windows install directory {} for target {} from rollback backup {}",
            rollback.install_dir.display(),
            rollback.target_path.display(),
            rollback.backup_dir.display()
        ),
    )?;
    best_effort_uninstall_windows_installation(rollback, log)?;
    if rollback.install_dir.exists() {
        fs::remove_dir_all(&rollback.install_dir).map_err(|error| {
            let _ = write_log_line(
                log,
                &format!(
                    "failed to remove partially installed Windows directory {} ({error})",
                    rollback.install_dir.display()
                ),
            );
            UpdateHelperExitCode::ReplacementFailed
        })?;
    }
    if let Some(parent) = rollback.install_dir.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);
            let _ = write_log_line(
                log,
                &format!(
                    "failed to recreate Windows install parent directory {} ({error})",
                    parent.display()
                ),
            );
            code
        })?;
    }
    copy_dir_recursive(&rollback.backup_dir, &rollback.install_dir).map_err(|error| {
        let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);
        let _ = write_log_line(
            log,
            &format!(
                "failed to restore Windows install directory {} from {} ({error})",
                rollback.install_dir.display(),
                rollback.backup_dir.display()
            ),
        );
        if code == UpdateHelperExitCode::InsufficientSpace {
            let _ = write_log_line(
                log,
                &format!(
                    "rollback backup is retained at {} for manual recovery",
                    rollback.backup_dir.display()
                ),
            );
        }
        code
    })?;
    restore_windows_registry_backup(rollback, log)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn best_effort_uninstall_windows_installation(
    rollback: &WindowsRollbackPlan,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let Some(exe_name) = rollback
        .target_path
        .file_name()
        .and_then(|value| value.to_str())
    else {
        return Ok(());
    };
    let Some(record) =
        query_windows_install_record_from_registry(exe_name, Some(&rollback.target_path))
    else {
        write_log_line(
            log,
            &format!(
                "no Windows uninstall registry entry matched {}; skipping best-effort uninstall",
                rollback.target_path.display()
            ),
        )?;
        return Ok(());
    };
    let Some(command_text) = build_silent_windows_uninstall_command(&record) else {
        write_log_line(
            log,
            &format!(
                "Windows uninstall registry entry {} has no usable uninstall command; continuing with file restore",
                record.key_path
            ),
        )?;
        return Ok(());
    };

    write_log_line(
        log,
        &format!(
            "running best-effort Windows uninstall before rollback: key={} command={}",
            record.key_path, command_text
        ),
    )?;
    let mut child = match windows_shell_command(&command_text).spawn() {
        Ok(child) => child,
        Err(error) => {
            write_log_line(
                log,
                &format!(
                    "failed to launch best-effort Windows uninstall for {} ({error}); continuing with file restore",
                    record.key_path
                ),
            )?;
            return Ok(());
        }
    };
    let status = match wait_for_installer_completion(&mut child, log) {
        Ok(status) => status,
        Err(error) => {
            write_log_line(
                log,
                &format!(
                    "best-effort Windows uninstall did not finish cleanly ({error:?}); continuing with file restore"
                ),
            )?;
            return Ok(());
        }
    };
    if !status.success() {
        write_log_line(
            log,
            &format!(
                "best-effort Windows uninstall exited with status {:?}; continuing with file restore",
                status.code()
            ),
        )?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn best_effort_uninstall_windows_installation(
    _rollback: &WindowsRollbackPlan,
    _log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn restore_windows_registry_backup(
    rollback: &WindowsRollbackPlan,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let Some(registry_backup) = rollback.registry_backup.as_ref() else {
        return Ok(());
    };
    let status = Command::new(reg_exe_path())
        .args(["import", &registry_backup.backup_path.to_string_lossy()])
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
        .status()
        .map_err(|error| {
            let _ = write_log_line(
                log,
                &format!(
                    "failed to import Windows uninstall registry backup {} ({error})",
                    registry_backup.backup_path.display()
                ),
            );
            UpdateHelperExitCode::ReplacementFailed
        })?;
    if !status.success() {
        write_log_line(
            log,
            &format!(
                "reg import failed for Windows uninstall registry backup {} with status {:?}",
                registry_backup.backup_path.display(),
                status.code()
            ),
        )?;
        return Err(UpdateHelperExitCode::ReplacementFailed);
    }
    write_log_line(
        log,
        &format!(
            "restored Windows uninstall registry key {} from {}",
            registry_backup.key_path,
            registry_backup.backup_path.display()
        ),
    )?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn restore_windows_registry_backup(
    _rollback: &WindowsRollbackPlan,
    _log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_shell_command(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.args(["/C", command]);
    process.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    process.stdin(std::process::Stdio::null());
    process.stdout(std::process::Stdio::null());
    process.stderr(std::process::Stdio::null());
    process
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        if source == to || source.starts_with(to) {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source, &target)?;
        } else {
            fs::copy(source, target)?;
        }
    }
    Ok(())
}

fn directory_size_bytes(path: &Path) -> Result<u64, std::io::Error> {
    let metadata = fs::metadata(path)?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }

    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        total = total.saturating_add(directory_size_bytes(&entry.path())?);
    }
    Ok(total)
}

fn windows_rollback_storage_parent(
    command: &UpdateHelperCommand,
    install_dir: &Path,
) -> Result<PathBuf, UpdateHelperExitCode> {
    let backup_parent = command
        .ready_path
        .parent()
        .ok_or(UpdateHelperExitCode::ReplacementFailed)?;
    if !backup_parent.starts_with(install_dir) {
        return Ok(backup_parent.to_path_buf());
    }

    Ok(std::env::temp_dir().join("floral-notepaper-update-rollback"))
}

fn existing_probe_path(path: &Path) -> PathBuf {
    path.ancestors()
        .find(|candidate| candidate.exists())
        .unwrap_or(path)
        .to_path_buf()
}

fn fallback_bucket_key(path: &Path) -> String {
    existing_probe_path(path).display().to_string()
}

#[cfg(target_os = "windows")]
fn storage_bucket_key(path: &Path) -> Option<String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW;

    let probe = existing_probe_path(path);
    let mut wide_path = probe.as_os_str().encode_wide().collect::<Vec<_>>();
    wide_path.push(0);
    let mut buffer = vec![0u16; 32768];
    let ok =
        unsafe { GetVolumePathNameW(wide_path.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    if ok == 0 {
        return None;
    }
    let len = buffer.iter().position(|value| *value == 0)?;
    buffer.truncate(len);
    Some(
        OsString::from_wide(&buffer)
            .to_string_lossy()
            .to_ascii_lowercase(),
    )
}

#[cfg(unix)]
fn storage_bucket_key(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    let probe = existing_probe_path(path);
    let metadata = fs::metadata(probe).ok()?;
    Some(metadata.dev().to_string())
}

#[cfg(not(any(unix, target_os = "windows")))]
fn storage_bucket_key(path: &Path) -> Option<String> {
    Some(existing_probe_path(path).display().to_string())
}

fn map_copy_error_code(
    error: &std::io::Error,
    fallback: UpdateHelperExitCode,
) -> UpdateHelperExitCode {
    if is_insufficient_space_error(error) {
        UpdateHelperExitCode::InsufficientSpace
    } else {
        fallback
    }
}

#[cfg(target_os = "windows")]
fn is_insufficient_space_error(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(39 | 112))
}

#[cfg(unix)]
fn is_insufficient_space_error(error: &std::io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(code) if code == libc::ENOSPC || code == libc::EDQUOT
    )
}

#[cfg(not(any(unix, target_os = "windows")))]
fn is_insufficient_space_error(_error: &std::io::Error) -> bool {
    false
}

fn cleanup_stale_macos_stage_dirs(
    parent: &Path,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Ok(());
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !is_update_stage_name(name) {
            continue;
        }

        let path = entry.path();
        let remove_result = if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };

        match remove_result {
            Ok(()) => {
                write_log_line(
                    log,
                    &format!("removed stale macOS update stage {}", path.display()),
                )?;
            }
            Err(error) => {
                write_log_line(
                    log,
                    &format!(
                        "failed to remove stale macOS update stage {} ({error})",
                        path.display()
                    ),
                )?;
            }
        }
    }

    Ok(())
}

fn is_update_stage_name(name: &str) -> bool {
    name.starts_with(UPDATE_STAGE_PREFIX)
}

fn select_app_bundle_from_mounted_dmg(
    mount_point: &Path,
    expected_name: Option<&str>,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    if let Some(expected_name) = expected_name {
        let exact_match = mount_point.join(expected_name);
        if is_real_app_bundle(&exact_match) {
            return Ok(exact_match);
        }
    }

    let bundles = find_top_level_app_bundles(mount_point);
    if bundles.is_empty() {
        write_log_line(log, "no app bundle found at dmg root")?;
        return Err(UpdateHelperExitCode::AssetExtractFailed);
    }

    if let Some(expected_name) = expected_name {
        if let Some(bundle) = bundles.iter().find(|bundle| {
            bundle
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name == expected_name)
        }) {
            return Ok(bundle.clone());
        }
    }

    if bundles.len() == 1 {
        return Ok(bundles
            .into_iter()
            .next()
            .expect("single dmg root app bundle"));
    }

    write_log_line(
        log,
        "multiple top-level app bundles found in dmg payload and none matched target name",
    )?;
    Err(UpdateHelperExitCode::AssetExtractFailed)
}

fn find_top_level_app_bundles(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut bundles = entries
        .flatten()
        .filter_map(|entry| {
            let candidate = entry.path();
            let file_type = entry.file_type().ok()?;
            if file_type.is_symlink() || !file_type.is_dir() || !is_real_app_bundle(&candidate) {
                return None;
            }
            Some(candidate)
        })
        .collect::<Vec<_>>();
    bundles.sort();
    bundles
}

fn is_real_app_bundle(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
        && fs::symlink_metadata(path)
            .map(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
            .unwrap_or(false)
}

fn should_skip_bundle_search_entry(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(is_update_stage_name)
}

fn find_app_bundles(root: &Path) -> Vec<PathBuf> {
    if is_real_app_bundle(root) {
        return vec![root.to_path_buf()];
    }

    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            if should_skip_bundle_search_entry(&entry) {
                continue;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }

            let candidate = entry.path();
            if file_type.is_dir() && is_real_app_bundle(&candidate) {
                results.push(candidate);
                continue;
            }
            if file_type.is_dir() {
                stack.push(candidate);
            }
        }
    }
    results.sort();
    results
}

fn mutate_state_snapshot<F>(
    command: &UpdateHelperCommand,
    log: &mut File,
    label: &str,
    mutate: F,
) -> Result<(), UpdateHelperExitCode>
where
    F: FnOnce(&mut UpdateStateDto),
{
    let _lock = acquire_helper_state_lock(&command.state_path, log)?;
    let mut state = load_state_snapshot_unlocked(command);
    mutate(&mut state);
    write_state_snapshot_unlocked(&command.state_path, &state).map_err(|_| {
        let _ = write_log_line(
            log,
            &format!(
                "failed to persist {label} state to {}",
                command.state_path.display()
            ),
        );
        UpdateHelperExitCode::StateWriteFailed
    })
}

fn acquire_helper_state_lock(
    state_path: &Path,
    log: &mut File,
) -> Result<super::file_lock::UpdateStateLock, UpdateHelperExitCode> {
    acquire_update_state_lock(state_path).map_err(|error| {
        let lock_path = state_path.with_extension("lock");
        let message = if error.kind() == std::io::ErrorKind::TimedOut {
            format!("timed out waiting for state lock {}", lock_path.display())
        } else {
            format!(
                "failed to acquire state lock {} ({error})",
                lock_path.display()
            )
        };
        let _ = write_log_line(log, &message);
        UpdateHelperExitCode::StateWriteFailed
    })
}

fn load_state_snapshot_unlocked(command: &UpdateHelperCommand) -> UpdateStateDto {
    match fs::read_to_string(&command.state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<UpdateStateDto>(&raw).ok())
    {
        Some(mut state) => {
            state.current_version = command.current_version.clone();
            if state.latest_version.is_none() {
                state.latest_version = Some(command.target_version.clone());
            }
            state
        }
        None => {
            let mut state = UpdateStateDto::idle_with_version(command.current_version.clone());
            state.latest_version = Some(command.target_version.clone());
            state
        }
    }
}

fn write_state_snapshot_unlocked(
    path: &Path,
    state: &UpdateStateDto,
) -> Result<(), std::io::Error> {
    write_json_atomic(path, state).map_err(|error| std::io::Error::other(error.message))
}

fn install_error_code(code: UpdateHelperExitCode) -> &'static str {
    debug_assert_ne!(code, UpdateHelperExitCode::Success);
    match code {
        UpdateHelperExitCode::InvalidArguments => "updateInstallHelperInvalidArguments",
        UpdateHelperExitCode::AssetMissing => "updateInstallAssetMissing",
        UpdateHelperExitCode::AssetUnreadable => "updateInstallAssetUnreadable",
        UpdateHelperExitCode::AssetSizeMismatch => "updateInstallAssetSizeMismatch",
        UpdateHelperExitCode::AssetHashMismatch => "updateInstallAssetHashMismatch",
        UpdateHelperExitCode::TargetMissing => "updateInstallTargetMissing",
        UpdateHelperExitCode::LogWriteFailed => "updateInstallLogWriteFailed",
        UpdateHelperExitCode::WaitTimedOut => "updateInstallWaitTimedOut",
        UpdateHelperExitCode::UnsupportedInstallKind => "updateInstallUnsupportedKind",
        UpdateHelperExitCode::AssetExtractFailed => "updateInstallAssetExtractFailed",
        UpdateHelperExitCode::ReplacementFailed => "updateInstallReplaceFailed",
        UpdateHelperExitCode::RelaunchFailed => "updateInstallRelaunchFailed",
        UpdateHelperExitCode::StateWriteFailed => "updateInstallStateWriteFailed",
        UpdateHelperExitCode::InstallerFailed => "updateInstallInstallerFailed",
        UpdateHelperExitCode::InsufficientSpace => "updateInstallInsufficientSpace",
        UpdateHelperExitCode::InstallerTimedOut => "updateInstallInstallerTimedOut",
        UpdateHelperExitCode::InstallerCancelled => "updateInstallInstallerCancelled",
        UpdateHelperExitCode::InstallerBusy => "updateInstallInstallerBusy",
        UpdateHelperExitCode::InstallerFatal => "updateInstallInstallerFatal",
        UpdateHelperExitCode::InstallerVersionMismatch => "updateInstallVersionMismatch",
        UpdateHelperExitCode::CleanupFailed => "updateInstallCleanupFailed",
        UpdateHelperExitCode::Success => "updateInstallCompleted",
    }
}

fn install_error_message(code: UpdateHelperExitCode) -> &'static str {
    debug_assert_ne!(code, UpdateHelperExitCode::Success);
    match code {
        UpdateHelperExitCode::InvalidArguments => "更新安装助手参数无效",
        UpdateHelperExitCode::AssetMissing => "更新包文件不存在或无法读取",
        UpdateHelperExitCode::AssetUnreadable => "更新包文件存在但无法读取",
        UpdateHelperExitCode::AssetSizeMismatch => "更新包大小校验失败",
        UpdateHelperExitCode::AssetHashMismatch => "更新包哈希校验失败",
        UpdateHelperExitCode::TargetMissing => "当前安装目标不存在，无法继续",
        UpdateHelperExitCode::LogWriteFailed => "无法写入安装日志",
        UpdateHelperExitCode::WaitTimedOut => "等待应用退出超时，未能继续安装",
        UpdateHelperExitCode::UnsupportedInstallKind => "当前安装形态暂不支持应用内更新",
        UpdateHelperExitCode::AssetExtractFailed => "无法解包更新资源，安装未完成",
        UpdateHelperExitCode::ReplacementFailed => "替换当前安装内容失败",
        UpdateHelperExitCode::RelaunchFailed => "更新完成后重新启动应用失败",
        UpdateHelperExitCode::StateWriteFailed => "无法写入安装状态文件",
        UpdateHelperExitCode::InstallerFailed => "更新安装程序执行失败",
        UpdateHelperExitCode::InsufficientSpace => "可用磁盘空间不足，无法继续安装更新",
        UpdateHelperExitCode::InstallerTimedOut => "更新安装程序执行超时",
        UpdateHelperExitCode::InstallerCancelled => "更新安装已被取消",
        UpdateHelperExitCode::InstallerBusy => "另一个安装程序正在运行，请稍后重试",
        UpdateHelperExitCode::InstallerFatal => "更新安装程序返回了致命错误",
        UpdateHelperExitCode::InstallerVersionMismatch => "安装完成后版本校验失败",
        UpdateHelperExitCode::CleanupFailed => "安装后清理临时文件失败",
        UpdateHelperExitCode::Success => "更新安装助手已完成",
    }
}

fn install_error_action(code: UpdateHelperExitCode) -> &'static str {
    match code {
        UpdateHelperExitCode::AssetMissing
        | UpdateHelperExitCode::AssetUnreadable
        | UpdateHelperExitCode::AssetSizeMismatch
        | UpdateHelperExitCode::AssetHashMismatch
        | UpdateHelperExitCode::AssetExtractFailed => "retryDownload",
        _ => "retryInstall",
    }
}

fn select_app_bundle(
    root: &Path,
    expected_name: Option<&str>,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    let bundles = find_app_bundles(root);
    if bundles.is_empty() {
        write_log_line(log, "no app bundle found in extracted update payload")?;
        return Err(UpdateHelperExitCode::AssetExtractFailed);
    }

    if let Some(expected_name) = expected_name {
        if let Some(bundle) = bundles.iter().find(|bundle| {
            bundle
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name == expected_name)
        }) {
            return Ok(bundle.clone());
        }
    }

    if bundles.len() == 1 {
        return Ok(bundles.into_iter().next().expect("single app bundle"));
    }

    write_log_line(
        log,
        "multiple app bundles found in update payload and none matched target name",
    )?;
    Err(UpdateHelperExitCode::AssetExtractFailed)
}

fn verify_macos_bundle(bundle: &Path, log: &mut File) -> Result<(), UpdateHelperExitCode> {
    let _ = bundle;
    let _ = log;

    let spctl_path = Path::new("/usr/sbin/spctl");
    if spctl_path.exists() {
        let status = Command::new(spctl_path)
            .args(["--assess", "--type", "execute"])
            .arg(bundle)
            .status()
            .map_err(|_| UpdateHelperExitCode::ReplacementFailed)?;
        if !status.success() {
            write_log_line(
                log,
                &format!("spctl assess failed with status {:?}", status.code()),
            )?;
            return Err(UpdateHelperExitCode::ReplacementFailed);
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn swap_macos_bundles(
    target_path: &Path,
    staged_bundle: &Path,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    use std::ffi::CString;

    let target = CString::new(target_path.to_string_lossy().as_bytes())
        .map_err(|_| UpdateHelperExitCode::ReplacementFailed)?;
    let staged = CString::new(staged_bundle.to_string_lossy().as_bytes())
        .map_err(|_| UpdateHelperExitCode::ReplacementFailed)?;

    let result = unsafe { libc::renamex_np(staged.as_ptr(), target.as_ptr(), libc::RENAME_SWAP) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        write_log_line(
            log,
            &format!(
                "failed to atomically swap app bundle {} with {} ({error})",
                staged_bundle.display(),
                target_path.display()
            ),
        )?;
        return Err(UpdateHelperExitCode::ReplacementFailed);
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn swap_macos_bundles(
    _target_path: &Path,
    _staged_bundle: &Path,
    _log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    Err(UpdateHelperExitCode::ReplacementFailed)
}

fn install_kind_as_str(kind: &InstallKind) -> &'static str {
    match kind {
        InstallKind::WindowsNsis => "windows-nsis",
        InstallKind::WindowsPortable => "windows-portable",
        InstallKind::MacosAppBundle => "macos-app-bundle",
        InstallKind::Unknown => "unknown",
    }
}

fn parse_install_kind(value: &str) -> Option<InstallKind> {
    match value.trim() {
        "windows-nsis" | "windowsNsis" => Some(InstallKind::WindowsNsis),
        "windows-portable" | "windowsPortable" => Some(InstallKind::WindowsPortable),
        "macos-app-bundle" | "macosAppBundle" => Some(InstallKind::MacosAppBundle),
        "unknown" => Some(InstallKind::Unknown),
        _ => None,
    }
}

fn unique_temp_path(parent: &Path, stem: &str, extension: Option<&str>) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut file_name = format!(".floral-notepaper-{stem}-{unique}");
    if let Some(extension) = extension.filter(|value| !value.is_empty()) {
        file_name.push('.');
        file_name.push_str(extension);
    }
    parent.join(file_name)
}

fn required_arg<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn require_text(value: Option<&String>, key: &str) -> Result<String, String> {
    let value = value
        .map(String::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))?
        .trim();
    if value.is_empty() {
        return Err(format!("argument cannot be empty: {key}"));
    }
    Ok(value.to_string())
}

fn open_log(path: &Path) -> Result<File, UpdateHelperExitCode> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| UpdateHelperExitCode::LogWriteFailed)?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| UpdateHelperExitCode::LogWriteFailed)
}

fn write_log_header(
    file: &mut File,
    command: &UpdateHelperCommand,
) -> Result<(), UpdateHelperExitCode> {
    write_log_line(file, "floral-notepaper update helper")?;
    write_log_line(file, &format!("mode={}", command.mode.as_str()))?;
    write_log_line(
        file,
        &format!(
            "install_kind={}",
            install_kind_as_str(&command.install_kind)
        ),
    )?;
    write_log_line(file, &format!("wait_pid={}", command.wait_pid))?;
    write_log_line(file, &format!("state={}", command.state_path.display()))?;
    write_log_line(file, &format!("asset={}", command.asset_path.display()))?;
    write_log_line(file, &format!("target={}", command.target_path.display()))?;
    write_log_line(file, &format!("ready={}", command.ready_path.display()))?;
    Ok(())
}

fn write_log_line(file: &mut File, line: &str) -> Result<(), UpdateHelperExitCode> {
    writeln!(file, "{line}").map_err(|_| UpdateHelperExitCode::LogWriteFailed)
}

fn sha256_hex(path: &Path) -> Result<String, std::io::Error> {
    super::sha256_hex(path)
}

#[cfg(target_os = "macos")]
pub(crate) fn cleanup_stale_macos_mounts(_paths: &UpdatePaths) {
    let temp_dir = std::env::temp_dir();
    let entries = match fs::read_dir(&temp_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.starts_with(MOUNT_STAGE_PREFIX) {
            continue;
        }
        let _ = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-force"])
            .arg(&path)
            .status();
        let _ = fs::remove_dir_all(&path);
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn cleanup_stale_macos_mounts(_paths: &UpdatePaths) {}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    struct TestDir {
        path: PathBuf,
    }

    impl std::ops::Deref for TestDir {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            self.path.as_path()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn temp_dir(name: &str) -> TestDir {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(format!("{name}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        TestDir { path }
    }

    fn helper_command(root: &Path) -> UpdateHelperCommand {
        let asset_path = root.join("asset.bin");
        fs::write(&asset_path, b"hello helper").expect("write asset");

        let target_path = root.join("target.app");
        fs::create_dir_all(&target_path).expect("create target");

        UpdateHelperCommand {
            mode: UpdateHelperMode::Test,
            install_kind: InstallKind::MacosAppBundle,
            wait_pid: 42,
            state_path: root.join("state.json"),
            asset_sha256: sha256_hex(&asset_path).expect("hash asset"),
            asset_size: fs::metadata(&asset_path).expect("asset metadata").len(),
            log_path: root.join("helper.log"),
            ready_path: root.join("helper.ready"),
            asset_path,
            target_path,
            current_version: "1.0.3".into(),
            target_version: "1.0.5".into(),
        }
    }

    #[cfg(unix)]
    fn temp_log(root: &Path, name: &str) -> File {
        open_log(&root.join(name)).expect("open temp log")
    }

    #[test]
    fn waits_long_enough_for_slow_main_process_shutdown() {
        assert!(WAIT_FOR_EXIT_TIMEOUT >= Duration::from_secs(120));
    }

    #[test]
    fn parses_strict_arguments() {
        let args: Vec<OsString> = vec![
            OsString::from("--mode"),
            OsString::from("apply"),
            OsString::from("--install-kind"),
            OsString::from("macos-app-bundle"),
            OsString::from("--wait-pid"),
            OsString::from("42"),
            OsString::from("--state-path"),
            OsString::from("/tmp/state.json"),
            OsString::from("--asset-path"),
            OsString::from("/tmp/asset.zip"),
            OsString::from("--asset-sha256"),
            OsString::from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            OsString::from("--asset-size"),
            OsString::from("42"),
            OsString::from("--target-path"),
            OsString::from("/Applications/Floral Notepaper.app"),
            OsString::from("--log-path"),
            OsString::from("/tmp/helper.log"),
            OsString::from("--ready-path"),
            OsString::from("/tmp/helper.ready"),
            OsString::from("--current-version"),
            OsString::from("1.0.3"),
            OsString::from("--target-version"),
            OsString::from("1.0.5"),
        ];

        let parsed = parse_args(args).expect("parse helper args");

        assert_eq!(parsed.mode, UpdateHelperMode::Apply);
        assert_eq!(parsed.install_kind, InstallKind::MacosAppBundle);
        assert_eq!(parsed.wait_pid, 42);
        assert_eq!(parsed.asset_size, 42);
        assert_eq!(parsed.current_version, "1.0.3");
    }

    #[test]
    fn rejects_duplicate_arguments() {
        let args: Vec<OsString> = vec![
            OsString::from("--mode"),
            OsString::from("apply"),
            OsString::from("--mode"),
            OsString::from("test"),
        ];

        let error = parse_args(args).expect_err("duplicate args should fail");

        assert!(error.contains("duplicate argument"));
    }

    #[test]
    fn validates_test_mode_request() {
        let root = temp_dir("helper-success");
        let command = helper_command(&root);

        execute(&command).expect("helper should validate request");
        assert!(command.log_path.exists());
        assert!(command.ready_path.exists());
    }

    #[test]
    fn returns_size_mismatch_exit_code() {
        let root = temp_dir("helper-size-mismatch");
        let mut command = helper_command(&root);
        command.asset_size += 1;

        let exit_code = execute(&command).expect_err("size mismatch should fail");

        assert_eq!(exit_code, UpdateHelperExitCode::AssetSizeMismatch);
    }

    #[test]
    fn returns_hash_mismatch_exit_code() {
        let root = temp_dir("helper-hash-mismatch");
        let mut command = helper_command(&root);
        command.asset_sha256 =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();

        let exit_code = execute(&command).expect_err("hash mismatch should fail");

        assert_eq!(exit_code, UpdateHelperExitCode::AssetHashMismatch);
    }

    #[test]
    fn returns_target_missing_exit_code() {
        let root = temp_dir("helper-target-missing");
        let mut command = helper_command(&root);
        command.target_path = root.join("missing-target.app");

        let exit_code = execute(&command).expect_err("missing target should fail");

        assert_eq!(exit_code, UpdateHelperExitCode::TargetMissing);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn runs_extensionless_asset_as_exe_for_nsis_install() {
        let root = temp_dir("helper-windows-installer-no-extension");
        let mut command = helper_command(&root);
        command.install_kind = InstallKind::WindowsNsis;
        command.asset_path = root.join("installer");
        fs::write(&command.asset_path, b"installer").expect("write installer");
        command.asset_size = fs::metadata(&command.asset_path)
            .expect("installer metadata")
            .len();
        command.asset_sha256 = sha256_hex(&command.asset_path).expect("hash installer");
        let mut log = open_log(&root.join("windows-installer.log")).expect("open log");

        let prepared =
            prepare_windows_installer_asset(&command, &mut log).expect("prepare fake installer");

        assert_eq!(
            prepared.installer_path,
            command.asset_path.with_extension("exe")
        );
        assert!(
            prepared.installer_path.exists(),
            "temporary .exe link should exist"
        );

        prepared.cleanup();

        assert!(
            !command.asset_path.with_extension("exe").exists(),
            "temporary .exe link should be cleaned up"
        );
    }

    #[test]
    fn compares_installed_windows_version_to_target_version() {
        assert!(installed_version_matches_target("1.0.5.0", "1.0.5"));
        assert!(installed_version_matches_target("1.0.5.17", "v1.0.5"));
        assert!(installed_version_matches_target("1.0.5.17", "1.0.5+stable"));
        assert!(!installed_version_matches_target("1.0.4.99", "1.0.5"));
        assert!(!installed_version_matches_target("1.0", "1.0.1"));
        assert!(!installed_version_matches_target("unknown", "1.0.5"));
    }

    #[test]
    fn maps_installer_version_mismatch_to_retry_install_error() {
        assert_eq!(
            install_error_code(UpdateHelperExitCode::InstallerVersionMismatch),
            "updateInstallVersionMismatch"
        );
        assert_eq!(
            install_error_action(UpdateHelperExitCode::InstallerVersionMismatch),
            "retryInstall"
        );
    }

    #[test]
    fn maps_nsis_cancel_exit_to_cancelled() {
        assert_eq!(
            map_installer_exit(WindowsInstallerFamily::Nsis, Some(1)),
            UpdateHelperExitCode::InstallerCancelled
        );
        assert_eq!(
            map_installer_exit(WindowsInstallerFamily::Nsis, Some(2)),
            UpdateHelperExitCode::InstallerFailed
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn expected_process_name_uses_macos_bundle_executable() {
        let root = temp_dir("helper-expected-process-name");
        let bundle = root.join("花笺.app");
        let macos_dir = bundle.join("Contents").join("MacOS");
        fs::create_dir_all(&macos_dir).expect("create macos dir");
        fs::write(macos_dir.join("floral-notepaper"), b"binary").expect("write binary");

        assert_eq!(
            expected_process_name(&bundle).as_deref(),
            Some("floral-notepaper")
        );
        assert_eq!(
            expected_process_name(&root.join("floral-notepaper")).as_deref(),
            Some("floral-notepaper")
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn expected_process_name_falls_back_for_ambiguous_bundle() {
        let root = temp_dir("helper-ambiguous-process-name");
        let bundle = root.join("花笺.app");
        let macos_dir = bundle.join("Contents").join("MacOS");
        fs::create_dir_all(&macos_dir).expect("create macos dir");
        fs::write(macos_dir.join("floral-notepaper"), b"binary").expect("write binary");
        fs::write(macos_dir.join("helper"), b"binary").expect("write helper");

        assert_eq!(expected_process_name(&bundle).as_deref(), Some("花笺"));
    }

    #[test]
    fn completion_marker_uses_ready_marker_stem() {
        let root = temp_dir("helper-completion-marker");
        let command = helper_command(&root);

        assert_eq!(completion_marker_path(&command), root.join("helper.done"));
        assert_eq!(
            relaunch_marker_path(&command),
            root.join("helper.relaunching")
        );
    }

    #[test]
    fn watchdog_handoff_marker_does_not_mark_install_failed() {
        let root = temp_dir("helper-watchdog-relaunch-handoff");
        let mut command = helper_command(&root);
        command.mode = UpdateHelperMode::Watchdog;
        command.target_path = root.join("missing-target.app");
        let installing_state = UpdateStateDto {
            status: UpdateStatus::Installing,
            latest_version: Some(command.target_version.clone()),
            asset_path: Some(command.asset_path.to_string_lossy().to_string()),
            asset_sha256: Some(command.asset_sha256.clone()),
            asset_size: Some(command.asset_size),
            install_log_path: Some(command.log_path.to_string_lossy().to_string()),
            ..UpdateStateDto::idle_with_version(command.current_version.clone())
        };
        write_json_atomic(&command.state_path, &installing_state).expect("write state");
        let mut log = open_log(&command.log_path).expect("open log");
        write_relaunch_marker(&command, &mut log).expect("write relaunch marker");

        let action =
            watchdog_post_exit_action(&command, &mut log).expect("resolve watchdog action");

        let saved_state: UpdateStateDto =
            serde_json::from_str(&fs::read_to_string(&command.state_path).expect("read state"))
                .expect("parse state");
        assert_eq!(action, WatchdogPostExitAction::Noop);
        assert_eq!(saved_state.status, UpdateStatus::Installing);
        assert!(saved_state.last_error.is_none());
    }

    #[test]
    fn watchdog_completion_marker_skips_failure_recovery() {
        let root = temp_dir("helper-watchdog-completion-marker");
        let mut command = helper_command(&root);
        command.mode = UpdateHelperMode::Watchdog;
        let state = UpdateStateDto {
            status: UpdateStatus::Failed,
            latest_version: Some(command.target_version.clone()),
            last_error: Some(UpdateErrorDto::recoverable(
                install_error_code(UpdateHelperExitCode::WaitTimedOut),
                install_error_message(UpdateHelperExitCode::WaitTimedOut),
                Some(install_error_action(UpdateHelperExitCode::WaitTimedOut).into()),
            )),
            ..UpdateStateDto::idle_with_version(command.current_version.clone())
        };
        write_json_atomic(&command.state_path, &state).expect("write state");
        let mut log = open_log(&command.log_path).expect("open log");
        write_completion_marker(&command, &mut log).expect("write completion marker");

        let action =
            watchdog_post_exit_action(&command, &mut log).expect("resolve watchdog action");

        let saved_state: UpdateStateDto =
            serde_json::from_str(&fs::read_to_string(&command.state_path).expect("read state"))
                .expect("parse state");
        assert_eq!(action, WatchdogPostExitAction::Noop);
        assert_eq!(
            saved_state
                .last_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("updateInstallWaitTimedOut")
        );
    }

    #[test]
    fn watchdog_propagates_state_write_failures() {
        let root = temp_dir("helper-watchdog-state-write-failed");
        let mut command = helper_command(&root);
        command.mode = UpdateHelperMode::Watchdog;
        command.wait_pid = u32::MAX;
        command.target_path = root.join("missing-target.app");
        command.state_path = root.join("state-directory");
        fs::create_dir_all(&command.state_path).expect("create state path directory");
        let mut log = open_log(&command.log_path).expect("open log");

        let error =
            execute_watchdog(&command, &mut log).expect_err("state write failure should surface");

        assert_eq!(error, UpdateHelperExitCode::StateWriteFailed);
    }

    #[test]
    fn failed_apply_preserves_downloaded_asset_directory() {
        let root = temp_dir("helper-failed-apply-preserves-download");
        let mut command = helper_command(&root);
        let download_dir = root.join("downloads").join("1.0.5");
        let asset_path = download_dir.join("asset.bin");
        fs::create_dir_all(&download_dir).expect("create download dir");
        fs::write(&asset_path, b"downloaded asset").expect("write downloaded asset");
        command.mode = UpdateHelperMode::Apply;
        command.install_kind = InstallKind::Unknown;
        command.wait_pid = u32::MAX;
        command.target_path = root.join("missing-target.app");
        command.asset_path = asset_path.clone();
        command.asset_size = fs::metadata(&asset_path).expect("asset metadata").len();
        command.asset_sha256 = sha256_hex(&asset_path).expect("hash asset");
        let mut log = open_log(&command.log_path).expect("open log");

        let error =
            execute_apply(&command, &mut log).expect_err("unknown install kind should fail");

        assert_eq!(error, UpdateHelperExitCode::UnsupportedInstallKind);
        assert!(download_dir.exists());
        assert!(asset_path.exists());
    }

    #[test]
    fn cleanup_after_install_reports_ready_marker_cleanup_failure() {
        let root = temp_dir("helper-cleanup-ready-marker-failed");
        let command = helper_command(&root);
        fs::create_dir_all(&command.ready_path).expect("create ready marker directory");
        let mut log = open_log(&command.log_path).expect("open log");

        let error =
            cleanup_after_install(&command, &mut log).expect_err("directory removal should fail");

        assert_eq!(error, UpdateHelperExitCode::CleanupFailed);
    }

    #[test]
    fn cleanup_after_install_removes_ready_marker_file() {
        let root = temp_dir("helper-cleanup-ready-marker");
        let command = helper_command(&root);
        fs::write(&command.ready_path, "ready").expect("write ready marker");
        let mut log = open_log(&command.log_path).expect("open log");

        cleanup_after_install(&command, &mut log).expect("cleanup ready marker");

        assert!(!command.ready_path.exists());
    }

    #[test]
    fn cleanup_applied_update_retains_macos_rollback_stage() {
        let root = temp_dir("helper-retain-rollback-stage");
        let stage_root = root.join(".floral-notepaper-update-stage-test");
        let backup_path = stage_root.join("backup.app");
        fs::create_dir_all(&backup_path).expect("create rollback backup");
        let applied_update = AppliedUpdate {
            launch_target: root.join("target.app"),
            rollback: Some(InstallRollbackPlan::Macos(MacosRollbackPlan {
                target_path: root.join("target.app"),
                backup_path,
                stage_root: stage_root.clone(),
            })),
        };
        let mut log = open_log(&root.join("cleanup.log")).expect("open log");

        cleanup_applied_update(&applied_update, &mut log).expect("cleanup applied update");

        assert!(stage_root.exists());
    }

    #[test]
    fn rollback_windows_update_restores_original_executable() {
        let root = temp_dir("helper-windows-rollback");
        let install_dir = root.join("install");
        let backup_dir = root.join("rollback-backup");
        let target_path = install_dir.join("floral-notepaper.exe");
        let extra_path = install_dir.join("new-file.dll");
        fs::create_dir_all(&install_dir).expect("create install dir");
        fs::create_dir_all(&backup_dir).expect("create backup dir");
        fs::write(&target_path, b"new version").expect("write target");
        fs::write(&extra_path, b"partial install").expect("write partial install file");
        fs::write(backup_dir.join("floral-notepaper.exe"), b"old version").expect("write backup");
        fs::write(backup_dir.join("stable.dll"), b"old dll").expect("write backup dll");
        let rollback = WindowsRollbackPlan {
            target_path: target_path.clone(),
            install_dir: install_dir.clone(),
            backup_dir: backup_dir.clone(),
            registry_backup: None,
        };
        let mut log = open_log(&root.join("rollback.log")).expect("open log");

        rollback_windows_update(&rollback, &mut log).expect("restore backup");

        assert_eq!(
            fs::read(&target_path).expect("read restored target"),
            b"old version"
        );
        assert_eq!(
            fs::read(install_dir.join("stable.dll")).expect("read restored dll"),
            b"old dll"
        );
        assert!(
            !extra_path.exists(),
            "partial install file should be removed"
        );
        assert!(
            backup_dir.exists(),
            "rollback backup should remain available"
        );
    }

    #[test]
    fn aggregates_windows_disk_space_requirements_on_same_volume() {
        let root = temp_dir("helper-disk-space-aggregate");
        let install_dir = root.join("install");
        let staging_dir = root.join("staging");
        fs::create_dir_all(&install_dir).expect("create install dir");
        fs::create_dir_all(&staging_dir).expect("create staging dir");
        let target_path = install_dir.join("floral-notepaper.exe");
        fs::write(&target_path, b"old version").expect("write target");
        fs::write(install_dir.join("stable.dll"), b"dll").expect("write install dll");
        let mut command = helper_command(&root);
        command.install_kind = InstallKind::WindowsNsis;
        command.target_path = target_path;
        command.ready_path = staging_dir.join("helper.ready");
        command.asset_size = 10;
        let mut log = open_log(&command.log_path).expect("open log");

        let requirements =
            aggregated_disk_space_requirements(&command, &mut log).expect("collect requirements");

        assert_eq!(requirements.len(), 1);
        let requirement = requirements.values().next().expect("single requirement");
        assert_eq!(requirement.required_bytes, 34);
        assert!(requirement.reasons.contains(&"installer workspace"));
        assert!(requirement.reasons.contains(&"Windows rollback backup"));
    }

    #[test]
    fn windows_rollback_storage_parent_moves_out_of_install_dir() {
        let root = temp_dir("helper-windows-rollback-parent");
        let install_dir = root.join("install");
        let staging_dir = install_dir.join("staging");
        fs::create_dir_all(&staging_dir).expect("create staging dir");
        let mut command = helper_command(&root);
        command.install_kind = InstallKind::WindowsNsis;
        command.target_path = install_dir.join("floral-notepaper.exe");
        command.ready_path = staging_dir.join("helper.ready");

        let backup_parent =
            windows_rollback_storage_parent(&command, &install_dir).expect("resolve backup dir");

        assert!(!backup_parent.starts_with(&install_dir));
        assert!(
            backup_parent.ends_with("floral-notepaper-update-rollback"),
            "fallback should use shared system temp rollback root"
        );
    }

    #[test]
    fn maps_disk_full_copy_errors_to_insufficient_space() {
        #[cfg(target_os = "windows")]
        let error = std::io::Error::from_raw_os_error(112);
        #[cfg(unix)]
        let error = std::io::Error::from_raw_os_error(libc::ENOSPC);
        #[cfg(not(any(unix, target_os = "windows")))]
        let error = std::io::Error::other("disk full");

        let code = map_copy_error_code(&error, UpdateHelperExitCode::ReplacementFailed);

        #[cfg(any(unix, target_os = "windows"))]
        assert_eq!(code, UpdateHelperExitCode::InsufficientSpace);
        #[cfg(not(any(unix, target_os = "windows")))]
        assert_eq!(code, UpdateHelperExitCode::ReplacementFailed);
    }

    #[test]
    fn parses_windows_registry_search_output_for_matching_install_key() {
        let output = r#"
HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall\OtherApp
    DisplayIcon    REG_SZ    C:\Program Files\Other App\other.exe,0

HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall\FloralNotepaper
    DisplayIcon    REG_SZ    C:\Program Files\Floral Notepaper\floral-notepaper.exe,0
    InstallLocation    REG_SZ    C:\Program Files\Floral Notepaper
"#;

        let key = parse_windows_install_registry_key_from_search_output(
            output,
            "floral-notepaper.exe",
            Some(Path::new(
                r"C:\Program Files\Floral Notepaper\floral-notepaper.exe",
            )),
        )
        .expect("resolve matching uninstall key");

        assert_eq!(
            key,
            r"HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall\FloralNotepaper"
        );
    }

    #[test]
    fn parses_windows_registry_record_for_launch_target_and_uninstall_commands() {
        let output = r#"
HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall\FloralNotepaper
    DisplayIcon    REG_SZ    C:\Program Files\Floral Notepaper\floral-notepaper.exe,0
    InstallLocation    REG_SZ    C:\Program Files\Floral Notepaper
    QuietUninstallString    REG_SZ    "C:\Program Files\Floral Notepaper\uninstall.exe" /S
    UninstallString    REG_SZ    "C:\Program Files\Floral Notepaper\uninstall.exe"
"#;

        let record = parse_windows_install_registry_record_output(
            output,
            "floral-notepaper.exe",
            Some(Path::new(
                r"C:\Program Files\Floral Notepaper\floral-notepaper.exe",
            )),
        )
        .expect("parse registry record");

        assert_eq!(
            record
                .launch_target
                .as_ref()
                .map(|path| normalize_windows_path(&path.to_string_lossy())),
            Some(normalize_windows_path(
                r"C:\Program Files\Floral Notepaper\floral-notepaper.exe"
            ))
        );
        assert_eq!(
            record.install_dir,
            Some(PathBuf::from(r"C:\Program Files\Floral Notepaper"))
        );
        assert_eq!(
            record.quiet_uninstall_command.as_deref(),
            Some(r#""C:\Program Files\Floral Notepaper\uninstall.exe" /S"#)
        );
        assert_eq!(
            record.uninstall_command.as_deref(),
            Some(r#""C:\Program Files\Floral Notepaper\uninstall.exe""#)
        );
    }

    #[test]
    fn builds_silent_windows_uninstall_command_from_registry_record() {
        let nsis_record = WindowsRegistryInstallRecord {
            key_path: "HKLM\\Software\\Floral".into(),
            launch_target: Some(PathBuf::from(
                r"C:\Program Files\Floral Notepaper\floral-notepaper.exe",
            )),
            install_dir: Some(PathBuf::from(r"C:\Program Files\Floral Notepaper")),
            quiet_uninstall_command: None,
            uninstall_command: Some(r#""C:\Program Files\Floral Notepaper\uninstall.exe""#.into()),
        };
        let msi_record = WindowsRegistryInstallRecord {
            key_path: "HKLM\\Software\\FloralMsi".into(),
            launch_target: Some(PathBuf::from(
                r"C:\Program Files\Floral Notepaper\floral-notepaper.exe",
            )),
            install_dir: Some(PathBuf::from(r"C:\Program Files\Floral Notepaper")),
            quiet_uninstall_command: None,
            uninstall_command: Some(
                r#"MsiExec.exe /I{ABCDEF12-3456-7890-ABCD-EF1234567890}"#.into(),
            ),
        };

        assert_eq!(
            build_silent_windows_uninstall_command(&nsis_record).as_deref(),
            Some(r#""C:\Program Files\Floral Notepaper\uninstall.exe" /S"#)
        );
        assert_eq!(
            build_silent_windows_uninstall_command(&msi_record).as_deref(),
            Some(r#"msiexec.exe /x {ABCDEF12-3456-7890-ABCD-EF1234567890} /qn /norestart"#)
        );
    }

    #[test]
    #[cfg(unix)]
    fn mounted_dmg_selection_prefers_root_app_bundle() {
        let root = temp_dir("helper-dmg-root-selection");
        let mounted_root = root.join("mounted");
        let fake_applications = root.join("Applications");
        let root_bundle = mounted_root.join("花笺.app");
        let stale_bundle = fake_applications
            .join(".floral-notepaper-update-stage-123")
            .join("花笺.app");

        fs::create_dir_all(&root_bundle).expect("create root bundle");
        fs::create_dir_all(&stale_bundle).expect("create stale bundle");
        fs::create_dir_all(&mounted_root).expect("create mounted root");
        symlink(&fake_applications, mounted_root.join("Applications")).expect("create symlink");

        let mut log = temp_log(&root, "dmg-select.log");
        let selected =
            select_app_bundle_from_mounted_dmg(&mounted_root, Some("花笺.app"), &mut log)
                .expect("select mounted bundle");

        assert_eq!(selected, root_bundle);
    }

    #[test]
    #[cfg(unix)]
    fn bundle_search_skips_symlink_dirs_and_update_stage_dirs() {
        let root = temp_dir("helper-find-bundles");
        let extracted_root = root.join("unzipped");
        let valid_bundle = extracted_root.join("Nested").join("花笺.app");
        let stage_bundle = extracted_root
            .join(".floral-notepaper-update-stage-123")
            .join("花笺.app");
        let symlink_target = root.join("linked");
        let symlink_bundle = symlink_target.join("花笺.app");

        fs::create_dir_all(valid_bundle.parent().expect("valid bundle parent"))
            .expect("create nested dir");
        fs::create_dir_all(&valid_bundle).expect("create valid bundle");
        fs::create_dir_all(&stage_bundle).expect("create stage bundle");
        fs::create_dir_all(&symlink_bundle).expect("create symlink bundle");
        symlink(&symlink_target, extracted_root.join("Applications")).expect("create symlink dir");

        let bundles = find_app_bundles(&extracted_root);

        assert_eq!(bundles, vec![valid_bundle]);
    }

    #[test]
    fn run_cli_maps_parse_errors() {
        let exit_code = run_cli(vec![OsString::from("--mode"), OsString::from("invalid")]);

        assert_eq!(exit_code, UpdateHelperExitCode::InvalidArguments);
    }
}
