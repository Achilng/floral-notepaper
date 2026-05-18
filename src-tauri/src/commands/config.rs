use crate::{
    desktop,
    notes::{default_store, AppConfig, AppError},
};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub fn config_get() -> Result<AppConfig, AppError> {
    default_store()?.load_config()
}

#[tauri::command]
pub fn config_save(app: AppHandle, config: AppConfig) -> Result<AppConfig, AppError> {
    let store = default_store()?;
    let previous = store.load_config()?;
    desktop::apply_runtime_config(&app, &previous, &config).map_err(|error| AppError {
        code: "desktopConfig".into(),
        message: error.to_string(),
    })?;
    store.save_config(config.clone())?;
    let _ = app.emit("config-changed", &config);
    Ok(config)
}
