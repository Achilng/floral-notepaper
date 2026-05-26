use super::{
    errors,
    platform::{Arch, Os, PlatformInfo},
    types::{InstallKind, UpdateChannel},
    version, APP_ID,
};
use crate::services::notes::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManifestAssetKind {
    Nsis,
    PortableExe,
    AppZip,
}

impl ManifestAssetKind {
    fn from_install_kind(os: &Os, install_kind: &InstallKind) -> Result<Self, AppError> {
        match (os, install_kind) {
            (Os::Windows, InstallKind::WindowsNsis) => Ok(Self::Nsis),
            (Os::Windows, InstallKind::WindowsPortable) => Ok(Self::PortableExe),
            (Os::Macos, InstallKind::MacosAppBundle) => Ok(Self::AppZip),
            _ => Err(errors::unsupported_platform()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifestAsset {
    pub os: Os,
    pub arch: Arch,
    pub kind: ManifestAssetKind,
    pub name: String,
    pub sha256: String,
    pub size: u64,
    pub github_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub schema_version: u32,
    pub app_id: String,
    pub product_name: String,
    pub channel: UpdateChannel,
    pub version: String,
    pub tag: String,
    pub published_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_supported_version: Option<String>,
    #[serde(default)]
    pub mandatory: bool,
    #[serde(default)]
    pub allow_downgrade: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,
    pub assets: Vec<UpdateManifestAsset>,
}

impl UpdateManifest {
    pub fn normalized_version(&self) -> Result<semver::Version, AppError> {
        version::normalize_version(&self.version)
    }

    pub fn normalized_minimum_supported_version(
        &self,
    ) -> Result<Option<semver::Version>, AppError> {
        self.minimum_supported_version
            .as_deref()
            .map(version::normalize_version)
            .transpose()
    }

    fn validate(&self) -> Result<(), AppError> {
        if self.schema_version != 1 {
            let error = errors::with_detail(
                errors::app_error(
                    "updateManifestUnsupportedSchema",
                    "更新清单 schemaVersion 暂不受支持",
                ),
                "schemaVersion",
                self.schema_version.to_string(),
            );
            return Err(error);
        }

        if self.app_id.trim() != APP_ID {
            let error = errors::with_detail(
                errors::app_error(
                    "updateManifestAppIdMismatch",
                    "更新清单 appId 与当前应用不匹配",
                ),
                "appId",
                self.app_id.trim(),
            );
            return Err(error);
        }

        if self.product_name.trim().is_empty() || self.tag.trim().is_empty() {
            return Err(errors::invalid_manifest("productName 或 tag 不能为空"));
        }

        self.normalized_version()?;
        let _ = self.normalized_minimum_supported_version()?;

        if self.assets.is_empty() {
            return Err(errors::app_error(
                "updateManifestMissingAssets",
                "更新清单未包含任何可下载资产",
            ));
        }

        for asset in &self.assets {
            validate_asset(asset)?;
        }

        Ok(())
    }
}

pub fn parse_manifest(manifest_bytes: &[u8]) -> Result<UpdateManifest, AppError> {
    let manifest: UpdateManifest =
        serde_json::from_slice(manifest_bytes).map_err(errors::invalid_manifest)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn select_asset(
    manifest: &UpdateManifest,
    platform: &PlatformInfo,
    install_kind: InstallKind,
) -> Result<UpdateManifestAsset, AppError> {
    if !platform.supports_update_assets() {
        return Err(errors::unsupported_platform());
    }

    let expected_kind = ManifestAssetKind::from_install_kind(&platform.os, &install_kind)?;
    manifest
        .assets
        .iter()
        .find(|asset| {
            asset.os == platform.os && asset.arch == platform.arch && asset.kind == expected_kind
        })
        .cloned()
        .ok_or_else(|| {
            let error = errors::with_detail(
                errors::manifest_asset_not_found(),
                "installKind",
                format!("{install_kind:?}"),
            );
            let error = errors::with_detail(error, "os", format!("{:?}", platform.os));
            errors::with_detail(error, "arch", format!("{:?}", platform.arch))
        })
}

fn validate_asset(asset: &UpdateManifestAsset) -> Result<(), AppError> {
    if asset.name.trim().is_empty() {
        return Err(errors::invalid_manifest("asset.name 不能为空"));
    }

    if asset.size == 0 {
        return Err(errors::invalid_manifest(format!(
            "asset.size 必须大于 0：{}",
            asset.name
        )));
    }

    if asset.sha256.len() != 64 || !asset.sha256.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(errors::invalid_manifest(format!(
            "asset.sha256 无效：{}",
            asset.name
        )));
    }

    if !asset.github_url.starts_with("https://") {
        return Err(errors::invalid_manifest(format!(
            "asset.githubUrl 必须使用 https：{}",
            asset.name
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST_BYTES: &[u8] = include_bytes!("fixtures/update-manifest.valid.json");

    fn platform(os: Os, arch: Arch, install_kind: InstallKind) -> PlatformInfo {
        PlatformInfo {
            os,
            arch,
            app_version: "1.0.4".into(),
            app_id: APP_ID.into(),
            install_kind,
            current_exe: None,
            current_app_bundle: None,
        }
    }

    #[test]
    fn parses_manifest_fixture() {
        let manifest = parse_manifest(VALID_MANIFEST_BYTES).expect("parse valid manifest");

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.app_id, APP_ID);
        assert_eq!(manifest.version, "1.0.5");
        assert_eq!(manifest.assets.len(), 4);
    }

    #[test]
    fn rejects_unsupported_schema() {
        let invalid = r#"{
          "schemaVersion": 2,
          "appId": "com.floral-notepaper.app",
          "productName": "花笺",
          "channel": "stable",
          "version": "1.0.5",
          "tag": "v1.0.5",
          "publishedAt": "2026-05-26T12:00:00Z",
          "assets": [
            {
              "os": "macos",
              "arch": "aarch64",
              "kind": "app_zip",
              "name": "asset.zip",
              "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
              "size": 1,
              "githubUrl": "https://github.com/Achilng/floral-notepaper/releases/download/v1.0.5/asset.zip"
            }
          ]
        }"#;

        let error = parse_manifest(invalid.as_bytes()).expect_err("schemaVersion 2 must fail");

        assert_eq!(error.code, "updateManifestUnsupportedSchema");
    }

    #[test]
    fn selects_windows_nsis_asset() {
        let manifest = parse_manifest(VALID_MANIFEST_BYTES).expect("parse valid manifest");
        let asset = select_asset(
            &manifest,
            &platform(Os::Windows, Arch::X86_64, InstallKind::WindowsNsis),
            InstallKind::WindowsNsis,
        )
        .expect("select nsis asset");

        assert_eq!(asset.name, "floral-notepaper_1.0.5_windows_x64_nsis.exe");
    }

    #[test]
    fn selects_windows_portable_asset() {
        let manifest = parse_manifest(VALID_MANIFEST_BYTES).expect("parse valid manifest");
        let asset = select_asset(
            &manifest,
            &platform(Os::Windows, Arch::X86_64, InstallKind::WindowsPortable),
            InstallKind::WindowsPortable,
        )
        .expect("select portable asset");

        assert_eq!(
            asset.name,
            "floral-notepaper_1.0.5_windows_x64_portable.exe"
        );
    }

    #[test]
    fn selects_macos_aarch64_asset() {
        let manifest = parse_manifest(VALID_MANIFEST_BYTES).expect("parse valid manifest");
        let asset = select_asset(
            &manifest,
            &platform(Os::Macos, Arch::Aarch64, InstallKind::MacosAppBundle),
            InstallKind::MacosAppBundle,
        )
        .expect("select macos asset");

        assert_eq!(asset.name, "floral-notepaper_1.0.5_macos_aarch64_app.zip");
    }

    #[test]
    fn rejects_unknown_install_kind_during_asset_selection() {
        let manifest = parse_manifest(VALID_MANIFEST_BYTES).expect("parse valid manifest");
        let error = select_asset(
            &manifest,
            &platform(Os::Macos, Arch::Aarch64, InstallKind::Unknown),
            InstallKind::Unknown,
        )
        .expect_err("unknown install kind must fail");

        assert_eq!(error.code, "updatePlatformUnsupported");
    }
}
