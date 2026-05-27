use super::{
    errors, manifest,
    platform::{self, PlatformInfo},
    settings::{self, StoredUpdateSettings},
    state,
    types::{
        CheckSourcePreference, DownloadSourcePreference, DownloadSourceUsed, UpdateCheckResult,
        UpdateCheckStatus, UpdateErrorDto, UpdateStateDto, UpdateStatus,
    },
    version, UpdatePaths,
};
use crate::services::notes::AppError;
use chrono::Utc;
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

const MIRROR_MANIFEST_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_MIRROR_MANIFEST_PATH";
const GITHUB_MANIFEST_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_GITHUB_MANIFEST_PATH";
const GITHUB_REPO_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_GITHUB_REPO";
const DEFAULT_GITHUB_REPO: &str = "Achilng/floral-notepaper";

#[derive(Debug, Clone)]
struct UpdateCheckContext {
    platform: PlatformInfo,
    current_version: Version,
    allow_prerelease: bool,
}

impl UpdateCheckContext {
    fn current_version_text(&self) -> String {
        self.current_version.to_string()
    }
}

#[derive(Debug, Clone)]
struct UpdateCandidate {
    priority: usize,
    version: String,
    normalized_version: Version,
    release_notes: Option<String>,
    mandatory: bool,
    asset_name: String,
    asset_sha256: Option<String>,
    asset_size: u64,
    asset_url: Option<String>,
    can_download_from_mirror: bool,
    can_download_from_github: bool,
}

#[derive(Debug, Clone)]
enum ProviderCheck {
    NotAvailable,
    Available(UpdateCandidate),
}

trait UpdateCheckProvider {
    fn label(&self) -> &'static str;
    fn check(
        &self,
        context: &UpdateCheckContext,
        priority: usize,
    ) -> Result<ProviderCheck, AppError>;
}

#[derive(Debug, Clone, Default)]
struct MirrorProvider {
    manifest_path: Option<PathBuf>,
}

impl MirrorProvider {
    pub fn from_env() -> Self {
        Self {
            manifest_path: env_manifest_path(MIRROR_MANIFEST_PATH_ENV),
        }
    }

    #[cfg(test)]
    fn with_manifest_path(path: PathBuf) -> Self {
        Self {
            manifest_path: Some(path),
        }
    }
}

impl UpdateCheckProvider for MirrorProvider {
    fn label(&self) -> &'static str {
        "Mirror"
    }

    fn check(
        &self,
        context: &UpdateCheckContext,
        priority: usize,
    ) -> Result<ProviderCheck, AppError> {
        let manifest_path = self
            .manifest_path
            .as_deref()
            .ok_or_else(|| errors::provider_not_configured(self.label()))?;
        load_manifest_candidate(self.label(), manifest_path, context, priority, false, true)
    }
}

#[derive(Debug, Clone, Default)]
struct GithubProvider {
    manifest_path: Option<PathBuf>,
    offline: bool,
}

impl GithubProvider {
    pub fn from_env() -> Self {
        Self {
            manifest_path: env_manifest_path(GITHUB_MANIFEST_PATH_ENV),
            offline: env::var("FLORAL_NOTEPAPER_UPDATE_OFFLINE").is_ok(),
        }
    }

    #[cfg(test)]
    fn with_manifest_path(path: PathBuf) -> Self {
        Self {
            manifest_path: Some(path),
            offline: false,
        }
    }

    #[cfg(test)]
    fn offline() -> Self {
        Self {
            manifest_path: None,
            offline: true,
        }
    }
}

