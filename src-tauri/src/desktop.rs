use crate::{
    locales::{self, Locale},
    services::notes::{default_store, AppConfig, AppError},
};
use serde::Deserialize;
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindowBuilder, Window, WindowEvent, Wry,
};
use uuid::Uuid;

#[cfg(desktop)]
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartExt};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "main-tray";
const TRAY_SHOW_MAIN_ID: &str = "show-main";
const TRAY_QUICK_NOTE_ID: &str = "quick-note";
const TRAY_TOGGLE_CLOSE_TO_TRAY_ID: &str = "toggle-close-to-tray";
const TRAY_TOGGLE_AUTOSTART_ID: &str = "toggle-autostart";
const TRAY_QUIT_ID: &str = "quit";
const NOTEPAD_POOL_CAPACITY: usize = 2;

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
pub enum ShortcutKey {
    Letter(char),
    Digit(u8),
    Function(u8),
    Space,
    Tab,
    Enter,
    Backspace,
    Delete,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeConfigChanges {
    pub autostart_changed: bool,
    pub global_shortcut_changed: bool,
    pub toggle_visibility_shortcut_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: ShortcutKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicWindowVisualOptions {
    pub transparent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainWindowCloseAction {
    AllowClose,
    HideToTray,
    ExitApp,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct WindowSizeSpec {
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
}

struct WindowOpenOptions {
    url: String,
    title: String,
    specs: WindowSizeSpec,
    decorations: bool,
    always_on_top: bool,
    shadow: bool,
    skip_taskbar: bool,
    bounds: Option<WindowBounds>,
}

#[derive(Default)]
struct RuntimeState {
    is_exiting: AtomicBool,
    windows_hidden: AtomicBool,
    hidden_window_labels: Mutex<Vec<String>>,
    #[cfg(desktop)]
    shortcut_bindings: Mutex<ShortcutBindings>,
}

#[cfg(desktop)]
#[derive(Clone, Default)]
struct ShortcutBindings {
    open_notepad: Option<Shortcut>,
    toggle_visibility: Option<Shortcut>,
}

#[cfg(desktop)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    OpenNotepad,
    ToggleVisibility,
}

#[derive(Default)]
struct NotepadPool {
    available: Mutex<Vec<String>>,
}

impl NotepadPool {
    fn take(&self) -> Option<String> {
        self.available.lock().ok()?.pop()
    }

    fn put(&self, label: String) -> bool {
        if let Ok(mut available) = self.available.lock() {
            if available.len() < NOTEPAD_POOL_CAPACITY {
                available.push(label);
                return true;
            }
        }
        false
    }

    fn is_below_capacity(&self) -> bool {
        self.available
            .lock()
            .map(|a| a.len() < NOTEPAD_POOL_CAPACITY)
            .unwrap_or(false)
    }
}

impl RuntimeState {
    fn allow_exit(&self) {
        self.is_exiting.store(true, Ordering::SeqCst);
    }

    fn is_exiting(&self) -> bool {
        self.is_exiting.load(Ordering::SeqCst)
    }

    fn clear_hidden_windows(&self) {
        if !self.windows_hidden.swap(false, Ordering::SeqCst) {
            return;
        }

        if let Ok(mut guard) = self.hidden_window_labels.lock() {
            guard.clear();
        }
    }

    fn take_hidden_window_labels(&self) -> Option<Vec<String>> {
        if !self.windows_hidden.swap(false, Ordering::SeqCst) {
            return None;
        }

        self.hidden_window_labels
            .lock()
            .map(|mut guard| guard.drain(..).collect())
            .ok()
    }

    fn hide_windows(&self, labels: Vec<String>) {
        if labels.is_empty() {
            self.clear_hidden_windows();
            return;
        }

        if let Ok(mut guard) = self.hidden_window_labels.lock() {
            *guard = labels;
            self.windows_hidden.store(true, Ordering::SeqCst);
        }
    }

    #[cfg(desktop)]
    fn set_shortcut_bindings(&self, bindings: ShortcutBindings) {
        if let Ok(mut guard) = self.shortcut_bindings.lock() {
            *guard = bindings;
        }
    }

    #[cfg(desktop)]
    fn shortcut_action(&self, shortcut: &Shortcut) -> ShortcutAction {
        self.shortcut_bindings
            .lock()
            .ok()
            .and_then(|bindings| bindings.action_for(shortcut))
            .unwrap_or(ShortcutAction::OpenNotepad)
    }
}

#[cfg(desktop)]
impl ShortcutBindings {
    fn action_for(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        if self
            .toggle_visibility
            .as_ref()
            .is_some_and(|s| s == shortcut)
        {
            Some(ShortcutAction::ToggleVisibility)
        } else if self.open_notepad.as_ref().is_some_and(|s| s == shortcut) {
            Some(ShortcutAction::OpenNotepad)
        } else {
            None
        }
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

pub fn tray_menu_specs(locale: Locale, close_to_tray: bool, autostart: bool) -> Vec<TrayMenuSpec> {
    vec![
        TrayMenuSpec {
            id: TRAY_SHOW_MAIN_ID,
            label: locales::tray_show_main_label(locale),
            checked: None,
        },
        TrayMenuSpec {
            id: TRAY_QUICK_NOTE_ID,
            label: locales::tray_quick_note_label(locale),
            checked: None,
        },
        TrayMenuSpec {
            id: TRAY_TOGGLE_CLOSE_TO_TRAY_ID,
            label: locales::tray_toggle_close_to_tray_label(locale),
            checked: Some(close_to_tray),
        },
        TrayMenuSpec {
            id: TRAY_TOGGLE_AUTOSTART_ID,
            label: locales::tray_toggle_autostart_label(locale),
            checked: Some(autostart),
        },
        TrayMenuSpec {
            id: TRAY_QUIT_ID,
            label: locales::tray_quit_label(locale),
            checked: None,
        },
    ]
}

fn locale_from_config(config: &AppConfig) -> Locale {
    Locale::from_tag(&config.locale)
}

fn configured_locale() -> Locale {
    load_config()
        .map(|config| locale_from_config(&config))
        .unwrap_or_default()
}

fn build_tray_menu(app: &AppHandle, config: &AppConfig) -> Result<Menu<Wry>, Box<dyn Error>> {
    let locale = locale_from_config(config);
    let autostart = autostart_enabled(app, config.autostart);
    let specs = tray_menu_specs(locale, config.close_to_tray, autostart);

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

    Ok(Menu::with_items(
        app,
        &[
            &show_main,
            &quick_note,
            &close_to_tray,
            &autostart,
            &separator,
            &quit,
        ],
    )?)
}

fn refresh_tray_menu(app: &AppHandle, config: &AppConfig) -> Result<(), Box<dyn Error>> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let menu = build_tray_menu(app, config)?;
    tray.set_menu(Some(menu))?;
    tray.set_tooltip(Some(locales::tray_tooltip(locale_from_config(config))))?;
    Ok(())
}

fn refresh_window_titles(app: &AppHandle, config: &AppConfig) -> Result<(), AppError> {
    let locale = locale_from_config(config);

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.set_title(locales::main_window_title(locale))?;
    }

    for (label, window) in app.webview_windows() {
        if label.starts_with("notepad-") {
            window.set_title(locales::notepad_window_title(locale))?;
        } else if label.starts_with("tile-") {
            window.set_title(locales::tile_window_title(locale))?;
        }
    }

    Ok(())
}

pub fn refresh_shell_state(app: &AppHandle, config: &AppConfig) -> Result<(), Box<dyn Error>> {
    refresh_window_titles(app, config)?;
    refresh_tray_menu(app, config)?;
    Ok(())
}

pub fn shortcut_from_config(value: &str) -> Option<ShortcutSpec> {
    let parts: Vec<_> = value
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();

    if parts.len() < 2 {
        return None;
    }

    let (modifier_parts, key_part) = parts.split_at(parts.len() - 1);

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;

    for m in modifier_parts {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => ctrl = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            "meta" | "cmd" | "command" | "super" => meta = true,
            _ => return None,
        }
    }

    if !ctrl && !alt && !meta {
        return None;
    }

    let key = parse_shortcut_key(key_part[0])?;

    Some(ShortcutSpec {
        ctrl,
        alt,
        shift,
        meta,
        key,
    })
}

fn parse_shortcut_key(key: &str) -> Option<ShortcutKey> {
    if key.len() == 1 {
        let c = key.chars().next()?;
        if c.is_ascii_alphabetic() {
            return Some(ShortcutKey::Letter(c.to_ascii_uppercase()));
        }
        if c.is_ascii_digit() {
            return Some(ShortcutKey::Digit(c.to_digit(10)? as u8));
        }
    }

    if let Some(rest) = key.strip_prefix('F').or_else(|| key.strip_prefix('f')) {
        if let Ok(num) = rest.parse::<u8>() {
            if (1..=12).contains(&num) {
                return Some(ShortcutKey::Function(num));
            }
        }
    }

    match key.to_ascii_lowercase().as_str() {
        "space" => Some(ShortcutKey::Space),
        "tab" => Some(ShortcutKey::Tab),
        "enter" => Some(ShortcutKey::Enter),
        "backspace" => Some(ShortcutKey::Backspace),
        "delete" => Some(ShortcutKey::Delete),
        "escape" => Some(ShortcutKey::Escape),
        "arrowup" => Some(ShortcutKey::ArrowUp),
        "arrowdown" => Some(ShortcutKey::ArrowDown),
        "arrowleft" => Some(ShortcutKey::ArrowLeft),
        "arrowright" => Some(ShortcutKey::ArrowRight),
        "home" => Some(ShortcutKey::Home),
        "end" => Some(ShortcutKey::End),
        "pageup" => Some(ShortcutKey::PageUp),
        "pagedown" => Some(ShortcutKey::PageDown),
        _ => None,
    }
}

pub fn runtime_config_changes(previous: &AppConfig, next: &AppConfig) -> RuntimeConfigChanges {
    RuntimeConfigChanges {
        autostart_changed: previous.autostart != next.autostart,
        global_shortcut_changed: previous.global_shortcut != next.global_shortcut,
        toggle_visibility_shortcut_changed: previous.toggle_visibility_shortcut
            != next.toggle_visibility_shortcut,
    }
}

fn clear_hidden_window_state(app: &AppHandle) {
    let labels = app
        .try_state::<RuntimeState>()
        .and_then(|state| state.take_hidden_window_labels());

    let Some(labels) = labels else {
        return;
    };

    for label in &labels {
        if label.starts_with("notepad-") || label.starts_with("tile-") {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.close();
            }
        }
    }
}

fn toggle_app_visibility(app: &AppHandle) {
    let Some(state) = app.try_state::<RuntimeState>() else {
        return;
    };

    if let Some(labels) = state.take_hidden_window_labels() {
        let mut focus_target = None;
        for label in &labels {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.unminimize();
                let _ = window.show();
                if focus_target.is_none() || label == MAIN_WINDOW_LABEL {
                    focus_target = Some(label.clone());
                }
            }
        }

        if let Some(label) = focus_target {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.set_focus();
            }
        }
        return;
    }

    let mut labels = Vec::new();
    for (label, window) in app.webview_windows() {
        if window.is_visible().unwrap_or(false) {
            labels.push(label.clone());
            let _ = window.hide();
        }
    }
    state.hide_windows(labels);
}

