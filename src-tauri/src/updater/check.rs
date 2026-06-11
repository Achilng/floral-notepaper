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

const MIRROR_CHYAN_MANIFEST_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_MIRROR_MANIFEST_PATH";
const GITHUB_MANIFEST_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_GITHUB_MANIFEST_PATH";
const GITHUB_REPO_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_GITHUB_REPO";
const DEFAULT_GITHUB_REPO: &str = "Achilng/floral-notepaper";
const MIRROR_CHYAN_API_BASE: &str = "https://mirrorchyan.com/api/resources";
const MIRROR_CHYAN_RES_ID: &str = "floral";
const MIRROR_CHYAN_RES_ID_OVERRIDE_ENV: &str = "FLORAL_NOTEPAPER_MIRROR_CHYAN_RES_ID";
const MIRROR_CHYAN_USER_AGENT: &str = "floral_notepaper";

fn mirror_chyan_res_id() -> &'static str {
    if let Ok(value) = env::var(MIRROR_CHYAN_RES_ID_OVERRIDE_ENV) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Box::leak(trimmed.into_boxed_str());
        }
    }
    MIRROR_CHYAN_RES_ID
}

macro_rules! debug_log {
    ($($arg:tt)*) => { super::debug_log!("check", $($arg)*) };
}

#[derive(Debug, Clone)]
struct UpdateCheckContext {
    platform: PlatformInfo,
    current_version: Version,
    allow_prerelease: bool,
    previous_state: UpdateStateDto,
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
    mirror_chyan_asset_url: Option<String>,
    github_asset_url: Option<String>,
    can_download_from_mirror_chyan: bool,
    can_download_from_github: bool,
    /// Whether SHA-256 hash verification is enabled for this candidate.
    /// Set to `false` for MirrorChyan source because the MirrorChyan API does not
    /// return asset checksums in its response.
    ///
    /// This field is consumed at the candidate-construction site for clarity
    /// and logging; the actual enforcement happens in `DownloadPlan`, which
    /// derives the flag from the download source at plan-resolution time.
    #[allow(dead_code)]
    sha256_verification_enabled: bool,
}

impl UpdateCandidate {
    fn asset_url_for_source(&self, source: Option<&DownloadSourceUsed>) -> Option<String> {
        match source {
            Some(DownloadSourceUsed::MirrorChyan) => self.mirror_chyan_asset_url.clone(),
            Some(DownloadSourceUsed::Github) => self.github_asset_url.clone(),
            None => None,
        }
        .or_else(|| self.asset_url.clone())
        .or_else(|| self.github_asset_url.clone())
        .or_else(|| self.mirror_chyan_asset_url.clone())
    }
}

#[derive(Debug, Clone)]
enum ProviderCheck {
    NotAvailable,
    Available(Box<UpdateCandidate>),
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
struct MirrorChyanProvider {
    manifest_path: Option<PathBuf>,
    cdk: Option<String>,
    offline: bool,
}

impl MirrorChyanProvider {
    pub fn from_env() -> Self {
        Self {
            manifest_path: env_manifest_path(MIRROR_CHYAN_MANIFEST_PATH_ENV),
            cdk: None,
            offline: env::var("FLORAL_NOTEPAPER_UPDATE_OFFLINE").is_ok(),
        }
    }

    pub fn with_cdk(mut self, cdk: Option<String>) -> Self {
        self.cdk = cdk;
        self
    }

    #[cfg(test)]
    fn with_manifest_path(path: PathBuf) -> Self {
        Self {
            manifest_path: Some(path),
            cdk: None,
            offline: false,
        }
    }

    #[cfg(test)]
    fn offline() -> Self {
        Self {
            manifest_path: None,
            cdk: None,
            offline: true,
        }
    }
}

impl UpdateCheckProvider for MirrorChyanProvider {
    fn label(&self) -> &'static str {
        "MirrorChyan"
    }

    fn check(
        &self,
        context: &UpdateCheckContext,
        priority: usize,
    ) -> Result<ProviderCheck, AppError> {
        if let Some(manifest_path) = &self.manifest_path {
            return load_manifest_candidate(self.label(), manifest_path, context, priority, true);
        }
        if self.offline {
            return Err(errors::provider_not_configured(self.label()));
        }
        check_mirror_chyan_api(context, priority, self.cdk.as_deref())
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
            return load_manifest_candidate(self.label(), manifest_path, context, priority, false);
        }

        if self.offline {
            return Err(errors::provider_not_configured(self.label()));
        }

        check_github_api(context, priority)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct UpdateCheckService {
    mirror_chyan: MirrorChyanProvider,
    github: GithubProvider,
    platform_override: Option<PlatformInfo>,
}

impl UpdateCheckService {
    pub(crate) fn from_env() -> Self {
        Self {
            mirror_chyan: MirrorChyanProvider::from_env(),
            github: GithubProvider::from_env(),
            platform_override: None,
        }
    }

