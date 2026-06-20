use crate::error::{Error, Result as CbmResult};
use crate::mcp::params::{
    IndexRepositoryArgs, IngestTracesArgs, ManageAdrArgs, ProjectArgs, QueryGraphArgs,
    SearchCodeArgs, SearchGraphArgs, SnippetArgs, TraceArgs,
};
use crate::mcp::tools::ToolHandler;
use crate::watcher::Watcher;
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{
    CallToolResult, Content, Implementation, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

pub const SERVER_NAME: &str = "codebase-memory-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct McpServer {
    handler: ToolHandler,
    watcher: Option<Arc<Watcher>>,
    tool_router: ToolRouter<Self>,
}

impl McpServer {
    pub fn new() -> Self {
        let watcher = if watcher_enabled() {
            let watcher = Arc::new(Watcher::new());
            watcher.refresh_from_disk();
            Some(watcher)
        } else {
            None
        };
        Self {
            handler: ToolHandler::new(watcher.clone()),
            watcher,
            tool_router: Self::tool_router(),
        }
    }

    pub fn watcher(&self) -> Option<Arc<Watcher>> {
        self.watcher.clone()
    }

    pub fn generated_tool_definitions() -> Vec<Value> {
        normalize_tool_schemas(Self::tool_router().list_all())
            .into_iter()
            .map(|tool| serde_json::to_value(tool).expect("rmcp tool must serialize"))
            .collect()
    }

    pub fn start_background_services(&self, shutdown: Option<Arc<crate::runtime::Shutdown>>) {
        if let Some(watcher) = &self.watcher {
            watcher.clone().spawn(shutdown);
        }
    }

    pub fn stop_services(&self) {
        if let Some(watcher) = &self.watcher {
            watcher.stop();
        }
    }

    pub async fn serve_stdio(self) -> CbmResult<()> {
        let service = self
            .clone()
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|error| Error::Other(format!("failed to start MCP stdio service: {error}")))?;
        let result = service.waiting().await;
        self.stop_services();
        result
            .map(|_| ())
            .map_err(|error| Error::Other(format!("MCP stdio service failed: {error}")))
    }

    async fn invoke<P>(&self, name: &'static str, params: P) -> CallToolResult
    where
        P: Serialize + Send + 'static,
    {
        let handler = self.handler.clone();
        let args = match serde_json::to_value(params) {
            Ok(args) => args,
            Err(error) => return tool_error(format!("invalid tool arguments: {error}")),
        };

        match tokio::task::spawn_blocking(move || handler.handle(name, &args)).await {
            Ok(Ok(value)) => match serde_json::to_string_pretty(&value) {
                Ok(text) => CallToolResult::success(vec![Content::text(text)]),
                Err(error) => tool_error(format!("failed to encode tool result: {error}")),
            },
            Ok(Err(error)) => tool_error(error.to_string()),
            Err(error) => {
                tracing::error!(tool = name, %error, "CBM tool worker failed");
                tool_error("internal tool worker failure")
            }
        }
    }

    async fn invoke_empty(&self, name: &'static str) -> CallToolResult {
        self.invoke(name, serde_json::json!({})).await
    }
}

pub fn tool_definitions() -> Vec<Value> {
    McpServer::generated_tool_definitions()
}

