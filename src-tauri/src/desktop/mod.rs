use crate::notes::{default_store, AppConfig, AppError};
use serde::Deserialize;
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{App, AppHandle, Emitter, Manager, Window, WindowEvent};

pub(crate) mod shortcut;
pub(crate) mod tray;
pub(crate) mod window;

pub use shortcut::apply_runtime_config;
pub use window::{
    extract_file_arg, open_notepad_window, open_tile_window, recycle_notepad_window,
    show_main_window,
};

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const NOTEPAD_POOL_CAPACITY: usize = 2;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub(crate) struct RuntimeState {
    is_exiting: AtomicBool,
}

#[derive(Default)]
pub(crate) struct NotepadPool {
    available: Mutex<Vec<String>>,
}

impl NotepadPool {
    pub(crate) fn take(&self) -> Option<String> {
        self.available.lock().ok()?.pop()
    }

    pub(crate) fn put(&self, label: String) -> bool {
        if let Ok(mut available) = self.available.lock() {
            if available.len() < NOTEPAD_POOL_CAPACITY {
                available.push(label);
                return true;
            }
        }
        false
    }

    pub(crate) fn is_below_capacity(&self) -> bool {
        self.available
            .lock()
            .map(|available| available.len() < NOTEPAD_POOL_CAPACITY)
            .unwrap_or(false)
    }
}

