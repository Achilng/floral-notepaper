use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CheckSourcePreference {
    #[default]
    MirrorChyanFirst,
    GithubFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum DownloadSourcePreference {
    #[default]
    MirrorChyanFirst,
    GithubFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadSourceUsed {
    MirrorChyan,
    Github,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Available,
    Downloading,
    Downloaded,
    Installing,
    InstallScheduled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateInstallMode {
    Apply,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateCheckStatus {
    NotAvailable,
    Available,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InstallKind {
    WindowsNsis,
    WindowsPortable,
    MacosAppBundle,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsDto {
    pub auto_check: bool,
    pub auto_download: bool,
    pub check_interval_hours: u32,
    pub check_source_preference: CheckSourcePreference,
    pub download_source_preference: DownloadSourcePreference,
    pub channel: UpdateChannel,
    pub allow_prerelease: bool,
    pub last_auto_check_at: Option<DateTime<Utc>>,
    pub has_mirror_chyan_cdk: bool,
    pub mirror_chyan_cdk_length: Option<u32>,
}

impl Default for UpdateSettingsDto {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_download: false,
            check_interval_hours: 24,
            check_source_preference: CheckSourcePreference::default(),
            download_source_preference: DownloadSourcePreference::default(),
            channel: UpdateChannel::default(),
            allow_prerelease: false,
            last_auto_check_at: None,
            has_mirror_chyan_cdk: false,
            mirror_chyan_cdk_length: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateErrorDto {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub action: Option<String>,
}

impl UpdateErrorDto {
    pub fn recoverable(
        code: impl Into<String>,
        message: impl Into<String>,
        action: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable: true,
            action,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStateDto {
    pub status: UpdateStatus,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub channel: UpdateChannel,
    pub asset_name: Option<String>,
    pub asset_path: Option<String>,
    pub asset_sha256: Option<String>,
    pub asset_size: Option<u64>,
    pub asset_url: Option<String>,
    pub source: Option<DownloadSourceUsed>,
    pub checked_at: Option<DateTime<Utc>>,
    pub downloaded_at: Option<DateTime<Utc>>,
    pub install_log_path: Option<String>,
    pub install_mode: Option<UpdateInstallMode>,
    pub install_started_at: Option<DateTime<Utc>>,
    pub install_scheduled_at: Option<DateTime<Utc>>,
    pub last_error: Option<UpdateErrorDto>,
}

impl UpdateStateDto {
    #[cfg(test)]
    pub fn idle() -> Self {
        Self::idle_with_version(super::version::CURRENT_APP_VERSION)
    }

    pub fn idle_with_version(current_version: impl Into<String>) -> Self {
        Self {
            status: UpdateStatus::Idle,
            current_version: current_version.into(),
            latest_version: None,
            channel: UpdateChannel::Stable,
            asset_name: None,
            asset_path: None,
            asset_sha256: None,
            asset_size: None,
            asset_url: None,
            source: None,
            checked_at: None,
            downloaded_at: None,
            install_log_path: None,
            install_mode: None,
            install_started_at: None,
            install_scheduled_at: None,
            last_error: None,
        }
    }

    pub fn failed_with_version(current_version: impl Into<String>, error: UpdateErrorDto) -> Self {
        Self {
            status: UpdateStatus::Failed,
            checked_at: Some(Utc::now()),
            last_error: Some(error),
            ..Self::idle_with_version(current_version)
        }
    }

    #[cfg(test)]
    pub fn failed(error: UpdateErrorDto) -> Self {
        Self::failed_with_version(super::version::CURRENT_APP_VERSION, error)
    }
}

#[cfg(test)]
impl Default for UpdateStateDto {
    fn default() -> Self {
        Self::idle()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub status: UpdateCheckStatus,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub mandatory: bool,
    pub can_download_from_mirror_chyan: bool,
    pub can_download_from_github: bool,
    pub recommended_source: Option<DownloadSourceUsed>,
    pub asset_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadResult {
    pub status: UpdateStatus,
    pub version: Option<String>,
    pub asset_path: Option<String>,
    pub source: Option<DownloadSourceUsed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstallResult {
    pub status: UpdateStatus,
    pub log_path: Option<String>,
    pub mode: UpdateInstallMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgressDto {
    pub version: String,
    pub asset_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f64>,
    pub bytes_per_second: u64,
    pub source: DownloadSourceUsed,
}