pub fn apply_runtime_config(
    app: &AppHandle,
    previous: &AppConfig,
    next: &AppConfig,
) -> Result<(), Box<dyn Error>> {
    let changes = runtime_config_changes(previous, next);

    if changes.global_shortcut_changed || changes.toggle_visibility_shortcut_changed {
        apply_global_shortcut_config(app, next)?;
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

pub fn extract_file_arg(args: &[String]) -> Option<String> {
    args.iter()
        .find(|arg| {
            let lower = arg.to_lowercase();
            lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".txt")
        })
        .cloned()
}

pub fn setup_desktop(app: &mut App) -> Result<(), Box<dyn Error>> {
    app.manage(RuntimeState::default());
    app.manage(NotepadPool::default());
    setup_autostart_plugin(app.handle())?;
    setup_global_shortcut_plugin(app.handle())?;
    sync_autostart_to_config(app.handle());
    register_configured_global_shortcut(app.handle());
    setup_tray(app)?;
    schedule_notepad_prewarm(app.handle());

    if !std::env::args().any(|a| a == "--silent") {
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
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    match main_window_close_action(app_is_exiting(window.app_handle()), close_to_tray_enabled()) {
        MainWindowCloseAction::AllowClose => {}
        MainWindowCloseAction::HideToTray => {
            api.prevent_close();
            if let Err(error) = window.hide() {
                eprintln!("failed to hide main window to tray: {error}");
            }
        }
        MainWindowCloseAction::ExitApp => {
            api.prevent_close();
            mark_app_exiting(window.app_handle());
            window.app_handle().exit(0);
        }
    }
}

fn main_window_close_action(app_is_exiting: bool, close_to_tray: bool) -> MainWindowCloseAction {
    if app_is_exiting {
        MainWindowCloseAction::AllowClose
    } else if close_to_tray {
        MainWindowCloseAction::HideToTray
    } else {
        MainWindowCloseAction::ExitApp
    }
}

fn setup_tray(app: &mut App) -> Result<(), Box<dyn Error>> {
    let config = load_config()?;
    let menu = build_tray_menu(app.handle(), &config)?;
    let locale = locale_from_config(&config);

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(
            app.default_window_icon()
                .expect("missing default window icon")
                .clone(),
        )
        .tooltip(locales::tray_tooltip(locale))
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
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
            let config = toggle_close_to_tray(app)?;
            if let Err(error) = refresh_shell_state(app, &config) {
                eprintln!("failed to refresh desktop shell state after tray toggle: {error}");
            }
            let _ = app.emit("config-changed", &config);
        }
        Some(TrayMenuAction::ToggleAutostart) => {
            let config = toggle_autostart(app)?;
            if let Err(error) = refresh_shell_state(app, &config) {
                eprintln!("failed to refresh desktop shell state after tray toggle: {error}");
            }
            let _ = app.emit("config-changed", &config);
        }
        Some(TrayMenuAction::Quit) => {
            mark_app_exiting(app);
            app.exit(0);
        }
        None => {}
    }

    Ok(())
}

