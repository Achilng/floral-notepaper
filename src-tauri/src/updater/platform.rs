use super::{errors, types::InstallKind, version, APP_ID};
use crate::services::notes::AppError;
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Os {
    Windows,
    Macos,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Arch {
    X86_64,
    Aarch64,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub os: Os,
    pub arch: Arch,
    pub app_version: String,
    pub app_id: String,
    pub install_kind: InstallKind,
    pub current_exe: Option<String>,
    pub current_app_bundle: Option<String>,
}

impl PlatformInfo {
    pub fn supports_update_assets(&self) -> bool {
        self.os != Os::Unsupported
            && self.arch != Arch::Unsupported
            && self.install_kind != InstallKind::Unknown
    }

    pub fn ensure_in_app_updates_supported(&self) -> Result<(), AppError> {
        if self.os == Os::Unsupported || self.arch == Arch::Unsupported {
            return Err(errors::unsupported_platform());
        }

        // When running via `npm run tauri dev` the binary sits inside
        // target/debug/ (or target/debug/deps/) rather than inside an app
        // bundle, so `detect_install_kind` returns `Unknown`.  Allow the
        // check-update flow to proceed in this case so the feature can be
        // tested during development.
        if self.install_kind == InstallKind::Unknown {
            if let Some(path) = &self.current_exe {
                let lower = path.to_lowercase();
                if lower.contains("target/debug") || lower.contains("target\\debug") {
                    return Ok(());
                }
            }
        }

        match self.install_kind {
            InstallKind::WindowsNsis | InstallKind::MacosAppBundle => Ok(()),
            InstallKind::WindowsPortable => Err(errors::portable_manual_only()),
            InstallKind::Unknown => Err(errors::unsupported_platform()),
        }
    }
}

pub fn current_platform() -> PlatformInfo {
    current_platform_with_version(version::CURRENT_APP_VERSION.to_string())
}

pub fn current_platform_with_version(app_version: impl Into<String>) -> PlatformInfo {
    let current_exe = env::current_exe().ok();
    let os = current_os();
    PlatformInfo {
        os: os.clone(),
        arch: current_arch(),
        app_version: app_version.into(),
        app_id: APP_ID.into(),
        install_kind: detect_install_kind(os, current_exe.as_deref()),
        current_exe: current_exe
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        current_app_bundle: current_exe
            .as_ref()
            .and_then(|path| find_macos_app_bundle(path.as_path()))
            .map(|path| path.to_string_lossy().to_string()),
    }
}

fn current_os() -> Os {
    match env::consts::OS {
        "windows" => Os::Windows,
        "macos" => Os::Macos,
        _ => Os::Unsupported,
    }
}

fn current_arch() -> Arch {
    match env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        _ => Arch::Unsupported,
    }
}

fn detect_install_kind(os: Os, current_exe: Option<&Path>) -> InstallKind {
    match os {
        Os::Macos => {
            if current_exe.and_then(find_macos_app_bundle).is_some() {
                InstallKind::MacosAppBundle
            } else {
                InstallKind::Unknown
            }
        }
        Os::Windows => {
            let Some(path) = current_exe else {
                return InstallKind::Unknown;
            };
            if registry_reports_windows_nsis(path) {
                return InstallKind::WindowsNsis;
            }
            let normalized = path.to_string_lossy().to_lowercase();
            if normalized.contains("\\program files\\")
                || normalized.contains("\\program files (x86)\\")
                || normalized.contains("\\appdata\\local\\programs\\")
            {
                InstallKind::WindowsNsis
            } else {
                InstallKind::WindowsPortable
            }
        }
        Os::Unsupported => InstallKind::Unknown,
    }
}

#[cfg(target_os = "windows")]
fn registry_reports_windows_nsis(current_exe: &Path) -> bool {
    const ROOTS: [&str; 4] = [
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    let exe_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let current_exe_normalized = normalize_windows_path(&current_exe.to_string_lossy());
    let install_dir_normalized = current_exe
        .parent()
        .map(|path| normalize_windows_path(&path.to_string_lossy()))
        .unwrap_or_default();

    ROOTS.iter().any(|root| {
        // Use raw_arg to prevent Rust from escaping inner double quotes —
        // cmd.exe does not understand backslash-escaped quotes.
        let raw = format!(
            r#"/c chcp 65001 >nul & "{}" query {} /s /f "{}""#,
            reg_exe_path().display(),
            root,
            exe_name
        );
        Command::new("cmd")
            .raw_arg(&raw)
            .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .is_some_and(|output| {
                registry_output_matches_installation(
                    &output,
                    &current_exe_normalized,
                    &install_dir_normalized,
                )
            })
    })
}

#[cfg(not(target_os = "windows"))]
fn registry_reports_windows_nsis(_current_exe: &Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn reg_exe_path() -> PathBuf {
    env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32").join("reg.exe"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\System32\reg.exe"))
}

#[cfg(target_os = "windows")]
fn registry_output_matches_installation(
    output: &str,
    current_exe_normalized: &str,
    install_dir_normalized: &str,
) -> bool {
    output.lines().any(|line| {
        let normalized = normalize_windows_path(line);
        normalized.contains(current_exe_normalized)
            || (!install_dir_normalized.is_empty()
                && normalized.contains(install_dir_normalized)
                && (normalized.contains("displayicon")
                    || normalized.contains("installdir")
                    || normalized.contains("installlocation")
                    || normalized.contains("uninstallstring")))
    })
}

#[cfg(target_os = "windows")]
pub(crate) fn normalize_windows_path(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
}

