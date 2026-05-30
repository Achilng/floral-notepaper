use super::{
    settings::write_json_atomic,
    types::{InstallKind, UpdateErrorDto, UpdateInstallMode, UpdateStateDto, UpdateStatus},
    UpdatePaths,
};
use crate::services::notes::AppError;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "windows")]
pub const HELPER_BINARY_NAME: &str = "floral-notepaper-update-helper.exe";
#[cfg(not(target_os = "windows"))]
pub const HELPER_BINARY_NAME: &str = "floral-notepaper-update-helper";

const WAIT_FOR_EXIT_TIMEOUT: Duration = Duration::from_secs(30);
const WAIT_FOR_REPLACEMENT_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const UPDATE_STAGE_PREFIX: &str = ".floral-notepaper-update-stage-";
#[cfg(target_os = "macos")]
const MOUNT_STAGE_PREFIX: &str = ".floral-notepaper-mounted-dmg-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateHelperMode {
    Apply,
    Test,
}

impl UpdateHelperMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Test => "test",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "apply" => Some(Self::Apply),
            "test" => Some(Self::Test),
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
    match wait_for_process_exit(command.wait_pid, log) {
        Ok(()) => {}
        Err(UpdateHelperExitCode::WaitTimedOut) => {
            write_log_line(
                log,
                "main process did not exit within timeout; proceeding with install anyway",
            )?;
        }
        Err(code) => return Err(code),
    }
    persist_installing_state(command, log)?;

    let launch_target = match apply_update(command, log) {
        Ok(path) => path,
        Err(code) => {
            let _ = persist_failed_state(command, code, log);
            let _ = cleanup_after_install(command, log, should_remove_download_dir(code));
            let _ = relaunch_existing_target(command, log);
            return Err(code);
        }
    };

    persist_pending_verification_state(command, log)?;
    cleanup_after_install(command, log, false)?;
    relaunch_target(&launch_target, log)
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
                    "asset missing: failed to read {} ({error})",
                    command.asset_path.display()
                ),
            )?;
            return Err(UpdateHelperExitCode::AssetMissing);
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
    fs::write(
        &command.ready_path,
        format!("ready {}\n", Utc::now().to_rfc3339()),
    )
    .map_err(|_| UpdateHelperExitCode::StateWriteFailed)?;
    write_log_line(
        log,
        &format!("wrote helper ready marker {}", command.ready_path.display()),
    )?;
    Ok(())
}

fn ensure_sufficient_disk_space(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let Some(target_dir) = command.target_path.parent() else {
        return Ok(());
    };
    let Some(available_bytes) = available_disk_space(target_dir) else {
        return Ok(());
    };

    let required_bytes = command.asset_size.saturating_mul(2);
    if available_bytes < required_bytes {
        write_log_line(
            log,
            &format!(
                "insufficient disk space: required {required_bytes} bytes, available {available_bytes} bytes"
            ),
        )?;
        return Err(UpdateHelperExitCode::InsufficientSpace);
    }

    Ok(())
}

fn apply_update(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
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
) -> Result<PathBuf, UpdateHelperExitCode> {
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
        Ok(())
    })();
    cleanup_stage_root(&stage_root, log)?;
    result?;
    write_log_line(
        log,
        &format!(
            "replaced macOS app bundle with version {}",
            command.target_version
        ),
    )?;
    Ok(command.target_path.clone())
}

fn install_windows_portable(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    write_log_line(log, "windows portable install is manual-only")?;
    let _ = command;
    Err(UpdateHelperExitCode::UnsupportedInstallKind)
}

