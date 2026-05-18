use super::{text, NoteStore};
use crate::notes::{error::AppError, model::NoteMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct MetadataFile {
    pub(super) notes: Vec<NoteMetadata>,
}

impl NoteStore {
    pub(super) fn load_metadata(&self) -> Result<MetadataFile, AppError> {
        self.ensure_base_dir()?;
        let path = self.metadata_path();
        if !path.exists() {
            let rebuilt = self.rebuild_metadata()?;
            self.save_metadata(&rebuilt)?;
            return Ok(rebuilt);
        }

        match serde_json::from_str(&fs::read_to_string(&path)?) {
            Ok(metadata) => Ok(metadata),
            Err(_error) => {
                let corrupt_name = format!(
                    "metadata.corrupt-{}.json",
                    Utc::now().format("%Y%m%d%H%M%S")
                );
                fs::rename(&path, self.base_dir.join(corrupt_name))?;
                let rebuilt = self.rebuild_metadata()?;
                self.save_metadata(&rebuilt)?;
                Ok(rebuilt)
            }
        }
    }

    pub(super) fn save_metadata(&self, metadata: &MetadataFile) -> Result<(), AppError> {
        self.ensure_base_dir()?;
        write_json_atomic(&self.metadata_path(), metadata)
    }

    fn rebuild_metadata(&self) -> Result<MetadataFile, AppError> {
        let notes_dir = self.notes_dir()?;
        fs::create_dir_all(&notes_dir)?;
        let mut notes = Vec::new();

        self.scan_dir_for_notes(&notes_dir, "", &mut notes)?;

        for entry in fs::read_dir(&notes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let category = entry.file_name().to_string_lossy().to_string();
                self.scan_dir_for_notes(&path, &category, &mut notes)?;
            }
        }

        Ok(MetadataFile { notes })
    }

    fn scan_dir_for_notes(
        &self,
        dir: &Path,
        category: &str,
        notes: &mut Vec<NoteMetadata>,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let Some(id) = text::id_from_file_name(&file_name) else {
                continue;
            };
            let content = fs::read_to_string(&path).unwrap_or_default();
            let title = text::infer_title(&file_name, &content);
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());

            notes.push(NoteMetadata {
                id,
                title,
                file_name,
                category: category.to_string(),
                created_at: modified,
                updated_at: modified,
                word_count: text::count_words(&content),
                preview: text::preview(&content),
            });
        }
        Ok(())
    }
}

pub(super) fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = temp_path_for(path);
    fs::write(&temp_path, serde_json::to_string_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)?;
    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    path.with_extension("json.tmp")
}
