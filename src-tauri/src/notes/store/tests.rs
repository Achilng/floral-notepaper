use super::*;
use std::{fs, path::PathBuf};

fn test_root(name: &str) -> PathBuf {
    let base = std::env::var_os("FLORAL_NOTEPAPER_TEST_TEMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("floral-notepaper-rust-tests"));
    let root = base.join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale test root");
    }
    fs::create_dir_all(&root).expect("create test root");
    root
}

#[test]
fn creates_updates_reads_and_deletes_markdown_notes() {
    let store = NoteStore::new(test_root("crud"));

    let created = store
        .create_note(SaveNoteRequest {
            title: "A/B:Test".into(),
            content: "hello\nworld".into(),
            category: String::new(),
        })
        .expect("create note");

    assert_eq!(created.title, "A/B:Test");
    assert_eq!(created.content, "hello\nworld");
    assert_eq!(created.word_count, 10);
    assert!(created.file_name.ends_with(".md"));
    assert!(created.file_name.contains("A_B_Test"));

    let loaded = store.read_note(&created.id).expect("read note");
    assert_eq!(loaded, created);

    let listed = store.list_notes().expect("list notes");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].preview, "hello world");

    let updated = store
        .update_note(
            &created.id,
            SaveNoteRequest {
                title: "".into(),
                content: "# 新标题\nsecond line".into(),
                category: String::new(),
            },
        )
        .expect("update note");

    assert_eq!(updated.title, "");
    assert_eq!(updated.content, "# 新标题\nsecond line");
    assert_ne!(updated.file_name, created.file_name);

    store.delete_note(&created.id).expect("delete note");
    assert!(store.read_note(&created.id).is_err());
    assert!(store.list_notes().expect("list after delete").is_empty());
}

#[test]
fn rebuilds_metadata_when_metadata_json_is_corrupt() {
    let store = NoteStore::new(test_root("repair"));
    let first = store
        .create_note(SaveNoteRequest {
            title: "第一条".into(),
            content: "# 第一条\n正文".into(),
            category: String::new(),
        })
        .expect("create first");
    let second = store
        .create_note(SaveNoteRequest {
            title: "第二条".into(),
            content: "第二条正文".into(),
            category: String::new(),
        })
        .expect("create second");

    fs::write(store.metadata_path(), "{ broken json").expect("corrupt metadata");

    let repaired = store.list_notes().expect("repair metadata");
    let ids: Vec<_> = repaired.iter().map(|note| note.id.as_str()).collect();

    assert_eq!(repaired.len(), 2);
    assert!(ids.contains(&first.id.as_str()));
    assert!(ids.contains(&second.id.as_str()));
    assert!(store
        .base_dir()
        .read_dir()
        .expect("read base dir")
        .any(|entry| entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .starts_with("metadata.corrupt-")));
}

#[test]
fn reads_and_writes_config_json() {
    let store = NoteStore::new(test_root("config"));

    let default_config = store.load_config().expect("load default config");
    assert_eq!(default_config.global_shortcut, "Ctrl+Space");
    assert!(default_config.note_auto_save);
    assert!(default_config.note_surface_auto_save);
    assert_eq!(default_config.tile_color, "#f6f3ec");
    assert_eq!(default_config.tile_color_mode, "system");
    assert_eq!(default_config.theme, "system");
    assert!(PathBuf::from(&default_config.notes_dir).ends_with("notes"));

    let custom_notes_dir = store.base_dir().join("custom-notes");
    let saved = AppConfig {
        notes_dir: custom_notes_dir.to_string_lossy().to_string(),
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
    };

    store.save_config(saved.clone()).expect("save config");

    let loaded = store.load_config().expect("reload config");
    assert_eq!(loaded, saved);
    assert!(custom_notes_dir.exists());
}

#[test]
fn loads_legacy_config_with_note_surface_auto_save_enabled() {
    let store = NoteStore::new(test_root("legacy-config"));
    let notes_dir = store.base_dir().join("notes");
    fs::create_dir_all(&notes_dir).expect("create notes dir");
    fs::write(
        store.config_path(),
        format!(
            r#"{{
  "notesDir": "{}",
  "globalShortcut": "Ctrl+Space",
  "closeToTray": true,
  "autostart": false,
  "defaultViewMode": "split"
}}"#,
            notes_dir.to_string_lossy().replace('\\', "\\\\")
        ),
    )
    .expect("write legacy config");

    let loaded = store.load_config().expect("load legacy config");

    assert!(loaded.note_auto_save);
    assert!(loaded.note_surface_auto_save);
    assert_eq!(loaded.tile_color, "#f6f3ec");
    assert_eq!(loaded.tile_color_mode, "system");
    assert_eq!(loaded.theme, "system");
    assert_eq!(loaded.font_size, 14);
    assert_eq!(loaded.surface_font_size, 14);
}

#[test]
fn imports_markdown_heading_title_without_stripping_content() {
    let root = test_root("import-heading-title");
    let source_path = root.join("外部文件.md");
    let source_content = "# 导入标题\n正文第一行\n正文第二行";
    fs::write(&source_path, source_content).expect("write source markdown");
    let store = NoteStore::new(root.join("store"));

    let imported = store
        .import_markdown_file(&source_path, "")
        .expect("import markdown");

    assert_eq!(imported.title, "导入标题");
    assert_eq!(imported.content, source_content);
    assert_eq!(
        store
            .read_note(&imported.id)
            .expect("read imported")
            .content,
        source_content
    );
}

#[test]
fn imports_markdown_title_from_file_name_without_heading() {
    let root = test_root("import-file-title");
    let source_path = root.join("会议记录.md");
    let source_content = "正文第一行\n# 不是第一行标题";
    fs::write(&source_path, source_content).expect("write source markdown");
    let store = NoteStore::new(root.join("store"));

    let imported = store
        .import_markdown_file(&source_path, "")
        .expect("import markdown");

    assert_eq!(imported.title, "会议记录");
    assert_eq!(imported.content, source_content);
}

#[test]
fn exports_markdown_file_without_rewriting_content() {
    let root = test_root("export-markdown");
    let store = NoteStore::new(root.join("store"));
    let content = "# 原始标题\n正文\n- 列表";
    let note = store
        .create_note(SaveNoteRequest {
            title: "导出标题".into(),
            content: content.into(),
            category: String::new(),
        })
        .expect("create note");
    let export_path = root.join("exports").join("导出.md");

    store
        .export_markdown_file(&note.id, &export_path)
        .expect("export markdown");

    assert_eq!(
        fs::read_to_string(export_path).expect("read exported markdown"),
        content
    );
}