    pub(crate) fn with_cdk(mut self, cdk: Option<String>) -> Self {
        self.mirror_chyan = self.mirror_chyan.with_cdk(cdk);
        self
    }

    pub(crate) fn run(
        &self,
        paths: &UpdatePaths,
        _manual: bool,
        current_version: &str,
    ) -> Result<UpdateCheckResult, AppError> {
        debug_log!("开始检查更新 当前版本=v{current_version}");
        let settings = settings::load(paths)?;
        debug_log!(
            "更新设置已加载 channel={:?} allow_prerelease={}",
            settings.channel,
            settings.allow_prerelease
        );
        let previous_state = state::load_with_current_version(paths, current_version)?;
        let context = UpdateCheckContext {
            platform: self.current_platform(current_version),
            current_version: version::normalize_version(current_version)?,
            allow_prerelease: version::allows_prerelease(
                &settings.channel,
                settings.allow_prerelease,
            ),
            previous_state,
        };
        debug_log!(
            "平台检查 os={:?} arch={:?} install_kind={:?}",
            context.platform.os,
            context.platform.arch,
            context.platform.install_kind
        );
        if let Err(error) = context.platform.ensure_in_app_updates_supported() {
            persist_last_auto_check_at(paths, &settings)?;
            state::save(paths, &failed_state(&context, &settings, &error))?;
            return Err(error);
        }

        let outcome = self.evaluate(&settings, &context);
        match outcome {
            Ok((result, next_state)) => {
                debug_log!(
                    "检查完成 status={:?} latest_version={:?}",
                    result.status,
                    result.latest_version
                );
                persist_last_auto_check_at(paths, &settings)?;
                state::save(paths, &next_state)?;
                Ok(result)
            }
            Err(error) => {
                debug_log!("检查失败 code={} message={}", error.code, error.message);
                persist_last_auto_check_at(paths, &settings)?;
                state::save(paths, &failed_state(&context, &settings, &error))?;
                Err(error)
            }
        }
    }

    #[cfg(test)]
    fn with_providers(mirror_chyan: MirrorChyanProvider, github: GithubProvider) -> Self {
        Self {
            mirror_chyan,
            github,
            platform_override: None,
        }
    }

    #[cfg(test)]
    fn with_providers_and_platform(
        mirror_chyan: MirrorChyanProvider,
        github: GithubProvider,
        platform: PlatformInfo,
    ) -> Self {
        Self {
            mirror_chyan,
            github,
            platform_override: Some(platform),
        }
    }

    fn current_platform(&self, current_version: &str) -> PlatformInfo {
        self.platform_override
            .clone()
            .unwrap_or_else(|| platform::current_platform_with_version(current_version.to_string()))
    }

