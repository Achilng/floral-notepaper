#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    ZhCn,
    EnUs,
    ZhHk,
}

impl Locale {
    pub fn from_tag(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en-us" | "en" => Self::EnUs,
            "zh-hk" | "zh-tw" | "zh-hant" => Self::ZhHk,
            _ => Self::ZhCn,
        }
    }
}

pub fn app_name(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "花笺",
        Locale::EnUs => "Floral Notepaper",
        Locale::ZhHk => "花箋",
    }
}

pub fn main_window_title(locale: Locale) -> &'static str {
    app_name(locale)
}

pub fn notepad_window_title(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "花笺便签",
        Locale::EnUs => "Floral Notepaper Quick Note",
        Locale::ZhHk => "花箋便箋",
    }
}

pub fn tile_window_title(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "花笺磁贴",
        Locale::EnUs => "Floral Notepaper Pin Mode",
        Locale::ZhHk => "花箋磁貼",
    }
}

pub fn tray_tooltip(locale: Locale) -> &'static str {
    app_name(locale)
}

pub fn tray_show_main_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "打开主窗口",
        Locale::EnUs => "Open Main Window",
        Locale::ZhHk => "打開主視窗",
    }
}

pub fn tray_quick_note_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "快速记录",
        Locale::EnUs => "Quick Note",
        Locale::ZhHk => "快速便箋",
    }
}

pub fn tray_toggle_close_to_tray_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "关闭到托盘",
        Locale::EnUs => "Close to Tray",
        Locale::ZhHk => "關閉到系統匣",
    }
}

pub fn tray_toggle_autostart_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "开机自启动",
        Locale::EnUs => "Launch on Startup",
        Locale::ZhHk => "開機自啟",
    }
}

pub fn tray_quit_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "退出",
        Locale::EnUs => "Quit",
        Locale::ZhHk => "退出",
    }
}

pub fn macos_menu_file_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "文件",
        Locale::EnUs => "File",
        Locale::ZhHk => "檔案",
    }
}

pub fn macos_menu_edit_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "编辑",
        Locale::EnUs => "Edit",
        Locale::ZhHk => "編輯",
    }
}

pub fn macos_menu_view_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "显示",
        Locale::EnUs => "View",
        Locale::ZhHk => "顯示",
    }
}

pub fn macos_menu_window_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "窗口",
        Locale::EnUs => "Window",
        Locale::ZhHk => "視窗",
    }
}

pub fn macos_menu_help_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "帮助",
        Locale::EnUs => "Help",
        Locale::ZhHk => "幫助",
    }
}

pub fn macos_menu_about_label(locale: Locale) -> String {
    match locale {
        Locale::ZhCn => format!("关于{}", app_name(locale)),
        Locale::EnUs => format!("About {}", app_name(locale)),
        Locale::ZhHk => format!("關於{}", app_name(locale)),
    }
}

pub fn macos_menu_services_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "服务",
        Locale::EnUs => "Services",
        Locale::ZhHk => "服務",
    }
}

pub fn macos_menu_hide_app_label(locale: Locale) -> String {
    match locale {
        Locale::ZhCn => format!("隐藏{}", app_name(locale)),
        Locale::EnUs => format!("Hide {}", app_name(locale)),
        Locale::ZhHk => format!("隱藏{}", app_name(locale)),
    }
}

pub fn macos_menu_hide_others_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "隐藏其他",
        Locale::EnUs => "Hide Others",
        Locale::ZhHk => "隱藏其他",
    }
}

pub fn macos_menu_quit_app_label(locale: Locale) -> String {
    match locale {
        Locale::ZhCn => format!("退出{}", app_name(locale)),
        Locale::EnUs => format!("Quit {}", app_name(locale)),
        Locale::ZhHk => format!("退出{}", app_name(locale)),
    }
}

pub fn macos_menu_close_window_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "关闭窗口",
        Locale::EnUs => "Close Window",
        Locale::ZhHk => "關閉視窗",
    }
}

pub fn macos_menu_minimize_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "最小化",
        Locale::EnUs => "Minimize",
        Locale::ZhHk => "最小化",
    }
}

pub fn macos_menu_zoom_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "缩放",
        Locale::EnUs => "Zoom",
        Locale::ZhHk => "縮放",
    }
}

pub fn macos_menu_fullscreen_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "进入全屏",
        Locale::EnUs => "Enter Full Screen",
        Locale::ZhHk => "進入全螢幕",
    }
}

