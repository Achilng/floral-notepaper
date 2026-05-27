use crate::services::notes::AppError;
use std::collections::BTreeMap;

pub fn app_error(code: impl Into<String>, message: impl Into<String>) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        details: BTreeMap::new(),
    }
}

pub fn with_detail(
    mut error: AppError,
    key: impl Into<String>,
    value: impl Into<String>,
) -> AppError {
    error.details.insert(key.into(), value.into());
    error
}

pub fn secure_store_unavailable(error: impl std::fmt::Display) -> AppError {
    app_error(
        "updateSecureStoreUnavailable",
        format!("系统安全存储不可用：{error}"),
    )
}

pub fn invalid_version(input: &str, error: impl std::fmt::Display) -> AppError {
    let trimmed = input.trim();
    let app_error = with_detail(
        app_error("updateVersionInvalid", format!("版本号格式无效：{trimmed}")),
        "input",
        trimmed,
    );
    with_detail(app_error, "reason", error.to_string())
}

pub fn invalid_manifest(error: impl std::fmt::Display) -> AppError {
    app_error(
        "updateManifestInvalid",
        format!("更新清单格式无效：{error}"),
    )
}

pub fn manifest_asset_not_found() -> AppError {
    app_error("updateManifestAssetNotFound", "当前平台没有匹配的更新包")
}

pub fn unsupported_platform() -> AppError {
    app_error(
        "updatePlatformUnsupported",
        "当前平台或安装形态暂不支持应用内更新",
    )
}

pub fn provider_not_configured(provider: &str) -> AppError {
    with_detail(
        app_error(
            "updateProviderNotConfigured",
            format!("{provider} 更新源尚未配置测试清单"),
        ),
        "provider",
        provider,
    )
}

pub fn source_not_configured() -> AppError {
    app_error(
        "updateSourceNotConfigured",
        "更新源尚未配置，当前阶段仅支持本地测试清单注入",
    )
}

pub fn github_api_error(message: impl Into<String>) -> AppError {
    app_error(
        "updateGithubApi",
        format!("GitHub API 请求失败：{}", message.into()),
    )
}

pub fn github_rate_limited() -> AppError {
    app_error("updateGithubRateLimited", "GitHub API 频率限制，请稍后重试")
}

pub fn github_release_no_assets() -> AppError {
    app_error("updateGithubNoAssets", "GitHub Release 中没有找到可用资产")
}
