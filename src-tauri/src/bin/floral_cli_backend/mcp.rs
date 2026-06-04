use floral_notepaper_lib::services::{addon::OperationService, notes::AppError};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        AnnotateAble, CallToolResult, GetPromptRequestParams, GetPromptResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, Prompt,
        PromptArgument, PromptMessage, PromptMessageRole, RawResource, RawResourceTemplate,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo,
    },
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

const DEFAULT_PAGE_LIMIT: usize = 50;
const MAX_PAGE_LIMIT: usize = 200;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListNotesInput {
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_page_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SearchNotesInput {
    query: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_page_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NoteIdInput {
    id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateNoteInput {
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    category: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpdateNoteInput {
    id: String,
    expected_updated_at: String,
    title: String,
    content: String,
    #[serde(default)]
    category: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MoveNoteInput {
    id: String,
    expected_updated_at: String,
    #[serde(default)]
    category: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateCategoryInput {
    name: String,
}

#[derive(Debug, Clone)]
pub struct FloralMcpServer {
    service: OperationService,
    tool_router: ToolRouter<Self>,
}

impl FloralMcpServer {
    pub fn new(service: OperationService) -> Self {
        Self {
            service,
            tool_router: Self::tool_router(),
        }
    }

    async fn execute(&self, operation: &'static str, input: Value) -> Result<Value, AppError> {
        let service = self.service.clone();
        tokio::task::spawn_blocking(move || service.execute(operation, input))
            .await
            .map_err(|error| AppError {
                code: "taskFailed".into(),
                message: error.to_string(),
                details: Default::default(),
            })?
    }

    async fn execute_tool(&self, operation: &'static str, input: Value) -> CallToolResult {
        tool_result(self.execute(operation, input).await)
    }

    #[cfg(test)]
    fn tool_names(&self) -> Vec<String> {
        self.tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }

    #[cfg(test)]
    fn resource_uris(&self) -> Vec<String> {
        vec!["floral://categories".into(), "floral://notes".into()]
    }

    #[cfg(test)]
    fn resource_template_uris(&self) -> Vec<String> {
        vec!["floral://notes/{id}".into()]
    }

    #[cfg(test)]
    fn prompt_names(&self) -> Vec<String> {
        vec![
            "create_note_from_conversation".into(),
            "organize_note".into(),
            "summarize_note".into(),
        ]
    }

    async fn read_resource_value(&self, uri: &str) -> Result<Value, AppError> {
        match uri {
            "floral://notes" => {
                let value = self.execute("notes.list", json!({})).await?;
                let mut notes = value.as_array().cloned().unwrap_or_default();
                let total = notes.len();
                notes.truncate(MAX_PAGE_LIMIT);
                Ok(json!({
                    "items": notes,
                    "total": total,
                    "truncated": total > MAX_PAGE_LIMIT,
                }))
            }
            "floral://categories" => self.execute("categories.list", json!({})).await,
            _ if uri.starts_with("floral://notes/") => {
                let id = uri.trim_start_matches("floral://notes/");
                self.execute("notes.get", json!({ "id": id })).await
            }
            _ => Err(AppError {
                code: "resourceNotFound".into(),
                message: format!("Unknown Floral resource: {uri}"),
                details: Default::default(),
            }),
        }
    }

    async fn prompt_result(
        &self,
        name: &str,
        arguments: Option<Map<String, Value>>,
    ) -> Result<GetPromptResult, McpError> {
        let arguments = arguments.unwrap_or_default();
        let result = match name {
            "summarize_note" => {
                let note_id = required_argument(&arguments, "noteId")?;
                let detail = optional_argument(&arguments, "detailLevel").unwrap_or("concise");
                let note = self
                    .execute("notes.get", json!({ "id": note_id }))
                    .await
                    .map_err(mcp_error)?;
                GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Summarize this Floral note at a {detail} level. Preserve factual meaning and do not modify the note unless the user explicitly asks.\n\n{}",
                        serde_json::to_string_pretty(&note).unwrap_or_else(|_| note.to_string())
                    ),
                )])
                .with_description("Summarize an existing Floral note")
            }
            "organize_note" => {
                let note_id = required_argument(&arguments, "noteId")?;
                let goal = optional_argument(&arguments, "goal")
                    .unwrap_or("improve structure and readability");
                let note = self
                    .execute("notes.get", json!({ "id": note_id }))
                    .await
                    .map_err(mcp_error)?;
                GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Propose an organized revision of this Floral note. Goal: {goal}. Show the proposed changes and obtain user confirmation before calling floral_notes_update. Use the note's updatedAt value as expectedUpdatedAt.\n\n{}",
                        serde_json::to_string_pretty(&note).unwrap_or_else(|_| note.to_string())
                    ),
                )])
                .with_description("Organize an existing Floral note safely")
            }
            "create_note_from_conversation" => {
                let title = optional_argument(&arguments, "title").unwrap_or("Untitled");
                let category = optional_argument(&arguments, "category").unwrap_or("");
                GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Create a Floral note from the useful information in this conversation. Suggested title: {title}. Suggested category: {category}. Present the final title and Markdown content for confirmation before calling floral_notes_create."
                    ),
                )])
                .with_description("Create a Floral note from the current conversation")
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("Unknown Floral prompt: {name}"),
                    None,
                ));
            }
        };
        Ok(result)
    }
}

