use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    ZhCN,
    En,
    ZhTW,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code {
            "en" => Lang::En,
            "zh-TW" => Lang::ZhTW,
            _ => Lang::ZhCN,
        }
    }
}

type StrMap = HashMap<&'static str, &'static str>;

fn tray_translations() -> HashMap<&'static str, StrMap> {
    let mut map = HashMap::new();

    let mut zh_cn = StrMap::new();
    zh_cn.insert("showMain", "打开主窗口");
    zh_cn.insert("quickNote", "快速记录");
    zh_cn.insert("closeToTray", "关闭到托盘");
    zh_cn.insert("autostart", "开机自启动");
    zh_cn.insert("quit", "退出");
    map.insert("zh-CN", zh_cn);

    let mut en = StrMap::new();
    en.insert("showMain", "Open Main Window");
    en.insert("quickNote", "Quick Note");
    en.insert("closeToTray", "Close to Tray");
    en.insert("autostart", "Auto Start");
    en.insert("quit", "Quit");
    map.insert("en", en);

    let mut zh_tw = StrMap::new();
    zh_tw.insert("showMain", "開啟主視窗");
    zh_tw.insert("quickNote", "快速記錄");
    zh_tw.insert("closeToTray", "關閉到系統匣");
    zh_tw.insert("autostart", "開機自啟動");
    zh_tw.insert("quit", "結束");
    map.insert("zh-TW", zh_tw);

    map
}

fn window_translations() -> HashMap<&'static str, StrMap> {
    let mut map = HashMap::new();

    let mut zh_cn = StrMap::new();
    zh_cn.insert("app", "花笺");
    zh_cn.insert("notepad", "花笺便签");
    zh_cn.insert("tile", "花笺磁贴");
    zh_cn.insert("tooltip", "花笺");
    map.insert("zh-CN", zh_cn);

    let mut en = StrMap::new();
    en.insert("app", "Floral Notepaper");
    en.insert("notepad", "Floral Notepad");
    en.insert("tile", "Floral Tile");
    en.insert("tooltip", "Floral Notepaper");
    map.insert("en", en);

    let mut zh_tw = StrMap::new();
    zh_tw.insert("app", "花箋");
    zh_tw.insert("notepad", "花箋便箋");
    zh_tw.insert("tile", "花箋磁貼");
    zh_tw.insert("tooltip", "花箋");
    map.insert("zh-TW", zh_tw);

    map
}

pub fn tray_label(lang_code: &str, key: &str) -> String {
    let translations = tray_translations();
    translations
        .get(lang_code)
        .and_then(|map| map.get(key))
        .or_else(|| translations.get("zh-CN").and_then(|map| map.get(key)))
        .unwrap_or(&key)
        .to_string()
}

pub fn window_title(lang_code: &str, key: &str) -> String {
    let translations = window_translations();
    translations
        .get(lang_code)
        .and_then(|map| map.get(key))
        .or_else(|| translations.get("zh-CN").and_then(|map| map.get(key)))
        .unwrap_or(&key)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_tray_labels_for_all_languages() {
        assert_eq!(tray_label("zh-CN", "showMain"), "打开主窗口");
        assert_eq!(tray_label("en", "showMain"), "Open Main Window");
        assert_eq!(tray_label("zh-TW", "showMain"), "開啟主視窗");
    }

    #[test]
    fn falls_back_to_zh_cn_for_unknown_language() {
        assert_eq!(tray_label("fr", "showMain"), "打开主窗口");
    }

    #[test]
    fn resolves_window_titles_for_all_languages() {
        assert_eq!(window_title("zh-CN", "app"), "花笺");
        assert_eq!(window_title("en", "app"), "Floral Notepaper");
        assert_eq!(window_title("zh-TW", "app"), "花箋");
        assert_eq!(window_title("zh-CN", "notepad"), "花笺便签");
        assert_eq!(window_title("en", "notepad"), "Floral Notepad");
    }

    #[test]
    fn parses_language_codes() {
        assert_eq!(Lang::from_code("zh-CN"), Lang::ZhCN);
        assert_eq!(Lang::from_code("en"), Lang::En);
        assert_eq!(Lang::from_code("zh-TW"), Lang::ZhTW);
        assert_eq!(Lang::from_code("unknown"), Lang::ZhCN);
    }
}