    fn evaluate(
        &self,
        settings: &StoredUpdateSettings,
        context: &UpdateCheckContext,
    ) -> Result<(UpdateCheckResult, UpdateStateDto), AppError> {
        let provider_order = check_provider_order(&settings.check_source_preference);
        debug_log!(
            "提供者检查顺序={:?} 检查源偏好={:?}",
            provider_order,
            settings.check_source_preference
        );
        let mut available = Vec::new();
        let mut saw_not_available = false;
        let mut provider_errors = Vec::new();

        for (priority, source) in provider_order.into_iter().enumerate() {
            debug_log!("尝试提供者 source={source:?} priority={priority}");
            let provider_result = match source {
                DownloadSourceUsed::MirrorChyan => self.mirror_chyan.check(context, priority),
                DownloadSourceUsed::Github => self.github.check(context, priority),
            };

            match provider_result {
                Ok(ProviderCheck::Available(candidate)) => {
                    debug_log!("提供者 {source:?} 返回可用版本={}", candidate.version);
                    available.push(*candidate);
                }
                Ok(ProviderCheck::NotAvailable) => {
                    debug_log!("提供者 {source:?} 返回无更新");
                    saw_not_available = true;
                    // Mirror 酱返回无更新说明当前已是最新版，无需再查 GitHub。
                    if matches!(source, DownloadSourceUsed::MirrorChyan) {
                        debug_log!("Mirror 酱已确认无更新，跳过后续提供者");
                        break;
                    }
                }
                Err(error) => {
                    debug_log!(
                        "提供者 {source:?} 返回错误 code={} message={}",
                        error.code,
                        error.message
                    );
                    provider_errors.push(error);
                }
            }
        }

        debug_log!("可用候选数={}", available.len());
        if let Some(candidate) = merge_candidates(available) {
            debug_log!(
                "合并后版本={} 推荐源={:?}",
                candidate.version,
                recommended_source(
                    &settings.download_source_preference,
                    candidate.can_download_from_mirror_chyan,
                    candidate.can_download_from_github,
                )
            );
            let recommended_source = recommended_source(
                &settings.download_source_preference,
                candidate.can_download_from_mirror_chyan,
                candidate.can_download_from_github,
            );
            let asset_url = candidate.asset_url_for_source(recommended_source.as_ref());
            let result = UpdateCheckResult {
                status: UpdateCheckStatus::Available,
                current_version: context.current_version_text(),
                latest_version: Some(candidate.version.clone()),
                release_notes: candidate.release_notes.clone(),
                mandatory: candidate.mandatory,
                can_download_from_mirror_chyan: candidate.can_download_from_mirror_chyan,
                can_download_from_github: candidate.can_download_from_github,
                recommended_source: recommended_source.clone(),
                asset_url: asset_url.clone(),
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
                asset_url,
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
                can_download_from_mirror_chyan: false,
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

pub(super) fn env_manifest_path(key: &str) -> Option<PathBuf> {
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

// --- Mirror酱 API ---

#[derive(Debug, Deserialize)]
struct MirrorChyanApiResponse {
    code: i32,
    msg: String,
    data: Option<MirrorChyanApiData>,
}

#[derive(Debug, Deserialize)]
struct MirrorChyanApiData {
    version_name: String,
    #[allow(dead_code)]
    version_number: Option<u64>,
    #[allow(dead_code)]
    channel: Option<String>,
    #[allow(dead_code)]
    os: Option<String>,
    #[allow(dead_code)]
    arch: Option<String>,
    release_note: Option<String>,
    url: Option<String>,
}

fn mirror_chyan_os_param(os: &platform::Os) -> &'static str {
    match os {
        platform::Os::Windows => "windows",
        platform::Os::Macos => "darwin",
        platform::Os::Unsupported => "unknown",
    }
}

fn mirror_chyan_arch_param(arch: &platform::Arch) -> &'static str {
    match arch {
        platform::Arch::X86_64 => "amd64",
        platform::Arch::Aarch64 => "arm64",
        platform::Arch::Unsupported => "unknown",
    }
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let name = path.rsplit('/').next()?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

fn build_mirror_chyan_api_client() -> Result<Client, AppError> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .user_agent("floral-notepaper-updater")
        .build()
        .map_err(|error| errors::mirror_chyan_api_error(format!("无法创建 HTTP 客户端：{error}")))
}

fn mirror_chyan_request_error(error: reqwest::Error) -> AppError {
    errors::mirror_chyan_api_error(mirror_chyan_request_error_message(&error))
}

fn mirror_chyan_request_error_message(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "请求超时".into();
    }
    if error.is_connect() {
        return "网络连接失败".into();
    }
    if error.is_redirect() {
        return "重定向失败".into();
    }
    if let Some(status) = error.status() {
        return format!("HTTP {}", status.as_u16());
    }
    "请求失败".into()
}

/// Common MirrorChyan API call: URL construction, HTTP request, JSON parsing,
/// and MirrorChyan error-code handling.  Returns [`MirrorChyanApiData`] on success.
fn call_mirror_chyan_api(
    current_version: &str,
    os: &str,
    arch: &str,
    cdk: Option<&str>,
) -> Result<MirrorChyanApiData, AppError> {
    let res_id = mirror_chyan_res_id();
    let mut url = reqwest::Url::parse(&format!("{MIRROR_CHYAN_API_BASE}/{res_id}/latest"))
        .map_err(|e| errors::mirror_chyan_api_error(format!("URL 构建失败：{e}")))?;
    url.query_pairs_mut()
        .append_pair("current_version", current_version)
        .append_pair("os", os)
        .append_pair("arch", arch)
        .append_pair("user_agent", MIRROR_CHYAN_USER_AGENT);
    if let Some(cdk) = cdk.filter(|s| !s.trim().is_empty()) {
        url.query_pairs_mut().append_pair("cdk", cdk);
    }

    let client = build_mirror_chyan_api_client()?;
    let response = client.get(url).send().map_err(mirror_chyan_request_error)?;

    let status = response.status();
    if !status.is_success() && status.as_u16() != 403 {
        return Err(errors::mirror_chyan_api_error(format!(
            "HTTP {}",
            status.as_u16()
        )));
    }

    let body = response.text().map_err(|error| {
        errors::mirror_chyan_api_error(format!(
            "响应读取失败：{}",
            mirror_chyan_request_error_message(&error)
        ))
    })?;
    let api_response: MirrorChyanApiResponse = serde_json::from_str(&body)
        .map_err(|error| errors::mirror_chyan_api_error(format!("响应解析失败：{error}")))?;

    if api_response.code < 0 {
        return Err(errors::mirror_chyan_api_error(format!(
            "服务端错误 (code={})：{}",
            api_response.code, api_response.msg
        )));
    }

    if api_response.code > 0 {
        let code = api_response.code;
        return Err(match code {
            7001..=7005 => errors::mirror_chyan_cdk_error(code, api_response.msg),
            _ => errors::mirror_chyan_resource_error(code, api_response.msg),
        });
    }

    api_response
        .data
        .ok_or_else(|| errors::mirror_chyan_api_error("响应缺少 data 字段"))
}

fn check_mirror_chyan_api(
    context: &UpdateCheckContext,
    priority: usize,
    cdk: Option<&str>,
) -> Result<ProviderCheck, AppError> {
    let os = mirror_chyan_os_param(&context.platform.os);
    let arch = mirror_chyan_arch_param(&context.platform.arch);
    let current_version = format!("v{}", context.current_version_text());
    debug_log!(
        "MirrorChyan API 请求 os={os} arch={arch} current={current_version} has_cdk={}",
        cdk.is_some()
    );

    let data = call_mirror_chyan_api(&current_version, os, arch, cdk)?;
    debug_log!(
        "MirrorChyan API 返回 version={} has_url={}",
        data.version_name,
        data.url.is_some()
    );

    let version_str = data
        .version_name
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

    let mirror_chyan_asset_url = data.url;
    let asset_name = mirror_chyan_asset_url
        .as_deref()
        .and_then(extract_filename_from_url)
        .unwrap_or_else(|| format!("floral-notepaper_{version_str}_{os}_{arch}"));
    let has_url = mirror_chyan_asset_url.is_some();
    // MirrorChyan API does not provide asset SHA-256 checksums in its response,
    // so hash verification is explicitly disabled for MirrorChyan-sourced downloads.
    debug_log!(
        "MirrorChyan API response does not include SHA-256 checksum; \
         verification disabled for MirrorChyan source downloads"
    );
    Ok(ProviderCheck::Available(Box::new(UpdateCandidate {
        priority,
        version: version_str.to_string(),
        normalized_version,
        release_notes: data.release_note.filter(|s| !s.trim().is_empty()),
        mandatory: false,
        asset_name,
        asset_sha256: None,
        asset_size: 0,
        asset_url: mirror_chyan_asset_url.clone(),
        mirror_chyan_asset_url,
        github_asset_url: None,
        can_download_from_mirror_chyan: has_url,
        can_download_from_github: false,
        sha256_verification_enabled: false,
    })))
}

#[derive(Debug, Clone)]
pub(crate) struct MirrorChyanDownloadInfo {
    pub url: String,
}

pub(crate) fn fetch_mirror_chyan_download_url(
    platform: &PlatformInfo,
    current_version: &str,
    cdk: Option<&str>,
) -> Result<MirrorChyanDownloadInfo, AppError> {
    let os = mirror_chyan_os_param(&platform.os);
    let arch = mirror_chyan_arch_param(&platform.arch);
    let version = format!("v{current_version}");

    let data = call_mirror_chyan_api(&version, os, arch, cdk)?;

    let download_url = data.url.ok_or_else(|| {
        errors::app_error(
            "updateMirrorChyanDownloadNeedCdk",
            "Mirror酱未返回下载链接，请配置有效的 CDK",
        )
    })?;

    Ok(MirrorChyanDownloadInfo { url: download_url })
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
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubApiRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: Option<String>,
    body: Option<String>,
    assets: Vec<GithubApiAsset>,
}

fn fetch_latest_github_release() -> Result<GithubApiRelease, AppError> {
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
    serde_json::from_str(&body)
        .map_err(|error| errors::github_api_error(format!("响应解析失败：{error}")))
}

pub(crate) struct GithubDownloadInfo {
    pub asset_name: String,
    pub asset_size: u64,
    pub url: String,
    pub sha256: Option<String>,
}

pub(crate) fn fetch_github_download_info(
    platform: &PlatformInfo,
    version: &str,
    asset_name: &str,
    expected_size: Option<u64>,
) -> Result<GithubDownloadInfo, AppError> {
    let release = fetch_latest_github_release()?;
    let release_version = release
        .tag_name
        .trim_start_matches('v')
        .trim_start_matches('V');
    if version::normalize_version(release_version)? != version::normalize_version(version)? {
        return Err(errors::with_detail(
            errors::app_error(
                "updateDownloadVersionMismatch",
                "GitHub 最新 Release 与当前待下载版本不一致",
            ),
            "expectedVersion",
            version,
        ));
    }

    let matched = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .or_else(|| {
            release.assets.iter().find(|asset| {
                platform::infer_asset_from_filename(
                    &asset.name,
                    &asset.browser_download_url,
                    asset.size,
                )
                .is_some_and(|inferred| {
                    inferred.os == platform.os
                        && inferred.arch == platform.arch
                        && matches_install_kind(&inferred.kind, &platform.install_kind)
                })
            })
        })
        .ok_or_else(|| {
            errors::with_detail(errors::manifest_asset_not_found(), "assetName", asset_name)
        })?;

    if expected_size.is_some_and(|size| size > 0 && matched.size != size) {
        return Err(errors::with_detail(
            errors::app_error(
                "updateDownloadAssetMismatch",
                "GitHub Release 中的更新包元数据与已检查结果不一致",
            ),
            "assetName",
            asset_name,
        ));
    }

    Ok(GithubDownloadInfo {
        asset_name: matched.name.clone(),
        asset_size: matched.size,
        url: matched.browser_download_url.clone(),
        sha256: matched.digest.as_deref().and_then(parse_sha256_from_digest),
    })
}

fn check_github_api(
    context: &UpdateCheckContext,
    priority: usize,
) -> Result<ProviderCheck, AppError> {
    debug_log!("GitHub API 请求 latest release");
    let release = fetch_latest_github_release()?;

    let version_str = release
        .tag_name
        .trim_start_matches('v')
        .trim_start_matches('V');
    let normalized_version = version::normalize_version(version_str)?;
    debug_log!(
        "GitHub Release tag={} assets数={}",
        release.tag_name,
        release.assets.len()
    );

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

    let matched_api_asset = release
        .assets
        .iter()
        .find(|asset| {
            platform::infer_asset_from_filename(
                &asset.name,
                &asset.browser_download_url,
                asset.size,
            )
            .is_some_and(|inferred| {
                inferred.os == context.platform.os
                    && inferred.arch == context.platform.arch
                    && matches_install_kind(&inferred.kind, &context.platform.install_kind)
            })
        })
        .ok_or_else(|| {
            errors::with_detail(
                errors::manifest_asset_not_found(),
                "platform",
                format!(
                    "{:?}-{:?}-{:?}",
                    context.platform.os, context.platform.arch, context.platform.install_kind
                ),
            )
        })?;

    let asset_sha256 = matched_api_asset
        .digest
        .as_deref()
        .and_then(parse_sha256_from_digest);

    Ok(ProviderCheck::Available(Box::new(UpdateCandidate {
        priority,
        version: version_str.to_string(),
        normalized_version,
        release_notes: release.body,
        mandatory: false,
        asset_name: matched_api_asset.name.clone(),
        asset_sha256,
        asset_size: matched_api_asset.size,
        asset_url: Some(matched_api_asset.browser_download_url.clone()),
        mirror_chyan_asset_url: None,
        github_asset_url: Some(matched_api_asset.browser_download_url.clone()),
        can_download_from_mirror_chyan: false,
        can_download_from_github: true,
        sha256_verification_enabled: true,
    })))
}

/// Parses `"sha256:<hex>"` → `Some("<hex>")`, returns `None` for other formats.
fn parse_sha256_from_digest(digest: &str) -> Option<String> {
    let hex = digest.strip_prefix("sha256:")?;
    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_lowercase())
    } else {
        None
    }
}

fn matches_install_kind(
    inferred: &super::types::InstallKind,
    current: &super::types::InstallKind,
) -> bool {
    *current == super::types::InstallKind::Unknown || inferred == current
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
    is_mirror_chyan_provider: bool,
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

    let mirror_chyan_asset_url = asset.mirror_chyan_url.clone();
    let github_asset_url = (!asset.github_url.trim().is_empty()).then(|| asset.github_url.clone());
    let has_mirror_chyan_url = mirror_chyan_asset_url.is_some();
    let has_github_url = github_asset_url.is_some();
    let asset_url = if is_mirror_chyan_provider {
        mirror_chyan_asset_url
            .clone()
            .or_else(|| github_asset_url.clone())
    } else {
        github_asset_url.clone()
    };

    Ok(ProviderCheck::Available(Box::new(UpdateCandidate {
        priority,
        version: manifest.version.clone(),
        normalized_version: candidate_version,
        release_notes: manifest.release_notes.clone(),
        mandatory: manifest.mandatory,
        asset_name: asset.name.clone(),
        asset_sha256: Some(asset.sha256),
        asset_size: asset.size,
        asset_url,
        mirror_chyan_asset_url,
        github_asset_url,
        can_download_from_mirror_chyan: has_mirror_chyan_url,
        can_download_from_github: has_github_url,
        sha256_verification_enabled: true,
    })))
}

fn check_provider_order(preference: &CheckSourcePreference) -> Vec<DownloadSourceUsed> {
    match preference {
        CheckSourcePreference::MirrorChyanFirst => {
            vec![DownloadSourceUsed::MirrorChyan, DownloadSourceUsed::Github]
        }
        CheckSourcePreference::GithubFirst => {
            vec![DownloadSourceUsed::Github, DownloadSourceUsed::MirrorChyan]
        }
    }
}

fn merge_candidates(mut candidates: Vec<UpdateCandidate>) -> Option<UpdateCandidate> {
    debug_log!("合并候选版本 候选数={}", candidates.len());
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
    let fallback_candidates = matching_candidates;

    primary.can_download_from_mirror_chyan |= fallback_candidates
        .iter()
        .any(|candidate| candidate.can_download_from_mirror_chyan);
    primary.can_download_from_github |= fallback_candidates
        .iter()
        .any(|candidate| candidate.can_download_from_github);
    primary.mandatory |= fallback_candidates
        .iter()
        .any(|candidate| candidate.mandatory);

    if primary.mirror_chyan_asset_url.is_none() {
        primary.mirror_chyan_asset_url = fallback_candidates
            .iter()
            .find_map(|candidate| candidate.mirror_chyan_asset_url.clone());
    }
    if primary.github_asset_url.is_none() {
        primary.github_asset_url = fallback_candidates
            .iter()
            .find_map(|candidate| candidate.github_asset_url.clone());
    }
    if primary.asset_sha256.is_none() {
        primary.asset_sha256 = fallback_candidates
            .iter()
            .find_map(|candidate| candidate.asset_sha256.clone());
    }
    if primary.asset_size == 0 {
        if let Some(candidate) = fallback_candidates
            .iter()
            .find(|candidate| candidate.asset_size > 0)
        {
            primary.asset_size = candidate.asset_size;
            primary.asset_name = candidate.asset_name.clone();
        }
    }

    if primary
        .release_notes
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        primary.release_notes = fallback_candidates.iter().find_map(|candidate| {
            candidate
                .release_notes
                .clone()
                .filter(|notes| !notes.trim().is_empty())
        });
    }

