use crate::services::notes::{default_store, AppConfig, AppError};
use serde::Deserialize;
use std::{
    error::Error,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window, WindowEvent,
};
use uuid::Uuid;

#[cfg(desktop)]
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartExt};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_SHOW_MAIN_ID: &str = "show-main";
const TRAY_QUICK_NOTE_ID: &str = "quick-note";
const TRAY_TOGGLE_CLOSE_TO_TRAY_ID: &str = "toggle-close-to-tray";
const TRAY_TOGGLE_AUTOSTART_ID: &str = "toggle-autostart";
const TRAY_QUIT_ID: &str = "quit";
const NOTE_SURFACE_CORNER_RADIUS: i32 = 14;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutModifier {
    Control,
    Alt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    Space,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeConfigChanges {
    pub autostart_changed: bool,
    pub global_shortcut_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub modifier: ShortcutModifier,
    pub key: ShortcutKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicWindowVisualOptions {
    pub transparent: bool,
    pub corner_radius: Option<i32>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
struct RuntimeState {
    is_exiting: AtomicBool,
}

impl RuntimeState {
    fn allow_exit(&self) {
        self.is_exiting.store(true, Ordering::SeqCst);
    }

    fn is_exiting(&self) -> bool {
        self.is_exiting.load(Ordering::SeqCst)
    }
}

trait RoundedWindowRegion {
    fn outer_size(&self) -> tauri::Result<PhysicalSize<u32>>;

    #[cfg(target_os = "windows")]
    fn hwnd_ptr(&self) -> tauri::Result<*mut std::ffi::c_void>;
}

impl RoundedWindowRegion for Window {
    fn outer_size(&self) -> tauri::Result<PhysicalSize<u32>> {
        Window::outer_size(self)
    }

    #[cfg(target_os = "windows")]
    fn hwnd_ptr(&self) -> tauri::Result<*mut std::ffi::c_void> {
        Ok(Window::hwnd(self)?.0)
    }
}

impl RoundedWindowRegion for WebviewWindow {
    fn outer_size(&self) -> tauri::Result<PhysicalSize<u32>> {
        WebviewWindow::outer_size(self)
    }

    #[cfg(target_os = "windows")]
    fn hwnd_ptr(&self) -> tauri::Result<*mut std::ffi::c_void> {
        Ok(WebviewWindow::hwnd(self)?.0)
    }
}

#[cfg(target_os = "windows")]
mod win32_rounding {
    use std::{ffi::c_void, io};

    type Hwnd = *mut c_void;
    type Hrgn = *mut c_void;
    type HgdiObj = *mut c_void;

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> Hrgn;
        fn DeleteObject(ho: HgdiObj) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowRgn(hwnd: Hwnd, hrgn: Hrgn, bredraw: i32) -> i32;
    }

    pub fn apply_rounded_region(
        hwnd: Hwnd,
        width: u32,
        height: u32,
        radius: i32,
    ) -> io::Result<()> {
        let width = width.min(i32::MAX as u32) as i32;
        let height = height.min(i32::MAX as u32) as i32;
        let diameter = radius.saturating_mul(2);

        if width <= 0 || height <= 0 || diameter <= 0 {
            return Ok(());
        }

        unsafe {
            let region = CreateRoundRectRgn(
                0,
                0,
                width.saturating_add(1),
                height.saturating_add(1),
                diameter,
                diameter,
            );
            if region.is_null() {
                return Err(io::Error::last_os_error());
            }

            if SetWindowRgn(hwnd, region, 1) == 0 {
                let _ = DeleteObject(region);
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }
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

pub fn shortcut_from_config(value: &str) -> Option<ShortcutSpec> {
    let parts: Vec<_> = value
        .split('+')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect();

    if parts.len() != 2 {
        return None;
    }

    let modifier = match parts[0].as_str() {
        "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => ShortcutModifier::Control,
        "alt" | "option" => ShortcutModifier::Alt,
        _ => return None,
    };

    let key = match parts[1].as_str() {
        "space" => ShortcutKey::Space,
        _ => return None,
    };

    Some(ShortcutSpec { modifier, key })
}

pub fn runtime_config_changes(previous: &AppConfig, next: &AppConfig) -> RuntimeConfigChanges {
    RuntimeConfigChanges {
        autostart_changed: previous.autostart != next.autostart,
        global_shortcut_changed: previous.global_shortcut != next.global_shortcut,
    }
}

pub fn apply_runtime_config(
    app: &AppHandle,
    previous: &AppConfig,
    next: &AppConfig,
) -> Result<(), Box<dyn Error>> {
    let changes = runtime_config_changes(previous, next);

    if changes.global_shortcut_changed {
        apply_global_shortcut_config(app, &next.global_shortcut)?;
    }

    if changes.autostart_changed {
        apply_autostart(app, next.autostart)?;
    }

    Ok(())
}

pub async fn open_notepad_window(
    app: AppHandle,
    note_id: Option<String>,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    open_notepad_window_now(&app, note_id.as_deref(), bounds)
}

pub async fn open_tile_window(
    app: AppHandle,
    note_id: String,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    open_tile_window_now(&app, &note_id, bounds)
}

pub fn setup_desktop(app: &mut App) -> Result<(), Box<dyn Error>> {
    app.manage(RuntimeState::default());
    setup_autostart_plugin(app.handle())?;
    setup_global_shortcut_plugin(app.handle())?;
    sync_autostart_to_config(app.handle());
    register_configured_global_shortcut(app.handle());
    setup_tray(app)?;
    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::Resized(_) = event {
        let visual_options = dynamic_window_visual_options(window.label());
        if visual_options.corner_radius.is_some() {
            if let Err(error) = apply_dynamic_window_visuals(window, visual_options) {
                eprintln!(
                    "failed to update rounded region for window {}: {error}",
                    window.label()
                );
            }
        }
    }

    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    if app_is_exiting(window.app_handle()) || !close_to_tray_enabled() {
        return;
    }

    api.prevent_close();
    if let Err(error) = window.hide() {
        eprintln!("failed to hide main window to tray: {error}");
    }
}

fn setup_tray(app: &mut App) -> Result<(), Box<dyn Error>> {
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

fn show_main_window(app: &AppHandle) -> Result<(), AppError> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    open_or_focus_window(
        app,
        MAIN_WINDOW_LABEL,
        "index.html".to_string(),
        "花笺",
        1180.0,
        760.0,
        900.0,
        620.0,
        false,
        false,
        true,
        None,
    )?;
    Ok(())
}

fn open_notepad_window_now(
    app: &AppHandle,
    note_id: Option<&str>,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    let label = notepad_window_label(note_id);
    let url = match note_id {
        Some(id) => format!("index.html?view=notepad&noteId={id}"),
        None => "index.html?view=notepad".to_string(),
    };

    open_or_focus_window(
        app,
        &label,
        url,
        "花笺便签",
        420.0,
        430.0,
        260.0,
        220.0,
        false,
        true,
        false,
        bounds,
    )
}

fn open_tile_window_now(
    app: &AppHandle,
    note_id: &str,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    let label = tile_window_label(note_id);
    let url = format!("index.html?view=tile&noteId={note_id}");

    open_or_focus_window(
        app,
        &label,
        url,
        "花笺磁贴",
        420.0,
        430.0,
        260.0,
        220.0,
        false,
        true,
        false,
        bounds,
    )
}

fn open_or_focus_window(
    app: &AppHandle,
    label: &str,
    url: String,
    title: &str,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
    decorations: bool,
    always_on_top: bool,
    shadow: bool,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    let visual_options = dynamic_window_visual_options(label);

    if let Some(window) = app.get_webview_window(label) {
        apply_window_bounds(&window, bounds)?;
        window.set_shadow(shadow)?;
        apply_dynamic_window_visuals(&window, visual_options)?;
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(label.to_string());
    }

    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(width, height)
        .min_inner_size(min_width, min_height)
        .resizable(true)
        .decorations(decorations)
        .transparent(visual_options.transparent)
        .always_on_top(always_on_top)
        .shadow(shadow);

    if let Some(bounds) = bounds {
        builder = builder
            .position(bounds.x as f64, bounds.y as f64)
            .inner_size(bounds.width as f64, bounds.height as f64);
    }

    let window = builder.build()?;
    apply_dynamic_window_visuals(&window, visual_options)?;

    Ok(label.to_string())
}

fn apply_window_bounds(
    window: &tauri::WebviewWindow,
    bounds: Option<WindowBounds>,
) -> Result<(), AppError> {
    if let Some(bounds) = bounds {
        window.set_position(PhysicalPosition::new(bounds.x, bounds.y))?;
        window.set_size(PhysicalSize::new(bounds.width, bounds.height))?;
    }

    Ok(())
}

fn apply_dynamic_window_visuals<W: RoundedWindowRegion>(
    window: &W,
    visual_options: DynamicWindowVisualOptions,
) -> Result<(), AppError> {
    apply_rounded_window_region(window, visual_options.corner_radius)
}

fn apply_rounded_window_region<W: RoundedWindowRegion>(
    window: &W,
    corner_radius: Option<i32>,
) -> Result<(), AppError> {
    let Some(corner_radius) = corner_radius else {
        return Ok(());
    };

    #[cfg(target_os = "windows")]
    {
        let size = window.outer_size()?;
        win32_rounding::apply_rounded_region(
            window.hwnd_ptr()?,
            size.width,
            size.height,
            corner_radius,
        )?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        let _ = corner_radius;
    }

    Ok(())
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

fn dynamic_window_visual_options(label: &str) -> DynamicWindowVisualOptions {
    let is_note_surface = label.starts_with("notepad-") || label.starts_with("tile-");

    DynamicWindowVisualOptions {
        transparent: is_note_surface,
        corner_radius: is_note_surface.then_some(NOTE_SURFACE_CORNER_RADIUS),
    }
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

fn load_config() -> Result<AppConfig, AppError> {
    default_store()?.load_config()
}

fn close_to_tray_enabled() -> bool {
    load_config()
        .map(|config| config.close_to_tray)
        .unwrap_or(true)
}

fn app_is_exiting(app: &AppHandle) -> bool {
    app.try_state::<RuntimeState>()
        .map(|state| state.is_exiting())
        .unwrap_or(false)
}

fn mark_app_exiting(app: &AppHandle) {
    if let Some(state) = app.try_state::<RuntimeState>() {
        state.allow_exit();
    }
}

#[cfg(desktop)]
fn setup_autostart_plugin(app: &AppHandle) -> tauri::Result<()> {
    app.plugin(tauri_plugin_autostart::init(
        MacosLauncher::LaunchAgent,
        None,
    ))
}

#[cfg(not(desktop))]
fn setup_autostart_plugin(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(desktop)]
fn setup_global_shortcut_plugin(app: &AppHandle) -> tauri::Result<()> {
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let app_for_closure = app.clone();
                    if let Err(error) = app.run_on_main_thread(move || {
                        if let Err(error) = open_notepad_window_now(&app_for_closure, None, None) {
                            eprintln!("failed to open notepad from global shortcut: {error}");
                        }
                    }) {
                        eprintln!("failed to dispatch global shortcut action: {error}");
                    }
                }
            })
            .build(),
    )
}

#[cfg(not(desktop))]
fn setup_global_shortcut_plugin(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(desktop)]
fn register_configured_global_shortcut(app: &AppHandle) {
    let Ok(config) = load_config() else {
        return;
    };

    if let Err(error) = register_global_shortcut(app, &config.global_shortcut) {
        eprintln!(
            "failed to register global shortcut {}: {error}",
            config.global_shortcut
        );
    }
}

#[cfg(not(desktop))]
fn register_configured_global_shortcut(_app: &AppHandle) {}

#[cfg(desktop)]
fn register_global_shortcut(app: &AppHandle, shortcut_config: &str) -> Result<(), Box<dyn Error>> {
    let Some(shortcut) = shortcut_from_config(shortcut_config).and_then(to_tauri_shortcut) else {
        return Err(Box::new(AppError {
            code: "unsupportedShortcut".into(),
            message: format!("unsupported global shortcut config: {shortcut_config}"),
        }));
    };

    app.global_shortcut().register(shortcut)?;
    Ok(())
}

#[cfg(not(desktop))]
fn register_global_shortcut(
    _app: &AppHandle,
    _shortcut_config: &str,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(desktop)]
fn apply_global_shortcut_config(
    app: &AppHandle,
    shortcut_config: &str,
) -> Result<(), Box<dyn Error>> {
    let Some(shortcut) = shortcut_from_config(shortcut_config).and_then(to_tauri_shortcut) else {
        return Err(Box::new(AppError {
            code: "unsupportedShortcut".into(),
            message: format!("unsupported global shortcut config: {shortcut_config}"),
        }));
    };

    app.global_shortcut().unregister_all()?;
    app.global_shortcut().register(shortcut)?;
    Ok(())
}

#[cfg(not(desktop))]
fn apply_global_shortcut_config(
    _app: &AppHandle,
    _shortcut_config: &str,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(desktop)]
fn to_tauri_shortcut(spec: ShortcutSpec) -> Option<Shortcut> {
    let modifier = match spec.modifier {
        ShortcutModifier::Control => Modifiers::CONTROL,
        ShortcutModifier::Alt => Modifiers::ALT,
    };
    let key = match spec.key {
        ShortcutKey::Space => Code::Space,
    };

    Some(Shortcut::new(Some(modifier), key))
}

#[cfg(desktop)]
fn sync_autostart_to_config(app: &AppHandle) {
    let Ok(config) = load_config() else {
        return;
    };

    if let Err(error) = apply_autostart(app, config.autostart) {
        eprintln!("failed to sync autostart config: {error}");
    }
}

#[cfg(not(desktop))]
fn sync_autostart_to_config(_app: &AppHandle) {}

#[cfg(desktop)]
fn autostart_enabled(app: &AppHandle, fallback: bool) -> bool {
    app.autolaunch().is_enabled().unwrap_or(fallback)
}

#[cfg(not(desktop))]
fn autostart_enabled(_app: &AppHandle, fallback: bool) -> bool {
    fallback
}

fn toggle_autostart(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let store = default_store()?;
    let mut config = store.load_config()?;
    let next_enabled = !config.autostart;
    apply_autostart(app, next_enabled)?;
    config.autostart = next_enabled;
    store.save_config(config)?;
    Ok(())
}

#[cfg(desktop)]
fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), Box<dyn Error>> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()?;
    } else {
        manager.disable()?;
    }
    Ok(())
}