pub(crate) fn find_macos_app_bundle(exe: &Path) -> Option<PathBuf> {
    let mut current = exe.parent();
    while let Some(path) = current {
        if path.extension().and_then(|ext| ext.to_str()) == Some("app") {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct InferredAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub os: Os,
    pub arch: Arch,
    pub kind: InstallKind,
}

pub(crate) fn infer_asset_from_filename(name: &str, url: &str, size: u64) -> Option<InferredAsset> {
    let lower = name.to_lowercase();

    let arch = if lower.contains("aarch64") || lower.contains("arm64") {
        Arch::Aarch64
    } else if lower.contains("x64") || lower.contains("x86_64") {
        Arch::X86_64
    } else {
        return None;
    };

    let (os, kind) = if lower.ends_with(".dmg") {
        (Os::Macos, InstallKind::MacosAppBundle)
    } else if lower.ends_with(".msi") || lower.ends_with("-setup.exe") || lower.contains("setup") {
        (Os::Windows, InstallKind::WindowsNsis)
    } else if lower.ends_with(".exe") {
        (Os::Windows, InstallKind::WindowsPortable)
    } else {
        return None;
    };

    Some(InferredAsset {
        name: name.to_string(),
        url: url.to_string(),
        size,
        os,
        arch,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_windows_nsis_installation() {
        let install_kind = detect_install_kind(
            Os::Windows,
            Some(Path::new(
                r"C:\Program Files\Floral Notepaper\floral-notepaper.exe",
            )),
        );

        assert_eq!(install_kind, InstallKind::WindowsNsis);
    }

    #[test]
    fn detects_windows_portable_installation() {
        let install_kind = detect_install_kind(
            Os::Windows,
            Some(Path::new(r"D:\Apps\Floral\floral-notepaper.exe")),
        );

        assert_eq!(install_kind, InstallKind::WindowsPortable);
    }

    #[test]
    fn detects_macos_app_bundle() {
        let bundle = find_macos_app_bundle(Path::new(
            "/Applications/Floral Notepaper.app/Contents/MacOS/floral-notepaper",
        ));

        assert_eq!(
            bundle,
            Some(PathBuf::from("/Applications/Floral Notepaper.app"))
        );
        assert_eq!(
            detect_install_kind(
                Os::Macos,
                Some(Path::new(
                    "/Applications/Floral Notepaper.app/Contents/MacOS/floral-notepaper",
                )),
            ),
            InstallKind::MacosAppBundle
        );
    }

    #[test]
    fn infers_macos_aarch64_dmg() {
        let asset = infer_asset_from_filename(
            "floral-notepaper_1.0.5_aarch64.dmg",
            "https://github.com/example/releases/download/v1.0.5/app.dmg",
            5000,
        );
        let asset = asset.expect("should match dmg");
        assert_eq!(asset.os, Os::Macos);
        assert_eq!(asset.arch, Arch::Aarch64);
        assert_eq!(asset.kind, InstallKind::MacosAppBundle);
        assert_eq!(asset.size, 5000);
    }

    #[test]
    fn infers_windows_x64_msi() {
        let asset = infer_asset_from_filename(
            "floral-notepaper_1.0.5_x64_en-US.msi",
            "https://github.com/example/releases/download/v1.0.5/app.msi",
            8000,
        );
        let asset = asset.expect("should match msi");
        assert_eq!(asset.os, Os::Windows);
        assert_eq!(asset.arch, Arch::X86_64);
        assert_eq!(asset.kind, InstallKind::WindowsNsis);
    }

    #[test]
    fn infers_windows_x64_setup_exe() {
        let asset = infer_asset_from_filename(
            "floral-notepaper_1.0.5_x64-setup.exe",
            "https://github.com/example/releases/download/v1.0.5/setup.exe",
            9000,
        );
        let asset = asset.expect("should match setup exe");
        assert_eq!(asset.os, Os::Windows);
        assert_eq!(asset.arch, Arch::X86_64);
        assert_eq!(asset.kind, InstallKind::WindowsNsis);
    }

    #[test]
    fn rejects_unknown_filename() {
        assert!(
            infer_asset_from_filename("README.md", "https://example.com/readme", 100).is_none()
        );
        assert!(
            infer_asset_from_filename("app_1.0.5.deb", "https://example.com/app.deb", 100)
                .is_none()
        );
    }

    #[test]
    fn treats_unbundled_macos_binary_as_unknown_install_kind() {
        let install_kind = detect_install_kind(
            Os::Macos,
            Some(Path::new("/Users/test/dev/floral-notepaper")),
        );

        assert_eq!(install_kind, InstallKind::Unknown);
    }

    #[test]
    fn rejects_unknown_install_kind_for_in_app_updates() {
        let platform = PlatformInfo {
            os: Os::Macos,
            arch: Arch::Aarch64,
            app_version: "1.0.3".into(),
            app_id: APP_ID.into(),
            install_kind: InstallKind::Unknown,
            current_exe: None,
            current_app_bundle: None,
        };

        let error = platform
            .ensure_in_app_updates_supported()
            .expect_err("unknown installs should not support in-app updates");

        assert_eq!(error.code, "updatePlatformUnsupported");
    }

    #[test]
    fn rejects_windows_portable_for_in_app_updates() {
        let platform = PlatformInfo {
            os: Os::Windows,
            arch: Arch::X86_64,
            app_version: "1.0.3".into(),
            app_id: APP_ID.into(),
            install_kind: InstallKind::WindowsPortable,
            current_exe: None,
            current_app_bundle: None,
        };

        let error = platform
            .ensure_in_app_updates_supported()
            .expect_err("portable installs should not support in-app updates");

        assert_eq!(error.code, "updatePortableManualOnly");
    }
}
