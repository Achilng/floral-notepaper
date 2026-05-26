use super::{types::InstallKind, version, APP_ID};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};

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
}

pub fn current_platform() -> PlatformInfo {
    let current_exe = env::current_exe().ok();
    PlatformInfo {
        os: current_os(),
        arch: current_arch(),
        app_version: version::CURRENT_APP_VERSION.to_string(),
        app_id: APP_ID.into(),
        install_kind: detect_install_kind(current_os(), current_exe.as_deref()),
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

fn find_macos_app_bundle(exe: &Path) -> Option<PathBuf> {
    let mut current = exe.parent();
    while let Some(path) = current {
        if path.extension().and_then(|ext| ext.to_str()) == Some("app") {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
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
    fn treats_unbundled_macos_binary_as_unknown_install_kind() {
        let install_kind = detect_install_kind(
            Os::Macos,
            Some(Path::new("/Users/test/dev/floral-notepaper")),
        );

        assert_eq!(install_kind, InstallKind::Unknown);
    }
}
