use super::notes::{AppConfig, AppError, NoteStore, SaveNoteRequest};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub const ADDON_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct OperationService {
    store: NoteStore,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdRequest {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateNoteRequest {
    id: String,
    request: SaveNoteRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedUpdateNoteRequest {
    id: String,
    expected_updated_at: DateTime<Utc>,
    request: SaveNoteRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveNoteRequest {
    id: String,
    category: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedMoveNoteRequest {
    id: String,
    expected_updated_at: DateTime<Utc>,
    category: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategoryRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameCategoryRequest {
    old_name: String,
    new_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    path: String,
    #[serde(default)]
    category: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportRequest {
    id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchNotesRequest {
    #[serde(default)]
    query: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_page_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct ConfigPatchRequest {
    patch: Value,
}

impl OperationService {
    pub fn new(store: NoteStore) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &NoteStore {
        &self.store
    }

    pub fn execute(&self, operation: &str, input: Value) -> Result<Value, AppError> {
        match operation {
            "protocol.version" => Ok(json!({
                "protocolVersion": ADDON_PROTOCOL_VERSION,
                "addonVersion": env!("CARGO_PKG_VERSION"),
            })),
            "store.info" => Ok(json!({ "baseDir": self.store.base_dir() })),
            "notes.list" => to_value(self.store.list_notes()?),
            "notes.search" => {
                let request: SearchNotesRequest = parse_input(input)?;
                to_value(self.store.search_notes(
                    &request.query,
                    request.category.as_deref(),
                    request.offset,
                    request.limit,
                )?)
            }
            "notes.get" => {
                let request: IdRequest = parse_input(input)?;
                to_value(self.store.read_note(&request.id)?)
            }
            "notes.create" => {
                let request: SaveNoteRequest = parse_input(input)?;
                to_value(self.store.create_note(request)?)
            }
            "notes.update" => {
                let request: UpdateNoteRequest = parse_input(input)?;
                to_value(self.store.update_note(&request.id, request.request)?)
            }
            "notes.updateIfUnchanged" => {
                let request: VersionedUpdateNoteRequest = parse_input(input)?;
                to_value(self.store.update_note_if_unchanged(
                    &request.id,
                    request.expected_updated_at,
                    request.request,
                )?)
            }
            "notes.delete" => {
                let request: IdRequest = parse_input(input)?;
                self.store.delete_note(&request.id)?;
                Ok(Value::Null)
            }
            "notes.move" => {
                let request: MoveNoteRequest = parse_input(input)?;
                to_value(
                    self.store
                        .move_note_to_category(&request.id, &request.category)?,
                )
            }
            "notes.moveIfUnchanged" => {
                let request: VersionedMoveNoteRequest = parse_input(input)?;
                to_value(self.store.move_note_to_category_if_unchanged(
                    &request.id,
                    request.expected_updated_at,
                    &request.category,
                )?)
            }
            "notes.import" => {
                let request: ImportRequest = parse_input(input)?;
                to_value(
                    self.store
                        .import_markdown_file(&PathBuf::from(request.path), &request.category)?,
                )
            }
            "notes.export" => {
                let request: ExportRequest = parse_input(input)?;
                self.store
                    .export_markdown_file(&request.id, &PathBuf::from(request.path))?;
                Ok(Value::Null)
            }
            "categories.list" => to_value(self.store.list_categories()?),
            "categories.create" => {
                let request: CategoryRequest = parse_input(input)?;
                self.store.create_category(&request.name)?;
                Ok(Value::Null)
            }
            "categories.rename" => {
                let request: RenameCategoryRequest = parse_input(input)?;
                self.store
                    .rename_category(&request.old_name, &request.new_name)?;
                Ok(Value::Null)
            }
            "categories.delete" => {
                let request: CategoryRequest = parse_input(input)?;
                self.store.delete_category(&request.name)?;
                Ok(Value::Null)
            }
            "config.get" => to_value(self.store.load_config()?),
            "config.patch" => {
                let request: ConfigPatchRequest = parse_input(input)?;
                let current = self.store.load_config()?;
                let mut value = serde_json::to_value(current)?;
                merge_object(&mut value, request.patch)?;
                let updated: AppConfig = serde_json::from_value(value)?;
                to_value(self.store.save_config(updated)?)
            }
            _ => Err(app_error(
                "unknownOperation",
                format!("Unknown backend operation: {operation}"),
            )),
        }
    }
}

fn default_page_limit() -> usize {
    50
}

fn parse_input<T: DeserializeOwned>(input: Value) -> Result<T, AppError> {
    serde_json::from_value(input).map_err(Into::into)
}

fn to_value(value: impl serde::Serialize) -> Result<Value, AppError> {
    serde_json::to_value(value).map_err(Into::into)
}

fn merge_object(target: &mut Value, patch: Value) -> Result<(), AppError> {
    let target = target.as_object_mut().ok_or_else(|| {
        app_error(
            "invalidConfig",
            "The current application config is not an object.",
        )
    })?;
    let patch = patch
        .as_object()
        .ok_or_else(|| app_error("invalidPatch", "Config patch must be a JSON object."))?;
    for (key, value) in patch {
        target.insert(key.clone(), value.clone());
    }
    Ok(())
}

fn app_error(code: impl Into<String>, message: impl Into<String>) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        details: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::notes::NoteStore;
    use serde_json::json;

    fn test_service(name: &str) -> OperationService {
        let root = std::env::temp_dir()
            .join("floral-notepaper-addon-tests")
            .join(name);
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove stale test data");
        }
        OperationService::new(NoteStore::new(root))
    }

    #[test]
    fn preserves_existing_json_operations() {
        let service = test_service("legacy-operations");
        let created = service
            .execute(
                "notes.create",
                json!({"title": "Draft", "content": "body", "category": ""}),
            )
            .expect("create note");

        let listed = service
            .execute("notes.list", json!({}))
            .expect("list notes");

        assert_eq!(listed.as_array().expect("note array").len(), 1);
        assert_eq!(listed[0]["id"], created["id"]);
    }

    #[test]
    fn exposes_version_search_and_conflict_safe_operations() {
        let service = test_service("safe-operations");
        let version = service
            .execute("protocol.version", json!({}))
            .expect("protocol version");
        assert_eq!(version["protocolVersion"], 1);

        let created = service
            .execute(
                "notes.create",
                json!({"title": "Draft", "content": "secret phrase", "category": ""}),
            )
            .expect("create note");
        let searched = service
            .execute(
                "notes.search",
                json!({"query": "secret", "offset": 0, "limit": 50}),
            )
            .expect("search notes");
        assert_eq!(searched["total"], 1);

        let updated = service
            .execute(
                "notes.updateIfUnchanged",
                json!({
                    "id": created["id"],
                    "expectedUpdatedAt": created["updatedAt"],
                    "request": {"title": "Draft", "content": "latest", "category": ""}
                }),
            )
            .expect("safe update");
        assert_eq!(updated["content"], "latest");

        let error = service
            .execute(
                "notes.moveIfUnchanged",
                json!({
                    "id": created["id"],
                    "expectedUpdatedAt": created["updatedAt"],
                    "category": "Archive"
                }),
            )
            .expect_err("reject stale move");
        assert_eq!(error.code, "noteConflict");
    }
}
