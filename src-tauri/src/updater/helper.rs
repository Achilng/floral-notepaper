use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
pub const HELPER_BINARY_NAME: &str = "floral-notepaper-update-helper.exe";
#[cfg(not(target_os = "windows"))]
pub const HELPER_BINARY_NAME: &str = "floral-notepaper-update-helper";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateHelperMode {
    DryRun,
    Test,
}

impl UpdateHelperMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Test => "test",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "dry-run" => Some(Self::DryRun),
            "test" => Some(Self::Test),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateHelperCommand {
    pub mode: UpdateHelperMode,
    pub asset_path: PathBuf,
    pub asset_sha256: String,
    pub asset_size: u64,
    pub target_path: PathBuf,
    pub log_path: PathBuf,
    pub current_version: String,
    pub target_version: String,
}

impl UpdateHelperCommand {
    pub fn to_args(&self) -> Vec<OsString> {
        vec![
            OsString::from("--mode"),
            OsString::from(self.mode.as_str()),
            OsString::from("--asset-path"),
            self.asset_path.clone().into_os_string(),
            OsString::from("--asset-sha256"),
            OsString::from(self.asset_sha256.clone()),
            OsString::from("--asset-size"),
            OsString::from(self.asset_size.to_string()),
            OsString::from("--target-path"),
            self.target_path.clone().into_os_string(),
            OsString::from("--log-path"),
            self.log_path.clone().into_os_string(),
            OsString::from("--current-version"),
            OsString::from(self.current_version.clone()),
            OsString::from("--target-version"),
            OsString::from(self.target_version.clone()),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum UpdateHelperExitCode {
    Success = 0,
    InvalidArguments = 2,
    AssetMissing = 3,
    AssetSizeMismatch = 4,
    AssetHashMismatch = 5,
    TargetMissing = 6,
    LogWriteFailed = 7,
}

impl UpdateHelperExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

pub fn run_cli<I, S>(args: I) -> UpdateHelperExitCode
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let command = match parse_args(args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("{message}");
            return UpdateHelperExitCode::InvalidArguments;
        }
    };

    match execute(&command) {
        Ok(()) => UpdateHelperExitCode::Success,
        Err(code) => code,
    }
}

pub fn parse_args<I, S>(args: I) -> Result<UpdateHelperCommand, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut values = BTreeMap::new();
    let mut iter = args.into_iter().map(Into::into);

    while let Some(flag) = iter.next() {
        let flag = flag
            .into_string()
            .map_err(|_| "helper arguments must be valid UTF-8".to_string())?;
        if !flag.starts_with("--") {
            return Err(format!("unexpected positional argument: {flag}"));
        }
        if values.contains_key(&flag) {
            return Err(format!("duplicate argument: {flag}"));
        }

        let value = iter
            .next()
            .ok_or_else(|| format!("missing value for argument: {flag}"))?
            .into_string()
            .map_err(|_| format!("argument value for {flag} must be valid UTF-8"))?;
        values.insert(flag, value);
    }

    let mode = UpdateHelperMode::parse(required_arg(&values, "--mode")?)
        .ok_or_else(|| "invalid value for --mode".to_string())?;
    let asset_path = PathBuf::from(required_arg(&values, "--asset-path")?);
    let asset_sha256 = required_arg(&values, "--asset-sha256")?
        .trim()
        .to_lowercase();
    if asset_sha256.len() != 64 || !asset_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid value for --asset-sha256".to_string());
    }

    let asset_size = required_arg(&values, "--asset-size")?
        .trim()
        .parse::<u64>()
        .map_err(|_| "invalid value for --asset-size".to_string())?;
    let target_path = PathBuf::from(required_arg(&values, "--target-path")?);
    let log_path = PathBuf::from(required_arg(&values, "--log-path")?);
    let current_version = require_text(values.get("--current-version"), "--current-version")?;
    let target_version = require_text(values.get("--target-version"), "--target-version")?;

    for key in values.keys() {
        if !matches!(
            key.as_str(),
            "--mode"
                | "--asset-path"
                | "--asset-sha256"
                | "--asset-size"
                | "--target-path"
                | "--log-path"
                | "--current-version"
                | "--target-version"
        ) {
            return Err(format!("unknown argument: {key}"));
        }
    }

    Ok(UpdateHelperCommand {
        mode,
        asset_path,
        asset_sha256,
        asset_size,
        target_path,
        log_path,
        current_version,
        target_version,
    })
}

pub fn execute(command: &UpdateHelperCommand) -> Result<(), UpdateHelperExitCode> {
    let mut log = open_log(&command.log_path)?;
    write_log_header(&mut log, command)?;

    if !command.target_path.exists() {
        write_log_line(
            &mut log,
            &format!("target missing: {}", command.target_path.display()),
        )?;
        return Err(UpdateHelperExitCode::TargetMissing);
    }

    let metadata = match fs::metadata(&command.asset_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            write_log_line(
                &mut log,
                &format!("asset missing: {} ({error})", command.asset_path.display()),
            )?;
            return Err(UpdateHelperExitCode::AssetMissing);
        }
    };

    if metadata.len() != command.asset_size {
        write_log_line(
            &mut log,
            &format!(
                "asset size mismatch: expected {}, actual {}",
                command.asset_size,
                metadata.len()
            ),
        )?;
        return Err(UpdateHelperExitCode::AssetSizeMismatch);
    }

    let actual_hash = match sha256_hex(&command.asset_path) {
        Ok(hash) => hash,
        Err(error) => {
            write_log_line(
                &mut log,
                &format!(
                    "asset missing: failed to read {} ({error})",
                    command.asset_path.display()
                ),
            )?;
            return Err(UpdateHelperExitCode::AssetMissing);
        }
    };

    if actual_hash != command.asset_sha256 {
        write_log_line(
            &mut log,
            &format!(
                "asset hash mismatch: expected {}, actual {}",
                command.asset_sha256, actual_hash
            ),
        )?;
        return Err(UpdateHelperExitCode::AssetHashMismatch);
    }

    write_log_line(
        &mut log,
        &format!(
            "validated {} request from {} to {} without replacing the current installation",
            command.mode.as_str(),
            command.current_version,
            command.target_version
        ),
    )?;
    Ok(())
}

