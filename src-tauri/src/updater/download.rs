use super::{
    errors, manifest, platform, state,
    types::{
        DownloadSourceUsed, UpdateDownloadProgressDto, UpdateDownloadResult, UpdateErrorDto,
        UpdateStateDto, UpdateStatus,
    },
    UpdatePaths,
};
use crate::services::notes::AppError;
use chrono::Utc;
use reqwest::{
    blocking::{Client, Response},
    header::LOCATION,
    redirect::Policy,
    StatusCode, Url,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

macro_rules! debug_log {
    ($($arg:tt)*) => { super::debug_log!("download", $($arg)*) };
}

const GITHUB_MANIFEST_PATH_ENV: &str = "FLORAL_NOTEPAPER_UPDATE_GITHUB_MANIFEST_PATH";
const MAX_REDIRECTS: usize = 5;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(3),
    Duration::from_secs(8),
];
const DOWNLOAD_HOST_ALLOWLIST: &[&str] = &[
    "mirrorchyan.com",
    "www.mirrorchyan.com",
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

#[derive(Debug, Clone)]
pub(crate) struct UpdateDownloadService {
    github_manifest_path: Option<PathBuf>,
    allow_insecure_localhost: bool,
    platform_override: Option<platform::PlatformInfo>,
    cdk: Option<String>,
}

#[derive(Debug, Clone)]
struct DownloadPlan {
    version: String,
    asset_name: String,
    asset_sha256: Option<String>,
    asset_size: u64,
    source: DownloadSourceUsed,
    url: Url,
    final_path: PathBuf,
    part_path: PathBuf,
    /// Whether SHA-256 hash verification is enabled for this download.
    /// Set to `false` for MirrorChyan source because the MirrorChyan API does not
    /// return asset checksums in its response.
    sha256_verification_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
struct DownloadValidationOptions {
    allow_insecure_localhost: bool,
}

impl UpdateDownloadService {
    pub(crate) fn from_env() -> Self {
        Self {
            github_manifest_path: env_manifest_path(GITHUB_MANIFEST_PATH_ENV),
            allow_insecure_localhost: false,
            platform_override: None,
            cdk: None,
        }
    }

    pub(crate) fn with_cdk(mut self, cdk: Option<String>) -> Self {
        self.cdk = cdk;
        self
    }

    #[cfg(test)]
    fn with_insecure_localhost() -> Self {
        Self {
            github_manifest_path: None,
            allow_insecure_localhost: true,
            platform_override: None,
            cdk: None,
        }
    }

    pub(crate) fn run<F>(
        &self,
        paths: &UpdatePaths,
        current_state: UpdateStateDto,
        source: Option<DownloadSourceUsed>,
        cancel_flag: Arc<AtomicBool>,
        mut emit_progress: F,
    ) -> Result<UpdateDownloadResult, AppError>
    where
        F: FnMut(UpdateDownloadProgressDto),
    {
        debug_log!("开始下载 version={:?}", current_state.latest_version);
        if let Err(error) = self.current_platform().ensure_in_app_updates_supported() {
            debug_log!("平台不支持下载 code={}", error.code);
            state::save(paths, &failed_state_without_plan(&current_state, &error))?;
            return Err(error);
        }
        let primary_source = selected_download_source(&current_state, source);
        debug_log!("下载源={:?}", primary_source);
        let primary_plan =
            match self.resolve_plan_for_source(paths, &current_state, &primary_source) {
                Ok(plan) => {
                    debug_log!(
                        "下载计划已解析 url={} source={:?} sha256_verification={}",
                        plan.url,
                        plan.source,
                        plan.sha256_verification_enabled
                    );
                    plan
                }
                Err(error) => {
                    debug_log!("下载计划解析失败 code={}", error.code);
                    if should_try_github_fallback(&primary_source, &error, &cancel_flag) {
                        debug_log!("尝试 GitHub 回退下载");
                        match self.resolve_plan_for_source(
                            paths,
                            &current_state,
                            &DownloadSourceUsed::Github,
                        ) {
                            Ok(fallback_plan) => {
                                return self
                                    .download_with_persisted_plan(
                                        paths,
                                        &current_state,
                                        &fallback_plan,
                                        cancel_flag,
                                        &mut emit_progress,
                                    )
                                    .inspect_err(|fallback_error| {
                                        let _ = state::save(
                                            paths,
                                            &failed_state(
                                                &current_state,
                                                &fallback_plan,
                                                fallback_error,
                                            ),
                                        );
                                    });
                            }
                            Err(_) => {
                                state::save(
                                    paths,
                                    &failed_state_without_plan(&current_state, &error),
                                )?;
                                return Err(error);
                            }
                        }
                    }

                    state::save(paths, &failed_state_without_plan(&current_state, &error))?;
                    return Err(error);
                }
            };

        match self.download_with_persisted_plan(
            paths,
            &current_state,
            &primary_plan,
            Arc::clone(&cancel_flag),
            &mut emit_progress,
        ) {
            Ok(result) => Ok(result),
            Err(error) => {
                if should_try_github_fallback(&primary_plan.source, &error, &cancel_flag) {
                    if let Ok(fallback_plan) = self.resolve_plan_for_source(
                        paths,
                        &current_state,
                        &DownloadSourceUsed::Github,
                    ) {
                        return self
                            .download_with_persisted_plan(
                                paths,
                                &current_state,
                                &fallback_plan,
                                cancel_flag,
                                &mut emit_progress,
                            )
                            .inspect_err(|fallback_error| {
                                let _ = state::save(
                                    paths,
                                    &failed_state(&current_state, &fallback_plan, fallback_error),
                                );
                            });
                    }
                }

                state::save(paths, &failed_state(&current_state, &primary_plan, &error))?;
                Err(error)
            }
        }
    }

    fn current_platform(&self) -> platform::PlatformInfo {
        self.platform_override
            .clone()
            .unwrap_or_else(platform::current_platform)
    }

    fn resolve_plan_for_source(
        &self,
        paths: &UpdatePaths,
        current_state: &UpdateStateDto,
        source: &DownloadSourceUsed,
    ) -> Result<DownloadPlan, AppError> {
        let version = current_state
            .latest_version
            .clone()
            .ok_or_else(|| errors::app_error("updateDownloadNotReady", "当前没有可下载的更新包"))?;
        let asset_name = current_state
            .asset_name
            .clone()
            .ok_or_else(|| errors::app_error("updateDownloadNotReady", "当前没有可下载的更新包"))?;
        let asset_size = current_state
            .asset_size
            .ok_or_else(|| errors::app_error("updateDownloadNotReady", "当前没有可下载的更新包"))?;

        if let Some(direct_url) = current_state
            .asset_url
            .as_deref()
            .filter(|url| direct_url_matches_source(url, source))
        {
            return self.resolve_direct_plan(
                paths,
                &version,
                &asset_name,
                current_state.asset_sha256.clone(),
                asset_size,
                direct_url,
                source,
            );
        }

        match source {
            DownloadSourceUsed::Github => self.resolve_github_plan(
                paths,
                &version,
                &asset_name,
                current_state.asset_sha256.as_deref(),
                asset_size,
            ),
            DownloadSourceUsed::MirrorChyan => self.resolve_mirror_chyan_plan(
                paths,
                &current_state.current_version,
                &version,
                &asset_name,
                current_state.asset_sha256.clone(),
                asset_size,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_direct_plan(
        &self,
        paths: &UpdatePaths,
        version: &str,
        asset_name: &str,
        asset_sha256: Option<String>,
        asset_size: u64,
        url: &str,
        source: &DownloadSourceUsed,
    ) -> Result<DownloadPlan, AppError> {
        let url = validate_download_url(
            url,
            DownloadValidationOptions {
                allow_insecure_localhost: self.allow_insecure_localhost,
            },
        )?;
        let version_dir = paths.downloads_dir().join(version);
        let final_path = version_dir.join(asset_name);
        let part_path = version_dir.join(format!("{asset_name}.part"));

        Ok(DownloadPlan {
            version: version.to_string(),
            asset_name: asset_name.to_string(),
            asset_sha256,
            asset_size,
            source: source.clone(),
            url,
            final_path,
            part_path,
            sha256_verification_enabled: !matches!(source, DownloadSourceUsed::MirrorChyan),
        })
    }

    fn resolve_github_plan(
        &self,
        paths: &UpdatePaths,
        version: &str,
        asset_name: &str,
        asset_sha256: Option<&str>,
        asset_size: u64,
    ) -> Result<DownloadPlan, AppError> {
        if self.github_manifest_path.is_none() {
            let platform = self.current_platform();
            let info = super::check::fetch_github_download_info(
                &platform,
                version,
                asset_name,
                (asset_size > 0).then_some(asset_size),
            )?;
            let url = validate_download_url(
                &info.url,
                DownloadValidationOptions {
                    allow_insecure_localhost: self.allow_insecure_localhost,
                },
            )?;
            let version_dir = paths.downloads_dir().join(version);
            let final_path = version_dir.join(&info.asset_name);
            let part_path = version_dir.join(format!("{}.part", info.asset_name));

            return Ok(DownloadPlan {
                version: version.to_string(),
                asset_name: info.asset_name,
                asset_sha256: info.sha256.or_else(|| asset_sha256.map(ToOwned::to_owned)),
                asset_size: info.asset_size,
                source: DownloadSourceUsed::Github,
                url,
                final_path,
                part_path,
                sha256_verification_enabled: true,
            });
        }

        let manifest_path = self
            .github_manifest_path
            .as_ref()
            .expect("manifest path checked above");
        let manifest_bytes = fs::read(manifest_path).map_err(|error| {
            let error = errors::app_error(
                "updateDownloadManifestUnreadable",
                format!("无法读取 GitHub 更新清单：{error}"),
            );
            errors::with_detail(error, "path", manifest_path.display().to_string())
        })?;
        let manifest = manifest::parse_manifest(&manifest_bytes)?;
        if manifest.version.trim() != version {
            return Err(errors::with_detail(
                errors::app_error(
                    "updateDownloadVersionMismatch",
                    "更新清单中的版本与当前待下载版本不一致",
                ),
                "expectedVersion",
                version,
            ));
        }

        let asset = manifest
            .assets
            .iter()
            .find(|candidate| candidate.name == asset_name)
            .ok_or_else(|| {
                errors::with_detail(
                    errors::app_error(
                        "updateManifestAssetNotFound",
                        "当前更新清单中找不到目标更新包",
                    ),
                    "assetName",
                    asset_name,
                )
            })?;

        if asset_sha256.is_some_and(|expected| asset.sha256 != expected)
            || (asset_size > 0 && asset.size != asset_size)
        {
            return Err(errors::with_detail(
                errors::app_error(
                    "updateDownloadAssetMismatch",
                    "更新清单中的更新包元数据与已检查结果不一致",
                ),
                "assetName",
                asset_name,
            ));
        }

        let url = validate_download_url(
            &asset.github_url,
            DownloadValidationOptions {
                allow_insecure_localhost: self.allow_insecure_localhost,
            },
        )?;
        let version_dir = paths.downloads_dir().join(version);
        let final_path = version_dir.join(asset_name);
        let part_path = version_dir.join(format!("{asset_name}.part"));

        Ok(DownloadPlan {
            version: version.to_string(),
            asset_name: asset_name.to_string(),
            asset_sha256: Some(
                asset_sha256
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| asset.sha256.clone()),
            ),
            asset_size: if asset_size > 0 {
                asset_size
            } else {
                asset.size
            },
            source: DownloadSourceUsed::Github,
            url,
            final_path,
            part_path,
            sha256_verification_enabled: true,
        })
    }

    fn resolve_mirror_chyan_plan(
        &self,
        paths: &UpdatePaths,
        current_version: &str,
        version: &str,
        asset_name: &str,
        asset_sha256: Option<String>,
        asset_size: u64,
    ) -> Result<DownloadPlan, AppError> {
        let platform = self.current_platform();
        let info = super::check::fetch_mirror_chyan_download_url(
            &platform,
            current_version,
            self.cdk.as_deref(),
        )?;

        let url = validate_download_url(
            &info.url,
            DownloadValidationOptions {
                allow_insecure_localhost: self.allow_insecure_localhost,
            },
        )?;
        let version_dir = paths.downloads_dir().join(version);
        let final_path = version_dir.join(asset_name);
        let part_path = version_dir.join(format!("{asset_name}.part"));

        Ok(DownloadPlan {
            version: version.to_string(),
            asset_name: asset_name.to_string(),
            asset_sha256,
            asset_size,
            source: DownloadSourceUsed::MirrorChyan,
            url,
            final_path,
            part_path,
            // MirrorChyan API does not provide asset SHA-256 checksums;
            // verification is explicitly disabled for MirrorChyan downloads.
            sha256_verification_enabled: false,
        })
    }

    fn download_with_persisted_plan<F>(
        &self,
        paths: &UpdatePaths,
        current_state: &UpdateStateDto,
        plan: &DownloadPlan,
        cancel_flag: Arc<AtomicBool>,
        emit_progress: &mut F,
    ) -> Result<UpdateDownloadResult, AppError>
    where
        F: FnMut(UpdateDownloadProgressDto),
    {
        let persisted_asset_url = current_state
            .asset_url
            .as_deref()
            .filter(|url| direct_url_matches_source(url, &plan.source))
            .map(ToOwned::to_owned);
        let downloading_state = UpdateStateDto {
            status: UpdateStatus::Downloading,
            current_version: current_state.current_version.clone(),
            latest_version: Some(plan.version.clone()),
            channel: current_state.channel.clone(),
            asset_name: Some(plan.asset_name.clone()),
            asset_path: None,
            asset_sha256: plan.asset_sha256.clone(),
            asset_size: Some(plan.asset_size),
            asset_url: persisted_asset_url.clone(),
            source: Some(plan.source.clone()),
            checked_at: current_state.checked_at,
            downloaded_at: None,
            install_log_path: None,
            install_mode: None,
            install_started_at: None,
            install_scheduled_at: None,
            last_error: None,
        };
        state::save(paths, &downloading_state)?;

        match self.download_with_plan(plan, cancel_flag, emit_progress) {
            Ok((asset_path, computed_sha256)) => {
                let asset_path_text = asset_path.to_string_lossy().to_string();
                let sha256_to_store = plan.asset_sha256.clone().or(computed_sha256);
                debug_log!(
                    "下载完成 version={} path={} sha256={}",
                    plan.version,
                    asset_path_text,
                    sha256_to_store.as_deref().unwrap_or("none")
                );
                let downloaded_state = UpdateStateDto {
                    status: UpdateStatus::Downloaded,
                    current_version: current_state.current_version.clone(),
                    latest_version: Some(plan.version.clone()),
                    channel: current_state.channel.clone(),
                    asset_name: Some(plan.asset_name.clone()),
                    asset_path: Some(asset_path_text.clone()),
                    asset_sha256: sha256_to_store,
                    asset_size: Some(plan.asset_size),
                    asset_url: persisted_asset_url,
                    source: Some(plan.source.clone()),
                    checked_at: downloading_state.checked_at,
                    downloaded_at: Some(Utc::now()),
                    install_log_path: None,
                    install_mode: None,
                    install_started_at: None,
                    install_scheduled_at: None,
                    last_error: None,
                };
                state::save(paths, &downloaded_state)?;

                Ok(UpdateDownloadResult {
                    status: UpdateStatus::Downloaded,
                    version: Some(plan.version.clone()),
                    asset_path: Some(asset_path_text),
                    source: Some(plan.source.clone()),
                })
            }
            Err(error) => {
                let _ = remove_file_if_exists(&plan.part_path);
                Err(error)
            }
        }
    }

    fn download_with_plan<F>(
        &self,
        plan: &DownloadPlan,
        cancel_flag: Arc<AtomicBool>,
        mut emit_progress: F,
    ) -> Result<(PathBuf, Option<String>), AppError>
    where
        F: FnMut(UpdateDownloadProgressDto),
    {
        debug_log!(
            "执行下载计划 asset={} 大小={}",
            plan.asset_name,
            plan.asset_size
        );
        if let Some(parent) = plan.final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if plan.final_path.exists() {
            debug_log!("下载文件已存在，验证中 path={}", plan.final_path.display());
            if let Some(computed) = verify_existing_file(plan)? {
                debug_log!("已存在文件验证通过，跳过下载");
                emit_progress(progress_payload(
                    plan,
                    plan.asset_size,
                    Some(plan.asset_size),
                    progress_speed(plan.asset_size, Instant::now()),
                ));
                return Ok((plan.final_path.clone(), computed));
            }
            remove_file_if_exists(&plan.final_path)?;
        }

        remove_file_if_exists(&plan.part_path)?;

        let mut attempt = 0usize;
        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                let _ = remove_file_if_exists(&plan.part_path);
                return Err(download_cancelled());
            }

            match self.download_attempt(plan, &cancel_flag, &mut emit_progress) {
                Ok(path) => return Ok(path),
                Err(error) if should_retry(&error) && attempt < RETRY_DELAYS.len() => {
                    debug_log!(
                        "下载重试 attempt={} 延迟={:?}",
                        attempt + 1,
                        RETRY_DELAYS[attempt]
                    );
                    let _ = remove_file_if_exists(&plan.part_path);
                    thread::sleep(RETRY_DELAYS[attempt]);
                    attempt += 1;
                }
                Err(error) => {
                    debug_log!("下载失败 code={}", error.code);
                    let _ = remove_file_if_exists(&plan.part_path);
                    return Err(error);
                }
            }
        }
    }

    fn download_attempt<F>(
        &self,
        plan: &DownloadPlan,
        cancel_flag: &Arc<AtomicBool>,
        emit_progress: &mut F,
    ) -> Result<(PathBuf, Option<String>), AppError>
    where
        F: FnMut(UpdateDownloadProgressDto),
    {
        let client = build_http_client()?;
        let mut response = fetch_response(
            &client,
            plan.url.clone(),
            DownloadValidationOptions {
                allow_insecure_localhost: self.allow_insecure_localhost,
            },
        )?;
        // Resolve the actual filename from the final URL after redirects.
        let final_path = resolved_final_path(plan, response.url());
        let content_length = response.content_length();
        debug_log!(
            "重定向后 url={} 文件={} Content-Length={:?}",
            response.url(),
            final_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?"),
            content_length
        );
        let expected_size = if plan.asset_size > 0 {
            if let Some(length) = content_length {
                if length != plan.asset_size {
                    return Err(size_mismatch_error(plan.asset_size, length));
                }
            }
            plan.asset_size
        } else {
            content_length.unwrap_or(0)
        };
        let total_for_progress = if expected_size > 0 {
            Some(expected_size)
        } else {
            None
        };

        emit_progress(progress_payload(plan, 0, total_for_progress, 0));

        let part_path = part_path_for(&final_path);
        if let Some(parent) = part_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let part_file = fs::File::create(&part_path)?;
        let mut writer = BufWriter::new(part_file);
        let mut hasher = Sha256::new();
        let started_at = Instant::now();
        let mut last_emit_at = Instant::now();
        let mut downloaded_bytes = 0u64;
        let mut buffer = [0u8; 64 * 1024];

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(download_cancelled());
            }

            let read = response
                .read(&mut buffer)
                .map_err(download_network_read_error)?;
            if read == 0 {
                break;
            }

            writer.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
            downloaded_bytes += read as u64;

            if last_emit_at.elapsed() >= PROGRESS_INTERVAL {
                emit_progress(progress_payload(
                    plan,
                    downloaded_bytes,
                    total_for_progress,
                    progress_speed(downloaded_bytes, started_at),
                ));
                last_emit_at = Instant::now();
            }
        }

        writer.flush()?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(download_cancelled());
        }

        if plan.asset_size > 0 && downloaded_bytes != plan.asset_size {
            return Err(size_mismatch_error(plan.asset_size, downloaded_bytes));
        }

        let actual_sha256 = format!("{:x}", hasher.finalize());
        // MirrorChyan API does not provide asset SHA-256 checksums, so verification
        // is explicitly disabled for MirrorChyan-sourced downloads. When verification
        // is enabled and no expected hash exists, that is a hard error because
        // it should never happen for GitHub-sourced releases.
        let computed = if let Some(ref expected) = plan.asset_sha256 {
            if actual_sha256 != *expected {
                return Err(hash_mismatch_error(expected, &actual_sha256));
            }
            None
        } else if !plan.sha256_verification_enabled {
            debug_log!(
                "SHA-256 verification is disabled for MirrorChyan source \
                 downloads (MirrorChyan API does not provide asset checksums)"
            );
            None
        } else {
            return Err(errors::app_error(
                "updateDownloadMissingSha256",
                "缺少 SHA-256 校验值，无法验证下载文件完整性",
            ));
        };

        if final_path != plan.final_path {
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::rename(&part_path, &final_path)?;
        emit_progress(progress_payload(
            plan,
            downloaded_bytes,
            total_for_progress.or(Some(downloaded_bytes)),
            progress_speed(downloaded_bytes, started_at),
        ));

        Ok((final_path, computed))
    }
}