#[tool_router]
impl FloralMcpServer {
    /// List Floral note metadata with optional category filtering and pagination.
    #[tool(
        name = "floral_notes_list",
        annotations(
            title = "List Floral notes",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn floral_notes_list(
        &self,
        Parameters(input): Parameters<ListNotesInput>,
    ) -> CallToolResult {
        let result = self.execute("notes.list", json!({})).await.map(|value| {
            let mut notes = value.as_array().cloned().unwrap_or_default();
            if let Some(category) = input.category {
                notes.retain(|note| note["category"].as_str() == Some(category.as_str()));
            }
            let total = notes.len();
            let limit = normalize_limit(input.limit);
            let items = notes
                .into_iter()
                .skip(input.offset)
                .take(limit)
                .collect::<Vec<_>>();
            json!({
                "items": items,
                "offset": input.offset,
                "limit": limit,
                "total": total,
                "truncated": input.offset.saturating_add(items.len()) < total,
            })
        });
        tool_result(result)
    }

    /// Search Floral note titles, previews, and Markdown bodies.
    #[tool(
        name = "floral_notes_search",
        annotations(
            title = "Search Floral notes",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn floral_notes_search(
        &self,
        Parameters(input): Parameters<SearchNotesInput>,
    ) -> CallToolResult {
        self.execute_tool(
            "notes.search",
            json!({
                "query": input.query,
                "category": input.category,
                "offset": input.offset,
                "limit": input.limit,
            }),
        )
        .await
    }

    /// Read one complete Floral note by ID.
    #[tool(
        name = "floral_notes_get",
        annotations(
            title = "Read a Floral note",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn floral_notes_get(&self, Parameters(input): Parameters<NoteIdInput>) -> CallToolResult {
        self.execute_tool("notes.get", json!({ "id": input.id }))
            .await
    }

    /// Create a new Floral note after the user has approved its content.
    #[tool(
        name = "floral_notes_create",
        annotations(
            title = "Create a Floral note",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn floral_notes_create(
        &self,
        Parameters(input): Parameters<CreateNoteInput>,
    ) -> CallToolResult {
        self.execute_tool(
            "notes.create",
            json!({
                "title": input.title,
                "content": input.content,
                "category": input.category,
            }),
        )
        .await
    }

    /// Update a Floral note only when it has not changed since it was read.
    #[tool(
        name = "floral_notes_update",
        annotations(
            title = "Update a Floral note",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn floral_notes_update(
        &self,
        Parameters(input): Parameters<UpdateNoteInput>,
    ) -> CallToolResult {
        self.execute_tool(
            "notes.updateIfUnchanged",
            json!({
                "id": input.id,
                "expectedUpdatedAt": input.expected_updated_at,
                "request": {
                    "title": input.title,
                    "content": input.content,
                    "category": input.category,
                }
            }),
        )
        .await
    }

    /// Move a Floral note only when it has not changed since it was read.
    #[tool(
        name = "floral_notes_move",
        annotations(
            title = "Move a Floral note",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn floral_notes_move(
        &self,
        Parameters(input): Parameters<MoveNoteInput>,
    ) -> CallToolResult {
        self.execute_tool(
            "notes.moveIfUnchanged",
            json!({
                "id": input.id,
                "expectedUpdatedAt": input.expected_updated_at,
                "category": input.category,
            }),
        )
        .await
    }

    /// List all Floral note categories.
    #[tool(
        name = "floral_categories_list",
        annotations(
            title = "List Floral categories",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn floral_categories_list(&self) -> CallToolResult {
        self.execute_tool("categories.list", json!({})).await
    }

    /// Create a Floral note category.
    #[tool(
        name = "floral_categories_create",
        annotations(
            title = "Create a Floral category",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn floral_categories_create(
        &self,
        Parameters(input): Parameters<CreateCategoryInput>,
    ) -> CallToolResult {
        self.execute_tool("categories.create", json!({ "name": input.name }))
            .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FloralMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "floral-notepaper",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Use Floral tools to read and safely manage local notes. Ask the user before write operations.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(vec![
            RawResource::new("floral://categories", "Floral categories")
                .with_description("All Floral note categories")
                .with_mime_type("application/json")
                .no_annotation(),
            RawResource::new("floral://notes", "Floral notes")
                .with_description("The 200 most recently updated Floral notes")
                .with_mime_type("application/json")
                .no_annotation(),
        ]))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(vec![
            RawResourceTemplate::new("floral://notes/{id}", "Floral note")
                .with_description("Read one complete Floral note by ID")
                .with_mime_type("application/json")
                .no_annotation(),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let value = self
            .read_resource_value(&request.uri)
            .await
            .map_err(mcp_error)?;
        let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            text,
            request.uri,
        )
        .with_mime_type("application/json")]))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(prompt_catalog()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        self.prompt_result(&request.name, request.arguments).await
    }
}

pub async fn serve(
    service: OperationService,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    FloralMcpServer::new(service)
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

fn default_page_limit() -> usize {
    DEFAULT_PAGE_LIMIT
}

fn normalize_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_PAGE_LIMIT
    } else {
        limit.min(MAX_PAGE_LIMIT)
    }
}

fn tool_result(result: Result<Value, AppError>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult::structured(value),
        Err(error) => CallToolResult::structured_error(error_value(error)),
    }
}

fn error_value(error: AppError) -> Value {
    json!({
        "code": error.code,
        "message": error.message,
        "details": error.details,
    })
}

fn mcp_error(error: AppError) -> McpError {
    McpError::invalid_params(error.message.clone(), Some(error_value(error)))
}

fn required_argument<'a>(
    arguments: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, McpError> {
    optional_argument(arguments, name).ok_or_else(|| {
        McpError::invalid_params(format!("Prompt argument '{name}' is required."), None)
    })
}

fn optional_argument<'a>(arguments: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    arguments.get(name).and_then(Value::as_str)
}

fn prompt_catalog() -> Vec<Prompt> {
    vec![
        Prompt::new(
            "create_note_from_conversation",
            Some("Prepare a Floral note from the current conversation"),
            Some(vec![
                PromptArgument::new("title")
                    .with_description("Optional suggested note title")
                    .with_required(false),
                PromptArgument::new("category")
                    .with_description("Optional suggested Floral category")
                    .with_required(false),
            ]),
        ),
        Prompt::new(
            "organize_note",
            Some("Propose a clearer structure for an existing Floral note"),
            Some(vec![
                PromptArgument::new("noteId")
                    .with_description("The Floral note ID")
                    .with_required(true),
                PromptArgument::new("goal")
                    .with_description("Optional organization goal")
                    .with_required(false),
            ]),
        ),
        Prompt::new(
            "summarize_note",
            Some("Summarize an existing Floral note"),
            Some(vec![
                PromptArgument::new("noteId")
                    .with_description("The Floral note ID")
                    .with_required(true),
                PromptArgument::new("detailLevel")
                    .with_description("Optional summary detail level")
                    .with_required(false),
            ]),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use floral_notepaper_lib::services::{addon::OperationService, notes::NoteStore};

    fn server(name: &str) -> FloralMcpServer {
        let root = std::env::temp_dir()
            .join("floral-notepaper-mcp-tests")
            .join(name);
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove stale test data");
        }
        FloralMcpServer::new(OperationService::new(NoteStore::new(root)))
    }

    #[test]
    fn exposes_only_the_safe_mcp_tools() {
        let server = server("tools");
        let names = server
            .tool_names()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(
            names,
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
    }

    #[test]
    fn exposes_resource_and_prompt_catalogs() {
        let server = server("catalogs");

        assert_eq!(
            server.resource_uris(),
            ["floral://categories", "floral://notes"]
        );
        assert_eq!(server.resource_template_uris(), ["floral://notes/{id}"]);
        assert_eq!(
            server.prompt_names(),
            [
                "create_note_from_conversation",
                "organize_note",
                "summarize_note"
            ]
        );
    }
}
