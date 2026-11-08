// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn configure_linux_runtime_env() {
    if std::env::var_os("GDK_BACKEND").is_none()
        && std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty())
    {
        std::env::set_var("GDK_BACKEND", "x11,wayland");
    }
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    if std::env::var_os("NO_AT_BRIDGE").is_none() {
        std::env::set_var("NO_AT_BRIDGE", "1");
    }
    if std::env::var_os("AT_SPI_BUS_ADDRESS").is_none() {
        std::env::set_var("AT_SPI_BUS_ADDRESS", "disabled");
    }
    if std::env::var_os("WEBKIT_A11Y_BUS_ADDRESS").is_none() {
        std::env::set_var("WEBKIT_A11Y_BUS_ADDRESS", "");
    }
}

fn main() {
    let mut args = std::env::args_os();
    let _ = args.next();
    if matches!(
        args.next().as_deref(),
        Some(flag) if flag == std::ffi::OsStr::new("--update-helper")
    ) {
        let exit_code = floral_notepaper_lib::updater::helper::run_cli(args);
        std::process::exit(exit_code.as_i32());
    }

    floral_notepaper_lib::try_exit_for_cli_version_or_help();
    #[cfg(target_os = "linux")]
    configure_linux_runtime_env();
    floral_notepaper_lib::run()
}