pub(crate) fn cleanup_partial_downloads(paths: &UpdatePaths) -> Result<(), AppError> {
    cleanup_partial_downloads_in_dir(&paths.downloads_dir())
}

fn cleanup_partial_downloads_in_dir(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_dir() {
            cleanup_partial_downloads_in_dir(&entry_path)?;
            continue;
        }

        if entry_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("part")
        {
            remove_file_if_exists(&entry_path)?;
        }
    }

    Ok(())
}

use super::check::env_manifest_path;

fn validate_download_url(
    raw_url: &str,
    options: DownloadValidationOptions,
) -> Result<Url, AppError> {
    let url = Url::parse(raw_url).map_err(|error| {
        errors::with_detail(
            errors::app_error(
                "updateDownloadUrlInvalid",
                format!("下载地址格式无效：{error}"),
            ),
            "url",
            raw_url.split('?').next().unwrap_or(raw_url),
        )
    })?;
    let host = url
        .host_str()
        .ok_or_else(|| errors::app_error("updateDownloadUrlInvalid", "下载地址缺少主机名"))?;

    let scheme_allowed = url.scheme() == "https"
        || (options.allow_insecure_localhost && url.scheme() == "http" && is_localhost(host));
    if !scheme_allowed {
        return Err(errors::with_detail(
            errors::app_error("updateDownloadUrlInvalid", "下载地址必须使用 HTTPS"),
            "url",
            sanitize_url(&url),
        ));
    }

    if !is_host_allowed(host, options.allow_insecure_localhost) {
        return Err(errors::with_detail(
            errors::app_error("updateDownloadUrlNotAllowed", "下载地址不在允许列表中"),
            "url",
            sanitize_url(&url),
        ));
    }

    Ok(url)
}

