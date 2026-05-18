use crate::{commands, desktop};
use tauri::Emitter;

fn handle_second_instance(app: &tauri::AppHandle, args: Vec<String>) {
    if let Some(file_path) = desktop::extract_file_arg(&args) {
        let _ = app.emit("open-external-file", file_path);
    }

    let _ = desktop::show_main_window(app);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            handle_second_instance(app, args)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            desktop::setup_desktop(app)?;
            Ok(())
        })
        .on_window_event(desktop::handle_window_event)
        .invoke_handler(commands::register_handlers!())
        .run(tauri::tauri_build_context!())
        .expect("error while running tauri application");
}
