pub mod desktop;
pub mod json_io;
pub mod locales;
pub mod services;
pub mod updater;

use locales::Locale;
use services::notes::{default_store, AppConfig, AppError, Note, NoteMetadata, SaveNoteRequest};
use std::{env, fs, io::Write, path::PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
fn app_name() -> Result<String, AppError> {
    let locale = Locale::from_tag(&default_store()?.load_config()?.locale);
    Ok(locales::app_name(locale).to_string())
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
fn notes_create(app: AppHandle, request: SaveNoteRequest) -> Result<Note, AppError> {
    let note = default_store()?.create_note(request)?;
    let _ = app.emit("notes-changed", ());
    Ok(note)
}

#[tauri::command]
fn notes_update(app: AppHandle, id: String, request: SaveNoteRequest) -> Result<Note, AppError> {
    let note = default_store()?.update_note(&id, request)?;
    let _ = app.emit("notes-changed", ());
    Ok(note)
}

#[tauri::command]
fn notes_delete(app: AppHandle, id: String) -> Result<(), AppError> {
    default_store()?.delete_note(&id)?;
    let _ = app.emit("notes-changed", ());
    Ok(())
}

#[tauri::command]
fn notes_import_markdown(
    app: AppHandle,
    path: String,
    category: Option<String>,
) -> Result<Note, AppError> {
    let note = default_store()?
        .import_markdown_file(&PathBuf::from(path), &category.unwrap_or_default())?;
    let _ = app.emit("notes-changed", ());
    Ok(note)
}

#[tauri::command]
fn notes_export_markdown(id: String, path: String) -> Result<(), AppError> {
    default_store()?.export_markdown_file(&id, &PathBuf::from(path))
}

#[tauri::command]
fn read_external_file(path: String) -> Result<String, AppError> {
    std::fs::read_to_string(&path).map_err(|e| AppError {
        code: "io".into(),
        message: e.to_string(),
        details: Default::default(),
    })
}

#[tauri::command]
fn get_file_modified_time(path: String) -> Result<f64, AppError> {
    let metadata = std::fs::metadata(&path).map_err(|e| AppError {
        code: "io".into(),
        message: e.to_string(),
        details: Default::default(),
    })?;
    let modified = metadata.modified().map_err(|e| AppError {
        code: "io".into(),
        message: e.to_string(),
        details: Default::default(),
    })?;
    let duration = modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(duration.as_secs_f64() * 1000.0)
}

#[tauri::command]
fn save_external_file(path: String, content: String) -> Result<(), AppError> {
    if let Some(parent) = PathBuf::from(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError {
            code: "io".into(),
            message: e.to_string(),
            details: Default::default(),
        })?;
    }
    std::fs::write(&path, content).map_err(|e| AppError {
        code: "io".into(),
        message: e.to_string(),
        details: Default::default(),
    })
}

#[tauri::command]
fn categories_list() -> Result<Vec<String>, AppError> {
    default_store()?.list_categories()
}

#[tauri::command]
fn categories_create(app: AppHandle, name: String) -> Result<(), AppError> {
    default_store()?.create_category(&name)?;
    let _ = app.emit("notes-changed", ());
    Ok(())
}

#[tauri::command]
fn categories_rename(app: AppHandle, old_name: String, new_name: String) -> Result<(), AppError> {
    default_store()?.rename_category(&old_name, &new_name)?;
    let _ = app.emit("notes-changed", ());
    Ok(())
}

#[tauri::command]
fn categories_delete(app: AppHandle, name: String) -> Result<(), AppError> {
    default_store()?.delete_category(&name)?;
    let _ = app.emit("notes-changed", ());
    Ok(())
}

#[tauri::command]
fn notes_move_category(
    app: AppHandle,
    id: String,
    category: String,
) -> Result<NoteMetadata, AppError> {
    let result = default_store()?.move_note_to_category(&id, &category)?;
    let _ = app.emit("notes-changed", ());
    Ok(result)
}

#[tauri::command]
fn images_save(note_id: String, data: Vec<u8>, extension: String) -> Result<String, AppError> {
    default_store()?.save_image(&note_id, &data, &extension)
}

#[tauri::command]
fn images_save_from_path(note_id: String, file_path: String) -> Result<String, AppError> {
    let path = PathBuf::from(&file_path);
    let data = std::fs::read(&path)?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png")
        .to_string();
    default_store()?.save_image(&note_id, &data, &extension)
}

#[tauri::command]
fn images_get_base_dir() -> Result<String, AppError> {
    let store = default_store()?;
    store
        .base_dir()
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| AppError {
            code: "path".into(),
            message: "invalid base dir path".into(),
            details: Default::default(),
        })
}