fn is_host_allowed(host: &str, allow_insecure_localhost: bool) -> bool {
    DOWNLOAD_HOST_ALLOWLIST
        .iter()
        .any(|allowed| host.eq_ignore_ascii_case(allowed))
        || is_mirror_chyan_download_host(host)
        || (allow_insecure_localhost && is_localhost(host))
}

fn is_localhost(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn selected_download_source(
    current_state: &UpdateStateDto,
    requested_source: Option<DownloadSourceUsed>,
) -> DownloadSourceUsed {
    requested_source
        .or_else(|| current_state.source.clone())
        .unwrap_or(DownloadSourceUsed::Github)
}

fn direct_url_matches_source(raw_url: &str, source: &DownloadSourceUsed) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };

    match source {
        DownloadSourceUsed::MirrorChyan => is_mirror_chyan_download_host(host),
        DownloadSourceUsed::Github => is_github_download_host(host),
    }
}

fn is_mirror_chyan_download_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "mirrorchyan.com" || host == "www.mirrorchyan.com" || host.ends_with(".mirrorchyan.com")
}

fn is_github_download_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "github.com"
        || host == "objects.githubusercontent.com"
        || host == "release-assets.githubusercontent.com"
        || host.ends_with(".github.com")
}

fn should_try_github_fallback(
    source: &DownloadSourceUsed,
    error: &AppError,
    cancel_flag: &Arc<AtomicBool>,
) -> bool {
    *source == DownloadSourceUsed::MirrorChyan
        && error.code != "updateDownloadCancelled"
        && !cancel_flag.load(Ordering::Relaxed)
}