impl UpdateCheckProvider for GithubProvider {
    fn label(&self) -> &'static str {
        "GitHub"
    }

    fn check(
        &self,
        context: &UpdateCheckContext,
        priority: usize,
    ) -> Result<ProviderCheck, AppError> {
        if let Some(manifest_path) = &self.manifest_path {
            return load_manifest_candidate(
                self.label(),
                manifest_path,
                context,
                priority,
                false,
                true,
            );
        }

        if self.offline {
            return Err(errors::provider_not_configured(self.label()));
        }

        check_github_api(context, priority)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UpdateCheckService {
    mirror: MirrorProvider,
    github: GithubProvider,
}

impl UpdateCheckService {
    pub(crate) fn from_env() -> Self {
        Self {
            mirror: MirrorProvider::from_env(),
            github: GithubProvider::from_env(),
        }
    }

    pub(crate) fn run(
        &self,
        paths: &UpdatePaths,
        manual: bool,
    ) -> Result<UpdateCheckResult, AppError> {
        let settings = settings::load(paths)?;
        let context = UpdateCheckContext {
            platform: platform::current_platform(),
            current_version: version::current_version()?,
            allow_prerelease: version::allows_prerelease(
                &settings.channel,
                settings.allow_prerelease,
            ),
        };

        let outcome = self.evaluate(&settings, &context);
        match outcome {
            Ok((result, next_state)) => {
                if !manual {
                    persist_last_auto_check_at(paths, &settings)?;
                }
                state::save(paths, &next_state)?;
                Ok(result)
            }
            Err(error) => {
                if !manual {
                    persist_last_auto_check_at(paths, &settings)?;
                }
                state::save(paths, &failed_state(&context, &settings, &error))?;
                Err(error)
            }
        }
    }

    #[cfg(test)]
    fn with_providers(mirror: MirrorProvider, github: GithubProvider) -> Self {
        Self { mirror, github }
    }

    fn evaluate(
        &self,
        settings: &StoredUpdateSettings,
        context: &UpdateCheckContext,
    ) -> Result<(UpdateCheckResult, UpdateStateDto), AppError> {
        let provider_order = check_provider_order(&settings.check_source_preference);
        let mut available = Vec::new();
        let mut saw_not_available = false;
        let mut provider_errors = Vec::new();

        for (priority, source) in provider_order.into_iter().enumerate() {
            let provider_result = match source {
                DownloadSourceUsed::Mirror => self.mirror.check(context, priority),
                DownloadSourceUsed::Github => self.github.check(context, priority),
            };

            match provider_result {
                Ok(ProviderCheck::Available(candidate)) => available.push(candidate),
                Ok(ProviderCheck::NotAvailable) => saw_not_available = true,
                Err(error) => provider_errors.push(error),
            }
        }

        if let Some(candidate) = merge_candidates(available) {
            let recommended_source = recommended_source(
                &settings.download_source_preference,
                candidate.can_download_from_mirror,
                candidate.can_download_from_github,
            );
            let result = UpdateCheckResult {
                status: UpdateCheckStatus::Available,
                current_version: context.current_version_text(),
                latest_version: Some(candidate.version.clone()),
                release_notes: candidate.release_notes.clone(),
                mandatory: candidate.mandatory,
                can_download_from_mirror: candidate.can_download_from_mirror,
                can_download_from_github: candidate.can_download_from_github,
                recommended_source: recommended_source.clone(),
                asset_url: candidate.asset_url.clone(),
            };
            let next_state = UpdateStateDto {
                status: UpdateStatus::Available,
                current_version: context.current_version_text(),
                latest_version: Some(candidate.version),
                channel: settings.channel.clone(),
                asset_name: Some(candidate.asset_name),
                asset_path: None,
                asset_sha256: candidate.asset_sha256,
                asset_size: Some(candidate.asset_size),
                asset_url: candidate.asset_url,
                source: recommended_source,
                checked_at: Some(Utc::now()),
                downloaded_at: None,
                install_log_path: None,
                install_mode: None,
                install_started_at: None,
                install_scheduled_at: None,
                last_error: None,
            };
            return Ok((result, next_state));
        }

        if saw_not_available {
            let result = UpdateCheckResult {
                status: UpdateCheckStatus::NotAvailable,
                current_version: context.current_version_text(),
                latest_version: None,
                release_notes: None,
                mandatory: false,
                can_download_from_mirror: false,
                can_download_from_github: false,
                recommended_source: None,
                asset_url: None,
            };
            let next_state = UpdateStateDto {
                status: UpdateStatus::Idle,
                current_version: context.current_version_text(),
                latest_version: None,
                channel: settings.channel.clone(),
                asset_name: None,
                asset_path: None,
                asset_sha256: None,
                asset_size: None,
                asset_url: None,
                source: None,
                checked_at: Some(Utc::now()),
                downloaded_at: None,
                install_log_path: None,
                install_mode: None,
                install_started_at: None,
                install_scheduled_at: None,
                last_error: None,
            };
            return Ok((result, next_state));
        }

        Err(aggregate_provider_errors(provider_errors))
    }
}

fn env_manifest_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).and_then(|value| {
        let value = value.to_string_lossy().trim().to_string();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

fn github_repo() -> String {
    env::var(GITHUB_REPO_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_GITHUB_REPO.to_string())
}

fn build_github_api_client() -> Result<Client, AppError> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .user_agent("floral-notepaper-updater")
        .build()
        .map_err(|error| errors::github_api_error(format!("无法创建 HTTP 客户端：{error}")))
}

#[derive(Debug, Deserialize)]
struct GithubApiAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GithubApiRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: Option<String>,
    body: Option<String>,
    assets: Vec<GithubApiAsset>,
}