fn install_windows_installer(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<PathBuf, UpdateHelperExitCode> {
    let extension = command
        .asset_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if extension == "msi" {
        write_log_line(log, "launching Windows MSI installer")?;
        let mut child = Command::new("msiexec.exe")
            .args([
                "/i",
                &command.asset_path.to_string_lossy(),
                "/passive",
                "/norestart",
            ])
            .spawn()
            .map_err(|_| UpdateHelperExitCode::InstallerFailed)?;

        let status = wait_for_installer_completion(&mut child, log)?;

        if !status.success() {
            write_log_line(
                log,
                &format!("installer exited with status {:?}", status.code()),
            )?;
            return Err(map_installer_exit(status.code()));
        }
    } else {
        write_log_line(log, "launching Windows NSIS installer")?;
        let exit_code = shell_execute_installer(&command.asset_path, "/S", log)?;
        if exit_code != 0 {
            write_log_line(log, &format!("installer exited with code {exit_code}"))?;
            return Err(map_installer_exit(Some(exit_code)));
        }
    }

    let launch_target = resolve_windows_launch_target(&command.target_path, log);
    wait_for_target_to_exist(&launch_target, log)?;
    write_log_line(
        log,
        &format!("installer completed for version {}", command.target_version),
    )?;
    Ok(launch_target)
}

#[cfg(target_os = "windows")]
fn shell_execute_installer(
    path: &Path,
    args: &str,
    log: &mut File,
) -> Result<i32, UpdateHelperExitCode> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
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

    if wait_result != WAIT_OBJECT_0 {
        unsafe { CloseHandle(handle) };
        write_log_line(log, "installer timed out before completion")?;
        return Err(UpdateHelperExitCode::InstallerTimedOut);
    }

    let mut exit_code: u32 = 0;
    unsafe {
        GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
    }

    Ok(exit_code as i32)
}

