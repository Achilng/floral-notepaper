pub(crate) mod app;
pub(crate) mod categories;
pub(crate) mod config;
pub(crate) mod files;
pub(crate) mod notes;
pub(crate) mod windows;

macro_rules! register_handlers {
    () => {
        tauri::generate_handler![
            crate::commands::app::app_name,
            crate::commands::notes::notes_list,
            crate::commands::notes::notes_get,
            crate::commands::notes::notes_create,
            crate::commands::notes::notes_update,
            crate::commands::notes::notes_delete,
            crate::commands::notes::notes_import_markdown,
            crate::commands::notes::notes_export_markdown,
            crate::commands::notes::notes_move_category,
            crate::commands::files::read_external_file,
            crate::commands::files::save_external_file,
            crate::commands::files::get_file_modified_time,
            crate::commands::categories::categories_list,
            crate::commands::categories::categories_create,
            crate::commands::categories::categories_rename,
            crate::commands::categories::categories_delete,
            crate::commands::config::config_get,
            crate::commands::config::config_save,
            crate::commands::windows::open_notepad_window,
            crate::commands::windows::recycle_notepad_window,
            crate::commands::windows::open_tile_window,
            crate::commands::windows::open_note_in_editor
        ]
    };
}

pub(crate) use register_handlers;