fn check_github_api(
    context: &UpdateCheckContext,
    priority: usize,
) -> Result<ProviderCheck, AppError> {
    let repo = github_repo();
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");

    let client = build_github_api_client()?;
    let response = client.get(&url).send().map_err(|error| {
        if error.is_timeout() {
            errors::github_api_error("请求超时")
        } else {
            errors::github_api_error(error.to_string())
        }
    })?;

    let status = response.status();
    if status.as_u16() == 403 || status.as_u16() == 429 {
        return Err(errors::github_rate_limited());
    }
    if !status.is_success() {
        return Err(errors::github_api_error(format!(
            "HTTP {}",
            status.as_u16()
        )));
    }

    let body = response
        .text()
        .map_err(|error| errors::github_api_error(format!("响应读取失败：{error}")))?;
    let release: GithubApiRelease = serde_json::from_str(&body)
        .map_err(|error| errors::github_api_error(format!("响应解析失败：{error}")))?;

    let version_str = release
        .tag_name
        .trim_start_matches('v')
        .trim_start_matches('V');
    let normalized_version = version::normalize_version(version_str)?;

    if !version::is_newer_version(
        &context.current_version,
        &normalized_version,
        context.allow_prerelease,
    ) {
        return Ok(ProviderCheck::NotAvailable);
    }

    if release.assets.is_empty() {
        return Err(errors::github_release_no_assets());
    }

    let matched = release
        .assets
        .iter()
        .filter_map(|asset| {
            platform::infer_asset_from_filename(
                &asset.name,
                &asset.browser_download_url,
                asset.size,
            )
        })
        .find(|inferred| {
            inferred.os == context.platform.os
                && inferred.arch == context.platform.arch
                && matches_install_kind(&inferred.kind, &context.platform.install_kind)
        });

    let matched = matched.ok_or_else(|| {
        errors::with_detail(
            errors::manifest_asset_not_found(),
            "platform",
            format!(
                "{:?}-{:?}-{:?}",
                context.platform.os, context.platform.arch, context.platform.install_kind
            ),
        )
    })?;

    Ok(ProviderCheck::Available(UpdateCandidate {
        priority,
        version: version_str.to_string(),
        normalized_version,
        release_notes: release.body,
        mandatory: false,
        asset_name: matched.name,
        asset_sha256: None,
        asset_size: matched.size,
        asset_url: Some(matched.url),
        can_download_from_mirror: false,
        can_download_from_github: true,
    }))
}

