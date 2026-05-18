use crate::notes::AppError;
use std::{fs, path::PathBuf, time::UNIX_EPOCH};

#[tauri::command]
pub fn read_external_file(path: String) -> Result<String, AppError> {
    fs::read_to_string(path).map_err(|error| AppError {
        code: "io".into(),
        message: error.to_string(),
    })
}

#[tauri::command]
pub fn get_file_modified_time(path: String) -> Result<f64, AppError> {
    let metadata = fs::metadata(&path).map_err(|error| AppError {
        code: "io".into(),
        message: error.to_string(),
    })?;
    let modified = metadata.modified().map_err(|error| AppError {
        code: "io".into(),
        message: error.to_string(),
    })?;
    let duration = modified.duration_since(UNIX_EPOCH).unwrap_or_default();

    Ok(duration.as_secs_f64() * 1000.0)
}

#[tauri::command]
pub fn save_external_file(path: String, content: String) -> Result<(), AppError> {
    if let Some(parent) = PathBuf::from(&path).parent() {
        fs::create_dir_all(parent).map_err(|error| AppError {
            code: "io".into(),
            message: error.to_string(),
        })?;
    }

    fs::write(path, content).map_err(|error| AppError {
        code: "io".into(),
        message: error.to_string(),
    })
}