#[tauri::command]
fn images_clean_unused(note_id: String, content: String) -> Result<Vec<String>, AppError> {
    default_store()?.clean_unused_images(&note_id, &content)
}

#[tauri::command]
fn config_get() -> Result<AppConfig, AppError> {
    default_store()?.load_config()
}

#[tauri::command]
fn copy_background_image(_app: AppHandle, source_path: String) -> Result<String, AppError> {
    let source = PathBuf::from(source_path.trim());
    if !source.is_file() {
        return Err(AppError {
            code: "invalidSource".into(),
            message: "background image source not found".into(),
            details: Default::default(),
        });
    }

    let store = default_store()?;
    let dir = store.base_dir().join("backgrounds");
    fs::create_dir_all(&dir)?;

    let old_config = store.load_config()?;
    if !old_config.background_image_path.is_empty() {
        let old_path = PathBuf::from(&old_config.background_image_path);
        if old_path.starts_with(&dir) && old_path.is_file() {
            let _ = fs::remove_file(&old_path);
        }
    }

    let ext = source
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("png");
    let dest = dir.join(format!("bg-{}.{}", uuid::Uuid::new_v4(), ext));
    fs::copy(&source, &dest)?;

    dest.to_str().map(str::to_string).ok_or_else(|| AppError {
        code: "path".into(),
        message: "invalid destination path".into(),
        details: Default::default(),
    })
}

#[tauri::command]
fn config_save(app: AppHandle, config: AppConfig) -> Result<AppConfig, AppError> {
    let store = default_store()?;
    let previous = store.load_config()?;
    desktop::apply_runtime_config(&app, &previous, &config).map_err(|error| {
        match error.downcast::<AppError>() {
            Ok(app_error) => *app_error,
            Err(error) => AppError {
                code: "desktopConfig".into(),
                message: error.to_string(),
                details: Default::default(),
            },
        }
    })?;
    let saved = store.save_config(config)?;
    if let Err(error) = desktop::refresh_shell_state(&app, &saved) {
        eprintln!("failed to refresh desktop shell state: {error}");
    }
    let _ = app.emit("config-changed", &saved);
    Ok(saved)
}

#[tauri::command]
fn global_shortcut_check(
    app: AppHandle,
    shortcut: String,
) -> Result<desktop::ShortcutCheckResult, AppError> {
    desktop::check_global_shortcut(&app, &shortcut)
}

#[tauri::command]
fn start_shortcut_recording(app: AppHandle) -> Result<(), AppError> {
    desktop::start_shortcut_recording(&app).map_err(|error| AppError {
        code: "shortcutRecording".into(),
        message: error.to_string(),
        details: Default::default(),
    })
}

