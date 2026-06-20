use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_search_limit() -> usize {
    20
}

fn default_trace_depth() -> usize {
    3
}

fn default_trace_direction() -> String {
    "both".to_string()
}

fn default_search_code_mode() -> String {
    "compact".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectArgs {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IndexRepositoryArgs {
    pub repo_path: String,
    pub project: Option<String>,
    pub mode: Option<String>,
    pub persistence: Option<bool>,
    pub incremental: Option<bool>,
    pub target_projects: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchGraphArgs {
    pub project: String,
    pub query: Option<String>,
    pub semantic_query: Option<String>,
    pub vector_query: Option<String>,
    pub label: Option<String>,
    pub name_pattern: Option<String>,
    pub qn_pattern: Option<String>,
    pub file_pattern: Option<String>,
    pub relationship: Option<String>,
    pub direction: Option<String>,
    pub min_degree: Option<usize>,
    pub max_degree: Option<usize>,
    pub include_connected: Option<bool>,
    pub exclude_entry_points: Option<bool>,
    #[serde(default = "default_search_limit")]
    #[schemars(default = "default_search_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TraceArgs {
    pub project: String,
    pub function_name: String,
    #[serde(default = "default_trace_direction")]
    #[schemars(default = "default_trace_direction")]
    pub direction: String,
    #[serde(default = "default_trace_depth")]
    #[schemars(default = "default_trace_depth")]
    pub depth: usize,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnippetArgs {
    pub project: String,
    pub qualified_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchCodeArgs {
    pub project: String,
    pub pattern: String,
    #[serde(default = "default_search_limit")]
    #[schemars(default = "default_search_limit")]
    pub limit: usize,
    #[serde(default = "default_search_code_mode")]
    #[schemars(default = "default_search_code_mode")]
    pub mode: String,
    pub file_pattern: Option<String>,
    pub path_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryGraphArgs {
    pub project: String,
    pub query: String,
    pub max_rows: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManageAdrArgs {
    pub project: String,
    pub mode: Option<String>,
    pub content: Option<String>,
    pub sections: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IngestTracesArgs {
    pub project: String,
    pub traces: Vec<Value>,
}
