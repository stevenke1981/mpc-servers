use memory_core::{
    models::{HybridWeights, MemoryScope, SearchQuery},
    service::MemoryService,
};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, ErrorCode, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
        ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    transport::stdio,
    ErrorData as McpError, ServiceExt,
};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryMcpServer {
    service: Arc<MemoryService>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool Input Schemas
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct AddMemoryInput {
    #[schemars(description = "Conversation text or fact to extract memories from")]
    pub content: String,
    #[schemars(description = "Scope of the memory (Global, Project, Session, Agent)")]
    pub scope: Option<String>,
    #[schemars(description = "Project path or ID (required when scope=Project)")]
    pub project_id: Option<String>,
    #[schemars(description = "Agent instance ID (required when scope=Agent)")]
    pub agent_id: Option<String>,
    #[schemars(description = "Session ID")]
    pub session_id: Option<String>,
    #[schemars(description = "Additional metadata key-value pairs")]
    pub metadata: Option<Value>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EndSessionInput {
    #[schemars(description = "Session ID to end")]
    pub session_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchWeightsInput {
    pub semantic: Option<f64>,
    pub bm25: Option<f64>,
    pub temporal: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchMemoriesInput {
    #[schemars(description = "Natural language search query")]
    pub query: String,
    #[schemars(description = "Number of memories to return")]
    pub top_k: Option<usize>,
    #[schemars(description = "Filter by scope")]
    pub scope: Option<String>,
    #[schemars(description = "Filter by project ID")]
    pub project_id: Option<String>,
    #[schemars(description = "Session ID (for session_stats tracking)")]
    pub session_id: Option<String>,
    #[schemars(description = "Filter by categories")]
    pub categories: Option<Vec<String>>,
    #[schemars(description = "Minimum importance score threshold")]
    pub min_importance: Option<f64>,
    #[schemars(description = "Weights for semantic, BM25, and temporal scores")]
    pub weights: Option<SearchWeightsInput>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetMemoriesInput {
    #[schemars(description = "List of memory IDs to fetch")]
    pub ids: Option<Vec<String>>,
    #[schemars(description = "Filter by scope")]
    pub scope: Option<String>,
    #[schemars(description = "Filter by project ID")]
    pub project_id: Option<String>,
    #[schemars(description = "Limit of results")]
    pub limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteMemoryInput {
    #[schemars(description = "Memory UUID to delete")]
    pub id: String,
}

#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ConsolidateMemoriesInput {
    #[schemars(description = "Filter consolidation by scope")]
    pub scope: Option<String>,
    #[schemars(description = "Filter consolidation by project ID")]
    pub project_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EmptyInput {}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryMcpServer Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl MemoryMcpServer {
    pub fn new(service: Arc<MemoryService>) -> Self {
        Self { service }
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let running = self.serve(stdio()).await?;
        running.waiting().await?;
        Ok(())
    }

    async fn add_memory(&self, input: AddMemoryInput) -> Result<CallToolResult, McpError> {
        let scope_raw = input.scope.as_deref().unwrap_or("Global");
        let scope = MemoryScope::from_str(scope_raw)
            .map_err(|e| McpError::invalid_params(format!("Invalid scope: {e}"), None))?;

        // Validate scope requirements
        match &scope {
            MemoryScope::Project => {
                if input.project_id.is_none() {
                    return Err(McpError::invalid_params(
                        "project_id is required when scope=Project",
                        None,
                    ));
                }
            }
            MemoryScope::Agent if input.agent_id.is_none() => {
                return Err(McpError::invalid_params(
                    "agent_id is required when scope=Agent",
                    None,
                ));
            }
            _ => {}
        }

        let session_id = input.session_id.unwrap_or_else(|| "default".to_string());

        let memories = self
            .service
            .add_memory(
                &input.content,
                scope,
                input.project_id,
                input.agent_id,
                session_id,
                input.metadata,
            )
            .await
            .map_err(|e| McpError::internal_error(format!("Failed to add memory: {}", e), None))?;

        let text = serde_json::to_string_pretty(&memories).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn search_memories(
        &self,
        input: SearchMemoriesInput,
    ) -> Result<CallToolResult, McpError> {
        let scope = input
            .scope
            .as_deref()
            .map(|s| {
                MemoryScope::from_str(s)
                    .map_err(|e| McpError::invalid_params(format!("Invalid scope: {e}"), None))
            })
            .transpose()?;

        let categories = input.categories.map(|arr| {
            arr.iter()
                .filter_map(|val| val.parse::<memory_core::models::MemoryCategory>().ok())
                .collect::<Vec<_>>()
        });

        let weights = input.weights.map(|w| {
            let semantic = w.semantic.unwrap_or(0.6);
            let bm25 = w.bm25.unwrap_or(0.3);
            let temporal = w.temporal.unwrap_or(0.1);
            HybridWeights {
                semantic,
                bm25,
                temporal,
            }
        });

        let query = SearchQuery {
            query: input.query,
            top_k: input.top_k.unwrap_or(10),
            scope,
            project_id: input.project_id,
            session_id: input.session_id,
            categories,
            created_after: None,
            min_importance: input.min_importance,
            include_decayed: false,
            weights,
        };

        query
            .validate()
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let results = self.service.search_memories(&query).await.map_err(|e| {
            McpError::internal_error(format!("Failed to search memories: {}", e), None)
        })?;

        let text = serde_json::to_string_pretty(&results).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn get_memories(&self, input: GetMemoriesInput) -> Result<CallToolResult, McpError> {
        let scope = input
            .scope
            .as_deref()
            .map(|s| {
                MemoryScope::from_str(s)
                    .map_err(|e| McpError::invalid_params(format!("Invalid scope: {e}"), None))
            })
            .transpose()?;

        let limit = input.limit.unwrap_or(20);

        let memories = self
            .service
            .get_memories(input.ids, scope, input.project_id, limit)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Failed to retrieve memories: {}", e), None)
            })?;

        let text = serde_json::to_string_pretty(&memories).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn delete_memory(&self, input: DeleteMemoryInput) -> Result<CallToolResult, McpError> {
        let deleted = self.service.delete_memory(&input.id).await.map_err(|e| {
            McpError::internal_error(format!("Failed to delete memory: {}", e), None)
        })?;

        let text = serde_json::to_string_pretty(&deleted).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn consolidate_memories(
        &self,
        input: ConsolidateMemoriesInput,
    ) -> Result<CallToolResult, McpError> {
        let scope = input
            .scope
            .as_deref()
            .map(|s| {
                MemoryScope::from_str(s)
                    .map_err(|e| McpError::invalid_params(format!("Invalid scope: {e}"), None))
            })
            .transpose()?;
        self.service
            .consolidate_memories(scope, input.project_id.as_deref())
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Failed to consolidate memories: {}", e), None)
            })?;

        let result = serde_json::json!({ "status": "success" });
        let text = serde_json::to_string_pretty(&result).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn get_memory_stats(&self, _input: EmptyInput) -> Result<CallToolResult, McpError> {
        let stats =
            self.service.get_stats().await.map_err(|e| {
                McpError::internal_error(format!("Failed to get stats: {}", e), None)
            })?;

        let text = serde_json::to_string_pretty(&stats).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn end_session(&self, input: EndSessionInput) -> Result<CallToolResult, McpError> {
        self.service
            .end_session(&input.session_id)
            .await
            .map_err(|e| McpError::internal_error(format!("Failed to end session: {}", e), None))?;

        let result = serde_json::json!({ "status": "success" });
        let text = serde_json::to_string_pretty(&result).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize result: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool definitions and request parsing
// ─────────────────────────────────────────────────────────────────────────────

fn schema_for<T: JsonSchema>() -> Arc<rmcp::model::JsonObject> {
    let schema = schemars::schema_for!(T);
    let mut value = serde_json::to_value(schema).expect("schema should serialize");
    normalize_schema(&mut value);
    match value {
        Value::Object(map) => Arc::new(map),
        _ => Arc::new(rmcp::model::JsonObject::new()),
    }
}

fn build_tool<T: JsonSchema>(name: &'static str, description: &'static str) -> Tool {
    Tool::new(name, description, schema_for::<T>()).with_annotations(
        ToolAnnotations::new()
            .destructive(false)
            .idempotent(false)
            .open_world(false),
    )
}

fn build_tools() -> Vec<Tool> {
    vec![
        build_tool::<AddMemoryInput>(
            "add_memory",
            "Extract and store memories from conversation text using Single-Pass LLM extraction. Automatically deduplicates via ADD-only consolidation.",
        ),
        build_tool::<SearchMemoriesInput>(
            "search_memories",
            "Hybrid semantic+BM25+temporal retrieval of relevant memories. Returns ranked results with score breakdown.",
        ),
        build_tool::<GetMemoriesInput>(
            "get_memories",
            "Retrieve memory records by IDs or list recent memories.",
        ),
        build_tool::<DeleteMemoryInput>(
            "delete_memory",
            "Delete a memory by ID. Use with caution; prefer decay archival for most cases.",
        ),
        build_tool::<ConsolidateMemoriesInput>(
            "consolidate_memories",
            "Trigger batch consolidation: deduplication, decay update, and index compaction.",
        ),
        build_tool::<EmptyInput>(
            "get_memory_stats",
            "Return memory system statistics: total count, category breakdown, index health.",
        ),
        build_tool::<EndSessionInput>(
            "end_session",
            "End a memory session, setting the ended_at timestamp. Call when a user conversation finishes or a session naturally ends.",
        ),
    ]
}

fn parse_tool_args<T: DeserializeOwned>(request: &CallToolRequestParams) -> Result<T, McpError> {
    let value = request
        .arguments
        .clone()
        .map(Value::Object)
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    serde_json::from_value(value).map_err(|error| {
        McpError::invalid_params(
            format!("Invalid arguments for {}: {error}", request.name),
            None,
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// ServerHandler implementation with Schema Normalization
// ─────────────────────────────────────────────────────────────────────────────

impl ServerHandler for MemoryMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        ServerInfo::new(caps).with_server_info(Implementation::new(
            "opencode-memory",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        build_tools()
            .into_iter()
            .find(|tool| tool.name.as_ref() == name)
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(build_tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "add_memory" => self.add_memory(parse_tool_args(&request)?).await,
            "search_memories" => self.search_memories(parse_tool_args(&request)?).await,
            "get_memories" => self.get_memories(parse_tool_args(&request)?).await,
            "delete_memory" => self.delete_memory(parse_tool_args(&request)?).await,
            "consolidate_memories" => self.consolidate_memories(parse_tool_args(&request)?).await,
            "get_memory_stats" => self.get_memory_stats(parse_tool_args(&request)?).await,
            "end_session" => self.end_session(parse_tool_args(&request)?).await,
            _ => Err(McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }
}

// Recursive helper to clean boolean schemas into empty objects
fn normalize_schema(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(properties) = map.get_mut("properties") {
                if let Some(prop_map) = properties.as_object_mut() {
                    for v in prop_map.values_mut() {
                        if v.is_boolean() {
                            *v = serde_json::json!({});
                        } else {
                            normalize_schema(v);
                        }
                    }
                }
            }
            if let Some(items) = map.get_mut("items") {
                if items.is_boolean() {
                    *items = serde_json::json!({});
                } else {
                    normalize_schema(items);
                }
            }
            for def_key in &["definitions", "$defs"] {
                if let Some(definitions) = map.get_mut(*def_key) {
                    if let Some(def_map) = definitions.as_object_mut() {
                        for v in def_map.values_mut() {
                            if v.is_boolean() {
                                *v = serde_json::json!({});
                            } else {
                                normalize_schema(v);
                            }
                        }
                    }
                }
            }
            for (k, v) in map.iter_mut() {
                if k != "properties" && k != "items" && k != "definitions" && k != "$defs" {
                    normalize_schema(v);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                normalize_schema(v);
            }
        }
        _ => {}
    }
}
