pub mod services;

use services::notes::{default_store, AppConfig, AppError, Note, NoteMetadata, SaveNoteRequest};

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
fn config_get() -> Result<AppConfig, AppError> {
    default_store()?.load_config()
}

#[tauri::command]
fn config_save(config: AppConfig) -> Result<AppConfig, AppError> {
    let store = default_store()?;
    store.save_config(config.clone())?;
    Ok(config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_name,
            notes_list,
            notes_get,
            notes_create,
            notes_update,
            notes_delete,
            config_get,
            config_save
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