#[cfg(not(target_os = "windows"))]
fn shell_execute_installer(
    _path: &Path,
    _args: &str,
    _log: &mut File,
) -> Result<i32, UpdateHelperExitCode> {
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
        .map_err(|_| UpdateHelperExitCode::AssetExtractFailed)?;

    if !attach_status.success() {
        write_log_line(
            log,
            &format!(
                "hdiutil attach exited with status {:?}",
                attach_status.code()
            ),
        )?;
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

    let _ = Command::new("/usr/bin/hdiutil")
        .args(["detach"])
        .arg(&mount_point)
        .status();
    let _ = fs::remove_dir_all(&mount_point);
    result
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

fn map_installer_exit(code: Option<i32>) -> UpdateHelperExitCode {
    match code {
        Some(1602) => UpdateHelperExitCode::InstallerCancelled,
        Some(1603) => UpdateHelperExitCode::InstallerFatal,
        Some(1618) => UpdateHelperExitCode::InstallerBusy,
        _ => UpdateHelperExitCode::InstallerFailed,
    }
}

fn wait_for_process_exit(pid: u32, log: &mut File) -> Result<(), UpdateHelperExitCode> {
    let deadline = Instant::now() + WAIT_FOR_EXIT_TIMEOUT;
    while Instant::now() < deadline {
        if !process_is_running(pid) {
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
fn resolve_windows_launch_target(target_path: &Path, log: &mut File) -> PathBuf {
    if target_path.exists() {
        return target_path.to_path_buf();
    }

    let Some(exe_name) = target_path.file_name().and_then(|value| value.to_str()) else {
        return target_path.to_path_buf();
    };
    let resolved = query_windows_install_target_from_registry(exe_name);
    if let Some(path) = resolved.as_ref() {
        let _ = write_log_line(
            log,
            &format!(
                "resolved Windows install target from registry: {}",
                path.display()
            ),
        );
    }
    resolved.unwrap_or_else(|| target_path.to_path_buf())
}

#[cfg(not(target_os = "windows"))]
fn resolve_windows_launch_target(target_path: &Path, _log: &mut File) -> PathBuf {
    target_path.to_path_buf()
}

#[cfg(target_os = "windows")]
fn query_windows_install_target_from_registry(exe_name: &str) -> Option<PathBuf> {
    const ROOTS: [&str; 4] = [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    ROOTS.iter().find_map(|root| {
        Command::new("reg")
            .args(["query", root, "/s", "/f", exe_name])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|output| parse_windows_install_target_from_registry_output(&output, exe_name))
    })
}

#[cfg(target_os = "windows")]
fn parse_windows_install_target_from_registry_output(
    output: &str,
    exe_name: &str,
) -> Option<PathBuf> {
    for line in output.lines() {
        let normalized = line.trim();
        if normalized.is_empty() {
            continue;
        }
        if let Some(value) = normalized
            .split("REG_SZ")
            .nth(1)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let candidate = PathBuf::from(value.trim_matches('"'));
            if candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(exe_name))
                && candidate.exists()
            {
                return Some(candidate);
            }
            if candidate.exists() && candidate.is_dir() {
                let joined = candidate.join(exe_name);
                if joined.exists() {
                    return Some(joined);
                }
            }
        }
    }
    None
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
fn process_is_running(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let wait_result = WaitForSingleObject(handle, 0);
        let _ = CloseHandle(handle);
        wait_result != WAIT_OBJECT_0
    }
}

#[cfg(not(target_os = "windows"))]
fn process_is_running(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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
    let mut state = load_state_snapshot(command);
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

    write_state_snapshot(&command.state_path, &state).map_err(|_| {
        let _ = write_log_line(
            log,
            &format!(
                "failed to persist pending verification state to {}",
                command.state_path.display()
            ),
        );
        UpdateHelperExitCode::StateWriteFailed
    })?;
    write_log_line(log, "persisted install state pending relaunch verification")?;
    Ok(())
}

fn persist_installing_state(
    command: &UpdateHelperCommand,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let mut state = load_state_snapshot(command);
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

    write_state_snapshot(&command.state_path, &state).map_err(|_| {
        let _ = write_log_line(
            log,
            &format!(
                "failed to persist installing state to {}",
                command.state_path.display()
            ),
        );
        UpdateHelperExitCode::StateWriteFailed
    })?;
    write_log_line(log, "persisted installing state")?;
    Ok(())
}

fn persist_failed_state(
    command: &UpdateHelperCommand,
    code: UpdateHelperExitCode,
    log: &mut File,
) -> Result<(), UpdateHelperExitCode> {
    let mut state = load_state_snapshot(command);
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

    write_state_snapshot(&command.state_path, &state).map_err(|_| {
        let _ = write_log_line(
            log,
            &format!(
                "failed to persist failed install state to {}",
                command.state_path.display()
            ),
        );
        UpdateHelperExitCode::StateWriteFailed
    })?;
    write_log_line(log, "persisted failed install state")?;
    Ok(())
}

fn cleanup_after_install(
    command: &UpdateHelperCommand,
    log: &mut File,
    remove_download_dir: bool,
) -> Result<(), UpdateHelperExitCode> {
    if command.ready_path.exists() {
        let _ = fs::remove_file(&command.ready_path);
    }
    if remove_download_dir {
        if let Some(download_dir) = command.asset_path.parent() {
            if download_dir.exists() {
                let _ = fs::remove_dir_all(download_dir);
                let _ = write_log_line(
                    log,
                    &format!(
                        "removed downloaded asset directory {}",
                        download_dir.display()
                    ),
                );
            }
        }
    }
    Ok(())
}

fn should_remove_download_dir(code: UpdateHelperExitCode) -> bool {
    matches!(
        code,
        UpdateHelperExitCode::AssetMissing
            | UpdateHelperExitCode::AssetSizeMismatch
            | UpdateHelperExitCode::AssetHashMismatch
            | UpdateHelperExitCode::AssetExtractFailed
    )
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

fn load_state_snapshot(command: &UpdateHelperCommand) -> UpdateStateDto {
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

fn write_state_snapshot(path: &Path, state: &UpdateStateDto) -> Result<(), std::io::Error> {
    write_json_atomic(path, state).map_err(|error| std::io::Error::other(error.message))
}

fn install_error_code(code: UpdateHelperExitCode) -> &'static str {
    match code {
        UpdateHelperExitCode::InvalidArguments => "updateInstallHelperInvalidArguments",
        UpdateHelperExitCode::AssetMissing => "updateInstallAssetMissing",
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
        UpdateHelperExitCode::Success => "updateInstallHelperFailed",
    }
}

fn install_error_message(code: UpdateHelperExitCode) -> &'static str {
    match code {
        UpdateHelperExitCode::InvalidArguments => "更新安装助手参数无效",
        UpdateHelperExitCode::AssetMissing => "更新包文件不存在或无法读取",
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
        UpdateHelperExitCode::Success => "更新安装助手执行失败",
    }
}

fn install_error_action(code: UpdateHelperExitCode) -> &'static str {
    match code {
        UpdateHelperExitCode::AssetMissing
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
    let file = File::open(path)?;
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
        hex.push(nibble_to_hex(byte >> 4));
        hex.push(nibble_to_hex(byte & 0x0f));
    }
    Ok(hex)
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn cleanup_stale_macos_mounts(_paths: &UpdatePaths) -> Result<(), AppError> {
    let temp_dir = std::env::temp_dir();
    let entries = match fs::read_dir(&temp_dir) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(AppError {
                code: "updateInstallCleanupFailed".into(),
                message: error.to_string(),
                details: Default::default(),
            });
        }
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

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn cleanup_stale_macos_mounts(_paths: &UpdatePaths) -> Result<(), AppError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(format!("{name}-{unique}"));
        fs::create_dir_all(&root).expect("create temp dir");
        root
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