fn toggle_close_to_tray(_app: &AppHandle) -> Result<AppConfig, Box<dyn Error>> {
    let store = default_store()?;
    let mut config = store.load_config()?;
    config.close_to_tray = !config.close_to_tray;
    store.save_config(config.clone())?;
    Ok(config)
}

pub fn show_main_window(app: &AppHandle) -> Result<(), AppError> {
    clear_hidden_window_state(app);
    let locale = configured_locale();

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.set_title(locales::main_window_title(locale))?;
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    open_or_focus_window(
        app,
        MAIN_WINDOW_LABEL,
        WindowOpenOptions {
            url: "index.html".to_string(),
            title: locales::main_window_title(locale).to_string(),
            specs: WindowSizeSpec {
                width: 1180.0,
                height: 760.0,
                min_width: 900.0,
                min_height: 620.0,
            },
            decorations: false,
            always_on_top: false,
            shadow: true,
            skip_taskbar: false,
            bounds: None,
        },
    )?;
    Ok(())
}

fn open_notepad_window_now(
    app: &AppHandle,
    note_id: Option<&str>,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    if note_id.is_none() {
        if let Some(reused) = activate_pooled_notepad(app, bounds) {
            clear_hidden_window_state(app);
            return Ok(reused);
        }
    }

    let locale = configured_locale();
    let label = notepad_window_label(note_id);
    let specs = saved_surface_specs(app);
    let url = match note_id {
        Some(id) => format!("index.html?view=notepad&noteId={id}"),
        None => "index.html?view=notepad".to_string(),
    };

    open_or_focus_window(
        app,
        &label,
        WindowOpenOptions {
            url,
            title: locales::notepad_window_title(locale).to_string(),
            specs,
            decorations: false,
            always_on_top: true,
            shadow: false,
            skip_taskbar: true,
            bounds,
        },
    )
}

