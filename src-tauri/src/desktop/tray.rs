use crate::{
    desktop::{load_config, mark_app_exiting},
    notes::default_store,
};
use std::error::Error;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle,
};

use super::{
    shortcut::{autostart_enabled, toggle_autostart},
    window::{open_notepad_window_now, show_main_window},
};

const TRAY_SHOW_MAIN_ID: &str = "show-main";
const TRAY_QUICK_NOTE_ID: &str = "quick-note";
const TRAY_TOGGLE_CLOSE_TO_TRAY_ID: &str = "toggle-close-to-tray";
const TRAY_TOGGLE_AUTOSTART_ID: &str = "toggle-autostart";
const TRAY_QUIT_ID: &str = "quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMenuAction {
    ShowMain,
    QuickNote,
    ToggleCloseToTray,
    ToggleAutostart,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayMenuSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub checked: Option<bool>,
}

pub fn tray_menu_action(id: &str) -> Option<TrayMenuAction> {
    match id {
        TRAY_SHOW_MAIN_ID => Some(TrayMenuAction::ShowMain),
        TRAY_QUICK_NOTE_ID => Some(TrayMenuAction::QuickNote),
        TRAY_TOGGLE_CLOSE_TO_TRAY_ID => Some(TrayMenuAction::ToggleCloseToTray),
        TRAY_TOGGLE_AUTOSTART_ID => Some(TrayMenuAction::ToggleAutostart),
        TRAY_QUIT_ID => Some(TrayMenuAction::Quit),
        _ => None,
    }
}

pub fn tray_menu_specs(close_to_tray: bool, autostart: bool) -> Vec<TrayMenuSpec> {
    vec![
        TrayMenuSpec {
            id: TRAY_SHOW_MAIN_ID,
            label: "打开主窗口",
            checked: None,
        },
        TrayMenuSpec {
            id: TRAY_QUICK_NOTE_ID,
            label: "快速记录",
            checked: None,
        },
        TrayMenuSpec {
            id: TRAY_TOGGLE_CLOSE_TO_TRAY_ID,
            label: "关闭到托盘",
            checked: Some(close_to_tray),
        },
        TrayMenuSpec {
            id: TRAY_TOGGLE_AUTOSTART_ID,
            label: "开机自启动",
            checked: Some(autostart),
        },
        TrayMenuSpec {
            id: TRAY_QUIT_ID,
            label: "退出",
            checked: None,
        },
    ]
}

pub(crate) fn setup_tray(app: &mut App) -> Result<(), Box<dyn Error>> {
    let config = load_config()?;
    let autostart = autostart_enabled(app.handle(), config.autostart);
    let specs = tray_menu_specs(config.close_to_tray, autostart);

    let show_main = MenuItem::with_id(app, specs[0].id, specs[0].label, true, None::<&str>)?;
    let quick_note = MenuItem::with_id(app, specs[1].id, specs[1].label, true, None::<&str>)?;
    let close_to_tray = CheckMenuItem::with_id(
        app,
        specs[2].id,
        specs[2].label,
        true,
        specs[2].checked.unwrap_or(false),
        None::<&str>,
    )?;
    let autostart = CheckMenuItem::with_id(
        app,
        specs[3].id,
        specs[3].label,
        true,
        specs[3].checked.unwrap_or(false),
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, specs[4].id, specs[4].label, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_main,
            &quick_note,
            &close_to_tray,
            &autostart,
            &separator,
            &quit,
        ],
    )?;

    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .expect("missing default window icon")
                .clone(),
        )
        .tooltip("花笺")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Err(error) = handle_tray_menu_event(app, event.id.as_ref()) {
                eprintln!("failed to handle tray menu event {:?}: {error}", event.id);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = show_main_window(tray.app_handle()) {
                    eprintln!("failed to show main window from tray: {error}");
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_tray_menu_event(app: &AppHandle, id: &str) -> Result<(), Box<dyn Error>> {
    match tray_menu_action(id) {
        Some(TrayMenuAction::ShowMain) => show_main_window(app)?,
        Some(TrayMenuAction::QuickNote) => {
            open_notepad_window_now(app, None, None)?;
        }
        Some(TrayMenuAction::ToggleCloseToTray) => {
            let store = default_store()?;
            let mut config = store.load_config()?;
            config.close_to_tray = !config.close_to_tray;
            store.save_config(config)?;
        }
        Some(TrayMenuAction::ToggleAutostart) => toggle_autostart(app)?,
        Some(TrayMenuAction::Quit) => {
            mark_app_exiting(app);
            app.exit(0);
        }
        None => {}
    }

    Ok(())
}
