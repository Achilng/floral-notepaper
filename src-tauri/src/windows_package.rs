use crate::services::notes::AppError;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
pub(crate) const STARTUP_TASK_ID: &str = "FloralNotepaperStartup";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowsInstallContext {
    Msix,
    Unpackaged,
}

/// Returns the current Windows packaging context.
///
/// Package identity is the only discriminator. In particular, this deliberately
/// does not inspect `current_exe`, Program Files, or WindowsApps paths because
/// the same executable is shipped by MSIX, NSIS, and as a portable binary.
#[cfg(target_os = "windows")]
pub(crate) fn windows_install_context() -> WindowsInstallContext {
    static CONTEXT: OnceLock<WindowsInstallContext> = OnceLock::new();
    *CONTEXT.get_or_init(|| {
        if detect_package_identity() {
            WindowsInstallContext::Msix
        } else {
            WindowsInstallContext::Unpackaged
        }
    })
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn windows_install_context() -> WindowsInstallContext {
    WindowsInstallContext::Unpackaged
}

pub(crate) fn has_package_identity() -> bool {
    windows_install_context() == WindowsInstallContext::Msix
}

/// Determines whether the process has package identity without reading or
/// mutating registry, environment, or file-system state.
#[cfg(target_os = "windows")]
fn detect_package_identity() -> bool {
    use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;

    const ERROR_SUCCESS: u32 = 0;
    const APPMODEL_ERROR_NO_PACKAGE: u32 = 15_700;

    let mut length = 0;
    let status = unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) };
    match status {
        ERROR_SUCCESS => true,
        APPMODEL_ERROR_NO_PACKAGE => false,
        // ERROR_INSUFFICIENT_BUFFER is the normal packaged result for a null
        // buffer. Treat all other unexpected results conservatively as
        // packaged as well, so a detection failure can never route an MSIX
        // process into the unpackaged AppData/registry backends.
        _ => true,
    }
}

/// Resolves the package-owned LocalState directory. Callers must first have a
/// package identity; failure is returned instead of falling back to `%APPDATA%`.
#[cfg(target_os = "windows")]
pub(crate) fn msix_local_state_dir() -> Result<PathBuf, AppError> {
    use windows::Storage::ApplicationData;

    if windows_install_context() != WindowsInstallContext::Msix {
        return Err(msix_local_state_error(
            "packageIdentity",
            "LocalState was requested by an unpackaged process",
        ));
    }

    let application_data = ApplicationData::Current()
        .map_err(|error| msix_local_state_error("ApplicationData.Current", error))?;
    let local_folder = application_data
        .LocalFolder()
        .map_err(|error| msix_local_state_error("ApplicationData.LocalFolder", error))?;
    let path = local_folder
        .Path()
        .map_err(|error| msix_local_state_error("StorageFolder.Path", error))?;
    Ok(PathBuf::from(path.to_os_string()))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn msix_local_state_dir() -> Result<PathBuf, AppError> {
    Err(msix_local_state_error(
        "platform",
        "MSIX LocalState is only available on Windows",
    ))
}

fn msix_local_state_error(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    AppError::new(
        "msixLocalStateUnavailable",
        format!("MSIX LocalState is unavailable: {error}"),
    )
    .with_detail("operation", operation)
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartupTaskStatus {
    Disabled,
    DisabledByUser,
    Enabled,
    DisabledByPolicy,
    EnabledByPolicy,
}

#[cfg(any(target_os = "windows", test))]
impl StartupTaskStatus {
    pub(crate) fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled | Self::EnabledByPolicy)
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupTaskAction {
    None,
    RequestEnable,
    Disable,
}

#[cfg(any(target_os = "windows", test))]
fn startup_task_action(
    status: StartupTaskStatus,
    requested_enabled: bool,
) -> Result<StartupTaskAction, AppError> {
    match status {
        StartupTaskStatus::DisabledByUser => Err(AppError::new(
            "autostartDisabledByUser",
            "Startup was disabled by the user in Windows settings",
        )),
        StartupTaskStatus::DisabledByPolicy | StartupTaskStatus::EnabledByPolicy => {
            Err(AppError::new(
                "autostartManagedByPolicy",
                "Startup is managed by Windows policy",
            ))
        }
        StartupTaskStatus::Disabled if requested_enabled => Ok(StartupTaskAction::RequestEnable),
        StartupTaskStatus::Enabled if !requested_enabled => Ok(StartupTaskAction::Disable),
        StartupTaskStatus::Disabled | StartupTaskStatus::Enabled => Ok(StartupTaskAction::None),
    }
}

#[cfg(target_os = "windows")]
fn native_startup_task() -> Result<windows::ApplicationModel::StartupTask, AppError> {
    use windows::{core::HSTRING, ApplicationModel::StartupTask};

    if windows_install_context() != WindowsInstallContext::Msix {
        return Err(msix_autostart_error(
            "packageIdentity",
            "StartupTask was requested by an unpackaged process",
        ));
    }

    StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))
        .and_then(|operation| operation.get())
        .map_err(|error| msix_autostart_error("StartupTask.GetAsync", error))
}