#[cfg(not(desktop))]
fn apply_autostart(_app: &AppHandle, _enabled: bool) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tray_menu_ids_to_actions() {
        assert_eq!(
            tray_menu_action("show-main"),
            Some(TrayMenuAction::ShowMain)
        );
        assert_eq!(
            tray_menu_action("quick-note"),
            Some(TrayMenuAction::QuickNote)
        );
        assert_eq!(
            tray_menu_action("toggle-close-to-tray"),
            Some(TrayMenuAction::ToggleCloseToTray)
        );
        assert_eq!(
            tray_menu_action("toggle-autostart"),
            Some(TrayMenuAction::ToggleAutostart)
        );
        assert_eq!(tray_menu_action("quit"), Some(TrayMenuAction::Quit));
        assert_eq!(tray_menu_action("unknown"), None);
    }

    #[test]
    fn builds_tray_menu_specs_with_configured_checked_state() {
        let specs = tray_menu_specs(true, false);
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
    fn parses_supported_shortcut_config_values() {
        assert_eq!(
            shortcut_from_config("Ctrl+Space"),
            Some(ShortcutSpec {
                modifier: ShortcutModifier::Control,
                key: ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut_from_config("CommandOrControl + Space"),
            Some(ShortcutSpec {
                modifier: ShortcutModifier::Control,
                key: ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut_from_config("Alt+Space"),
            Some(ShortcutSpec {
                modifier: ShortcutModifier::Alt,
                key: ShortcutKey::Space,
            })
        );
    }

    #[test]
    fn rejects_unsupported_shortcut_config_values() {
        assert_eq!(shortcut_from_config(""), None);
        assert_eq!(shortcut_from_config("Ctrl+Shift+Space"), None);
        assert_eq!(shortcut_from_config("Ctrl+K"), None);
    }

    #[test]
    fn detects_runtime_config_changes() {
        let previous = AppConfig {
            notes_dir: "D:\\notes".into(),
            global_shortcut: "Ctrl+Space".into(),
            close_to_tray: true,
            autostart: false,
            default_view_mode: "split".into(),
            note_surface_auto_save: true,
        };
        let next = AppConfig {
            notes_dir: "D:\\other-notes".into(),
            global_shortcut: "Alt+Space".into(),
            close_to_tray: false,
            autostart: true,
            default_view_mode: "preview".into(),
            note_surface_auto_save: false,
        };

        assert_eq!(
            runtime_config_changes(&previous, &next),
            RuntimeConfigChanges {
                autostart_changed: true,
                global_shortcut_changed: true,
            }
        );
        assert_eq!(
            runtime_config_changes(&previous, &previous),
            RuntimeConfigChanges {
                autostart_changed: false,
                global_shortcut_changed: false,
            }
        );
    }

    #[test]
    fn builds_stable_dynamic_window_labels() {
        assert_eq!(notepad_window_label(Some("abc-123")), "notepad-abc-123");
        assert!(notepad_window_label(None).starts_with("notepad-"));
        assert_eq!(tile_window_label("note-1"), "tile-note-1");
    }

    #[test]
    fn makes_note_surfaces_transparent_for_rounded_tile_corners() {
        assert_eq!(
            dynamic_window_visual_options("notepad-note-1"),
            DynamicWindowVisualOptions {
                transparent: true,
                corner_radius: Some(14),
            }
        );
        assert_eq!(
            dynamic_window_visual_options("tile-note-1"),
            DynamicWindowVisualOptions {
                transparent: true,
                corner_radius: Some(14),
            }
        );
        assert_eq!(
            dynamic_window_visual_options("main"),
            DynamicWindowVisualOptions {
                transparent: false,
                corner_radius: None,
            }
        );
    }
}
