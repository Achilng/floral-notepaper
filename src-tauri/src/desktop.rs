use crate::{
    locales::{self, Locale},
    services::notes::{default_store, AppConfig, AppError},
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

#[cfg(target_os = "windows")]
mod keyboard_hook {
    use serde::Serialize;
    use std::sync::{
        atomic::{AtomicIsize, AtomicU32, AtomicU8, Ordering},
        Mutex,
    };
    use tauri::{AppHandle, Emitter};

    const WH_KEYBOARD_LL: i32 = 13;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const WM_SYSKEYUP: u32 = 0x0105;
    const WM_QUIT: u32 = 0x0012;
    const WM_HOOK_KEY: u32 = 0x0400 + 1;

    const MOD_CTRL: u8 = 1;
    const MOD_ALT: u8 = 2;
    const MOD_SHIFT: u8 = 4;
    const MOD_META: u8 = 8;

    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    struct KBDLLHOOKSTRUCT {
        vk_code: u32,
        scan_code: u32,
        flags: u32,
        time: u32,
        dw_extra_info: usize,
    }

    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    struct MSG {
        hwnd: isize,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        pt_x: i32,
        pt_y: i32,
    }

    #[allow(clippy::upper_case_acronyms)]
    type HOOKPROC = extern "system" fn(i32, usize, isize) -> isize;

    extern "system" {
        fn SetWindowsHookExW(id_hook: i32, lpfn: HOOKPROC, hmod: isize, dw_thread_id: u32)
            -> isize;
        fn UnhookWindowsHookEx(hhk: isize) -> i32;
        fn CallNextHookEx(hhk: isize, n_code: i32, w_param: usize, l_param: isize) -> isize;
        fn GetMessageW(
            lp_msg: *mut MSG,
            hwnd: isize,
            msg_filter_min: u32,
            msg_filter_max: u32,
        ) -> i32;
        fn PostThreadMessageW(id_thread: u32, msg: u32, w_param: usize, l_param: isize) -> i32;
        fn GetCurrentThreadId() -> u32;
        fn GetModuleHandleW(lp_module_name: *const u16) -> isize;
    }

    #[link(name = "imm32")]
    extern "system" {
        fn ImmGetHotKey(
            dw_hot_key_id: u32,
            lpu_modifiers: *mut u32,
            lpu_vkey: *mut u32,
            phkl: *mut isize,
        ) -> i32;
        fn ImmSetHotKey(dw_hot_key_id: u32, u_modifiers: u32, u_vkey: u32, hkl: isize) -> i32;
    }

    static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);
    static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
    static HOOK_MODS: AtomicU8 = AtomicU8::new(0);
    static HOOK_APP: Mutex<Option<AppHandle>> = Mutex::new(None);
    static HOOK_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
    static SAVED_IME_HOTKEYS: Mutex<Vec<(u32, u32, u32, isize)>> = Mutex::new(Vec::new());

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct HookKeyEvent {
        key: String,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    }

    fn is_modifier_vk(vk: u32) -> bool {
        matches!(
            vk,
            0x10 | 0x11 | 0x12 | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0x5B | 0x5C
        )
    }

    fn update_modifier_state(vk: u32, pressed: bool) {
        let bit = match vk {
            0x10 | 0xA0 | 0xA1 => MOD_SHIFT,
            0x11 | 0xA2 | 0xA3 => MOD_CTRL,
            0x12 | 0xA4 | 0xA5 => MOD_ALT,
            0x5B | 0x5C => MOD_META,
            _ => return,
        };
        if pressed {
            HOOK_MODS.fetch_or(bit, Ordering::SeqCst);
        } else {
            HOOK_MODS.fetch_and(!bit, Ordering::SeqCst);
        }
    }

    fn vk_to_key_name(vk: u32) -> Option<&'static str> {
        Some(match vk {
            0x41 => "A",
            0x42 => "B",
            0x43 => "C",
            0x44 => "D",
            0x45 => "E",
            0x46 => "F",
            0x47 => "G",
            0x48 => "H",
            0x49 => "I",
            0x4A => "J",
            0x4B => "K",
            0x4C => "L",
            0x4D => "M",
            0x4E => "N",
            0x4F => "O",
            0x50 => "P",
            0x51 => "Q",
            0x52 => "R",
            0x53 => "S",
            0x54 => "T",
            0x55 => "U",
            0x56 => "V",
            0x57 => "W",
            0x58 => "X",
            0x59 => "Y",
            0x5A => "Z",
            0x30 => "0",
            0x31 => "1",
            0x32 => "2",
            0x33 => "3",
            0x34 => "4",
            0x35 => "5",
            0x36 => "6",
            0x37 => "7",
            0x38 => "8",
            0x39 => "9",
            0x70 => "F1",
            0x71 => "F2",
            0x72 => "F3",
            0x73 => "F4",
            0x74 => "F5",
            0x75 => "F6",
            0x76 => "F7",
            0x77 => "F8",
            0x78 => "F9",
            0x79 => "F10",
            0x7A => "F11",
            0x7B => "F12",
            0x20 => "Space",
            0x09 => "Tab",
            0x0D => "Enter",
            0x08 => "Backspace",
            0x2E => "Delete",
            0x1B => "Escape",
            0x26 => "ArrowUp",
            0x28 => "ArrowDown",
            0x25 => "ArrowLeft",
            0x27 => "ArrowRight",
            0x24 => "Home",
            0x23 => "End",
            0x21 => "PageUp",
            0x22 => "PageDown",
            0x2D => "Insert",
            0x60 => "Numpad0",
            0x61 => "Numpad1",
            0x62 => "Numpad2",
            0x63 => "Numpad3",
            0x64 => "Numpad4",
            0x65 => "Numpad5",
            0x66 => "Numpad6",
            0x67 => "Numpad7",
            0x68 => "Numpad8",
            0x69 => "Numpad9",
            0x6A => "NumpadMultiply",
            0x6B => "NumpadAdd",
            0x6D => "NumpadSubtract",
            0x6E => "NumpadDecimal",
            0x6F => "NumpadDivide",
            0xBA => ";",
            0xBB => "=",
            0xBC => ",",
            0xBD => "-",
            0xBE => ".",
            0xBF => "/",
            0xC0 => "`",
            0xDB => "[",
            0xDC => "\\",
            0xDD => "]",
            0xDE => "'",
            _ => return None,
        })
    }

    // hook_proc must return ASAP to avoid Windows removing the hook (200ms timeout).
    // We only do atomic reads + PostThreadMessageW here; Tauri event emission
    // happens in the message-pump thread which has no timeout constraint.
    extern "system" fn hook_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
        if n_code >= 0 {
            let kb = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
            let vk = kb.vk_code;

            match w_param as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    if is_modifier_vk(vk) {
                        update_modifier_state(vk, true);
                    } else {
                        let mods = HOOK_MODS.load(Ordering::SeqCst);
                        let tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
                        if tid != 0 {
                            unsafe {
                                PostThreadMessageW(
                                    tid,
                                    WM_HOOK_KEY,
                                    (vk as usize) | ((mods as usize) << 16),
                                    0,
                                );
                            }
                        }
                        return 1;
                    }
                }
                WM_KEYUP | WM_SYSKEYUP => {
                    if is_modifier_vk(vk) {
                        update_modifier_state(vk, false);
                    } else {
                        return 1;
                    }
                }
                _ => {}
            }
        }

        unsafe { CallNextHookEx(HOOK_HANDLE.load(Ordering::SeqCst), n_code, w_param, l_param) }
    }

    fn emit_from_message(w_param: usize) {
        let vk = (w_param & 0xFFFF) as u32;
        let mods = ((w_param >> 16) & 0xFF) as u8;
        if let Some(key_name) = vk_to_key_name(vk) {
            let event = HookKeyEvent {
                key: key_name.to_string(),
                ctrl: mods & MOD_CTRL != 0,
                alt: mods & MOD_ALT != 0,
                shift: mods & MOD_SHIFT != 0,
                meta: mods & MOD_META != 0,
            };
            if let Ok(guard) = HOOK_APP.lock() {
                if let Some(app) = guard.as_ref() {
                    let _ = app.emit("shortcut-hook-key", &event);
                }
            }
        }
    }

    const IME_HOTKEY_IDS: &[u32] = &[
        0x10, // IME_CHOTKEY_IME_NONIME_TOGGLE  (Ctrl+Space)
        0x11, // IME_CHOTKEY_SHAPE_TOGGLE        (Shift+Space)
        0x12, // IME_CHOTKEY_SYMBOL_TOGGLE       (Ctrl+.)
        0x30, // IME_JHOTKEY_CLOSE_OPEN
        0x50, // IME_KHOTKEY_SHAPE_TOGGLE
        0x51, // IME_KHOTKEY_HANJACONVERT
        0x52, // IME_KHOTKEY_ENGLISH
        0x70, // IME_THOTKEY_IME_NONIME_TOGGLE
        0x71, // IME_THOTKEY_SHAPE_TOGGLE
        0x72, // IME_THOTKEY_SYMBOL_TOGGLE
    ];

    fn disable_ime_hotkeys() {
        let mut saved = Vec::new();
        for &id in IME_HOTKEY_IDS {
            let mut modifiers = 0u32;
            let mut vkey = 0u32;
            let mut hkl: isize = 0;
            if unsafe { ImmGetHotKey(id, &mut modifiers, &mut vkey, &mut hkl) } != 0 {
                saved.push((id, modifiers, vkey, hkl));
                unsafe {
                    ImmSetHotKey(id, 0, 0, 0);
                }
            }
        }
        if let Ok(mut guard) = SAVED_IME_HOTKEYS.lock() {
            *guard = saved;
        }
    }

    fn restore_ime_hotkeys() {
        if let Ok(mut guard) = SAVED_IME_HOTKEYS.lock() {
            for &(id, modifiers, vkey, hkl) in guard.iter() {
                unsafe {
                    ImmSetHotKey(id, modifiers, vkey, hkl);
                }
            }
            guard.clear();
        }
    }

    pub fn start(app: AppHandle) {
        stop();
        disable_ime_hotkeys();

        if let Ok(mut guard) = HOOK_APP.lock() {
            *guard = Some(app);
        }
        HOOK_MODS.store(0, Ordering::SeqCst);

        let (tx, rx) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let thread_id = unsafe { GetCurrentThreadId() };
            let hmod = unsafe { GetModuleHandleW(std::ptr::null()) };
            let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, hook_proc, hmod, 0) };

            if hook == 0 {
                eprintln!("[keyboard_hook] SetWindowsHookExW failed");
                let _ = tx.send(false);
                return;
            }

            HOOK_HANDLE.store(hook, Ordering::SeqCst);
            HOOK_THREAD_ID.store(thread_id, Ordering::SeqCst);
            let _ = tx.send(true);

            let mut msg: MSG = unsafe { std::mem::zeroed() };
            loop {
                let ret = unsafe { GetMessageW(&mut msg, 0, 0, 0) };
                if ret <= 0 {
                    break;
                }
                if msg.message == WM_HOOK_KEY {
                    emit_from_message(msg.w_param);
                }
            }

            unsafe {
                UnhookWindowsHookEx(hook);
            }
            HOOK_HANDLE.store(0, Ordering::SeqCst);
            HOOK_THREAD_ID.store(0, Ordering::SeqCst);
        });

        match rx.recv() {
            Ok(true) => {}
            _ => eprintln!("[keyboard_hook] hook thread failed to start"),
        }

        if let Ok(mut guard) = HOOK_THREAD.lock() {
            *guard = Some(handle);
        }
    }

    pub fn stop() {
        let thread_id = HOOK_THREAD_ID.swap(0, Ordering::SeqCst);
        if thread_id != 0 {
            unsafe {
                PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
            }
        }

        if let Ok(mut guard) = HOOK_THREAD.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }

        if let Ok(mut guard) = HOOK_APP.lock() {
            *guard = None;
        }
        HOOK_MODS.store(0, Ordering::SeqCst);
        restore_ime_hotkeys();
    }
}
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