#[cfg(target_os = "windows")]
fn native_startup_task_status(
    task: &windows::ApplicationModel::StartupTask,
) -> Result<StartupTaskStatus, AppError> {
    use windows::ApplicationModel::StartupTaskState;

    let state = task
        .State()
        .map_err(|error| msix_autostart_error("StartupTask.State", error))?;
    match state {
        StartupTaskState::Disabled => Ok(StartupTaskStatus::Disabled),
        StartupTaskState::DisabledByUser => Ok(StartupTaskStatus::DisabledByUser),
        StartupTaskState::Enabled => Ok(StartupTaskStatus::Enabled),
        StartupTaskState::DisabledByPolicy => Ok(StartupTaskStatus::DisabledByPolicy),
        StartupTaskState::EnabledByPolicy => Ok(StartupTaskStatus::EnabledByPolicy),
        _ => Err(msix_autostart_error(
            "StartupTask.State",
            format!("unknown StartupTaskState value {}", state.0),
        )),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn msix_autostart_enabled() -> Result<bool, AppError> {
    let task = native_startup_task()?;
    Ok(native_startup_task_status(&task)?.is_enabled())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn msix_autostart_enabled() -> Result<bool, AppError> {
    Err(msix_autostart_error(
        "platform",
        "MSIX StartupTask is only available on Windows",
    ))
}

#[cfg(target_os = "windows")]
pub(crate) fn set_msix_autostart(requested_enabled: bool) -> Result<bool, AppError> {
    let task = native_startup_task()?;
    let initial_status = native_startup_task_status(&task)?;

    match startup_task_action(initial_status, requested_enabled)? {
        StartupTaskAction::None => {}
        StartupTaskAction::RequestEnable => {
            task.RequestEnableAsync()
                .and_then(|operation| operation.get())
                .map_err(|error| msix_autostart_error("StartupTask.RequestEnableAsync", error))?;
        }
        StartupTaskAction::Disable => task
            .Disable()
            .map_err(|error| msix_autostart_error("StartupTask.Disable", error))?,
    }

    let final_status = native_startup_task_status(&task)?;
    let actual_enabled = final_status.is_enabled();
    if actual_enabled == requested_enabled {
        return Ok(actual_enabled);
    }

    // RequestEnableAsync may complete with a user/policy-controlled state.
    // Preserve those actionable error codes instead of reporting success for
    // a request that Windows did not apply.
    startup_task_action(final_status, requested_enabled)?;
    Err(msix_autostart_error(
        "StartupTask.State",
        format!(
            "Windows kept StartupTask in {final_status:?} after requesting enabled={requested_enabled}"
        ),
    ))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn set_msix_autostart(_requested_enabled: bool) -> Result<bool, AppError> {
    Err(msix_autostart_error(
        "platform",
        "MSIX StartupTask is only available on Windows",
    ))
}

fn msix_autostart_error(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    AppError::new(
        "msixAutostartUnavailable",
        format!("MSIX startup setting is unavailable: {error}"),
    )
    .with_detail("operation", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_startup_task_states_to_enabled_status() {
        assert!(!StartupTaskStatus::Disabled.is_enabled());
        assert!(!StartupTaskStatus::DisabledByUser.is_enabled());
        assert!(StartupTaskStatus::Enabled.is_enabled());
        assert!(!StartupTaskStatus::DisabledByPolicy.is_enabled());
        assert!(StartupTaskStatus::EnabledByPolicy.is_enabled());
    }

    #[test]
    fn only_plain_disabled_and_enabled_states_are_mutable() {
        assert_eq!(
            startup_task_action(StartupTaskStatus::Disabled, true).unwrap(),
            StartupTaskAction::RequestEnable
        );
        assert_eq!(
            startup_task_action(StartupTaskStatus::Enabled, false).unwrap(),
            StartupTaskAction::Disable
        );
        assert_eq!(
            startup_task_action(StartupTaskStatus::Disabled, false).unwrap(),
            StartupTaskAction::None
        );
        assert_eq!(
            startup_task_action(StartupTaskStatus::Enabled, true).unwrap(),
            StartupTaskAction::None
        );
    }

    #[test]
    fn reports_user_and_policy_controlled_states() {
        assert_eq!(
            startup_task_action(StartupTaskStatus::DisabledByUser, true)
                .unwrap_err()
                .code,
            "autostartDisabledByUser"
        );
        for status in [
            StartupTaskStatus::DisabledByPolicy,
            StartupTaskStatus::EnabledByPolicy,
        ] {
            assert_eq!(
                startup_task_action(status, !status.is_enabled())
                    .unwrap_err()
                    .code,
                "autostartManagedByPolicy"
            );
        }
    }
}
