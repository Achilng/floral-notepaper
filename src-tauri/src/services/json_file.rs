use serde::Serialize;
use std::{
    error::Error,
    fmt, fs, io,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};
use uuid::Uuid;

static METADATA_JSON_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CONFIG_JSON_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug)]
pub enum JsonFileError {
    Io(io::Error),
    Json(serde_json::Error),
    LockPoisoned(&'static str),
}

impl fmt::Display for JsonFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::LockPoisoned(name) => write!(f, "{name} lock was poisoned"),
        }
    }
}

impl Error for JsonFileError {}

impl From<io::Error> for JsonFileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for JsonFileError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn metadata_json_guard() -> Result<MutexGuard<'static, ()>, JsonFileError> {
    METADATA_JSON_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| JsonFileError::LockPoisoned("metadata json"))
}

pub fn config_json_guard() -> Result<MutexGuard<'static, ()>, JsonFileError> {
    CONFIG_JSON_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| JsonFileError::LockPoisoned("config json"))
}

pub fn recover_json_after_interrupted_write(path: &Path) -> Result<(), JsonFileError> {
    if path.exists() {
        return Ok(());
    }

    let backup_path = backup_path(path);
    if backup_path.exists() {
        fs::copy(&backup_path, path)?;
    }

    Ok(())
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), JsonFileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = unique_temp_path(path);
    let backup_path = backup_path(path);
    let json = serde_json::to_string_pretty(value)?;

    write_all_and_sync(&temp_path, json.as_bytes())?;

    if path.exists() {
        let _ = fs::remove_file(&backup_path);
        fs::copy(path, &backup_path)?;
        fs::remove_file(path)?;
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        restore_backup_if_missing(path, &backup_path);
        return Err(JsonFileError::Io(error));
    }

    let _ = fs::remove_file(&backup_path);
    Ok(())
}

fn write_all_and_sync(path: &Path, bytes: &[u8]) -> Result<(), JsonFileError> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn restore_backup_if_missing(path: &Path, backup_path: &Path) {
    if !path.exists() && backup_path.exists() {
        let _ = fs::copy(backup_path, path);
    }
}

fn unique_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "json".into());
    path.with_file_name(format!("{file_name}.{}.tmp", Uuid::new_v4()))
}

fn backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "json".into());
    path.with_file_name(format!("{file_name}.bak"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct TestJson {
        value: String,
    }

    fn test_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir()
            .join("floral-notepaper-json-file-tests")
            .join(format!("{name}-{nonce}"));
        fs::create_dir_all(&root).expect("create test root");
        root
    }

    #[test]
    fn writes_json_and_removes_backup_after_success() {
        let root = test_root("write");
        let path = root.join("metadata.json");

        write_json_atomic(
            &path,
            &TestJson {
                value: "first".into(),
            },
        )
        .expect("write first");
        write_json_atomic(
            &path,
            &TestJson {
                value: "second".into(),
            },
        )
        .expect("write second");

        let loaded: TestJson =
            serde_json::from_str(&fs::read_to_string(&path).expect("read written json"))
                .expect("parse written json");
        assert_eq!(loaded.value, "second");
        assert!(!backup_path(&path).exists());
        assert!(!fs::read_dir(&root)
            .expect("read test root")
            .any(|entry| entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
    }

    #[test]
    fn restores_backup_when_json_is_missing() {
        let root = test_root("recover");
        let path = root.join("metadata.json");
        let backup = backup_path(&path);
        fs::write(&backup, r#"{"value":"backup"}"#).expect("write backup");

        recover_json_after_interrupted_write(&path).expect("recover missing json");

        let loaded: TestJson =
            serde_json::from_str(&fs::read_to_string(&path).expect("read recovered json"))
                .expect("parse recovered json");
        assert_eq!(loaded.value, "backup");
    }
}
