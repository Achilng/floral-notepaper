#[tauri::command]
fn app_name() -> &'static str {
    "花笺"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![app_name])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