    Some(primary)
}

fn recommended_source(
    preference: &DownloadSourcePreference,
    can_download_from_mirror_chyan: bool,
    can_download_from_github: bool,
) -> Option<DownloadSourceUsed> {
    match preference {
        DownloadSourcePreference::MirrorChyanFirst => {
            if can_download_from_mirror_chyan {
                Some(DownloadSourceUsed::MirrorChyan)
            } else if can_download_from_github {
                Some(DownloadSourceUsed::Github)
            } else {
                None
            }
        }
        DownloadSourcePreference::GithubFirst => {
            if can_download_from_github {
                Some(DownloadSourceUsed::Github)
            } else if can_download_from_mirror_chyan {
                Some(DownloadSourceUsed::MirrorChyan)
            } else {
                None
            }
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
        latest_version: context.previous_state.latest_version.clone(),
        channel: settings.channel.clone(),
        asset_name: context.previous_state.asset_name.clone(),
        asset_path: context.previous_state.asset_path.clone(),
        asset_sha256: context.previous_state.asset_sha256.clone(),
        asset_size: context.previous_state.asset_size,
        asset_url: context.previous_state.asset_url.clone(),
        source: context.previous_state.source.clone(),
        checked_at: Some(Utc::now()),
        downloaded_at: context.previous_state.downloaded_at,
        install_log_path: context.previous_state.install_log_path.clone(),
        install_mode: context.previous_state.install_mode.clone(),
        install_started_at: context.previous_state.install_started_at,
        install_scheduled_at: context.previous_state.install_scheduled_at,
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
        "updatePlatformUnsupported" | "updatePortableManualOnly" => Some("useSupportedInstall"),
        "updateGithubApi" | "updateGithubRateLimited" | "updateGithubNoAssets" => Some("retry"),
        "updateMirrorChyanApi" => Some("retry"),
        "updateMirrorChyanCdkExpired"
        | "updateMirrorChyanCdkInvalid"
        | "updateMirrorChyanCdkMismatched"
        | "updateMirrorChyanCdkBlocked"
        | "updateMirrorChyanCdk" => Some("checkCdk"),
        "updateMirrorChyanCdkQuotaExhausted" => Some("waitOrUpgrade"),
        "updateMirrorChyanInvalidParams"
        | "updateMirrorChyanInvalidOs"
        | "updateMirrorChyanInvalidArch"
        | "updateMirrorChyanInvalidChannel" => Some("reportBug"),
        "updateMirrorChyanResourceNotFound" => Some("retry"),
        "updateMirrorChyanBusiness" => Some("retry"),
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
            platform: test_platform(Os::Macos, Arch::Aarch64, install_kind),
            current_version: Version::new(1, 0, 3),
            allow_prerelease: false,
            previous_state: UpdateStateDto::idle_with_version("1.0.3"),
        }
    }

    fn test_platform(os: Os, arch: Arch, install_kind: InstallKind) -> PlatformInfo {
        PlatformInfo {
            os,
            arch,
            app_version: "1.0.3".into(),
            app_id: super::super::APP_ID.into(),
            install_kind,
            current_exe: None,
            current_app_bundle: None,
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
        let service = UpdateCheckService::with_providers_and_platform(
            MirrorChyanProvider::offline(),
            GithubProvider::offline(),
            test_platform(Os::Macos, Arch::Aarch64, InstallKind::MacosAppBundle),
        );
        let settings = test_settings(CheckSourcePreference::GithubFirst);

        let result = service.evaluate(&settings, &test_context(InstallKind::MacosAppBundle));
        assert!(result.is_err());
    }

    #[test]
    fn falls_back_to_mirror_chyan_when_github_is_not_configured() {
        let paths = test_paths("check-github-first-mirror-chyan-fallback");
        let mirror_chyan_manifest = write_manifest(&paths, "mirror.json", "1.0.5");
        let service = UpdateCheckService::with_providers(
            MirrorChyanProvider::with_manifest_path(mirror_chyan_manifest),
            GithubProvider::offline(),
        );
        let settings = test_settings(CheckSourcePreference::GithubFirst);

        let (result, _) = service
            .evaluate(&settings, &test_context(InstallKind::MacosAppBundle))
            .expect("github fails but mirror_chyan succeeds as fallback");

        assert_eq!(result.status, UpdateCheckStatus::Available);
        assert_eq!(result.latest_version.as_deref(), Some("1.0.5"));
    }

    #[test]
    fn prefers_highest_available_version_across_providers() {
        let paths = test_paths("check-highest-version");
        let mirror_chyan_manifest = write_manifest(&paths, "mirror.json", "1.0.5");
        let github_manifest = write_manifest(&paths, "github.json", "1.0.6");
        let service = UpdateCheckService::with_providers(
            MirrorChyanProvider::with_manifest_path(mirror_chyan_manifest),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let settings = test_settings(CheckSourcePreference::MirrorChyanFirst);

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
            MirrorChyanProvider::offline(),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let settings = test_settings(CheckSourcePreference::GithubFirst);

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
            MirrorChyanProvider::offline(),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let settings = test_settings(CheckSourcePreference::GithubFirst);

        let (result, next_state) = service
            .evaluate(&settings, &test_context(InstallKind::MacosAppBundle))
            .expect("available update should have asset url");

        assert!(result.asset_url.is_some());
        assert!(next_state.asset_url.is_some());
    }

    #[test]
    fn stores_asset_url_for_recommended_download_source() {
        let paths = test_paths("check-recommended-source-url");
        let mirror_chyan_manifest = write_manifest(&paths, "mirror.json", "1.0.5");
        let github_manifest = write_manifest(&paths, "github.json", "1.0.5");
        let service = UpdateCheckService::with_providers(
            MirrorChyanProvider::with_manifest_path(mirror_chyan_manifest),
            GithubProvider::with_manifest_path(github_manifest),
        );
        let mut settings = test_settings(CheckSourcePreference::GithubFirst);
        settings.download_source_preference = DownloadSourcePreference::MirrorChyanFirst;

        let (result, next_state) = service
            .evaluate(&settings, &test_context(InstallKind::MacosAppBundle))
            .expect("available update should have recommended asset url");

        assert_eq!(
            result.recommended_source,
            Some(DownloadSourceUsed::MirrorChyan)
        );
        assert_eq!(next_state.source, Some(DownloadSourceUsed::MirrorChyan));
        assert_eq!(
            result.asset_url.as_deref(),
            Some("https://mirrorchyan.com/resources/download/floral-notepaper-1.0.5-macos-aarch64")
        );
        assert_eq!(next_state.asset_url, result.asset_url);
    }

    #[test]
    fn merge_candidates_enriches_mirror_chyan_result_with_github_metadata() {
        let mirror_chyan_candidate = UpdateCandidate {
            priority: 0,
            version: "1.0.5".into(),
            normalized_version: Version::new(1, 0, 5),
            release_notes: None,
            mandatory: false,
            asset_name: "mirror-chyan-generated.zip".into(),
            asset_sha256: None,
            asset_size: 0,
            asset_url: Some("https://mirrorchyan.com/resources/download/floral".into()),
            mirror_chyan_asset_url: Some(
                "https://mirrorchyan.com/resources/download/floral".into(),
            ),
            github_asset_url: None,
            can_download_from_mirror_chyan: true,
            can_download_from_github: false,
            sha256_verification_enabled: false,
        };
        let github_candidate = UpdateCandidate {
            priority: 1,
            version: "1.0.5".into(),
            normalized_version: Version::new(1, 0, 5),
            release_notes: Some("GitHub notes".into()),
            mandatory: false,
            asset_name: "floral-notepaper_1.0.5_macos_aarch64_app.zip".into(),
            asset_sha256: Some("3333333333333333333333333333333333333333333333333333333333333333".into()),
            asset_size: 22345678,
            asset_url: Some("https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/floral-notepaper_1.0.5_macos_aarch64_app.zip".into()),
            mirror_chyan_asset_url: None,
            github_asset_url: Some("https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/floral-notepaper_1.0.5_macos_aarch64_app.zip".into()),
            can_download_from_mirror_chyan: false,
            can_download_from_github: true,
            sha256_verification_enabled: true,
        };

        let merged = merge_candidates(vec![mirror_chyan_candidate, github_candidate])
            .expect("same-version candidates should merge");

        assert_eq!(
            merged.asset_name,
            "floral-notepaper_1.0.5_macos_aarch64_app.zip"
        );
        assert_eq!(merged.asset_size, 22345678);
        assert!(merged.asset_sha256.is_some());
        assert!(merged.can_download_from_mirror_chyan);
        assert!(merged.can_download_from_github);
        assert!(merged.github_asset_url.is_some());
        assert!(merged.mirror_chyan_asset_url.is_some());
    }

    #[test]
    fn mirror_chyan_request_error_message_does_not_include_sensitive_url() {
        let error = Client::new()
            .get("http://127.0.0.1:0/latest?cdk=secret-token")
            .send()
            .expect_err("invalid URL should fail before network I/O");

        let message = mirror_chyan_request_error_message(&error);

        assert!(!message.contains("secret-token"));
        assert!(!message.contains("cdk="));
        assert!(!message.contains("127.0.0.1"));
    }

    #[test]
    fn manual_run_updates_last_auto_check_timestamp() {
        let paths = test_paths("check-manual-updates-last-auto-check-at");
        let github_manifest = write_manifest(&paths, "github.json", "1.0.5");
        let service = UpdateCheckService::with_providers_and_platform(
            MirrorChyanProvider::offline(),
            GithubProvider::with_manifest_path(github_manifest),
            test_platform(Os::Macos, Arch::Aarch64, InstallKind::MacosAppBundle),
        );

        service
            .run(&paths, true, "1.0.3")
            .expect("manual check should succeed");

        let saved_settings = settings::load(&paths).expect("load settings");
        assert!(saved_settings.last_auto_check_at.is_some());
    }

    #[test]
    fn run_rejects_unknown_install_kind() {
        let paths = test_paths("check-run-unknown-platform");
        let service = UpdateCheckService::with_providers_and_platform(
            MirrorChyanProvider::offline(),
            GithubProvider::offline(),
            test_platform(Os::Macos, Arch::Aarch64, InstallKind::Unknown),
        );

        let error = service
            .run(&paths, true, "1.0.3")
            .expect_err("unknown install kind should be rejected");

        assert_eq!(error.code, "updatePlatformUnsupported");
        let saved_state = state::load(&paths).expect("load failed state");
        assert_eq!(saved_state.status, UpdateStatus::Failed);
        assert_eq!(
            saved_state
                .last_error
                .as_ref()
                .and_then(|error| error.action.as_deref()),
            Some("useSupportedInstall")
        );
    }

    #[test]
    fn run_rejects_windows_portable_install_kind() {
        let paths = test_paths("check-run-portable-platform");
        let service = UpdateCheckService::with_providers_and_platform(
            MirrorChyanProvider::offline(),
            GithubProvider::offline(),
            test_platform(Os::Windows, Arch::X86_64, InstallKind::WindowsPortable),
        );

        let error = service
            .run(&paths, true, "1.0.3")
            .expect_err("portable install kind should be rejected");

        assert_eq!(error.code, "updatePortableManualOnly");
        let saved_state = state::load(&paths).expect("load failed state");
        assert_eq!(saved_state.status, UpdateStatus::Failed);
        assert_eq!(
            saved_state
                .last_error
                .as_ref()
                .and_then(|error| error.action.as_deref()),
            Some("useSupportedInstall")
        );
    }

    #[test]
    fn run_preserves_previous_available_update_when_check_fails() {
        let paths = test_paths("check-preserve-available-on-failure");
        let mut previous = UpdateStateDto::idle_with_version("1.0.3");
        previous.status = UpdateStatus::Available;
        previous.latest_version = Some("1.0.5".into());
        previous.asset_name = Some("floral-notepaper_1.0.5_macos_aarch64_app.zip".into());
        previous.asset_sha256 =
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        previous.asset_size = Some(42);
        previous.source = Some(DownloadSourceUsed::Github);
        state::save(&paths, &previous).expect("seed available state");

        let service = UpdateCheckService::with_providers_and_platform(
            MirrorChyanProvider::offline(),
            GithubProvider::offline(),
            test_platform(Os::Macos, Arch::Aarch64, InstallKind::MacosAppBundle),
        );

        let error = service
            .run(&paths, false, "1.0.3")
            .expect_err("unconfigured providers should fail");

        assert_eq!(error.code, "updateSourceNotConfigured");
        let saved_state = state::load(&paths).expect("load failed state");
        assert_eq!(saved_state.status, UpdateStatus::Failed);
        assert_eq!(saved_state.latest_version.as_deref(), Some("1.0.5"));
        assert_eq!(
            saved_state.asset_name.as_deref(),
            Some("floral-notepaper_1.0.5_macos_aarch64_app.zip")
        );
        assert_eq!(saved_state.asset_size, Some(42));
    }
}
