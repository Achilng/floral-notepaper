use crate::{
    desktop::load_config,
    notes::{AppConfig, AppError},
};
use std::error::Error;
use tauri::AppHandle;

#[cfg(desktop)]
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartExt};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use super::window::open_notepad_window_now;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: ShortcutKey,
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

    for modifier in modifier_parts {
        match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => ctrl = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            _ => return None,
        }
    }

    if !ctrl && !alt {
        return None;
    }

    let key = parse_shortcut_key(key_part[0])?;

    Some(ShortcutSpec {
        ctrl,
        alt,
        shift,
        key,
    })
}

fn parse_shortcut_key(key: &str) -> Option<ShortcutKey> {
    if key.len() == 1 {
        let character = key.chars().next()?;
        if character.is_ascii_alphabetic() {
            return Some(ShortcutKey::Letter(character.to_ascii_uppercase()));
        }
        if character.is_ascii_digit() {
            return Some(ShortcutKey::Digit(character.to_digit(10)? as u8));
        }
    }

    if let Some(rest) = key.strip_prefix('F').or_else(|| key.strip_prefix('f')) {
        if let Ok(number) = rest.parse::<u8>() {
            if (1..=12).contains(&number) {
                return Some(ShortcutKey::Function(number));
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

#[cfg(desktop)]
pub(crate) fn setup_autostart_plugin(app: &AppHandle) -> tauri::Result<()> {
    app.plugin(tauri_plugin_autostart::init(
        MacosLauncher::LaunchAgent,
        Some(vec!["--silent"]),
    ))
}

#[cfg(not(desktop))]
pub(crate) fn setup_autostart_plugin(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(desktop)]
pub(crate) fn setup_global_shortcut_plugin(app: &AppHandle) -> tauri::Result<()> {
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
pub(crate) fn setup_global_shortcut_plugin(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(desktop)]
pub(crate) fn register_configured_global_shortcut(app: &AppHandle) {
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
pub(crate) fn register_configured_global_shortcut(_app: &AppHandle) {}

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

    let code = shortcut_key_to_code(spec.key)?;
    let modifiers = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };

    Some(Shortcut::new(modifiers, code))
}

#[cfg(desktop)]
fn shortcut_key_to_code(key: ShortcutKey) -> Option<Code> {
    Some(match key {
        ShortcutKey::Letter(character) => match character {
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
        ShortcutKey::Digit(digit) => match digit {
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
        ShortcutKey::Function(number) => match number {
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
pub(crate) fn sync_autostart_to_config(app: &AppHandle) {
    let Ok(config) = load_config() else {
        return;
    };

    if let Err(error) = apply_autostart(app, config.autostart) {
        eprintln!("failed to sync autostart config: {error}");
    }
}

#[cfg(not(desktop))]
pub(crate) fn sync_autostart_to_config(_app: &AppHandle) {}

#[cfg(desktop)]
pub(crate) fn autostart_enabled(app: &AppHandle, fallback: bool) -> bool {
    app.autolaunch().is_enabled().unwrap_or(fallback)
}

#[cfg(not(desktop))]
pub(crate) fn autostart_enabled(_app: &AppHandle, fallback: bool) -> bool {
    fallback
}

pub(crate) fn toggle_autostart(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let store = crate::notes::default_store()?;
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
