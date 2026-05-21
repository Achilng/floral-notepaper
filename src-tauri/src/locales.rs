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
}
