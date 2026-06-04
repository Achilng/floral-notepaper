use floral_notepaper_lib::services::notes::{default_store, AppConfig, AppError, SaveNoteRequest};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    env,
    io::{self, Read},
    path::PathBuf,
};

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
struct MoveNoteRequest {
    id: String,
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
struct ConfigPatchRequest {
    patch: Value,
}

fn main() {
    let result = run();
    match result {
        Ok(data) => println!("{}", json!({ "ok": true, "data": data })),
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "error": {
                        "code": error.code,
                        "message": error.message,
                        "details": error.details,
                    }
                })
            );
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Value, AppError> {
    let operation = env::args()
        .nth(1)
        .ok_or_else(|| app_error("missingOperation", "A backend operation is required."))?;
    let input = read_input()?;
    let store = default_store()?;

    match operation.as_str() {
        "store.info" => Ok(json!({ "baseDir": store.base_dir() })),
        "notes.list" => to_value(store.list_notes()?),
        "notes.get" => {
            let request: IdRequest = parse_input(input)?;
            to_value(store.read_note(&request.id)?)
        }
        "notes.create" => {
            let request: SaveNoteRequest = parse_input(input)?;
            to_value(store.create_note(request)?)
        }
        "notes.update" => {
            let request: UpdateNoteRequest = parse_input(input)?;
            to_value(store.update_note(&request.id, request.request)?)
        }
        "notes.delete" => {
            let request: IdRequest = parse_input(input)?;
            store.delete_note(&request.id)?;
            Ok(Value::Null)
        }
        "notes.move" => {
            let request: MoveNoteRequest = parse_input(input)?;
            to_value(store.move_note_to_category(&request.id, &request.category)?)
        }
        "notes.import" => {
            let request: ImportRequest = parse_input(input)?;
            to_value(store.import_markdown_file(&PathBuf::from(request.path), &request.category)?)
        }
        "notes.export" => {
            let request: ExportRequest = parse_input(input)?;
            store.export_markdown_file(&request.id, &PathBuf::from(request.path))?;
            Ok(Value::Null)
        }
        "categories.list" => to_value(store.list_categories()?),
        "categories.create" => {
            let request: CategoryRequest = parse_input(input)?;
            store.create_category(&request.name)?;
            Ok(Value::Null)
        }
        "categories.rename" => {
            let request: RenameCategoryRequest = parse_input(input)?;
            store.rename_category(&request.old_name, &request.new_name)?;
            Ok(Value::Null)
        }
        "categories.delete" => {
            let request: CategoryRequest = parse_input(input)?;
            store.delete_category(&request.name)?;
            Ok(Value::Null)
        }
        "config.get" => to_value(store.load_config()?),
        "config.patch" => {
            let request: ConfigPatchRequest = parse_input(input)?;
            let current = store.load_config()?;
            let mut value = serde_json::to_value(current)?;
            merge_object(&mut value, request.patch)?;
            let updated: AppConfig = serde_json::from_value(value)?;
            to_value(store.save_config(updated)?)
        }
        _ => Err(app_error(
            "unknownOperation",
            format!("Unknown backend operation: {operation}"),
        )),
    }
}

fn read_input() -> Result<Value, AppError> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        Ok(Value::Object(Default::default()))
    } else {
        serde_json::from_str(&raw).map_err(Into::into)
    }
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
