pub mod desktop;
pub mod services;

use services::notes::{default_store, AppConfig, AppError, Note, NoteMetadata, SaveNoteRequest};
use std::path::PathBuf;
use tauri::AppHandle;

#[tauri::command]
fn app_name() -> &'static str {
    "花笺"
}

#[tauri::command]
fn notes_list() -> Result<Vec<NoteMetadata>, AppError> {
    default_store()?.list_notes()
}

#[tauri::command]
fn notes_get(id: String) -> Result<Note, AppError> {
    default_store()?.read_note(&id)
}

#[tauri::command]
fn notes_create(request: SaveNoteRequest) -> Result<Note, AppError> {
    default_store()?.create_note(request)
}

#[tauri::command]
fn notes_update(id: String, request: SaveNoteRequest) -> Result<Note, AppError> {
    default_store()?.update_note(&id, request)
}

#[tauri::command]
fn notes_delete(id: String) -> Result<(), AppError> {
    default_store()?.delete_note(&id)
}

#[tauri::command]
fn notes_import_markdown(path: String) -> Result<Note, AppError> {
    default_store()?.import_markdown_file(&PathBuf::from(path))
}

#[tauri::command]
fn notes_export_markdown(id: String, path: String) -> Result<(), AppError> {
    default_store()?.export_markdown_file(&id, &PathBuf::from(path))
}

#[tauri::command]
fn config_get() -> Result<AppConfig, AppError> {
    default_store()?.load_config()
}

#[tauri::command]
fn config_save(app: AppHandle, config: AppConfig) -> Result<AppConfig, AppError> {
    let store = default_store()?;
    let previous = store.load_config()?;
    desktop::apply_runtime_config(&app, &previous, &config).map_err(|error| AppError {
        code: "desktopConfig".into(),
        message: error.to_string(),
    })?;
    store.save_config(config.clone())?;
    Ok(config)
}

#[tauri::command]
async fn open_notepad_window(
    app: AppHandle,
    note_id: Option<String>,
    bounds: Option<desktop::WindowBounds>,
) -> Result<String, AppError> {
    desktop::open_notepad_window(app, note_id, bounds).await
}

#[tauri::command]
async fn recycle_notepad_window(app: AppHandle, label: String) -> Result<(), AppError> {
    desktop::recycle_notepad_window(&app, &label)
}

#[tauri::command]
async fn open_tile_window(
    app: AppHandle,
    note_id: String,
    bounds: Option<desktop::WindowBounds>,
) -> Result<String, AppError> {
    desktop::open_tile_window(app, note_id, bounds).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            desktop::setup_desktop(app)?;
            Ok(())
        })
        .on_window_event(desktop::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            app_name,
            notes_list,
            notes_get,
            notes_create,
            notes_update,
            notes_delete,
            notes_import_markdown,
            notes_export_markdown,
            config_get,
            config_save,
            open_notepad_window,
            recycle_notepad_window,
            open_tile_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
