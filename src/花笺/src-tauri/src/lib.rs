pub mod services;

use services::notes::{default_store, AppConfig, AppError, Note, NoteMetadata, SaveNoteRequest};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

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

#[tauri::command]
fn open_notepad_window(app: AppHandle, note_id: Option<String>) -> Result<String, AppError> {
    let label = notepad_window_label(note_id.as_deref());
    let url = match note_id {
        Some(id) => format!("index.html?view=notepad&noteId={id}"),
        None => "index.html?view=notepad".to_string(),
    };

    open_or_focus_window(&app, &label, url, "花笺便签", 420.0, 430.0, false, true)
}

#[tauri::command]
fn open_tile_window(app: AppHandle, note_id: String) -> Result<String, AppError> {
    let label = tile_window_label(&note_id);
    let url = format!("index.html?view=tile&noteId={note_id}");

    open_or_focus_window(&app, &label, url, "花笺磁贴", 320.0, 320.0, false, true)
}

fn open_or_focus_window(
    app: &AppHandle,
    label: &str,
    url: String,
    title: &str,
    width: f64,
    height: f64,
    decorations: bool,
    always_on_top: bool,
) -> Result<String, AppError> {
    if let Some(window) = app.get_webview_window(label) {
        window.set_focus()?;
        return Ok(label.to_string());
    }

    WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(width, height)
        .min_inner_size(260.0, 220.0)
        .resizable(true)
        .decorations(decorations)
        .always_on_top(always_on_top)
        .build()?
        .set_focus()?;

    Ok(label.to_string())
}

fn notepad_window_label(note_id: Option<&str>) -> String {
    match note_id {
        Some(id) => format!("notepad-{}", sanitize_label_part(id)),
        None => format!("notepad-{}", Uuid::new_v4()),
    }
}

fn tile_window_label(note_id: &str) -> String {
    format!("tile-{}", sanitize_label_part(note_id))
}

fn sanitize_label_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();

    sanitized.trim_matches('-').to_string()
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
            config_save,
            open_notepad_window,
            open_tile_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn builds_stable_dynamic_window_labels() {
        assert_eq!(notepad_window_label(Some("abc-123")), "notepad-abc-123");
        assert!(notepad_window_label(None).starts_with("notepad-"));
        assert_eq!(tile_window_label("note-1"), "tile-note-1");
    }
}