fn sanitize_url(url: &Url) -> String {
    format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str().unwrap_or(""),
        url.path()
    )
}

fn resolved_final_path(plan: &DownloadPlan, effective_url: &Url) -> PathBuf {
    let filename = effective_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|name| !name.is_empty())
        .unwrap_or(&plan.asset_name);
    plan.final_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(filename)
}

fn part_path_for(final_path: &Path) -> PathBuf {
    let mut name = final_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download")
        .to_string();
    name.push_str(".part");
    final_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(name)
}

fn build_http_client() -> Result<Client, AppError> {
    Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .redirect(Policy::limited(10))
        .build()
        .map_err(|error| {
            errors::app_error(
                "updateDownloadClientUnavailable",
                format!("无法创建下载客户端：{error}"),
            )
        })
}

fn fetch_response(
    client: &Client,
    mut url: Url,
    options: DownloadValidationOptions,
) -> Result<Response, AppError> {
    for _ in 0..=MAX_REDIRECTS {
        let response = client
            .get(url.clone())
            .send()
            .map_err(download_request_error)?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    errors::app_error(
                        "updateDownloadRedirectInvalid",
                        "下载地址返回了无效的重定向目标",
                    )
                })?;
            let next_url = url.join(location).map_err(|error| {
                errors::app_error(
                    "updateDownloadRedirectInvalid",
                    format!("无法解析重定向地址：{error}"),
                )
            })?;
            url = validate_download_url(next_url.as_str(), options)?;
            continue;
        }

        if !response.status().is_success() {
            return Err(download_http_status_error(response.status(), &url));
        }

        return Ok(response);
    }

    Err(errors::app_error(
        "updateDownloadRedirectLoop",
        "下载地址重定向次数过多",
    ))
}