/// Stores the file path passed as a command-line argument on cold start.
/// The frontend retrieves and clears this value after initialization via
/// the `take_startup_file` command, avoiding a race condition with the
/// previous approach of emitting an event after a hardcoded delay.
static STARTUP_FILE: Mutex<Option<String>> = Mutex::new(None);

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
    Punctuation(char),
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutCheckResult {
    pub available: bool,
    pub conflict_type: String,
    pub message: String,
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
        if c.is_ascii_punctuation() {
            return Some(ShortcutKey::Punctuation(c));
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

pub async fn toggle_tile_window(
    app: AppHandle,
    note_id: String,
    bounds: Option<WindowBounds>,
) -> Result<bool, AppError> {
    toggle_tile_window_now(&app, &note_id, bounds)
}

pub fn extract_file_arg(args: &[String]) -> Option<String> {
    args.iter()
        .find(|arg| {
            let lower = arg.to_lowercase();
            lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".txt")
        })
        .cloned()
}

/// Takes the startup file path stored during cold start, consuming it so
/// subsequent calls return `None`. Called by the frontend after it finishes
/// initializing to deterministically load the file without any timing risk.
pub fn take_startup_file() -> Option<String> {
    STARTUP_FILE.lock().ok()?.take()
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
        if let Ok(mut guard) = STARTUP_FILE.lock() {
            *guard = Some(file_path);
        }
    }

    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if matches!(event, WindowEvent::Destroyed) {
        if let Some(note_id) = window.label().strip_prefix("tile-") {
            let _ = window
                .app_handle()
                .emit("tile-window-closed", note_id.to_string());
        }
        return;
    }

    if matches!(event, WindowEvent::CloseRequested { .. })
        && should_save_surface_size_before_close(window.label())
    {
        if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
            save_surface_size(&webview);
        }
    }

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

    let label = open_or_focus_window(
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
    if let Some(window) = app.get_webview_window(&label) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }
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