fn required_arg<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))
}

fn require_text(value: Option<&String>, key: &str) -> Result<String, String> {
    let value = value
        .map(String::as_str)
        .ok_or_else(|| format!("missing required argument: {key}"))?
        .trim();
    if value.is_empty() {
        return Err(format!("argument cannot be empty: {key}"));
    }
    Ok(value.to_string())
}

fn open_log(path: &Path) -> Result<File, UpdateHelperExitCode> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| UpdateHelperExitCode::LogWriteFailed)?;
    }

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| UpdateHelperExitCode::LogWriteFailed)
}

fn write_log_header(
    file: &mut File,
    command: &UpdateHelperCommand,
) -> Result<(), UpdateHelperExitCode> {
    write_log_line(file, "floral-notepaper update helper")?;
    write_log_line(file, &format!("mode={}", command.mode.as_str()))?;
    write_log_line(file, &format!("asset={}", command.asset_path.display()))?;
    write_log_line(file, &format!("target={}", command.target_path.display()))?;
    Ok(())
}

fn write_log_line(file: &mut File, line: &str) -> Result<(), UpdateHelperExitCode> {
    writeln!(file, "{line}").map_err(|_| UpdateHelperExitCode::LogWriteFailed)
}

fn sha256_hex(path: &Path) -> Result<String, std::io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }

    let bytes = digest.finalize();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(nibble_to_hex(byte >> 4));
        hex.push(nibble_to_hex(byte & 0x0f));
    }
    Ok(hex)
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir()
            .join("floral-notepaper-updater-tests")
            .join(format!("{name}-{unique}"));
        fs::create_dir_all(&root).expect("create temp dir");
        root
    }

    fn helper_command(root: &Path) -> UpdateHelperCommand {
        let asset_path = root.join("asset.bin");
        fs::write(&asset_path, b"hello helper").expect("write asset");

        let target_path = root.join("target.app");
        fs::create_dir_all(&target_path).expect("create target");

        UpdateHelperCommand {
            mode: UpdateHelperMode::Test,
            asset_sha256: sha256_hex(&asset_path).expect("hash asset"),
            asset_size: fs::metadata(&asset_path).expect("asset metadata").len(),
            log_path: root.join("helper.log"),
            asset_path,
            target_path,
            current_version: "1.0.4".into(),
            target_version: "1.0.5".into(),
        }
    }

    #[test]
    fn parses_strict_arguments() {
        let args: Vec<OsString> = vec![
            OsString::from("--mode"),
            OsString::from("dry-run"),
            OsString::from("--asset-path"),
            OsString::from("/tmp/asset.zip"),
            OsString::from("--asset-sha256"),
            OsString::from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            OsString::from("--asset-size"),
            OsString::from("42"),
            OsString::from("--target-path"),
            OsString::from("/Applications/Floral Notepaper.app"),
            OsString::from("--log-path"),
            OsString::from("/tmp/helper.log"),
            OsString::from("--current-version"),
            OsString::from("1.0.4"),
            OsString::from("--target-version"),
            OsString::from("1.0.5"),
        ];

        let parsed = parse_args(args).expect("parse helper args");

        assert_eq!(parsed.mode, UpdateHelperMode::DryRun);
        assert_eq!(parsed.asset_size, 42);
        assert_eq!(parsed.current_version, "1.0.4");
    }

    #[test]
    fn rejects_duplicate_arguments() {
        let args: Vec<OsString> = vec![
            OsString::from("--mode"),
            OsString::from("dry-run"),
            OsString::from("--mode"),
            OsString::from("test"),
        ];

        let error = parse_args(args).expect_err("duplicate args should fail");

        assert!(error.contains("duplicate argument"));
    }

    #[test]
    fn validates_test_mode_request() {
        let root = temp_dir("helper-success");
        let command = helper_command(&root);

        execute(&command).expect("helper should validate request");
        assert!(command.log_path.exists());
    }

    #[test]
    fn returns_size_mismatch_exit_code() {
        let root = temp_dir("helper-size-mismatch");
        let mut command = helper_command(&root);
        command.asset_size += 1;

        let exit_code = execute(&command).expect_err("size mismatch should fail");

        assert_eq!(exit_code, UpdateHelperExitCode::AssetSizeMismatch);
    }

    #[test]
    fn returns_hash_mismatch_exit_code() {
        let root = temp_dir("helper-hash-mismatch");
        let mut command = helper_command(&root);
        command.asset_sha256 =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();

        let exit_code = execute(&command).expect_err("hash mismatch should fail");

        assert_eq!(exit_code, UpdateHelperExitCode::AssetHashMismatch);
    }

    #[test]
    fn returns_target_missing_exit_code() {
        let root = temp_dir("helper-target-missing");
        let mut command = helper_command(&root);
        command.target_path = root.join("missing-target.app");

        let exit_code = execute(&command).expect_err("missing target should fail");

        assert_eq!(exit_code, UpdateHelperExitCode::TargetMissing);
    }

    #[test]
    fn run_cli_maps_parse_errors() {
        let exit_code = run_cli(vec![OsString::from("--mode"), OsString::from("invalid")]);

        assert_eq!(exit_code, UpdateHelperExitCode::InvalidArguments);
    }
}