fn activate_pooled_notepad(app: &AppHandle, bounds: Option<WindowBounds>) -> Option<String> {
    let pool = app.try_state::<NotepadPool>()?;
    let label = pool.take()?;
    let window = app.get_webview_window(&label)?;
    let locale = configured_locale();

    let specs = saved_surface_specs(app);
    let _ = window.set_title(locales::notepad_window_title(locale));
    let _ = window.set_size(tauri::LogicalSize::new(specs.width, specs.height));
    let _ = apply_window_bounds(&window, bounds);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("notepad:activate", label.clone());

    schedule_notepad_replenish(app, 100);

    Some(label)
}

pub fn recycle_notepad_window(app: &AppHandle, label: &str) -> Result<(), AppError> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };

    save_surface_size(&window);

    window.hide()?;

    let recycled = app
        .try_state::<NotepadPool>()
        .map(|pool| pool.put(label.to_string()))
        .unwrap_or(false);

    if !recycled {
        window.close()?;
    }

    Ok(())
}

fn save_surface_size(window: &tauri::WebviewWindow) {
    let Ok(store) = default_store() else {
        return;
    };
    let Ok(mut config) = store.load_config() else {
        return;
    };
    if !config.remember_surface_size {
        return;
    }
    let Ok(size) = window.inner_size() else {
        return;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let logical = size.to_logical::<f64>(scale);
    let w = logical.width.round() as u32;
    let h = logical.height.round() as u32;
    if w == 0 || h == 0 {
        return;
    }
    if config.surface_width == Some(w) && config.surface_height == Some(h) {
        return;
    }
    config.surface_width = Some(w);
    config.surface_height = Some(h);
    let _ = store.save_config(config);
}

fn schedule_notepad_prewarm(app: &AppHandle) {
    for i in 0..NOTEPAD_POOL_CAPACITY {
        let delay = 800 + i as u64 * 400;
        schedule_notepad_replenish(app, delay);
    }
}

fn schedule_notepad_replenish(app: &AppHandle, delay_ms: u64) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        let handle_inner = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if let Err(error) = prewarm_notepad(&handle_inner) {
                eprintln!("failed to replenish notepad pool: {error}");
            }
        });
    });
}

