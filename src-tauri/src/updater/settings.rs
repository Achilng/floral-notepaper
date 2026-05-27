use super::{types::*, UpdatePaths};
use crate::services::notes::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoredUpdateSettings {
    #[serde(default = "default_true")]
    pub auto_check: bool,
    #[serde(default)]
    pub auto_download: bool,
    #[serde(default = "default_check_interval_hours")]
    pub check_interval_hours: u32,
    #[serde(default)]
    pub check_source_preference: CheckSourcePreference,
    #[serde(default)]
    pub download_source_preference: DownloadSourcePreference,
    #[serde(default)]
    pub channel: UpdateChannel,
    #[serde(default)]
    pub allow_prerelease: bool,
    #[serde(default)]
    pub last_auto_check_at: Option<DateTime<Utc>>,
}

impl StoredUpdateSettings {
    pub fn from_dto(settings: UpdateSettingsDto) -> Self {
        Self {
            auto_check: settings.auto_check,
            auto_download: settings.auto_download,
            check_interval_hours: normalize_check_interval(settings.check_interval_hours),
            check_source_preference: settings.check_source_preference,
            download_source_preference: settings.download_source_preference,
            channel: settings.channel,
            allow_prerelease: settings.allow_prerelease,
            last_auto_check_at: settings.last_auto_check_at,
        }
    }

    pub fn into_dto(self, has_mirror_cdk: bool) -> UpdateSettingsDto {
        UpdateSettingsDto {
            auto_check: self.auto_check,
            auto_download: self.auto_download,
            check_interval_hours: self.check_interval_hours,
            check_source_preference: self.check_source_preference,
            download_source_preference: self.download_source_preference,
            channel: self.channel,
            allow_prerelease: self.allow_prerelease,
            last_auto_check_at: self.last_auto_check_at,
            has_mirror_cdk,
        }
    }
}

impl Default for StoredUpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_download: false,
            check_interval_hours: 24,
            check_source_preference: CheckSourcePreference::GithubFirst,
            download_source_preference: DownloadSourcePreference::MirrorFirst,
            channel: UpdateChannel::Stable,
            allow_prerelease: false,
            last_auto_check_at: None,
        }
    }
}

pub fn load(paths: &UpdatePaths) -> Result<StoredUpdateSettings, AppError> {
    paths.ensure_dirs()?;
    let path = paths.settings_path();
    if !path.exists() {
        let settings = StoredUpdateSettings::default();
        save(paths, &settings)?;
        return Ok(settings);
    }

    match serde_json::from_str::<StoredUpdateSettings>(&fs::read_to_string(&path)?) {
        Ok(mut settings) => {
            settings.check_interval_hours = normalize_check_interval(settings.check_interval_hours);
            Ok(settings)
        }
        Err(_error) => {
            rename_corrupt_file(&path, "settings")?;
            let settings = StoredUpdateSettings::default();
            save(paths, &settings)?;
            Ok(settings)
        }
    }
}

pub fn save(paths: &UpdatePaths, settings: &StoredUpdateSettings) -> Result<(), AppError> {
    paths.ensure_dirs()?;
    write_json_atomic(&paths.settings_path(), settings)
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = temporary_json_path(path);
    fs::write(&temp_path, serde_json::to_string_pretty(value)?)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

pub fn rename_corrupt_file(path: &Path, stem: &str) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let corrupt_name = format!("{stem}.corrupt-{timestamp}.json");
    let corrupt_path = path
        .parent()
        .map(|parent| parent.join(&corrupt_name))
        .unwrap_or_else(|| PathBuf::from(corrupt_name));
    fs::rename(path, corrupt_path)?;
    Ok(())
}

fn temporary_json_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    path.with_file_name(format!("{file_name}.tmp"))
}

fn normalize_check_interval(value: u32) -> u32 {
    value.clamp(1, 24 * 30)
}

fn default_true() -> bool {
    true
}

fn default_check_interval_hours() -> u32 {
    24
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn creates_default_settings_file() {
        let paths = test_paths("settings-default");

        let settings = load(&paths).expect("load default settings");

        assert!(settings.auto_check);
        assert!(!settings.auto_download);
        assert_eq!(settings.check_interval_hours, 24);
        assert_eq!(
            settings.check_source_preference,
            CheckSourcePreference::GithubFirst
        );
        assert_eq!(
            settings.download_source_preference,
            DownloadSourcePreference::MirrorFirst
        );
        assert!(paths.settings_path().exists());
    }

    #[test]
    fn saves_settings_without_has_cdk_field() {
        let paths = test_paths("settings-save");
        let settings = StoredUpdateSettings::from_dto(UpdateSettingsDto {
            auto_check: false,
            auto_download: true,
            check_interval_hours: 168,
            check_source_preference: CheckSourcePreference::GithubOnly,
            download_source_preference: DownloadSourcePreference::GithubOnly,
            channel: UpdateChannel::Beta,
            allow_prerelease: true,
            last_auto_check_at: None,
            has_mirror_cdk: true,
        });

        save(&paths, &settings).expect("save settings");
        let raw = fs::read_to_string(paths.settings_path()).expect("read settings file");
        let loaded = load(&paths).expect("load saved settings");

        assert!(!raw.contains("hasMirrorCdk"));
        assert_eq!(loaded, settings);
    }

    #[test]
    fn resets_corrupt_settings_file() {
        let paths = test_paths("settings-corrupt");
        paths.ensure_dirs().expect("create dirs");
        fs::write(paths.settings_path(), "{ broken").expect("write corrupt settings");

        let settings = load(&paths).expect("corrupt settings should recover");

        assert_eq!(settings, StoredUpdateSettings::default());
        assert!(paths.settings_path().exists());
        assert!(paths
            .root_dir()
            .read_dir()
            .expect("read updates dir")
            .any(|entry| entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with("settings.corrupt-")));
    }
}