#[tauri::command]
fn stop_shortcut_recording(app: AppHandle) -> Result<(), AppError> {
    desktop::stop_shortcut_recording(&app).map_err(|error| AppError {
        code: "shortcutRecording".into(),
        message: error.to_string(),
        details: Default::default(),
    })
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

#[tauri::command]
async fn toggle_tile_window(
    app: AppHandle,
    note_id: String,
    bounds: Option<desktop::WindowBounds>,
) -> Result<bool, AppError> {
    desktop::toggle_tile_window(app, note_id, bounds).await
}

#[tauri::command]
async fn open_note_in_editor(app: AppHandle, note_id: String) -> Result<(), AppError> {
    desktop::show_main_window(&app)?;
    let _ = app.emit("open-note", &note_id);
    Ok(())
}

#[tauri::command]
fn take_startup_file() -> Option<String> {
    desktop::take_startup_file()
}

fn cli_version_or_help_requested() -> bool {
    env::args().any(|arg| matches!(arg.as_str(), "--version" | "-V" | "--help" | "-h"))
}

#[cfg(windows)]
fn ensure_console() {
    use windows_sys::Win32::System::Console::{AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS};

    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            let _ = AllocConsole();
        }
    }
}

#[cfg(not(windows))]
fn ensure_console() {}

fn flush_attached_console_stdout() {
    let _ = std::io::stdout().flush();
}

fn print_cli_version() {
    let _ = writeln!(
        std::io::stdout(),
        "floral-notepaper {}",
        env!("CARGO_PKG_VERSION")
    );
    flush_attached_console_stdout();
}

fn print_cli_help() {
    let _ = writeln!(
        std::io::stdout(),
        "floral-notepaper {}\nFloral Notepaper - lightweight local note app\n\nUSAGE:\n    floral-notepaper [OPTIONS]\n\nOPTIONS:\n    -V, --version\n            Print version\n    -h, --help\n            Print help",
        env!("CARGO_PKG_VERSION"),
    );
    flush_attached_console_stdout();
}

pub fn try_exit_for_cli_version_or_help() {
    if !cli_version_or_help_requested() {
        return;
    }

    ensure_console();

    let wants_version = env::args().any(|arg| arg == "--version" || arg == "-V");
    let wants_help = env::args().any(|arg| arg == "--help" || arg == "-h");

    if wants_version {
        print_cli_version();
        std::process::exit(0);
    }

    if wants_help {
        print_cli_help();
        std::process::exit(0);
    }

    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(file_path) = desktop::extract_file_arg(&args) {
                let _ = app.emit("open-external-file", file_path);
            }
            let _ = desktop::show_main_window(app);
        }))
        .setup(|app| {
            if let Ok(store) = default_store() {
                let base = store.base_dir();
                let scope = app.asset_protocol_scope();
                let _ = scope.allow_directory(base.join("images"), true);
                let _ = scope.allow_directory(base.join("backgrounds"), true);
            }
            let updater_state = updater::UpdaterState::new(app.package_info().version.to_string());
            if let Err(error) = updater_state.initialize() {
                eprintln!("failed to initialize updater infrastructure: {error}");
            }
            app.manage(updater_state);
            updater::start_auto_check_scheduler(app.handle().clone());
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
            notes_move_category,
            read_external_file,
            save_external_file,
            get_file_modified_time,
            categories_list,
            categories_create,
            categories_rename,
            categories_delete,
            images_save,
            images_save_from_path,
            images_get_base_dir,
            images_clean_unused,
            config_get,
            copy_background_image,
            config_save,
            global_shortcut_check,
            start_shortcut_recording,
            stop_shortcut_recording,
            open_notepad_window,
            recycle_notepad_window,
            open_tile_window,
            toggle_tile_window,
            open_note_in_editor,
            updater::commands::update_status,
            updater::commands::update_settings_get,
            updater::commands::update_settings_save,
            updater::commands::update_mirror_chyan_cdk_set,
            updater::commands::update_mirror_chyan_cdk_clear,
            updater::commands::update_mirror_chyan_cdk_get,
            updater::commands::update_check,
            updater::commands::update_download,
            updater::commands::update_install,
            updater::commands::update_install_prepare_report,
            updater::commands::update_cancel,
            take_startup_file
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app_handle, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = _event
            {
                if !has_visible_windows {
                    if let Err(error) = desktop::show_main_window(_app_handle) {
                        eprintln!("failed to show main window on dock click: {error}");
                    }
                }
            }
        });
}
