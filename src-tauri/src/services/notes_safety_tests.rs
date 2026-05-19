use super::notes::{NoteStore, SaveNoteRequest};
use std::{fs, path::PathBuf, thread};

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
fn restores_metadata_from_backup_when_current_json_is_missing() {
    let store = NoteStore::new(test_root("metadata-backup-repair"));
    let created = store
        .create_note(SaveNoteRequest {
            title: "第一条".into(),
            content: "第一条正文".into(),
            category: String::new(),
        })
        .expect("create note");

    fs::copy(
        store.metadata_path(),
        store.base_dir().join("metadata.json.bak"),
    )
    .expect("create metadata backup");
    fs::remove_file(store.metadata_path()).expect("remove current metadata");

    let repaired = store.list_notes().expect("recover missing metadata");
    let ids: Vec<_> = repaired.iter().map(|note| note.id.as_str()).collect();

    assert_eq!(repaired.len(), 1);
    assert!(ids.contains(&created.id.as_str()));
    assert!(store.metadata_path().exists());
}

#[test]
fn serializes_concurrent_metadata_writes() {
    let store = NoteStore::new(test_root("concurrent-create"));

    let handles: Vec<_> = (0..16)
        .map(|index| {
            let store = store.clone();
            thread::spawn(move || {
                store
                    .create_note(SaveNoteRequest {
                        title: format!("并发笔记 {index}"),
                        content: format!("第 {index} 条正文"),
                        category: String::new(),
                    })
                    .expect("create note concurrently");
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("join create thread");
    }

    let listed = store.list_notes().expect("list concurrent notes");
    assert_eq!(listed.len(), 16);
    assert!(!fs::read_dir(store.base_dir())
        .expect("read store dir")
        .any(|entry| entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
}
