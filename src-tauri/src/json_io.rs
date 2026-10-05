use crate::services::notes::AppError;
use serde::Serialize;
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = temporary_json_path(path);
    let mut temp_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temp_path)?;
    serde_json::to_writer_pretty(&mut temp_file, value)?;
    temp_file.write_all(b"\n")?;
    temp_file.sync_all()?;
    drop(temp_file);
    fs::rename(&temp_path, path)?;
    sync_parent_dir(path)?;
    Ok(())
}

fn temporary_json_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state.json");
    path.with_file_name(format!("{file_name}.tmp"))
}

#[cfg(not(target_os = "windows"))]
fn sync_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn sync_parent_dir(_path: &Path) -> Result<(), AppError> {
    Ok(())
}
