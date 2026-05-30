use super::{errors, types::UpdateChannel};
use crate::services::notes::AppError;
use semver::Version;

pub const CURRENT_APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn current_version() -> Result<Version, AppError> {
    normalize_version(CURRENT_APP_VERSION)
}

pub fn normalize_version(input: &str) -> Result<Version, AppError> {
    let trimmed = input.trim();
    let normalized = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    Version::parse(normalized).map_err(|error| errors::invalid_version(trimmed, error))
}

pub fn allows_prerelease(channel: &UpdateChannel, allow_prerelease: bool) -> bool {
    allow_prerelease || matches!(channel, UpdateChannel::Beta)
}

pub fn is_newer_version(current: &Version, candidate: &Version, allow_prerelease: bool) -> bool {
    candidate > current && (candidate.pre.is_empty() || allow_prerelease)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_versions_with_v_prefix() {
        assert_eq!(
            normalize_version("v1.2.3").expect("normalize with v"),
            Version::new(1, 2, 3)
        );
        assert_eq!(
            normalize_version("V1.2.3").expect("normalize with V"),
            Version::new(1, 2, 3)
        );
    }

    #[test]
    fn rejects_invalid_versions() {
        let error = normalize_version("version-one").expect_err("invalid version");

        assert_eq!(error.code, "updateVersionInvalid");
        assert_eq!(
            error.details.get("input").map(String::as_str),
            Some("version-one")
        );
    }

    #[test]
    fn prerelease_follows_channel_or_setting() {
        let stable_candidate = normalize_version("1.0.5").expect("stable candidate");
        let prerelease_candidate = normalize_version("1.0.5-beta.1").expect("beta candidate");
        let current = normalize_version("1.0.3").expect("current");

        assert!(is_newer_version(&current, &stable_candidate, false));
        assert!(!is_newer_version(&current, &prerelease_candidate, false));
        assert!(is_newer_version(&current, &prerelease_candidate, true));
        assert!(allows_prerelease(&UpdateChannel::Beta, false));
        assert!(!allows_prerelease(&UpdateChannel::Stable, false));
    }

    #[test]
    fn parses_current_app_version() {
        let current = current_version().expect("current version");

        assert_eq!(current.to_string(), CURRENT_APP_VERSION);
    }
}
