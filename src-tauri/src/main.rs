// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

    floral_notepaper_lib::run()
}
