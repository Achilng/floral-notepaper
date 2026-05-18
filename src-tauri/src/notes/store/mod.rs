mod metadata;
mod text;

#[cfg(test)]
mod tests;

use self::text::{count_words, imported_markdown_title, is_markdown_path, preview, safe_file_stem};
use crate::notes::{
    error::AppError,
    model::{
        default_external_file_auto_save, default_font_size, default_note_auto_save,
        default_note_surface_auto_save, default_surface_font_size, default_theme,
        default_tile_color, default_tile_color_mode, AppConfig, Note, NoteMetadata,
        SaveNoteRequest,
    },
};
use chrono::Utc;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NoteStore {
    base_dir: PathBuf,
}

pub fn default_store() -> Result<NoteStore, AppError> {
    Ok(NoteStore::new(default_base_dir()?))
}

fn default_base_dir() -> Result<PathBuf, AppError> {
    if let Ok(path) = env::var("FLORAL_NOTEPAPER_DATA_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Ok(user_profile) = env::var("USERPROFILE") {
        return Ok(PathBuf::from(user_profile).join("Documents").join("花笺"));
    }

    Ok(env::current_dir()?.join("data"))
}

impl NoteStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    #[cfg(test)]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.base_dir.join("metadata.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn load_config(&self) -> Result<AppConfig, AppError> {
        self.ensure_base_dir()?;
        let path = self.config_path();
        if !path.exists() {
            let config = self.default_config();
            self.save_config(config.clone())?;
            return Ok(config);
        }

        let config: AppConfig = serde_json::from_str(&fs::read_to_string(path)?)?;
        fs::create_dir_all(&config.notes_dir)?;
        Ok(config)
    }

    pub fn save_config(&self, config: AppConfig) -> Result<(), AppError> {
        self.ensure_base_dir()?;
        fs::create_dir_all(&config.notes_dir)?;
        metadata::write_json_atomic(&self.config_path(), &config)
    }

    pub fn list_notes(&self) -> Result<Vec<NoteMetadata>, AppError> {
        self.ensure_storage()?;
        let mut metadata = self.load_metadata()?.notes;
        metadata.retain(|note| {
            self.note_path_in_category(&note.file_name, &note.category)
                .exists()
        });
        metadata.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(metadata)
    }

    pub fn read_note(&self, id: &str) -> Result<Note, AppError> {
        self.ensure_storage()?;
        let metadata = self.find_metadata(id)?;
        let content = fs::read_to_string(
            self.note_path_in_category(&metadata.file_name, &metadata.category),
        )?;
        Ok(Note {
            id: metadata.id,
            title: metadata.title,
            file_name: metadata.file_name,
            category: metadata.category,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            word_count: metadata.word_count,
            content,
        })
    }

    pub fn create_note(&self, request: SaveNoteRequest) -> Result<Note, AppError> {
        self.ensure_storage()?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let file_name = self.file_name_for(&id, &request.title);
        let word_count = count_words(&request.content);
        let category = request.category.clone();
        let note_path = self.note_path_in_category(&file_name, &category);
        if let Some(parent) = note_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let metadata = NoteMetadata {
            id: id.clone(),
            title: request.title,
            file_name: file_name.clone(),
            category: category.clone(),
            created_at: now,
            updated_at: now,
            word_count,
            preview: preview(&request.content),
        };

        fs::write(&note_path, &request.content)?;
        let mut metadata_file = self.load_metadata()?;
        metadata_file.notes.push(metadata.clone());
        self.save_metadata(&metadata_file)?;

        Ok(Note {
            id,
            title: metadata.title,
            file_name,
            category,
            created_at: now,
            updated_at: now,
            word_count,
            content: request.content,
        })
    }

    pub fn update_note(&self, id: &str, request: SaveNoteRequest) -> Result<Note, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let note = metadata_file
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::not_found(format!("Note {id} was not found")))?;

        let old_file_name = note.file_name.clone();
        let old_category = note.category.clone();
        let new_file_name = self.file_name_for(id, &request.title);
        let new_category = request.category.clone();
        let now = Utc::now();
        let word_count = count_words(&request.content);

        let new_path = self.note_path_in_category(&new_file_name, &new_category);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&new_path, &request.content)?;

        if old_file_name != new_file_name || old_category != new_category {
            let old_path = self.note_path_in_category(&old_file_name, &old_category);
            if old_path.exists() && old_path != new_path {
                fs::remove_file(old_path)?;
            }
        }

        note.title = request.title;
        note.file_name = new_file_name.clone();
        note.category = new_category.clone();
        note.updated_at = now;
        note.word_count = word_count;
        note.preview = preview(&request.content);

        let result = Note {
            id: note.id.clone(),
            title: note.title.clone(),
            file_name: note.file_name.clone(),
            category: new_category,
            created_at: note.created_at,
            updated_at: note.updated_at,
            word_count: note.word_count,
            content: request.content,
        };

        self.save_metadata(&metadata_file)?;
        Ok(result)
    }

    pub fn delete_note(&self, id: &str) -> Result<(), AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let index = metadata_file
            .notes
            .iter()
            .position(|note| note.id == id)
            .ok_or_else(|| AppError::not_found(format!("Note {id} was not found")))?;
        let metadata = metadata_file.notes.remove(index);
        let path = self.note_path_in_category(&metadata.file_name, &metadata.category);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.save_metadata(&metadata_file)
    }

    pub fn import_markdown_file(&self, path: &Path, category: &str) -> Result<Note, AppError> {
        if !is_markdown_path(path) {
            return Err(AppError::new("unsupportedFile", "只支持导入 .md 文件"));
        }

        let content = fs::read_to_string(path)?;
        let title = imported_markdown_title(path, &content);
        self.create_note(SaveNoteRequest {
            title,
            content,
            category: category.to_string(),
        })
    }

    pub fn export_markdown_file(&self, id: &str, path: &Path) -> Result<(), AppError> {
        let note = self.read_note(id)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, note.content)?;
        Ok(())
    }

    pub fn list_categories(&self) -> Result<Vec<String>, AppError> {
        let notes_dir = self.notes_dir()?;
        fs::create_dir_all(&notes_dir)?;
        let mut categories = Vec::new();
        for entry in fs::read_dir(&notes_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                categories.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        categories.sort();
        Ok(categories)
    }

    pub fn create_category(&self, name: &str) -> Result<(), AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::new("invalidCategory", "分类名不能为空"));
        }

        let path = self.notes_dir()?.join(name);
        fs::create_dir_all(path)?;
        Ok(())
    }

    pub fn rename_category(&self, old_name: &str, new_name: &str) -> Result<(), AppError> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(AppError::new("invalidCategory", "分类名不能为空"));
        }

        let notes_dir = self.notes_dir()?;
        let old_path = notes_dir.join(old_name);
        let new_path = notes_dir.join(new_name);
        if !old_path.exists() {
            return Err(AppError::not_found(format!("分类「{old_name}」不存在")));
        }
        if new_path.exists() {
            return Err(AppError::new(
                "conflict",
                format!("分类「{new_name}」已存在"),
            ));
        }

        fs::rename(&old_path, &new_path)?;

        let mut metadata_file = self.load_metadata()?;
        for note in &mut metadata_file.notes {
            if note.category == old_name {
                note.category = new_name.to_string();
            }
        }
        self.save_metadata(&metadata_file)?;
        Ok(())
    }

    pub fn delete_category(&self, name: &str) -> Result<(), AppError> {
        let notes_dir = self.notes_dir()?;
        let category_path = notes_dir.join(name);
        if !category_path.exists() {
            return Err(AppError::not_found(format!("分类「{name}」不存在")));
        }

        let mut metadata_file = self.load_metadata()?;
        for note in &mut metadata_file.notes {
            if note.category == name {
                let old_path = category_path.join(&note.file_name);
                let new_path = notes_dir.join(&note.file_name);
                if old_path.exists() {
                    fs::rename(&old_path, &new_path)?;
                }
                note.category = String::new();
            }
        }
        self.save_metadata(&metadata_file)?;

        if category_path.exists() {
            fs::remove_dir_all(category_path)?;
        }
        Ok(())
    }

    pub fn move_note_to_category(
        &self,
        id: &str,
        new_category: &str,
    ) -> Result<NoteMetadata, AppError> {
        self.ensure_storage()?;
        let mut metadata_file = self.load_metadata()?;
        let note = metadata_file
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::not_found(format!("Note {id} was not found")))?;

        let old_category = note.category.clone();
        if old_category == new_category {
            return Ok(note.clone());
        }

        let old_path = self.note_path_in_category(&note.file_name, &old_category);
        let new_path = self.note_path_in_category(&note.file_name, new_category);
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if old_path.exists() {
            fs::rename(&old_path, &new_path)?;
        }

        note.category = new_category.to_string();
        let result = note.clone();
        self.save_metadata(&metadata_file)?;
        Ok(result)
    }

    fn default_config(&self) -> AppConfig {
        AppConfig {
            notes_dir: self.base_dir.join("notes").to_string_lossy().to_string(),
            global_shortcut: "Ctrl+Space".into(),
            close_to_tray: true,
            autostart: false,
            default_view_mode: "split".into(),
            note_auto_save: default_note_auto_save(),
            note_surface_auto_save: default_note_surface_auto_save(),
            tile_color: default_tile_color(),
            tile_color_mode: default_tile_color_mode(),
            theme: default_theme(),
            font_size: default_font_size(),
            surface_font_size: default_surface_font_size(),
            external_file_auto_save: default_external_file_auto_save(),
        }
    }

    fn ensure_base_dir(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    fn ensure_storage(&self) -> Result<(), AppError> {
        self.ensure_base_dir()?;
        let config = self.load_config()?;
        fs::create_dir_all(&config.notes_dir)?;
        if !self.metadata_path().exists() {
            self.save_metadata(&metadata::MetadataFile::default())?;
        }
        Ok(())
    }

    fn notes_dir(&self) -> Result<PathBuf, AppError> {
        Ok(PathBuf::from(self.load_config()?.notes_dir))
    }

    fn note_path_in_category(&self, file_name: &str, category: &str) -> PathBuf {
        let notes_dir = self
            .notes_dir()
            .unwrap_or_else(|_| self.base_dir.join("notes"));
        if category.is_empty() {
            notes_dir.join(file_name)
        } else {
            notes_dir.join(category).join(file_name)
        }
    }

    fn find_metadata(&self, id: &str) -> Result<NoteMetadata, AppError> {
        self.load_metadata()?
            .notes
            .into_iter()
            .find(|note| note.id == id)
            .ok_or_else(|| AppError::not_found(format!("Note {id} was not found")))
    }

    fn file_name_for(&self, id: &str, title: &str) -> String {
        let safe_title = safe_file_stem(title);
        if safe_title.is_empty() {
            format!("{id}.md")
        } else {
            format!("{id}_{safe_title}.md")
        }
    }
}
