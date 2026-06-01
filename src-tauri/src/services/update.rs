use crate::services::notes::{AppConfig, UpdateInfo};
use once_cell::sync::Lazy;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/Achilng/floral-notepaper/releases/latest";
const KK_GITHUB_RELEASES_URL: &str =
    "https://api.kkgithub.com/repos/Achilng/floral-notepaper/releases/latest";
const OWNER: &str = "Achilng";
const REPO: &str = "floral-notepaper";
static RELEASE_ASSET_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"floral-notepaper_(\d+\.)+exe").unwrap());

#[derive(Debug, Clone, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GitHubAsset {
    name: String,
    #[serde(rename = "browser_download_url")]
    _browser_download_url: String,
    digest: Option<String>, // GitHub API 返回的 SHA256
}

fn matches_release_asset_pattern(name: &str) -> bool {
    RELEASE_ASSET_PATTERN.is_match(name)
}

pub async fn check_for_updates(config: &AppConfig) -> Result<UpdateInfo, String> {
    // 手动检查更新时不检查配置项，只在启动时自动检查才受配置项限制

    if !cfg!(target_os = "windows") {
        return Err("仅支持 Windows 系统".to_string());
    }

    let client = Client::new();

    let releases_url = build_releases_url(
        config.github_owner.as_deref(),
        config.github_repo.as_deref(),
    );

    // 首先尝试主源
    let release = match fetch_release(&client, &releases_url).await {
        Ok(release) => release,
        Err(_) => {
            // 主源失败，如果是默认仓库，尝试镜像源
            let is_default_repo = config.github_owner.is_none() && config.github_repo.is_none();
            if is_default_repo {
                let kk_releases_url = KK_GITHUB_RELEASES_URL;
                fetch_release(&client, kk_releases_url).await?
            } else {
                return Err("获取发布信息失败，请检查网络连接".to_string());
            }
        }
    };

    let version = release.tag_name.trim_start_matches('v').to_string();
    let release_url = release.html_url;

    let exe_asset = release
        .assets
        .iter()
        .find(|asset| matches_release_asset_pattern(&asset.name))
        .ok_or_else(|| "未找到匹配的可执行文件".to_string())?;

    let latest_sha256 = fetch_asset_sha256(exe_asset).await.unwrap_or_default();
    let current_sha256 = calculate_current_exe_sha256()?;

    let has_update = latest_sha256 != current_sha256;

    Ok(UpdateInfo {
        version,
        release_url,
        latest_sha256,
        current_sha256,
        has_update,
    })
}

pub async fn check_for_updates_with_url(
    _config: &AppConfig,
    api_url: &str,
) -> Result<UpdateInfo, String> {
    // 手动检查更新时不检查配置项，只在启动时自动检查才受配置项限制

    if !cfg!(target_os = "windows") {
        return Err("仅支持 Windows 系统".to_string());
    }

    let client = Client::new();

    // 使用提供的 API URL 获取发布信息
    let release = fetch_release(&client, api_url).await?;

    let version = release.tag_name.trim_start_matches('v').to_string();
    let release_url = release.html_url;

    let exe_asset = release
        .assets
        .iter()
        .find(|asset| matches_release_asset_pattern(&asset.name))
        .ok_or_else(|| "未找到匹配的可执行文件".to_string())?;

    let latest_sha256 = fetch_asset_sha256(exe_asset).await.unwrap_or_default();
    let current_sha256 = calculate_current_exe_sha256()?;

    let has_update = latest_sha256 != current_sha256;

    Ok(UpdateInfo {
        version,
        release_url,
        latest_sha256,
        current_sha256,
        has_update,
    })
}

async fn fetch_release(client: &Client, url: &str) -> Result<GitHubRelease, String> {
    let response = client
        .get(url)
        .header("User-Agent", "floral-notepaper-update-checker")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("获取响应体失败：{}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API 错误：{}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("解析 JSON 响应失败：{}", e))
}

fn build_releases_url(owner: Option<&str>, repo: Option<&str>) -> String {
    let owner = owner.unwrap_or(OWNER);
    let repo = repo.unwrap_or(REPO);

    if owner == OWNER && repo == REPO {
        GITHUB_RELEASES_URL.to_string()
    } else {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        )
    }
}

async fn fetch_asset_sha256(asset: &GitHubAsset) -> Result<String, String> {
    // 直接使用 GitHub API 返回的 digest 信息
    asset
        .digest
        .clone()
        .ok_or_else(|| "GitHub API 未返回 digest 信息".to_string())
}

fn calculate_current_exe_sha256() -> Result<String, String> {
    let exe_path = env::current_exe().map_err(|e| format!("获取当前可执行文件路径失败：{}", e))?;

    let mut file =
        fs::File::open(&exe_path).map_err(|e| format!("读取当前可执行文件失败：{}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("读取当前可执行文件失败：{}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

pub fn get_exe_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|e| format!("获取可执行文件路径失败：{}", e))
}