fn matches_install_kind(
    inferred: &super::types::InstallKind,
    current: &super::types::InstallKind,
) -> bool {
    use super::types::InstallKind;
    if *current == InstallKind::Unknown {
        return true;
    }
    matches!(
        (inferred, current),
        (InstallKind::MacosAppBundle, InstallKind::MacosAppBundle)
            | (InstallKind::WindowsNsis, InstallKind::WindowsNsis)
            | (InstallKind::WindowsNsis, InstallKind::WindowsPortable)
            | (InstallKind::WindowsPortable, InstallKind::WindowsPortable)
            | (InstallKind::WindowsPortable, InstallKind::WindowsNsis)
    )
}

fn persist_last_auto_check_at(
    paths: &UpdatePaths,
    settings: &StoredUpdateSettings,
) -> Result<(), AppError> {
    let mut settings = settings.clone();
    settings.last_auto_check_at = Some(Utc::now());
    settings::save(paths, &settings)
}

fn load_manifest_candidate(
    provider: &str,
    manifest_path: &Path,
    context: &UpdateCheckContext,
    priority: usize,
    can_download_from_mirror: bool,
    can_download_from_github: bool,
) -> Result<ProviderCheck, AppError> {
    let manifest_bytes = fs::read(manifest_path).map_err(|error| {
        let error = errors::with_detail(
            errors::app_error(
                "updateProviderFixtureUnreadable",
                format!("无法读取 {provider} 更新测试清单：{error}"),
            ),
            "provider",
            provider,
        );
        errors::with_detail(error, "path", manifest_path.display().to_string())
    })?;
    let manifest = manifest::parse_manifest(&manifest_bytes)?;
    let asset = manifest::select_asset(
        &manifest,
        &context.platform,
        context.platform.install_kind.clone(),
    )?;
    let candidate_version = manifest.normalized_version()?;
    if !version::is_newer_version(
        &context.current_version,
        &candidate_version,
        context.allow_prerelease,
    ) {
        return Ok(ProviderCheck::NotAvailable);
    }

    let github_url = asset.github_url.clone();
    Ok(ProviderCheck::Available(UpdateCandidate {
        priority,
        version: manifest.version.clone(),
        normalized_version: candidate_version,
        release_notes: manifest.release_notes.clone(),
        mandatory: manifest.mandatory,
        asset_name: asset.name.clone(),
        asset_sha256: Some(asset.sha256),
        asset_size: asset.size,
        asset_url: Some(github_url.clone()),
        can_download_from_mirror,
        can_download_from_github: can_download_from_github && !github_url.trim().is_empty(),
    }))
}

fn check_provider_order(preference: &CheckSourcePreference) -> Vec<DownloadSourceUsed> {
    match preference {
        CheckSourcePreference::MirrorFirst => {
            vec![DownloadSourceUsed::Mirror, DownloadSourceUsed::Github]
        }
        CheckSourcePreference::GithubFirst => {
            vec![DownloadSourceUsed::Github, DownloadSourceUsed::Mirror]
        }
        CheckSourcePreference::MirrorOnly => vec![DownloadSourceUsed::Mirror],
        CheckSourcePreference::GithubOnly => vec![DownloadSourceUsed::Github],
    }
}

fn merge_candidates(mut candidates: Vec<UpdateCandidate>) -> Option<UpdateCandidate> {
    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|left, right| {
        right
            .normalized_version
            .cmp(&left.normalized_version)
            .then(left.priority.cmp(&right.priority))
    });

    let best_version = candidates.first()?.normalized_version.clone();
    let mut matching_candidates = candidates
        .into_iter()
        .filter(|candidate| candidate.normalized_version == best_version)
        .collect::<Vec<_>>();
    matching_candidates.sort_by_key(|candidate| candidate.priority);

    let mut primary = matching_candidates.remove(0);
    primary.can_download_from_mirror |= matching_candidates
        .iter()
        .any(|candidate| candidate.can_download_from_mirror);
    primary.can_download_from_github |= matching_candidates
        .iter()
        .any(|candidate| candidate.can_download_from_github);
    primary.mandatory |= matching_candidates
        .iter()
        .any(|candidate| candidate.mandatory);
    if primary
        .release_notes
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        primary.release_notes = matching_candidates.into_iter().find_map(|candidate| {
            candidate
                .release_notes
                .filter(|notes| !notes.trim().is_empty())
        });
    }

    Some(primary)
}