#[tool_router(router = tool_router)]
impl McpServer {
    #[tool(
        name = "index_repository",
        description = "Index a repository into the knowledge graph."
    )]
    async fn index_repository(
        &self,
        Parameters(params): Parameters<IndexRepositoryArgs>,
    ) -> CallToolResult {
        self.invoke("index_repository", params).await
    }

    #[tool(
        name = "index_status",
        description = "Check index status for a project."
    )]
    async fn index_status(&self, Parameters(params): Parameters<ProjectArgs>) -> CallToolResult {
        self.invoke("index_status", params).await
    }

    #[tool(
        name = "search_graph",
        description = "Search the code knowledge graph."
    )]
    async fn search_graph(
        &self,
        Parameters(params): Parameters<SearchGraphArgs>,
    ) -> CallToolResult {
        self.invoke("search_graph", params).await
    }

    #[tool(name = "trace_path", description = "Trace call paths.")]
    async fn trace_path(&self, Parameters(params): Parameters<TraceArgs>) -> CallToolResult {
        self.invoke("trace_path", params).await
    }

    #[tool(
        name = "get_code_snippet",
        description = "Read source code for a symbol."
    )]
    async fn get_code_snippet(
        &self,
        Parameters(params): Parameters<SnippetArgs>,
    ) -> CallToolResult {
        self.invoke("get_code_snippet", params).await
    }

    #[tool(
        name = "get_graph_schema",
        description = "Get the schema of the knowledge graph."
    )]
    async fn get_graph_schema(
        &self,
        Parameters(params): Parameters<ProjectArgs>,
    ) -> CallToolResult {
        self.invoke("get_graph_schema", params).await
    }

    #[tool(name = "get_architecture", description = "Architecture overview.")]
    async fn get_architecture(
        &self,
        Parameters(params): Parameters<ProjectArgs>,
    ) -> CallToolResult {
        self.invoke("get_architecture", params).await
    }

    #[tool(
        name = "query_graph",
        description = "Execute a read-only graph query (SELECT on symbols/edges/files)."
    )]
    async fn query_graph(&self, Parameters(params): Parameters<QueryGraphArgs>) -> CallToolResult {
        self.invoke("query_graph", params).await
    }

    #[tool(
        name = "search_code",
        description = "Graph-augmented code search. Modes: compact, files."
    )]
    async fn search_code(&self, Parameters(params): Parameters<SearchCodeArgs>) -> CallToolResult {
        self.invoke("search_code", params).await
    }

    #[tool(name = "list_projects", description = "List indexed projects.")]
    async fn list_projects(&self) -> CallToolResult {
        self.invoke_empty("list_projects").await
    }

    #[tool(name = "delete_project", description = "Delete a project index.")]
    async fn delete_project(&self, Parameters(params): Parameters<ProjectArgs>) -> CallToolResult {
        self.invoke("delete_project", params).await
    }

    #[tool(name = "detect_changes", description = "Detect git-changed files.")]
    async fn detect_changes(&self, Parameters(params): Parameters<ProjectArgs>) -> CallToolResult {
        self.invoke("detect_changes", params).await
    }

    #[tool(
        name = "manage_adr",
        description = "Create or update Architecture Decision Records."
    )]
    async fn manage_adr(&self, Parameters(params): Parameters<ManageAdrArgs>) -> CallToolResult {
        self.invoke("manage_adr", params).await
    }

    #[tool(
        name = "ingest_traces",
        description = "Ingest runtime traces to enhance the knowledge graph."
    )]
    async fn ingest_traces(
        &self,
        Parameters(params): Parameters<IngestTracesArgs>,
    ) -> CallToolResult {
        self.invoke("ingest_traces", params).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpServer {
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: normalize_tool_schemas(self.tool_router.list_all()),
            meta: None,
            next_cursor: None,
        })
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_router
            .get(name)
            .cloned()
            .map(normalize_tool_schema)
    }

    fn get_info(&self) -> ServerInfo {
        let watcher_on = self.watcher.is_some();
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, SERVER_VERSION))
            .with_instructions(format!(
                "CBM graph server. Index with index_repository, then use search_graph, trace_path, or query_graph. Git watcher: {watcher_on}. RLM tools are provided by the independent rlm-mcp server."
            ))
    }
}

fn normalize_tool_schemas(tools: Vec<Tool>) -> Vec<Tool> {
    tools.into_iter().map(normalize_tool_schema).collect()
}

fn normalize_tool_schema(mut tool: Tool) -> Tool {
    let mut schema = Value::Object(tool.input_schema.as_ref().clone());
    normalize_json_schema_node(&mut schema);
    if let Value::Object(object) = schema {
        tool.input_schema = Arc::new(object);
    }
    tool
}

fn normalize_json_schema_node(value: &mut Value) {
    match value {
        Value::Bool(_) => *value = Value::Object(Default::default()),
        Value::Object(object) => {
            for key in ["properties", "patternProperties", "$defs", "definitions"] {
                if let Some(Value::Object(children)) = object.get_mut(key) {
                    for child in children.values_mut() {
                        normalize_json_schema_node(child);
                    }
                }
            }
            for key in [
                "items",
                "additionalProperties",
                "contains",
                "not",
                "if",
                "then",
                "else",
                "propertyNames",
            ] {
                if let Some(child) = object.get_mut(key) {
                    normalize_json_schema_node(child);
                }
            }
            for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
                if let Some(Value::Array(items)) = object.get_mut(key) {
                    for item in items {
                        normalize_json_schema_node(item);
                    }
                }
            }
        }
        _ => {}
    }
}

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![Content::text(message.into())])
}

fn watcher_enabled() -> bool {
    let value = std::env::var("CBM_WATCHER")
        .or_else(|_| std::env::var("CBRLM_WATCHER"))
        .unwrap_or_default();
    !matches!(value.as_str(), "0" | "false" | "off")
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