fn prewarm_notepad(app: &AppHandle) -> Result<(), AppError> {
    let pool = app.try_state::<NotepadPool>().ok_or_else(|| AppError {
        code: "noPool".into(),
        message: "notepad pool not initialized".into(),
        details: Default::default(),
    })?;

    if !pool.is_below_capacity() {
        return Ok(());
    }

    let label = notepad_window_label(None);
    let specs = notepad_window_specs();
    let visual_options = dynamic_window_visual_options(&label);
    let locale = configured_locale();

    WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App("index.html?view=notepad&standby=1".into()),
    )
    .title(locales::notepad_window_title(locale))
    .inner_size(specs.width, specs.height)
    .min_inner_size(specs.min_width, specs.min_height)
    .resizable(true)
    .decorations(false)
    .transparent(visual_options.transparent)
    .always_on_top(true)
    .shadow(false)
    .skip_taskbar(true)
    .visible(false)
    .focused(false)
    .build()?;

    pool.put(label);

    Ok(())
}

fn notepad_window_specs() -> WindowSizeSpec {
    WindowSizeSpec {
        width: 260.0,
        height: 260.0,
        min_width: 220.0,
        min_height: 220.0,
    }
}

fn saved_surface_specs(app: &AppHandle) -> WindowSizeSpec {
    let defaults = notepad_window_specs();
    let Ok(config) = load_config() else {
        return defaults;
    };
    if !config.remember_surface_size {
        return defaults;
    }
    if let Some((w, h)) = visible_surface_size(app) {
        return WindowSizeSpec {
            width: w.max(defaults.min_width),
            height: h.max(defaults.min_height),
            ..defaults
        };
    }
    match (config.surface_width, config.surface_height) {
        (Some(w), Some(h)) => WindowSizeSpec {
            width: (w as f64).max(defaults.min_width),
            height: (h as f64).max(defaults.min_height),
            ..defaults
        },
        _ => defaults,
    }
}

fn visible_surface_size(app: &AppHandle) -> Option<(f64, f64)> {
    let mut fallback: Option<(f64, f64)> = None;
    for (label, window) in app.webview_windows() {
        if !label.starts_with("notepad-") && !label.starts_with("tile-") {
            continue;
        }
        if !window.is_visible().unwrap_or(false) {
            continue;
        }
        let size = window.inner_size().ok()?;
        let scale = window.scale_factor().unwrap_or(1.0);
        let logical = size.to_logical::<f64>(scale);
        if logical.width <= 0.0 || logical.height <= 0.0 {
            continue;
        }
        if window.is_focused().unwrap_or(false) {
            return Some((logical.width, logical.height));
        }
        if fallback.is_none() {
            fallback = Some((logical.width, logical.height));
        }
    }
    fallback
}

fn open_tile_window_now(
    app: &AppHandle,
    note_id: &str,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    let locale = configured_locale();
    let label = tile_window_label(note_id);
    let url = format!("index.html?view=tile&noteId={note_id}");

    let specs = saved_surface_specs(app);

    open_or_focus_window(
        app,
        &label,
        WindowOpenOptions {
            url,
            title: locales::tile_window_title(locale).to_string(),
            specs,
            decorations: false,
            always_on_top: true,
            shadow: false,
            skip_taskbar: true,
            bounds,
        },
    )
}