fn recommended_source(
    preference: &DownloadSourcePreference,
    can_download_from_mirror: bool,
    can_download_from_github: bool,
) -> Option<DownloadSourceUsed> {
    match preference {
        DownloadSourcePreference::MirrorFirst => {
            if can_download_from_mirror {
                Some(DownloadSourceUsed::Mirror)
            } else if can_download_from_github {
                Some(DownloadSourceUsed::Github)
            } else {
                None
            }
        }
        DownloadSourcePreference::GithubFirst => {
            if can_download_from_github {
                Some(DownloadSourceUsed::Github)
            } else if can_download_from_mirror {
                Some(DownloadSourceUsed::Mirror)
            } else {
                None
            }
        }
        DownloadSourcePreference::MirrorOnly => {
            can_download_from_mirror.then_some(DownloadSourceUsed::Mirror)
        }
        DownloadSourcePreference::GithubOnly => {
            can_download_from_github.then_some(DownloadSourceUsed::Github)
        }
    }
}

fn aggregate_provider_errors(errors_list: Vec<AppError>) -> AppError {
    if errors_list.is_empty() {
        return errors::source_not_configured();
    }

    if errors_list
        .iter()
        .all(|error| error.code == "updateProviderNotConfigured")
    {
        let providers = errors_list
            .iter()
            .filter_map(|error| error.details.get("provider"))
            .cloned()
            .collect::<Vec<_>>()
            .join(",");
        let error = errors::source_not_configured();
        return if providers.is_empty() {
            error
        } else {
            errors::with_detail(error, "providers", providers)
        };
    }

    errors_list
        .into_iter()
        .find(|error| error.code != "updateProviderNotConfigured")
        .unwrap_or_else(errors::source_not_configured)
}

fn failed_state(
    context: &UpdateCheckContext,
    settings: &StoredUpdateSettings,
    error: &AppError,
) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::Failed,
        current_version: context.current_version_text(),
        latest_version: None,
        channel: settings.channel.clone(),
        asset_name: None,
        asset_path: None,
        asset_sha256: None,
        asset_size: None,
        asset_url: None,
        source: None,
        checked_at: Some(Utc::now()),
        downloaded_at: None,
        install_log_path: None,
        install_mode: None,
        install_started_at: None,
        install_scheduled_at: None,
        last_error: Some(UpdateErrorDto::recoverable(
            error.code.clone(),
            error.message.clone(),
            update_error_action(error).map(str::to_string),
        )),
    }
}

