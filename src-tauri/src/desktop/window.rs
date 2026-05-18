use crate::notes::AppError;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    Window, WindowEvent,
};
use uuid::Uuid;

use super::{
    app_is_exiting, close_to_tray_enabled, mark_app_exiting, NotepadPool, WindowBounds,
    MAIN_WINDOW_LABEL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicWindowVisualOptions {
    pub transparent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainWindowCloseAction {
    AllowClose,
    HideToTray,
    ExitApp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WindowSizeSpec {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) min_width: f64,
    pub(crate) min_height: f64,
}

pub(crate) struct WindowOpenSpec<'a> {
    label: &'a str,
    url: String,
    title: &'a str,
    size: WindowSizeSpec,
    decorations: bool,
    always_on_top: bool,
    shadow: bool,
    skip_taskbar: bool,
    bounds: Option<WindowBounds>,
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
            lower.ends_with(".md") || lower.ends_with(".markdown")
        })
        .cloned()
}

pub(crate) fn handle_window_event(window: &Window, event: &WindowEvent) {
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

pub(crate) fn main_window_close_action(
    app_is_exiting: bool,
    close_to_tray: bool,
) -> MainWindowCloseAction {
    if app_is_exiting {
        MainWindowCloseAction::AllowClose
    } else if close_to_tray {
        MainWindowCloseAction::HideToTray
    } else {
        MainWindowCloseAction::ExitApp
    }
}

pub fn show_main_window(app: &AppHandle) -> Result<(), AppError> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    open_or_focus_window(
        app,
        WindowOpenSpec {
            label: MAIN_WINDOW_LABEL,
            url: "index.html".to_string(),
            title: "花笺",
            size: WindowSizeSpec {
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

pub(crate) fn open_notepad_window_now(
    app: &AppHandle,
    note_id: Option<&str>,
    bounds: Option<WindowBounds>,
) -> Result<String, AppError> {
    if note_id.is_none() {
        if let Some(reused) = activate_pooled_notepad(app, bounds) {
            return Ok(reused);
        }
    }

    let label = notepad_window_label(note_id);
    let specs = notepad_window_specs();
    let url = match note_id {
        Some(id) => format!("index.html?view=notepad&noteId={id}"),
        None => "index.html?view=notepad".to_string(),
    };

    open_or_focus_window(
        app,
        WindowOpenSpec {
            label: &label,
            url,
            title: "花笺便签",
            size: specs,
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

    let specs = notepad_window_specs();
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

pub(crate) fn schedule_notepad_prewarm(app: &AppHandle) {
    for index in 0..super::NOTEPAD_POOL_CAPACITY {
        let delay = 800 + index as u64 * 400;
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
    })?;

    if !pool.is_below_capacity() {
        return Ok(());
    }

    let label = notepad_window_label(None);
    let specs = notepad_window_specs();
    let builder = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App("index.html?view=notepad&standby=1".into()),
    )
    .title("花笺便签")
    .inner_size(specs.width, specs.height)
    .min_inner_size(specs.min_width, specs.min_height)
    .resizable(true)
    .decorations(false)
    .transparent(dynamic_window_visual_options(&label).transparent);
    let builder = builder
        .always_on_top(true)
        .shadow(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false);
    builder.build()?;

    pool.put(label);

    Ok(())
}

pub(crate) fn notepad_window_specs() -> WindowSizeSpec {
    WindowSizeSpec {
        width: 260.0,
        height: 260.0,
        min_width: 220.0,
        min_height: 220.0,
    }
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
        WindowOpenSpec {
            label: &label,
            url,
            title: "花笺磁贴",
            size: notepad_window_specs(),
            decorations: false,
            always_on_top: true,
            shadow: false,
            skip_taskbar: true,
            bounds,
        },
    )
}

fn open_or_focus_window(app: &AppHandle, spec: WindowOpenSpec<'_>) -> Result<String, AppError> {
    let WindowOpenSpec {
        label,
        url,
        title,
        size,
        decorations,
        always_on_top,
        shadow,
        skip_taskbar,
        bounds,
    } = spec;

    if let Some(window) = app.get_webview_window(label) {
        apply_window_bounds(&window, bounds)?;
        window.set_shadow(shadow)?;
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(label.to_string());
    }

    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(size.width, size.height)
        .min_inner_size(size.min_width, size.min_height)
        .resizable(true)
        .decorations(decorations)
        .transparent(dynamic_window_visual_options(label).transparent);
    let mut builder = builder
        .always_on_top(always_on_top)
        .shadow(shadow)
        .skip_taskbar(skip_taskbar)
        .visible(false);

    if let Some(bounds) = bounds {
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

pub(crate) fn notepad_window_label(note_id: Option<&str>) -> String {
    match note_id {
        Some(id) => format!("notepad-{}", sanitize_label_part(id)),
        None => format!("notepad-{}", Uuid::new_v4()),
    }
}

pub(crate) fn tile_window_label(note_id: &str) -> String {
    format!("tile-{}", sanitize_label_part(note_id))
}

pub(crate) fn dynamic_window_visual_options(label: &str) -> DynamicWindowVisualOptions {
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