fn open_or_focus_window(
    app: &AppHandle,
    label: &str,
    opts: WindowOpenOptions,
) -> Result<String, AppError> {
    clear_hidden_window_state(app);

    let visual_options = dynamic_window_visual_options(label);

    if let Some(window) = app.get_webview_window(label) {
        window.set_title(&opts.title)?;
        apply_window_bounds(&window, opts.bounds)?;
        window.set_shadow(opts.shadow)?;
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(label.to_string());
    }

    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(opts.url.into()))
        .title(opts.title)
        .inner_size(opts.specs.width, opts.specs.height)
        .min_inner_size(opts.specs.min_width, opts.specs.min_height)
        .resizable(true)
        .decorations(opts.decorations)
        .transparent(visual_options.transparent)
        .always_on_top(opts.always_on_top)
        .shadow(opts.shadow)
        .skip_taskbar(opts.skip_taskbar)
        .visible(false);

    if let Some(bounds) = opts.bounds {
        builder = builder
            .position(bounds.x as f64, bounds.y as f64)
            .inner_size(bounds.width as f64, bounds.height as f64);
    }

    builder.build()?;

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
        Some(vec!["--silent"]),
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
            .with_handler(|app, shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }

                let action = app
                    .try_state::<RuntimeState>()
                    .map(|state| state.shortcut_action(shortcut))
                    .unwrap_or(ShortcutAction::OpenNotepad);

                let app_for_closure = app.clone();
                match action {
                    ShortcutAction::ToggleVisibility => {
                        if let Err(error) = app.run_on_main_thread(move || {
                            toggle_app_visibility(&app_for_closure);
                        }) {
                            eprintln!("failed to dispatch visibility toggle action: {error}");
                        }
                    }
                    ShortcutAction::OpenNotepad => {
                        if let Err(error) = app.run_on_main_thread(move || {
                            if let Err(error) =
                                open_notepad_window_now(&app_for_closure, None, None)
                            {
                                eprintln!("failed to open notepad from global shortcut: {error}");
                            }
                        }) {
                            eprintln!("failed to dispatch global shortcut action: {error}");
                        }
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

    if let Err(error) = install_global_shortcut_bindings(app, &config, false) {
        let msg = format!("failed to register global shortcuts: {error}");
        eprintln!("{msg}");
        let _ = app.emit("shortcut-register-failed", &msg);
    }
}

#[cfg(not(desktop))]
fn register_configured_global_shortcut(_app: &AppHandle) {}

#[cfg(desktop)]
fn parse_configured_shortcut(field: &str, value: &str) -> Result<Shortcut, Box<dyn Error>> {
    shortcut_from_config(value)
        .and_then(to_tauri_shortcut)
        .ok_or_else(|| {
            Box::new(AppError {
                code: "unsupportedShortcut".into(),
                message: format!("unsupported {field} shortcut config: {value}"),
                details: [("field".to_string(), field.to_string())]
                    .into_iter()
                    .collect(),
            }) as Box<dyn Error>
        })
}

#[cfg(desktop)]
fn shortcut_bindings_from_config(config: &AppConfig) -> Result<ShortcutBindings, Box<dyn Error>> {
    let open_notepad = parse_configured_shortcut("globalShortcut", &config.global_shortcut)?;
    let toggle_visibility = if config.toggle_visibility_shortcut.is_empty() {
        None
    } else {
        Some(parse_configured_shortcut(
            "toggleVisibilityShortcut",
            &config.toggle_visibility_shortcut,
        )?)
    };

    if toggle_visibility
        .as_ref()
        .is_some_and(|shortcut| shortcut == &open_notepad)
    {
        return Err(Box::new(AppError {
            code: "duplicateShortcut".into(),
            message: "visibility toggle shortcut must differ from global shortcut".into(),
            details: Default::default(),
        }));
    }

    Ok(ShortcutBindings {
        open_notepad: Some(open_notepad),
        toggle_visibility,
    })
}

#[cfg(desktop)]
fn install_global_shortcut_bindings(
    app: &AppHandle,
    config: &AppConfig,
    replace_existing: bool,
) -> Result<(), Box<dyn Error>> {
    let bindings = shortcut_bindings_from_config(config)?;

    if replace_existing {
        app.global_shortcut().unregister_all()?;
    }

    if let Some(shortcut) = &bindings.open_notepad {
        app.global_shortcut().register(*shortcut)?;
    }
    if let Some(shortcut) = &bindings.toggle_visibility {
        app.global_shortcut().register(*shortcut)?;
    }

    if let Some(state) = app.try_state::<RuntimeState>() {
        state.set_shortcut_bindings(bindings);
    }

    Ok(())
}

#[cfg(desktop)]
fn apply_global_shortcut_config(app: &AppHandle, config: &AppConfig) -> Result<(), Box<dyn Error>> {
    install_global_shortcut_bindings(app, config, true)
}

#[cfg(not(desktop))]
fn apply_global_shortcut_config(
    _app: &AppHandle,
    _config: &AppConfig,
) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(desktop)]
fn to_tauri_shortcut(spec: ShortcutSpec) -> Option<Shortcut> {
    let mut modifiers = Modifiers::empty();
    if spec.ctrl {
        modifiers |= Modifiers::CONTROL;
    }
    if spec.alt {
        modifiers |= Modifiers::ALT;
    }
    if spec.shift {
        modifiers |= Modifiers::SHIFT;
    }
    if spec.meta {
        modifiers |= Modifiers::META;
    }

    let code = shortcut_key_to_code(spec.key)?;
    let mod_opt = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Some(Shortcut::new(mod_opt, code))
}