fn should_save_surface_size_before_close(label: &str) -> bool {
    label.starts_with("notepad-") || label.starts_with("tile-")
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

#[cfg(target_os = "windows")]
#[allow(clippy::upper_case_acronyms)]
fn cursor_centered_bounds(specs: &WindowSizeSpec) -> Option<WindowBounds> {
    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    struct MONITORINFO {
        cb_size: u32,
        rc_monitor: RECT,
        rc_work: RECT,
        dw_flags: u32,
    }
    type HMONITOR = isize;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    extern "system" {
        fn GetCursorPos(lp_point: *mut POINT) -> i32;
        fn MonitorFromPoint(pt: POINT, dw_flags: u32) -> HMONITOR;
        fn GetMonitorInfoW(h_monitor: HMONITOR, lpmi: *mut MONITORINFO) -> i32;
        fn GetDpiForSystem() -> u32;
    }
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) } == 0 {
        return None;
    }
    let scale = unsafe { GetDpiForSystem() } as f64 / 96.0;
    let w = (specs.width * scale) as i32;
    let h = (specs.height * scale) as i32;
    let mut x = pt.x - w / 2;
    let mut y = pt.y - h / 2;

    let hmon = unsafe { MonitorFromPoint(POINT { x: pt.x, y: pt.y }, MONITOR_DEFAULTTONEAREST) };
    if hmon != 0 {
        let mut mi = MONITORINFO {
            cb_size: std::mem::size_of::<MONITORINFO>() as u32,
            rc_monitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rc_work: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dw_flags: 0,
        };
        if unsafe { GetMonitorInfoW(hmon, &mut mi) } != 0 {
            let work = &mi.rc_work;
            x = x.max(work.left).min(work.right - w);
            y = y.max(work.top).min(work.bottom - h);
        }
    }

    Some(WindowBounds {
        x,
        y,
        width: w as u32,
        height: h as u32,
    })
}