fn update_error_action(error: &AppError) -> Option<&'static str> {
    match error.code.as_str() {
        "updateSourceNotConfigured" | "updateProviderNotConfigured" => {
            Some("configureUpdateSource")
        }
        "updateProviderFixtureUnreadable" => Some("fixFixturePath"),
        "updatePlatformUnsupported" => Some("useSupportedInstall"),
        "updateGithubApi" | "updateGithubRateLimited" | "updateGithubNoAssets" => Some("retry"),
        _ => Some("retry"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::{
        platform::{Arch, Os},
        types::InstallKind,
        UpdatePaths,
    };

    const VALID_MANIFEST_BYTES: &[u8] = include_bytes!("fixtures/update-manifest.valid.json");

    fn test_paths(name: &str) -> UpdatePaths {
        let root = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale test dir");
        }
        UpdatePaths::new(root)
    }

    fn test_context(install_kind: InstallKind) -> UpdateCheckContext {
        UpdateCheckContext {
            platform: PlatformInfo {
                os: Os::Macos,
                arch: Arch::Aarch64,
                app_version: "1.0.3".into(),
                app_id: super::super::APP_ID.into(),
                install_kind,
                current_exe: None,
                current_app_bundle: None,
            },
            current_version: Version::new(1, 0, 3),
            allow_prerelease: false,
        }
    }

    fn test_settings(preference: CheckSourcePreference) -> StoredUpdateSettings {
        StoredUpdateSettings {
            download_source_preference: DownloadSourcePreference::GithubFirst,
            check_source_preference: preference,
            channel: super::super::types::UpdateChannel::Stable,
            ..StoredUpdateSettings::default()
        }
    }

    fn write_manifest(paths: &UpdatePaths, name: &str, version: &str) -> PathBuf {
        paths.ensure_dirs().expect("create test dirs");
        let raw = std::str::from_utf8(VALID_MANIFEST_BYTES)
            .expect("fixture utf8")
            .replace("1.0.5", version);
        let path = paths.root_dir().join(name);
        fs::write(&path, raw).expect("write manifest fixture");
        path
    }

    #[test]
    fn returns_source_not_configured_when_no_provider_fixture_exists_and_github_only() {
        let service = UpdateCheckService::with_providers(
            MirrorProvider::default(),
            GithubProvider::offline(),
        );
        let settings = test_settings(CheckSourcePreference::GithubOnly);

        let result = service.evaluate(&settings, &test_context(InstallKind::MacosAppBundle));
        assert!(result.is_err());
    }

    #[test]
    fn returns_not_available_when_only_mirror_is_configured_but_github_only_is_selected() {
        let paths = test_paths("check-github-only-mirror-present");
        let mirror_manifest = write_manifest(&paths, "mirror.json", "1.0.5");
        let service = UpdateCheckService::with_providers(
            MirrorProvider::with_manifest_path(mirror_manifest),
            GithubProvider::offline(),
        );
        let settings = test_settings(CheckSourcePreference::GithubOnly);

        let result = service.evaluate(&settings, &test_context(InstallKind::MacosAppBundle));
        assert!(result.is_err());
    }

    #[test]
    fn prefers_highest_available_version_across_providers() {
        let paths = test_paths("check-highest-version");
        let mirror_manifest = write_manifest(&paths, "mirror.json", "1.0.5");
        let github_manifest = write_manifest(&paths, "github.json", "1.0.6");
        let service = UpdateCheckService::with_providers(
            MirrorProvider::with_manifest_path(mirror_manifest),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let settings = test_settings(CheckSourcePreference::MirrorFirst);

        let (result, next_state) = service
            .evaluate(&settings, &test_context(InstallKind::MacosAppBundle))
            .expect("configured manifests should return result");

        assert_eq!(result.status, UpdateCheckStatus::Available);
        assert_eq!(result.latest_version.as_deref(), Some("1.0.6"));
        assert_eq!(result.recommended_source, Some(DownloadSourceUsed::Github));
        assert_eq!(next_state.status, UpdateStatus::Available);
        assert_eq!(next_state.latest_version.as_deref(), Some("1.0.6"));
    }

    #[test]
    fn returns_not_available_when_candidate_is_not_newer() {
        let paths = test_paths("check-not-available");
        let github_manifest = write_manifest(&paths, "github.json", "1.0.3");
        let service = UpdateCheckService::with_providers(
            MirrorProvider::default(),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let settings = test_settings(CheckSourcePreference::GithubOnly);

        let (result, next_state) = service
            .evaluate(&settings, &test_context(InstallKind::MacosAppBundle))
            .expect("matching version should not error");

        assert_eq!(result.status, UpdateCheckStatus::NotAvailable);
        assert_eq!(next_state.status, UpdateStatus::Idle);
        assert!(next_state.latest_version.is_none());
    }

    #[test]
    fn stores_asset_url_in_state_from_manifest_fixture() {
        let paths = test_paths("check-asset-url");
        let github_manifest = write_manifest(&paths, "github.json", "1.0.5");
        let service = UpdateCheckService::with_providers(
            MirrorProvider::default(),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let settings = test_settings(CheckSourcePreference::GithubOnly);

        let (result, next_state) = service
            .evaluate(&settings, &test_context(InstallKind::MacosAppBundle))
            .expect("available update should have asset url");

        assert!(result.asset_url.is_some());
        assert!(next_state.asset_url.is_some());
    }
}