#[cfg(desktop)]
fn shortcut_key_to_code(key: ShortcutKey) -> Option<Code> {
    Some(match key {
        ShortcutKey::Letter(c) => match c {
            'A' => Code::KeyA,
            'B' => Code::KeyB,
            'C' => Code::KeyC,
            'D' => Code::KeyD,
            'E' => Code::KeyE,
            'F' => Code::KeyF,
            'G' => Code::KeyG,
            'H' => Code::KeyH,
            'I' => Code::KeyI,
            'J' => Code::KeyJ,
            'K' => Code::KeyK,
            'L' => Code::KeyL,
            'M' => Code::KeyM,
            'N' => Code::KeyN,
            'O' => Code::KeyO,
            'P' => Code::KeyP,
            'Q' => Code::KeyQ,
            'R' => Code::KeyR,
            'S' => Code::KeyS,
            'T' => Code::KeyT,
            'U' => Code::KeyU,
            'V' => Code::KeyV,
            'W' => Code::KeyW,
            'X' => Code::KeyX,
            'Y' => Code::KeyY,
            'Z' => Code::KeyZ,
            _ => return None,
        },
        ShortcutKey::Digit(d) => match d {
            0 => Code::Digit0,
            1 => Code::Digit1,
            2 => Code::Digit2,
            3 => Code::Digit3,
            4 => Code::Digit4,
            5 => Code::Digit5,
            6 => Code::Digit6,
            7 => Code::Digit7,
            8 => Code::Digit8,
            9 => Code::Digit9,
            _ => return None,
        },
        ShortcutKey::Function(n) => match n {
            1 => Code::F1,
            2 => Code::F2,
            3 => Code::F3,
            4 => Code::F4,
            5 => Code::F5,
            6 => Code::F6,
            7 => Code::F7,
            8 => Code::F8,
            9 => Code::F9,
            10 => Code::F10,
            11 => Code::F11,
            12 => Code::F12,
            _ => return None,
        },
        ShortcutKey::Space => Code::Space,
        ShortcutKey::Tab => Code::Tab,
        ShortcutKey::Enter => Code::Enter,
        ShortcutKey::Backspace => Code::Backspace,
        ShortcutKey::Delete => Code::Delete,
        ShortcutKey::Escape => Code::Escape,
        ShortcutKey::ArrowUp => Code::ArrowUp,
        ShortcutKey::ArrowDown => Code::ArrowDown,
        ShortcutKey::ArrowLeft => Code::ArrowLeft,
        ShortcutKey::ArrowRight => Code::ArrowRight,
        ShortcutKey::Home => Code::Home,
        ShortcutKey::End => Code::End,
        ShortcutKey::PageUp => Code::PageUp,
        ShortcutKey::PageDown => Code::PageDown,
    })
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

fn toggle_autostart(app: &AppHandle) -> Result<AppConfig, Box<dyn Error>> {
    let store = default_store()?;
    let mut config = store.load_config()?;
    let next_enabled = !config.autostart;
    apply_autostart(app, next_enabled)?;
    config.autostart = next_enabled;
    store.save_config(config.clone())?;
    Ok(config)
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
        let specs = tray_menu_specs(Locale::ZhCn, true, false);
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
            shortcut_from_config("Ctrl+Space"),
            Some(ShortcutSpec {
                ctrl: true,
                alt: false,
                shift: false,
                meta: false,
                key: ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut_from_config("CommandOrControl + Space"),
            Some(ShortcutSpec {
                ctrl: true,
                alt: false,
                shift: false,
                meta: false,
                key: ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut_from_config("Alt+Space"),
            Some(ShortcutSpec {
                ctrl: false,
                alt: true,
                shift: false,
                meta: false,
                key: ShortcutKey::Space,
            })
        );
        assert_eq!(
            shortcut_from_config("Ctrl+Shift+K"),
            Some(ShortcutSpec {
                ctrl: true,
                alt: false,
                shift: true,
                meta: false,
                key: ShortcutKey::Letter('K'),
            })
        );
        assert_eq!(
            shortcut_from_config("Alt+F2"),
            Some(ShortcutSpec {
                ctrl: false,
                alt: true,
                shift: false,
                meta: false,
                key: ShortcutKey::Function(2),
            })
        );
        assert_eq!(
            shortcut_from_config("Ctrl+Alt+3"),
            Some(ShortcutSpec {
                ctrl: true,
                alt: true,
                shift: false,
                meta: false,
                key: ShortcutKey::Digit(3),
            })
        );
        assert_eq!(
            shortcut_from_config("Command+K"),
            Some(ShortcutSpec {
                ctrl: false,
                alt: false,
                shift: false,
                meta: true,
                key: ShortcutKey::Letter('K'),
            })
        );
        assert_eq!(
            shortcut_from_config("Meta+Shift+P"),
            Some(ShortcutSpec {
                ctrl: false,
                alt: false,
                shift: true,
                meta: true,
                key: ShortcutKey::Letter('P'),
            })
        );
    }

    #[test]
    fn rejects_invalid_shortcut_config_values() {
        assert_eq!(shortcut_from_config(""), None);
        assert_eq!(shortcut_from_config("Space"), None);
        assert_eq!(shortcut_from_config("Shift+K"), None);
        assert_eq!(shortcut_from_config("Ctrl+Unknown"), None);
    }

    #[cfg(desktop)]
    #[test]
    fn rejects_duplicate_shortcut_bindings() {
        let config = AppConfig {
            locale: "zh-CN".into(),
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
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: "Ctrl+Space".into(),
        };

        let error = match shortcut_bindings_from_config(&config) {
            Ok(_) => panic!("expected duplicate shortcut error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("must differ"));
    }

    #[test]
    fn chooses_exit_when_main_window_closes_without_close_to_tray() {
        assert_eq!(
            main_window_close_action(false, false),
            MainWindowCloseAction::ExitApp
        );
    }

    #[test]
    fn detects_runtime_config_changes() {
        let previous = AppConfig {
            locale: "zh-CN".into(),
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
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: String::new(),
        };
        let next = AppConfig {
            locale: "en-US".into(),
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
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: "Ctrl+Shift+H".into(),
        };

        assert_eq!(
            runtime_config_changes(&previous, &next),
            RuntimeConfigChanges {
                autostart_changed: true,
                global_shortcut_changed: true,
                toggle_visibility_shortcut_changed: true,
            }
        );
        assert_eq!(
            runtime_config_changes(&previous, &previous),
            RuntimeConfigChanges {
                autostart_changed: false,
                global_shortcut_changed: false,
                toggle_visibility_shortcut_changed: false,
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
    fn keeps_notepad_initial_window_compact() {
        let specs = notepad_window_specs();

        assert_eq!(specs.width, 260.0);
        assert_eq!(specs.height, 260.0);
        assert_eq!(specs.min_width, 220.0);
        assert_eq!(specs.min_height, 220.0);
    }

    #[test]
    fn makes_note_surfaces_transparent() {
        assert_eq!(
            dynamic_window_visual_options("notepad-note-1"),
            DynamicWindowVisualOptions { transparent: true }
        );
        assert_eq!(
            dynamic_window_visual_options("tile-note-1"),
            DynamicWindowVisualOptions { transparent: true }
        );
        assert_eq!(
            dynamic_window_visual_options("main"),
            DynamicWindowVisualOptions { transparent: false }
        );
    }

    #[test]
    fn capability_allows_frontend_window_focus_for_notepad_surfaces() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
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