#[cfg(not(target_os = "windows"))]
fn cursor_centered_bounds(_specs: &WindowSizeSpec) -> Option<WindowBounds> {
    None
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

fn toggle_tile_window_now(
    app: &AppHandle,
    note_id: &str,
    bounds: Option<WindowBounds>,
) -> Result<bool, AppError> {
    let label = tile_window_label(note_id);
    if let Some(window) = app.get_webview_window(&label) {
        window.close()?;
        return Ok(false);
    }

    open_tile_window_now(app, note_id, bounds)?;
    Ok(true)
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

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App(opts.url.into()))
        .title(opts.title)
        .inner_size(opts.specs.width, opts.specs.height)
        .min_inner_size(opts.specs.min_width, opts.specs.min_height)
        .resizable(true)
        .decorations(opts.decorations)
        .transparent(visual_options.transparent)
        .always_on_top(opts.always_on_top)
        .shadow(opts.shadow)
        .skip_taskbar(opts.skip_taskbar)
        .visible(false)
        .build()?;

    apply_window_bounds(&window, opts.bounds)?;

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
    let is_app_surface =
        label == MAIN_WINDOW_LABEL || label.starts_with("notepad-") || label.starts_with("tile-");

    DynamicWindowVisualOptions {
        transparent: is_app_surface,
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
                        let bounds = if load_config().map(|c| c.open_at_cursor).unwrap_or(true) {
                            let specs = saved_surface_specs(app);
                            cursor_centered_bounds(&specs)
                        } else {
                            None
                        };
                        if let Err(error) = app.run_on_main_thread(move || {
                            if let Err(error) =
                                open_notepad_window_now(&app_for_closure, None, bounds)
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
        let msg = format!("快捷键注册失败：{error}");
        eprintln!("{msg}");
        let _ = app.emit("shortcut-register-failed", &msg);
    }
}

pub fn check_global_shortcut(
    app: &AppHandle,
    shortcut_config: &str,
) -> Result<ShortcutCheckResult, AppError> {
    let Some(shortcut) = shortcut_from_config(shortcut_config).and_then(to_tauri_shortcut) else {
        return Ok(shortcut_check_result(
            false,
            "invalid",
            format!("快捷键 {shortcut_config} 不受支持"),
        ));
    };

    if let Some(conflict) = system_shortcut_conflict(shortcut_config) {
        return Ok(conflict);
    }

    if app.global_shortcut().is_registered(shortcut) {
        return Ok(shortcut_check_result(
            true,
            "current",
            format!("快捷键 {shortcut_config} 当前正在使用"),
        ));
    }

    match app.global_shortcut().register(shortcut) {
        Ok(()) => {
            if let Err(error) = app.global_shortcut().unregister(shortcut) {
                return Ok(shortcut_check_result(
                    false,
                    "unknown",
                    format!("检测完成，但释放临时快捷键失败：{error}"),
                ));
            }

            Ok(shortcut_check_result(
                true,
                "none",
                format!("快捷键 {shortcut_config} 未检测到冲突"),
            ))
        }
        Err(error) => Ok(shortcut_check_result(
            false,
            "registered",
            format!("快捷键 {shortcut_config} 注册失败，可能已被系统或其他应用占用：{error}"),
        )),
    }
}

fn shortcut_check_result(
    available: bool,
    conflict_type: impl Into<String>,
    message: impl Into<String>,
) -> ShortcutCheckResult {
    ShortcutCheckResult {
        available,
        conflict_type: conflict_type.into(),
        message: message.into(),
    }
}

#[cfg(target_os = "macos")]
fn system_shortcut_conflict(shortcut_config: &str) -> Option<ShortcutCheckResult> {
    let spec = shortcut_from_config(shortcut_config)?;
    let message = if shortcut_matches(&spec, false, false, false, true, ShortcutKey::Space) {
        Some("与 macOS 系统快捷键 Spotlight 冲突")
    } else if shortcut_matches(&spec, false, true, false, true, ShortcutKey::Space) {
        Some("与 macOS 系统快捷键 Finder 搜索窗口冲突")
    } else if shortcut_matches(&spec, true, false, false, false, ShortcutKey::Space)
        || shortcut_matches(&spec, true, true, false, false, ShortcutKey::Space)
    {
        Some("与 macOS 输入法切换快捷键冲突")
    } else if shortcut_matches(&spec, false, true, false, false, ShortcutKey::Space) {
        Some("Option+Space 容易与输入法或系统服务快捷键冲突")
    } else {
        None
    }?;

    Some(shortcut_check_result(false, "system", message))
}

#[cfg(not(target_os = "macos"))]
fn system_shortcut_conflict(_shortcut_config: &str) -> Option<ShortcutCheckResult> {
    None
}

#[cfg(target_os = "macos")]
fn shortcut_matches(
    spec: &ShortcutSpec,
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
    key: ShortcutKey,
) -> bool {
    spec.ctrl == ctrl
        && spec.alt == alt
        && spec.shift == shift
        && spec.meta == meta
        && spec.key == key
}

#[cfg(not(desktop))]
fn register_configured_global_shortcut(_app: &AppHandle) {}

#[cfg(desktop)]
fn parse_configured_shortcut(field: &str, value: &str) -> Result<Shortcut, Box<dyn Error>> {
    if let Some(conflict) = system_shortcut_conflict(value) {
        return Err(Box::new(AppError {
            code: "shortcutConflict".into(),
            message: conflict.message,
            details: [
                ("field".to_string(), field.to_string()),
                ("shortcut".to_string(), value.to_string()),
            ]
            .into_iter()
            .collect(),
        }));
    }

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
        ShortcutKey::Punctuation(c) => match c {
            '[' => Code::BracketLeft,
            ']' => Code::BracketRight,
            ';' => Code::Semicolon,
            '\'' => Code::Quote,
            '`' => Code::Backquote,
            ',' => Code::Comma,
            '.' => Code::Period,
            '/' => Code::Slash,
            '\\' => Code::Backslash,
            '-' => Code::Minus,
            '=' => Code::Equal,
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

#[cfg(desktop)]
pub fn start_shortcut_recording(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    app.global_shortcut().unregister_all()?;

    #[cfg(target_os = "windows")]
    keyboard_hook::start(app.clone());

    Ok(())
}

#[cfg(desktop)]
pub fn stop_shortcut_recording(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    keyboard_hook::stop();

    let config = load_config()?;
    if let Err(e) = install_global_shortcut_bindings(app, &config, false) {
        eprintln!("failed to re-register global shortcuts after recording: {e}");
    }

    Ok(())
}

#[cfg(not(desktop))]
pub fn start_shortcut_recording(_app: &AppHandle) -> Result<(), Box<dyn Error>> {
    Ok(())
}

#[cfg(not(desktop))]
pub fn stop_shortcut_recording(_app: &AppHandle) -> Result<(), Box<dyn Error>> {
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
            global_shortcut: "Ctrl+Shift+K".into(),
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
            tab_indent_size: 2,
            external_file_auto_save: true,
            background_image_path: String::new(),
            background_fit: "cover".into(),
            background_dim: 0.25,
            background_blur: 0.0,
            background_scale: 1.0,
            background_position_x: 50.0,
            background_position_y: 50.0,
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            render_html_markdown: false,
            open_at_cursor: true,
            surface_width: None,
            surface_height: None,
            toggle_visibility_shortcut: "Ctrl+Shift+K".into(),
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
            tab_indent_size: 2,
            external_file_auto_save: true,
            background_image_path: String::new(),
            background_fit: "cover".into(),
            background_dim: 0.25,
            background_blur: 0.0,
            background_scale: 1.0,
            background_position_x: 50.0,
            background_position_y: 50.0,
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            render_html_markdown: false,
            open_at_cursor: true,
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
            tab_indent_size: 4,
            external_file_auto_save: true,
            background_image_path: String::new(),
            background_fit: "cover".into(),
            background_dim: 0.25,
            background_blur: 0.0,
            background_scale: 1.0,
            background_position_x: 50.0,
            background_position_y: 50.0,
            remember_surface_size: true,
            tile_ctrl_close: true,
            tile_render_markdown: false,
            render_html_markdown: false,
            open_at_cursor: true,
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
            DynamicWindowVisualOptions { transparent: true }
        );
    }

    #[test]
    fn saves_surface_size_before_notepad_and_tile_windows_close() {
        assert!(should_save_surface_size_before_close("notepad-note-1"));
        assert!(should_save_surface_size_before_close("tile-note-1"));
        assert!(!should_save_surface_size_before_close(MAIN_WINDOW_LABEL));
        assert!(!should_save_surface_size_before_close("settings"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_known_macos_system_shortcut_conflicts() {
        let conflict = system_shortcut_conflict("Command+Space").expect("conflict");

        assert_eq!(conflict.conflict_type, "system");
        assert!(conflict.message.contains("Spotlight"));
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