fn verify_existing_file(plan: &DownloadPlan) -> Result<Option<Option<String>>, AppError> {
    let metadata = fs::metadata(&plan.final_path)?;
    if plan.asset_size > 0 && metadata.len() != plan.asset_size {
        return Ok(None);
    }

    let mut reader = BufReader::new(fs::File::open(&plan.final_path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let actual = format!("{:x}", hasher.finalize());
    // MirrorChyan API does not provide asset SHA-256 checksums, so verification
    // is explicitly disabled for MirrorChyan-sourced downloads. When verification
    // is enabled and no expected hash exists, that is a hard error because
    // it should never happen for GitHub-sourced releases.
    if let Some(ref expected) = plan.asset_sha256 {
        if actual != *expected {
            return Ok(None);
        }
        Ok(Some(None))
    } else if !plan.sha256_verification_enabled {
        Ok(Some(None))
    } else {
        Err(errors::app_error(
            "updateDownloadMissingSha256",
            "缺少 SHA-256 校验值，无法验证下载文件完整性",
        ))
    }
}

fn progress_payload(
    plan: &DownloadPlan,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    bytes_per_second: u64,
) -> UpdateDownloadProgressDto {
    let percent = total_bytes
        .filter(|total| *total > 0)
        .map(|total| ((downloaded_bytes as f64 / total as f64) * 100.0).min(100.0));

    UpdateDownloadProgressDto {
        version: plan.version.clone(),
        asset_name: plan.asset_name.clone(),
        downloaded_bytes,
        total_bytes,
        percent,
        bytes_per_second,
        source: plan.source.clone(),
    }
}

fn progress_speed(downloaded_bytes: u64, started_at: Instant) -> u64 {
    let elapsed = started_at.elapsed().as_secs_f64();
    if elapsed <= f64::EPSILON {
        0
    } else {
        (downloaded_bytes as f64 / elapsed).round() as u64
    }
}

fn remove_file_if_exists(path: &Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn download_request_error(error: reqwest::Error) -> AppError {
    let code = if error.is_timeout() {
        "updateDownloadTimeout"
    } else {
        "updateDownloadNetwork"
    };
    errors::app_error(code, format!("下载请求失败：{error}"))
}

fn download_network_read_error(error: std::io::Error) -> AppError {
    errors::app_error(
        "updateDownloadNetwork",
        format!("下载过程中网络中断：{error}"),
    )
}

fn download_http_status_error(status: StatusCode, url: &Url) -> AppError {
    let error = errors::with_detail(
        errors::app_error(
            "updateDownloadHttpStatus",
            format!("下载请求失败（HTTP {}）", status.as_u16()),
        ),
        "statusCode",
        status.as_u16().to_string(),
    );
    errors::with_detail(error, "url", sanitize_url(url))
}

fn size_mismatch_error(expected_size: u64, actual_size: u64) -> AppError {
    let error = errors::with_detail(
        errors::app_error("updateDownloadSizeMismatch", "下载文件大小校验失败"),
        "expectedSize",
        expected_size.to_string(),
    );
    errors::with_detail(error, "actualSize", actual_size.to_string())
}

fn hash_mismatch_error(expected_hash: &str, actual_hash: &str) -> AppError {
    let error = errors::with_detail(
        errors::app_error("updateDownloadHashMismatch", "下载文件哈希校验失败"),
        "expectedSha256",
        expected_hash,
    );
    errors::with_detail(error, "actualSha256", actual_hash)
}

fn download_cancelled() -> AppError {
    errors::app_error("updateDownloadCancelled", "下载已取消")
}

fn should_retry(error: &AppError) -> bool {
    match error.code.as_str() {
        "updateDownloadNetwork" | "updateDownloadTimeout" => true,
        "updateDownloadHttpStatus" => error
            .details
            .get("statusCode")
            .and_then(|value| value.parse::<u16>().ok())
            .is_some_and(|status| matches!(status, 408 | 429 | 500..=599)),
        _ => false,
    }
}

fn download_failure_action(code: &str) -> Option<String> {
    match code {
        "updateDownloadCancelled" => Some("retryDownload".to_string()),
        "updateDownloadUrlInvalid"
        | "updateDownloadUrlNotAllowed"
        | "updateDownloadManifestUnavailable"
        | "updateDownloadManifestUnreadable" => Some("configureUpdateSource".to_string()),
        "updatePlatformUnsupported" | "updatePortableManualOnly" => {
            Some("useSupportedInstall".to_string())
        }
        _ => Some("retryDownload".to_string()),
    }
}

fn failed_state(
    current_state: &UpdateStateDto,
    plan: &DownloadPlan,
    error: &AppError,
) -> UpdateStateDto {
    let action = download_failure_action(&error.code);

    let status = if error.code == "updateDownloadCancelled" {
        UpdateStatus::Available
    } else {
        UpdateStatus::Failed
    };

    UpdateStateDto {
        status,
        current_version: current_state.current_version.clone(),
        latest_version: Some(plan.version.clone()),
        channel: current_state.channel.clone(),
        asset_name: Some(plan.asset_name.clone()),
        asset_path: None,
        asset_sha256: plan.asset_sha256.clone(),
        asset_size: Some(plan.asset_size),
        asset_url: current_state.asset_url.clone(),
        source: Some(plan.source.clone()),
        checked_at: current_state.checked_at,
        downloaded_at: None,
        install_log_path: None,
        install_mode: None,
        install_started_at: None,
        install_scheduled_at: None,
        last_error: Some(UpdateErrorDto::recoverable(
            error.code.clone(),
            error.message.clone(),
            action,
        )),
    }
}

fn failed_state_without_plan(current_state: &UpdateStateDto, error: &AppError) -> UpdateStateDto {
    UpdateStateDto {
        status: UpdateStatus::Failed,
        current_version: current_state.current_version.clone(),
        latest_version: current_state.latest_version.clone(),
        channel: current_state.channel.clone(),
        asset_name: current_state.asset_name.clone(),
        asset_path: None,
        asset_sha256: current_state.asset_sha256.clone(),
        asset_size: current_state.asset_size,
        asset_url: current_state.asset_url.clone(),
        source: current_state.source.clone(),
        checked_at: current_state.checked_at,
        downloaded_at: None,
        install_log_path: None,
        install_mode: None,
        install_started_at: None,
        install_scheduled_at: None,
        last_error: Some(UpdateErrorDto::recoverable(
            error.code.clone(),
            error.message.clone(),
            download_failure_action(&error.code),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::{
        platform::{Arch, Os, PlatformInfo},
        types::{InstallKind, UpdateChannel},
    };
    use std::{net::TcpListener, sync::mpsc};

    fn test_paths(name: &str) -> UpdatePaths {
        let root = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("remove stale test dir");
        }
        UpdatePaths::new(root)
    }

    fn test_plan(paths: &UpdatePaths, name: &str, bytes: &[u8], url: &str) -> DownloadPlan {
        let hash = format!("{:x}", Sha256::digest(bytes));
        let version_dir = paths.downloads_dir().join("1.0.5");
        DownloadPlan {
            version: "1.0.5".into(),
            asset_name: name.into(),
            asset_sha256: Some(hash),
            asset_size: bytes.len() as u64,
            source: DownloadSourceUsed::Github,
            url: validate_download_url(
                url,
                DownloadValidationOptions {
                    allow_insecure_localhost: true,
                },
            )
            .expect("test url"),
            final_path: version_dir.join(name),
            part_path: version_dir.join(format!("{name}.part")),
            sha256_verification_enabled: true,
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

    fn available_state(asset_name: &str, body: &[u8]) -> UpdateStateDto {
        UpdateStateDto {
            status: UpdateStatus::Available,
            current_version: "1.0.3".into(),
            latest_version: Some("1.0.5".into()),
            channel: UpdateChannel::Stable,
            asset_name: Some(asset_name.into()),
            asset_path: None,
            asset_sha256: Some(format!("{:x}", Sha256::digest(body))),
            asset_size: Some(body.len() as u64),
            asset_url: None,
            source: Some(DownloadSourceUsed::Github),
            checked_at: Some(Utc::now()),
            downloaded_at: None,
            install_log_path: None,
            install_mode: None,
            install_started_at: None,
            install_scheduled_at: None,
            last_error: None,
        }
    }

    fn spawn_http_server(
        body: Vec<u8>,
        chunk_size: usize,
        chunk_delay: Duration,
    ) -> (String, mpsc::Receiver<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server addr");
        let (done_tx, done_rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            if stream.write_all(header.as_bytes()).is_ok() {
                for chunk in body.chunks(chunk_size.max(1)) {
                    if stream.write_all(chunk).is_err() {
                        break;
                    }
                    let _ = stream.flush();
                    if !chunk_delay.is_zero() {
                        thread::sleep(chunk_delay);
                    }
                }
            }

            let _ = done_tx.send(());
        });

        (format!("http://{address}/asset.bin"), done_rx)
    }

    #[test]
    fn downloads_asset_and_emits_progress() {
        let paths = test_paths("download-success");
        let body = vec![42u8; 32 * 1024];
        let (url, done_rx) = spawn_http_server(body.clone(), 8 * 1024, Duration::ZERO);
        let plan = test_plan(&paths, "asset.zip", &body, &url);
        let service = UpdateDownloadService::with_insecure_localhost();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let mut progress = Vec::new();

        let (final_path, _) = service
            .download_with_plan(&plan, cancel_flag, |payload| progress.push(payload))
            .expect("download success");

        done_rx.recv().expect("server finished");
        assert!(final_path.exists());
        assert!(!plan.part_path.exists());
        assert_eq!(progress.first().map(|item| item.downloaded_bytes), Some(0));
        assert_eq!(progress.last().and_then(|item| item.percent), Some(100.0));
    }

    #[test]
    fn deletes_partial_file_on_hash_mismatch() {
        let paths = test_paths("download-hash-mismatch");
        let body = b"real payload".to_vec();
        let (url, done_rx) = spawn_http_server(body, 1024, Duration::ZERO);
        let plan = test_plan(&paths, "asset.zip", b"fake payload", &url);
        let service = UpdateDownloadService::with_insecure_localhost();
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let error = service
            .download_with_plan(&plan, cancel_flag, |_| {})
            .expect_err("hash mismatch should fail");

        done_rx.recv().expect("server finished");
        assert_eq!(error.code, "updateDownloadHashMismatch");
        assert!(!plan.part_path.exists());
        assert!(!plan.final_path.exists());
    }

    #[test]
    fn cancels_download_and_cleans_partial_file() {
        let paths = test_paths("download-cancel");
        let body = vec![7u8; 256 * 1024];
        let (url, done_rx) = spawn_http_server(body.clone(), 8 * 1024, Duration::from_millis(20));
        let plan = test_plan(&paths, "asset.zip", &body, &url);
        let service = UpdateDownloadService::with_insecure_localhost();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_handle = Arc::clone(&cancel_flag);
        let expected_final_path = plan.final_path.clone();

        let result = thread::spawn(move || service.download_with_plan(&plan, cancel_flag, |_| {}));
        thread::sleep(Duration::from_millis(60));
        cancel_handle.store(true, Ordering::Relaxed);

        let error = result
            .join()
            .expect("download thread should join")
            .expect_err("cancel should fail");

        done_rx.recv().expect("server finished");
        assert_eq!(error.code, "updateDownloadCancelled");
        assert!(!expected_final_path.exists());
    }

    #[test]
    fn reuses_verified_existing_download_in_run() {
        let paths = test_paths("download-reuse-existing");
        let asset_name = "asset.zip";
        let body = b"reusable payload";
        let final_path = paths.downloads_dir().join("1.0.5").join(asset_name);
        fs::create_dir_all(final_path.parent().expect("download dir")).expect("create dir");
        fs::write(&final_path, body).expect("write asset");
        let sha256 = format!("{:x}", Sha256::digest(body));

        let manifest_path = paths.root_dir().join("manifest.json");
        let manifest = format!(
            r#"{{
  "schemaVersion": 1,
  "appId": "com.floral-notepaper.app",
  "productName": "花笺",
  "channel": "stable",
  "version": "1.0.5",
  "tag": "v1.0.5",
  "publishedAt": "2026-05-26T12:00:00Z",
  "assets": [
    {{
      "os": "macos",
      "arch": "aarch64",
      "kind": "app_zip",
      "name": "{asset_name}",
      "sha256": "{sha256}",
      "size": {size},
      "githubUrl": "https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/{asset_name}"
    }}
  ]
}}"#,
            sha256 = sha256,
            size = body.len()
        );
        fs::write(&manifest_path, manifest).expect("write manifest");

        let service = UpdateDownloadService {
            github_manifest_path: Some(manifest_path),
            allow_insecure_localhost: false,
            platform_override: Some(test_platform(
                Os::Macos,
                Arch::Aarch64,
                InstallKind::MacosAppBundle,
            )),
            cdk: None,
        };
        let current_state = available_state(asset_name, body);

        let result = service
            .run(
                &paths,
                current_state,
                Some(DownloadSourceUsed::Github),
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
            .expect("reuse existing download");

        assert_eq!(result.status, UpdateStatus::Downloaded);
        assert_eq!(
            result.asset_path.as_deref(),
            Some(final_path.to_string_lossy().as_ref())
        );
        let saved_state = state::load(&paths).expect("load saved state");
        assert_eq!(saved_state.status, UpdateStatus::Downloaded);
        assert_eq!(
            saved_state.asset_path.as_deref(),
            Some(final_path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn github_plan_ignores_mirror_chyan_direct_url_from_state() {
        let paths = test_paths("download-github-plan-ignores-mirror-chyan-url");
        let asset_name = "asset.zip";
        let body = b"payload";
        let sha256 = format!("{:x}", Sha256::digest(body));
        let manifest_path = paths.root_dir().join("manifest.json");
        let manifest = format!(
            r#"{{
  "schemaVersion": 1,
  "appId": "com.floral-notepaper.app",
  "productName": "花笺",
  "channel": "stable",
  "version": "1.0.5",
  "tag": "v1.0.5",
  "publishedAt": "2026-05-26T12:00:00Z",
  "assets": [
    {{
      "os": "macos",
      "arch": "aarch64",
      "kind": "app_zip",
      "name": "{asset_name}",
      "sha256": "{sha256}",
      "size": {size},
      "githubUrl": "https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/{asset_name}",
      "mirrorChyanUrl": "https://mirrorchyan.com/resources/download/floral-notepaper-1.0.5-macos-aarch64"
    }}
  ]
}}"#,
            sha256 = sha256,
            size = body.len()
        );
        fs::create_dir_all(paths.root_dir()).expect("create root");
        fs::write(&manifest_path, manifest).expect("write manifest");

        let service = UpdateDownloadService {
            github_manifest_path: Some(manifest_path),
            allow_insecure_localhost: false,
            platform_override: Some(test_platform(
                Os::Macos,
                Arch::Aarch64,
                InstallKind::MacosAppBundle,
            )),
            cdk: None,
        };
        let mut current_state = available_state(asset_name, body);
        current_state.source = Some(DownloadSourceUsed::Github);
        current_state.asset_url =
            Some("https://mirrorchyan.com/resources/download/floral-notepaper-1.0.5".into());

        let plan = service
            .resolve_plan_for_source(&paths, &current_state, &DownloadSourceUsed::Github)
            .expect("github plan should come from manifest, not mirror direct url");

        assert_eq!(plan.source, DownloadSourceUsed::Github);
        assert_eq!(plan.url.host_str(), Some("github.com"));
    }

    #[test]
    fn mirror_chyan_resolution_failure_falls_back_to_existing_github_download() {
        let paths = test_paths("download-mirror-chyan-fallback-github-existing");
        let asset_name = "asset.zip";
        let body = b"fallback payload";
        let sha256 = format!("{:x}", Sha256::digest(body));
        let manifest_path = paths.root_dir().join("manifest.json");
        let manifest = format!(
            r#"{{
  "schemaVersion": 1,
  "appId": "com.floral-notepaper.app",
  "productName": "花笺",
  "channel": "stable",
  "version": "1.0.5",
  "tag": "v1.0.5",
  "publishedAt": "2026-05-26T12:00:00Z",
  "assets": [
    {{
      "os": "macos",
      "arch": "aarch64",
      "kind": "app_zip",
      "name": "{asset_name}",
      "sha256": "{sha256}",
      "size": {size},
      "githubUrl": "https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/{asset_name}"
    }}
  ]
}}"#,
            sha256 = sha256,
            size = body.len()
        );
        fs::create_dir_all(paths.root_dir()).expect("create root");
        fs::write(&manifest_path, manifest).expect("write manifest");
        let final_path = paths.downloads_dir().join("1.0.5").join(asset_name);
        fs::create_dir_all(final_path.parent().expect("download dir")).expect("create dir");
        fs::write(&final_path, body).expect("write existing github asset");

        let service = UpdateDownloadService {
            github_manifest_path: Some(manifest_path),
            allow_insecure_localhost: false,
            platform_override: Some(test_platform(
                Os::Macos,
                Arch::Aarch64,
                InstallKind::MacosAppBundle,
            )),
            cdk: None,
        };
        let mut current_state = available_state(asset_name, body);
        current_state.source = Some(DownloadSourceUsed::MirrorChyan);
        current_state.asset_url = Some("ftp://mirrorchyan.com/resources/download/bad".into());

        let result = service
            .run(
                &paths,
                current_state,
                None,
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
            .expect("mirror failure should fall back to existing github asset");

        assert_eq!(result.status, UpdateStatus::Downloaded);
        assert_eq!(result.source, Some(DownloadSourceUsed::Github));
        let saved_state = state::load(&paths).expect("load saved state");
        assert_eq!(saved_state.status, UpdateStatus::Downloaded);
        assert_eq!(saved_state.source, Some(DownloadSourceUsed::Github));
        assert_eq!(
            saved_state.asset_path.as_deref(),
            Some(final_path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn direct_url_source_detection_distinguishes_hosts() {
        assert!(direct_url_matches_source(
            "https://mirrorchyan.com/resources/download/floral",
            &DownloadSourceUsed::MirrorChyan
        ));
        assert!(!direct_url_matches_source(
            "https://mirrorchyan.com/resources/download/floral",
            &DownloadSourceUsed::Github
        ));
        assert!(direct_url_matches_source(
            "https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/asset.zip",
            &DownloadSourceUsed::Github
        ));
        assert!(!direct_url_matches_source(
            "https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/asset.zip",
            &DownloadSourceUsed::MirrorChyan
        ));
    }

    #[test]
    fn run_rejects_unknown_install_kind() {
        let paths = test_paths("download-run-unknown-platform");
        let service = UpdateDownloadService {
            github_manifest_path: None,
            allow_insecure_localhost: false,
            platform_override: Some(test_platform(
                Os::Macos,
                Arch::Aarch64,
                InstallKind::Unknown,
            )),
            cdk: None,
        };

        let error = service
            .run(
                &paths,
                available_state("asset.zip", b"payload"),
                Some(DownloadSourceUsed::Github),
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
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
        let paths = test_paths("download-run-portable-platform");
        let service = UpdateDownloadService {
            github_manifest_path: None,
            allow_insecure_localhost: false,
            platform_override: Some(test_platform(
                Os::Windows,
                Arch::X86_64,
                InstallKind::WindowsPortable,
            )),
            cdk: None,
        };

        let error = service
            .run(
                &paths,
                available_state("asset.zip", b"payload"),
                Some(DownloadSourceUsed::Github),
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
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
    fn cleanup_partial_downloads_removes_stale_part_files() {
        let paths = test_paths("download-cleanup-partials");
        let part_path = paths.downloads_dir().join("1.0.5").join("asset.zip.part");
        fs::create_dir_all(part_path.parent().expect("partial dir")).expect("create dir");
        fs::write(&part_path, "partial").expect("write partial");

        cleanup_partial_downloads(&paths).expect("cleanup partials");

        assert!(!part_path.exists());
    }
}
