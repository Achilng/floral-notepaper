#![cfg(feature = "floral-ai-addon")]

use rmcp::{
    model::{CallToolRequestParams, GetPromptRequestParams, ReadResourceRequestParams},
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeSet,
    error::Error,
    path::{Path, PathBuf},
};

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn real_stdio_server_exposes_safe_capabilities_and_survives_conflicts() -> TestResult {
    let data_dir = fresh_data_dir("capabilities-and-conflicts");
    let transport = TokioChildProcess::new(
        tokio::process::Command::new(env!("CARGO_BIN_EXE_floral_cli_backend")).configure(
            |command| {
                command.arg("mcp").arg("--data-dir").arg(&data_dir);
            },
        ),
    )?;
    let client = ().serve(transport).await?;

    let tools = client
        .list_all_tools()
        .await?
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        tools,
        [
            "floral_categories_create",
            "floral_categories_list",
            "floral_notes_create",
            "floral_notes_get",
            "floral_notes_list",
            "floral_notes_move",
            "floral_notes_search",
            "floral_notes_update",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );
    assert!(!tools.iter().any(|name| {
        name.contains("delete")
            || name.contains("rename")
            || name.contains("config")
            || name.contains("import")
            || name.contains("export")
    }));

    let created = call_tool(
        &client,
        "floral_notes_create",
        json!({
            "title": "MCP draft",
            "content": "A searchable sentence.",
            "category": ""
        }),
    )
    .await?;
    assert_eq!(created.is_error, Some(false));
    let created = created.structured_content.expect("created note");
    let note_id = created["id"].as_str().expect("note id").to_string();
    let original_updated_at = created["updatedAt"]
        .as_str()
        .expect("initial updatedAt")
        .to_string();

    let updated = call_tool(
        &client,
        "floral_notes_update",
        json!({
            "id": note_id,
            "expectedUpdatedAt": original_updated_at,
            "title": "MCP draft",
            "content": "The latest content.",
            "category": ""
        }),
    )
    .await?;
    assert_eq!(updated.is_error, Some(false));

    let conflict = call_tool(
        &client,
        "floral_notes_move",
        json!({
            "id": note_id,
            "expectedUpdatedAt": original_updated_at,
            "category": "Archive"
        }),
    )
    .await?;
    assert_eq!(conflict.is_error, Some(true));
    assert_eq!(
        conflict
            .structured_content
            .as_ref()
            .and_then(|value| value["code"].as_str()),
        Some("noteConflict")
    );

    let fetched = call_tool(&client, "floral_notes_get", json!({ "id": note_id })).await?;
    assert_eq!(fetched.is_error, Some(false));
    assert_eq!(
        fetched
            .structured_content
            .as_ref()
            .and_then(|value| value["content"].as_str()),
        Some("The latest content.")
    );

    let resource_uris = client
        .list_all_resources()
        .await?
        .into_iter()
        .map(|resource| resource.raw.uri)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        resource_uris,
        ["floral://categories", "floral://notes"]
            .into_iter()
            .map(str::to_string)
            .collect()
    );

    let templates = client.list_all_resource_templates().await?;
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].raw.uri_template, "floral://notes/{id}");

    let prompts = client
        .list_all_prompts()
        .await?
        .into_iter()
        .map(|prompt| prompt.name)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        prompts,
        [
            "create_note_from_conversation",
            "organize_note",
            "summarize_note",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );

    let note_resource = client
        .read_resource(ReadResourceRequestParams::new(format!(
            "floral://notes/{note_id}"
        )))
        .await?;
    assert_eq!(note_resource.contents.len(), 1);

    let prompt = client
        .get_prompt(
            GetPromptRequestParams::new("summarize_note")
                .with_arguments(arguments(json!({ "noteId": note_id }))),
        )
        .await?;
    assert_eq!(prompt.messages.len(), 1);

    // Closing the client side of stdio sends EOF, which must let the add-on exit cleanly.
    client.cancel().await?;
    Ok(())
}

async fn call_tool(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    name: &'static str,
    value: Value,
) -> Result<rmcp::model::CallToolResult, rmcp::service::ServiceError> {
    client
        .call_tool(CallToolRequestParams::new(name).with_arguments(arguments(value)))
        .await
}

fn arguments(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("tool arguments object")
}

fn fresh_data_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir()
        .join("floral-notepaper-mcp-e2e")
        .join(name);
    remove_stale_dir(&path);
    path
}

fn remove_stale_dir(path: &Path) {
    if path.exists() {
        std::fs::remove_dir_all(path).expect("remove stale MCP test data");
    }
}