impl RuntimeState {
    pub(crate) fn allow_exit(&self) {
        self.is_exiting.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_exiting(&self) -> bool {
        self.is_exiting.load(Ordering::SeqCst)
    }
}

pub(crate) fn load_config() -> Result<AppConfig, AppError> {
    default_store()?.load_config()
}

pub(crate) fn close_to_tray_enabled() -> bool {
    load_config()
        .map(|config| config.close_to_tray)
        .unwrap_or(true)
}

pub(crate) fn app_is_exiting(app: &AppHandle) -> bool {
    app.try_state::<RuntimeState>()
        .map(|state| state.is_exiting())
        .unwrap_or(false)
}

pub(crate) fn mark_app_exiting(app: &AppHandle) {
    if let Some(state) = app.try_state::<RuntimeState>() {
        state.allow_exit();
    }
}

pub fn setup_desktop(app: &mut App) -> Result<(), Box<dyn Error>> {
    app.manage(RuntimeState::default());
    app.manage(NotepadPool::default());
    shortcut::setup_autostart_plugin(app.handle())?;
    shortcut::setup_global_shortcut_plugin(app.handle())?;
    shortcut::sync_autostart_to_config(app.handle());
    shortcut::register_configured_global_shortcut(app.handle());
    tray::setup_tray(app)?;
    window::schedule_notepad_prewarm(app.handle());

    if !std::env::args().any(|arg| arg == "--silent") {
        if let Err(error) = show_main_window(app.handle()) {
            eprintln!("failed to show main window on startup: {error}");
        }
    }

    let args: Vec<String> = std::env::args().collect();
    if let Some(file_path) = extract_file_arg(&args) {
        let app_handle = app.handle().clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = app_handle.emit("open-external-file", file_path);
        });
    }

    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    window::handle_window_event(window, event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tray_menu_ids_to_actions() {
        assert_eq!(
            tray::tray_menu_action("show-main"),
            Some(tray::TrayMenuAction::ShowMain)
        );
        assert_eq!(
            tray::tray_menu_action("quick-note"),
            Some(tray::TrayMenuAction::QuickNote)
        );
        assert_eq!(
            tray::tray_menu_action("toggle-close-to-tray"),
            Some(tray::TrayMenuAction::ToggleCloseToTray)
        );
        assert_eq!(
            tray::tray_menu_action("toggle-autostart"),
            Some(tray::TrayMenuAction::ToggleAutostart)
        );
        assert_eq!(
            tray::tray_menu_action("quit"),
            Some(tray::TrayMenuAction::Quit)
        );
        assert_eq!(tray::tray_menu_action("unknown"), None);
    }

    #[test]
    fn builds_tray_menu_specs_with_configured_checked_state() {
        let specs = tray::tray_menu_specs(true, false);
        let ids: Vec<_> = specs.iter().map(|spec| spec.id).collect();

        assert_eq!(
            ids,
            vec![
                "show-main",
                "quick-note",
                "toggle-close-to-tray",
                "toggle-autostart",
                "quit"
            ]
        );
        assert_eq!(specs[2].checked, Some(true));
        assert_eq!(specs[3].checked, Some(false));
    }

    #[test]
    fn parses_shortcut_config_values() {
        assert_eq!(
            shortcut::shortcut_from_config("Ctrl+Space"),
            Some(shortcut::ShortcutSpec {
                ctrl: true,
                alt: false,
                shift: false,
                key: shortcut::ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut::shortcut_from_config("CommandOrControl + Space"),
            Some(shortcut::ShortcutSpec {
                ctrl: true,
                alt: false,
                shift: false,
                key: shortcut::ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut::shortcut_from_config("Alt+Space"),
            Some(shortcut::ShortcutSpec {
                ctrl: false,
                alt: true,
                shift: false,
                key: shortcut::ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut::shortcut_from_config("Ctrl+Shift+K"),
            Some(shortcut::ShortcutSpec {
                ctrl: true,
                alt: false,
                shift: true,
                key: shortcut::ShortcutKey::Letter('K'),
            })
        );
        assert_eq!(
            shortcut::shortcut_from_config("Alt+F2"),
            Some(shortcut::ShortcutSpec {
                ctrl: false,
                alt: true,
                shift: false,
                key: shortcut::ShortcutKey::Function(2),
            })
        );
        assert_eq!(
            shortcut::shortcut_from_config("Ctrl+Alt+3"),
            Some(shortcut::ShortcutSpec {
                ctrl: true,
                alt: true,
                shift: false,
                key: shortcut::ShortcutKey::Digit(3),
            })
        );
    }

    #[test]
    fn rejects_invalid_shortcut_config_values() {
        assert_eq!(shortcut::shortcut_from_config(""), None);
        assert_eq!(shortcut::shortcut_from_config("Space"), None);
        assert_eq!(shortcut::shortcut_from_config("Shift+K"), None);
        assert_eq!(shortcut::shortcut_from_config("Ctrl+Unknown"), None);
    }

    #[test]
    fn chooses_exit_when_main_window_closes_without_close_to_tray() {
        assert_eq!(
            window::main_window_close_action(false, false),
            window::MainWindowCloseAction::ExitApp
        );
    }

    #[test]
    fn detects_runtime_config_changes() {
        let previous = AppConfig {
            notes_dir: "D:\\notes".into(),
            global_shortcut: "Ctrl+Space".into(),
            close_to_tray: true,
            autostart: false,
            default_view_mode: "split".into(),
            note_auto_save: true,
            note_surface_auto_save: true,
            tile_color: "#f6f3ec".into(),
            tile_color_mode: "system".into(),
            theme: "light".into(),
            font_size: 14,
            surface_font_size: 14,
            external_file_auto_save: true,
        };
        let next = AppConfig {
            notes_dir: "D:\\other-notes".into(),
            global_shortcut: "Alt+Space".into(),
            close_to_tray: false,
            autostart: true,
            default_view_mode: "preview".into(),
            note_auto_save: false,
            note_surface_auto_save: false,
            tile_color: "#efe8dc".into(),
            tile_color_mode: "custom".into(),
            theme: "dark".into(),
            font_size: 16,
            surface_font_size: 16,
            external_file_auto_save: true,
        };

        assert_eq!(
            shortcut::runtime_config_changes(&previous, &next),
            shortcut::RuntimeConfigChanges {
                autostart_changed: true,
                global_shortcut_changed: true,
            }
        );
        assert_eq!(
            shortcut::runtime_config_changes(&previous, &previous),
            shortcut::RuntimeConfigChanges {
                autostart_changed: false,
                global_shortcut_changed: false,
            }
        );
    }

    #[test]
    fn builds_stable_dynamic_window_labels() {
        assert_eq!(
            window::notepad_window_label(Some("abc-123")),
            "notepad-abc-123"
        );
        assert!(window::notepad_window_label(None).starts_with("notepad-"));
        assert_eq!(window::tile_window_label("note-1"), "tile-note-1");
    }

    #[test]
    fn keeps_notepad_initial_window_compact() {
        let specs = window::notepad_window_specs();

        assert_eq!(specs.width, 260.0);
        assert_eq!(specs.height, 260.0);
        assert_eq!(specs.min_width, 220.0);
        assert_eq!(specs.min_height, 220.0);
    }

    #[test]
    fn makes_note_surfaces_transparent() {
        assert_eq!(
            window::dynamic_window_visual_options("notepad-note-1"),
            window::DynamicWindowVisualOptions { transparent: true }
        );
        assert_eq!(
            window::dynamic_window_visual_options("tile-note-1"),
            window::DynamicWindowVisualOptions { transparent: true }
        );
        assert_eq!(
            window::dynamic_window_visual_options("main"),
            window::DynamicWindowVisualOptions { transparent: false }
        );
    }

    #[test]
    fn capability_allows_frontend_window_focus_for_notepad_surfaces() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../../capabilities/default.json"))
                .expect("default capability should be valid json");
        let windows = capability["windows"]
            .as_array()
            .expect("capability should define windows");
        let permissions = capability["permissions"]
            .as_array()
            .expect("capability should define permissions");

        assert!(windows
            .iter()
            .any(|window| window.as_str() == Some("notepad-*")));
        assert!(permissions
            .iter()
            .any(|permission| permission.as_str() == Some("core:window:allow-set-focus")));
    }
}