pub fn macos_menu_undo_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "撤销",
        Locale::EnUs => "Undo",
        Locale::ZhHk => "復原",
    }
}

pub fn macos_menu_redo_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "重做",
        Locale::EnUs => "Redo",
        Locale::ZhHk => "重做",
    }
}

pub fn macos_menu_cut_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "剪切",
        Locale::EnUs => "Cut",
        Locale::ZhHk => "剪下",
    }
}

pub fn macos_menu_copy_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "复制",
        Locale::EnUs => "Copy",
        Locale::ZhHk => "複製",
    }
}

pub fn macos_menu_paste_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "粘贴",
        Locale::EnUs => "Paste",
        Locale::ZhHk => "貼上",
    }
}

pub fn macos_menu_select_all_label(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "全选",
        Locale::EnUs => "Select All",
        Locale::ZhHk => "全選",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_locales_and_falls_back_to_source_locale() {
        assert_eq!(Locale::from_tag("zh-CN"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("en-US"), Locale::EnUs);
        assert_eq!(Locale::from_tag("zh-HK"), Locale::ZhHk);
        assert_eq!(Locale::from_tag("zh-TW"), Locale::ZhHk);
        assert_eq!(Locale::from_tag("fr-FR"), Locale::ZhCn);
    }

    #[test]
    fn localizes_native_shell_strings_for_supported_locales() {
        assert_eq!(app_name(Locale::ZhCn), "花笺");
        assert_eq!(app_name(Locale::EnUs), "Floral Notepaper");
        assert_eq!(app_name(Locale::ZhHk), "花箋");

        assert_eq!(
            notepad_window_title(Locale::EnUs),
            "Floral Notepaper Quick Note"
        );
        assert_eq!(tile_window_title(Locale::ZhHk), "花箋磁貼");
        assert_eq!(tray_tooltip(Locale::EnUs), "Floral Notepaper");
        assert_eq!(tray_show_main_label(Locale::EnUs), "Open Main Window");
        assert_eq!(tray_quick_note_label(Locale::ZhHk), "快速便箋");
        assert_eq!(
            tray_toggle_close_to_tray_label(Locale::EnUs),
            "Close to Tray"
        );
        assert_eq!(tray_toggle_autostart_label(Locale::ZhHk), "開機自啟");
        assert_eq!(tray_quit_label(Locale::EnUs), "Quit");
    }

    #[test]
    fn localizes_macos_native_menu_strings_for_supported_locales() {
        assert_eq!(macos_menu_file_label(Locale::ZhCn), "文件");
        assert_eq!(macos_menu_edit_label(Locale::ZhHk), "編輯");
        assert_eq!(macos_menu_view_label(Locale::EnUs), "View");
        assert_eq!(macos_menu_window_label(Locale::ZhHk), "視窗");
        assert_eq!(macos_menu_help_label(Locale::ZhCn), "帮助");
        assert_eq!(macos_menu_about_label(Locale::ZhCn), "关于花笺");
        assert_eq!(
            macos_menu_about_label(Locale::EnUs),
            "About Floral Notepaper"
        );
        assert_eq!(macos_menu_services_label(Locale::ZhHk), "服務");
        assert_eq!(macos_menu_hide_app_label(Locale::ZhCn), "隐藏花笺");
        assert_eq!(macos_menu_hide_others_label(Locale::EnUs), "Hide Others");
        assert_eq!(
            macos_menu_quit_app_label(Locale::EnUs),
            "Quit Floral Notepaper"
        );
        assert_eq!(macos_menu_close_window_label(Locale::ZhHk), "關閉視窗");
        assert_eq!(macos_menu_minimize_label(Locale::EnUs), "Minimize");
        assert_eq!(macos_menu_zoom_label(Locale::ZhCn), "缩放");
        assert_eq!(macos_menu_fullscreen_label(Locale::ZhHk), "進入全螢幕");
        assert_eq!(macos_menu_undo_label(Locale::ZhHk), "復原");
        assert_eq!(macos_menu_redo_label(Locale::ZhCn), "重做");
        assert_eq!(macos_menu_cut_label(Locale::ZhHk), "剪下");
        assert_eq!(macos_menu_copy_label(Locale::ZhCn), "复制");
        assert_eq!(macos_menu_paste_label(Locale::EnUs), "Paste");
        assert_eq!(macos_menu_select_all_label(Locale::ZhHk), "全選");
    }
}
