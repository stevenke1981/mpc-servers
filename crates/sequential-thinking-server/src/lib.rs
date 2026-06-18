use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ErrorCode, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
        ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    ErrorData as McpError,
};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThoughtData {
    pub thought: String,
    #[serde(rename = "thoughtNumber")]
    pub thought_number: i64,
    #[serde(rename = "totalThoughts")]
    pub total_thoughts: i64,
    #[serde(rename = "nextThoughtNeeded")]
    pub next_thought_needed: bool,
    #[serde(rename = "isRevision", skip_serializing_if = "Option::is_none")]
    pub is_revision: Option<bool>,
    #[serde(rename = "revisesThought", skip_serializing_if = "Option::is_none")]
    pub revises_thought: Option<i64>,
    #[serde(rename = "branchFromThought", skip_serializing_if = "Option::is_none")]
    pub branch_from_thought: Option<i64>,
    #[serde(rename = "branchId", skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(rename = "needsMoreThoughts", skip_serializing_if = "Option::is_none")]
    pub needs_more_thoughts: Option<bool>,
}

// ---------------------------------------------------------------------------
// Boolean coercion helper
// ---------------------------------------------------------------------------

/// Coerce a JSON value into a boolean, supporting string "true"/"false".
fn coerce_bool(val: &serde_json::Value) -> Result<bool, McpError> {
    match val {
        serde_json::Value::Bool(b) => Ok(*b),
        serde_json::Value::String(s) => match s.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(McpError::invalid_params(
                format!("Cannot coerce string '{s}' to boolean"),
                None,
            )),
        },
        _ => Err(McpError::invalid_params(
            format!("Expected boolean or string 'true'/'false', got {}", val),
            None,
        )),
    }
}

/// Optionally coerce a JSON value to bool, returning None if absent.
fn coerce_bool_opt(val: Option<&serde_json::Value>) -> Result<Option<bool>, McpError> {
    match val {
        None => Ok(None),
        Some(v) => coerce_bool(v).map(Some),
    }
}

/// Coerce a JSON value into a positive integer, matching upstream z.coerce.number().int().min(1).
fn coerce_positive_i64(field: &str, val: &serde_json::Value) -> Result<i64, McpError> {
    let number = match val {
        serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| {
            McpError::invalid_params(
                format!("Invalid integer for argument '{field}': {val}"),
                None,
            )
        })?,
        serde_json::Value::String(s) => s.parse::<i64>().map_err(|_| {
            McpError::invalid_params(
                format!("Invalid integer string for argument '{field}': {s}"),
                None,
            )
        })?,
        _ => {
            return Err(McpError::invalid_params(
                format!("Expected integer or integer string for argument '{field}', got {val}"),
                None,
            ));
        }
    };

    if number < 1 {
        return Err(McpError::invalid_params(
            format!("Argument '{field}' must be >= 1"),
            None,
        ));
    }

    Ok(number)
}

fn coerce_positive_i64_opt(
    field: &str,
    val: Option<&serde_json::Value>,
) -> Result<Option<i64>, McpError> {
    match val {
        None => Ok(None),
        Some(v) => coerce_positive_i64(field, v).map(Some),
    }
}

// ---------------------------------------------------------------------------
// Server implementation
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct SequentialThinkingServer {
    thought_history: Mutex<Vec<ThoughtData>>,
    branches: Mutex<HashMap<String, Vec<ThoughtData>>>,
}

impl SequentialThinkingServer {
    pub fn new() -> Self {
        Self {
            thought_history: Mutex::new(Vec::new()),
            branches: Mutex::new(HashMap::new()),
        }
    }

    fn process_thought(&self, input: ThoughtData) -> Result<serde_json::Value, McpError> {
        let mut thought = input;

        // Adjust totalThoughts if thoughtNumber exceeds it
        if thought.thought_number > thought.total_thoughts {
            thought.total_thoughts = thought.thought_number;
        }

        // Push to thought history
        {
            let mut history = self.thought_history.lock().unwrap();
            history.push(thought.clone());
        }

        // Track branches
        if let (Some(branch_from), Some(ref branch_id)) =
            (thought.branch_from_thought, thought.branch_id.clone())
        {
            let mut branches = self.branches.lock().unwrap();
            branches
                .entry(branch_id.clone())
                .or_default()
                .push(thought.clone());
            let _ = branch_from; // used for semantic matching with upstream
        }

        // Build response
        let history_len = self.thought_history.lock().unwrap().len();
        let branch_keys: Vec<String> = {
            let branches = self.branches.lock().unwrap();
            let mut keys: Vec<String> = branches.keys().cloned().collect();
            keys.sort();
            keys
        };

        Ok(serde_json::json!({
            "thoughtNumber": thought.thought_number,
            "totalThoughts": thought.total_thoughts,
            "nextThoughtNeeded": thought.next_thought_needed,
            "branches": branch_keys,
            "thoughtHistoryLength": history_len
        }))
    }
}

impl Default for SequentialThinkingServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for SequentialThinkingServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        ServerInfo::new(caps).with_server_info(Implementation::new(
            "sequential-thinking-server",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        if name == "sequentialthinking" {
            Some(build_sequentialthinking_tool())
        } else {
            None
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![
            build_sequentialthinking_tool(),
        ]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if request.name.as_ref() != "sequentialthinking" {
            return Err(McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            ));
        }

        let args = request
            .arguments
            .as_ref()
            .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;

        let thought = args
            .get("thought")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_params("Missing required argument: 'thought'", None)
            })?;

        let thought_number = args
            .get("thoughtNumber")
            .map(|v| coerce_positive_i64("thoughtNumber", v))
            .transpose()?
            .ok_or_else(|| {
                McpError::invalid_params(
                    "Missing or invalid required argument: 'thoughtNumber'",
                    None,
                )
            })?;

        let total_thoughts = args
            .get("totalThoughts")
            .map(|v| coerce_positive_i64("totalThoughts", v))
            .transpose()?
            .ok_or_else(|| {
                McpError::invalid_params(
                    "Missing or invalid required argument: 'totalThoughts'",
                    None,
                )
            })?;

        let next_thought_needed = {
            let val = args.get("nextThoughtNeeded").ok_or_else(|| {
                McpError::invalid_params("Missing required argument: 'nextThoughtNeeded'", None)
            })?;
            coerce_bool(val)?
        };

        let is_revision = coerce_bool_opt(args.get("isRevision"))?;
        let revises_thought =
            coerce_positive_i64_opt("revisesThought", args.get("revisesThought"))?;
        let branch_from_thought =
            coerce_positive_i64_opt("branchFromThought", args.get("branchFromThought"))?;
        let branch_id = args
            .get("branchId")
            .and_then(|v| v.as_str())
            .map(String::from);
        let needs_more_thoughts = coerce_bool_opt(args.get("needsMoreThoughts"))?;

        let input = ThoughtData {
            thought: thought.to_string(),
            thought_number,
            total_thoughts,
            next_thought_needed,
            is_revision,
            revises_thought,
            branch_from_thought,
            branch_id,
            needs_more_thoughts,
        };

        let result = self.process_thought(input)?;
        Ok(CallToolResult::structured(result))
    }
}

// ---------------------------------------------------------------------------
// Tool schema
// ---------------------------------------------------------------------------

fn build_sequentialthinking_tool() -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "thought": {
                "type": "string",
                "description": "Your current thinking step"
            },
            "nextThoughtNeeded": {
                "oneOf": [
                    { "type": "boolean", "description": "Whether another thought step is needed" },
                    { "type": "string", "description": "Boolean string 'true' or 'false'" }
                ],
                "description": "Whether another thought step is needed"
            },
            "thoughtNumber": {
                "oneOf": [
                    { "type": "integer" },
                    { "type": "string", "description": "Integer string, coerced like upstream z.coerce.number()" }
                ],
                "minimum": 1,
                "description": "Current thought number"
            },
            "totalThoughts": {
                "oneOf": [
                    { "type": "integer" },
                    { "type": "string", "description": "Integer string, coerced like upstream z.coerce.number()" }
                ],
                "minimum": 1,
                "description": "Estimated total thoughts needed"
            },
            "isRevision": {
                "oneOf": [
                    { "type": "boolean" },
                    { "type": "string", "description": "Boolean string 'true' or 'false'" }
                ],
                "description": "Whether this revises previous thinking"
            },
            "revisesThought": {
                "oneOf": [
                    { "type": "integer" },
                    { "type": "string", "description": "Integer string, coerced like upstream z.coerce.number()" }
                ],
                "minimum": 1,
                "description": "Which thought is being reconsidered"
            },
            "branchFromThought": {
                "oneOf": [
                    { "type": "integer" },
                    { "type": "string", "description": "Integer string, coerced like upstream z.coerce.number()" }
                ],
                "minimum": 1,
                "description": "Branching point thought number"
            },
            "branchId": {
                "type": "string",
                "description": "Branch identifier"
            },
            "needsMoreThoughts": {
                "oneOf": [
                    { "type": "boolean" },
                    { "type": "string", "description": "Boolean string 'true' or 'false'" }
                ],
                "description": "If more thoughts are needed"
            }
        },
        "required": ["thought", "thoughtNumber", "totalThoughts", "nextThoughtNeeded"]
    });

    let schema_obj = if let serde_json::Value::Object(obj) = schema {
        Arc::new(obj)
    } else {
        unreachable!()
    };

    Tool::new(
        "sequentialthinking",
        "A detailed tool for dynamic and reflective problem-solving through thoughts.\n\
         This tool helps analyze problems through a flexible thinking process that can adapt and evolve.\n\
         Each thought can build on, question, or revise previous insights as understanding deepens.\n\
         \n\
         When to use this tool:\n\
         - Breaking down complex problems into steps\n\
         - Planning and design with room for revision\n\
         - Analysis that might need course correction\n\
         - Problems where the full scope might not be clear initially\n\
         - Problems that require a multi-step solution\n\
         - Tasks that need to maintain context over multiple steps\n\
         - Situations where irrelevant information needs to be filtered out",
        schema_obj,
    )
    .with_annotations(
        ToolAnnotations::new()
            .read_only(true)
            .destructive(false)
            .idempotent(true)
            .open_world(false),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_server() -> SequentialThinkingServer {
        SequentialThinkingServer::new()
    }

    fn make_thought(
        thought: &str,
        thought_number: i64,
        total_thoughts: i64,
        next_thought_needed: bool,
    ) -> ThoughtData {
        ThoughtData {
            thought: thought.to_string(),
            thought_number,
            total_thoughts,
            next_thought_needed,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
        }
    }

    // -----------------------------------------------------------------------
    // Basic thought tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_accept_valid_basic_thought() {
        let server = make_server();
        let input = make_thought("This is my first thought", 1, 3, true);
        let result = server.process_thought(input).unwrap();

        assert_eq!(result["thoughtNumber"], 1);
        assert_eq!(result["totalThoughts"], 3);
        assert_eq!(result["nextThoughtNeeded"], true);
        assert_eq!(result["thoughtHistoryLength"], 1);
    }

    #[test]
    fn test_accept_thought_with_optional_fields() {
        let server = make_server();
        let input = ThoughtData {
            thought: "Revising my earlier idea".to_string(),
            thought_number: 2,
            total_thoughts: 3,
            next_thought_needed: true,
            is_revision: Some(true),
            revises_thought: Some(1),
            needs_more_thoughts: Some(false),
            branch_from_thought: None,
            branch_id: None,
        };
        let result = server.process_thought(input).unwrap();

        assert_eq!(result["thoughtNumber"], 2);
        assert_eq!(result["thoughtHistoryLength"], 1);
    }

    #[test]
    fn test_track_multiple_thoughts_in_history() {
        let server = make_server();

        let r1 = server
            .process_thought(make_thought("First thought", 1, 3, true))
            .unwrap();
        assert_eq!(r1["thoughtHistoryLength"], 1);

        let r2 = server
            .process_thought(make_thought("Second thought", 2, 3, true))
            .unwrap();
        assert_eq!(r2["thoughtHistoryLength"], 2);

        let r3 = server
            .process_thought(make_thought("Final thought", 3, 3, false))
            .unwrap();
        assert_eq!(r3["thoughtHistoryLength"], 3);
        assert_eq!(r3["nextThoughtNeeded"], false);
    }

    #[test]
    fn test_auto_adjust_total_thoughts() {
        let server = make_server();
        let input = make_thought("Thought 5", 5, 3, true);
        let result = server.process_thought(input).unwrap();

        assert_eq!(result["totalThoughts"], 5);
    }

    // -----------------------------------------------------------------------
    // Revision tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_revision_tracking() {
        let server = make_server();

        server
            .process_thought(make_thought("First thought", 1, 3, true))
            .unwrap();

        let input = ThoughtData {
            thought: "Revised".to_string(),
            thought_number: 2,
            total_thoughts: 3,
            next_thought_needed: true,
            is_revision: Some(true),
            revises_thought: Some(1),
            needs_more_thoughts: None,
            branch_from_thought: None,
            branch_id: None,
        };
        let result = server.process_thought(input).unwrap();
        assert_eq!(result["thoughtHistoryLength"], 2);
    }

    // -----------------------------------------------------------------------
    // Branching tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_branch_tracking() {
        let server = make_server();

        server
            .process_thought(make_thought("Main thought", 1, 3, true))
            .unwrap();

        let input_a = ThoughtData {
            thought: "Branch A thought".to_string(),
            thought_number: 2,
            total_thoughts: 3,
            next_thought_needed: true,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: Some(1),
            branch_id: Some("branch-a".to_string()),
            needs_more_thoughts: None,
        };
        server.process_thought(input_a).unwrap();

        let input_b = ThoughtData {
            thought: "Branch B thought".to_string(),
            thought_number: 2,
            total_thoughts: 3,
            next_thought_needed: false,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: Some(1),
            branch_id: Some("branch-b".to_string()),
            needs_more_thoughts: None,
        };
        let result = server.process_thought(input_b).unwrap();

        let branches = result["branches"].as_array().unwrap();
        let branch_strs: Vec<&str> = branches.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(branch_strs.contains(&"branch-a"));
        assert!(branch_strs.contains(&"branch-b"));
        assert_eq!(branches.len(), 2);
        assert_eq!(result["thoughtHistoryLength"], 3);
    }

    #[test]
    fn test_multiple_thoughts_in_same_branch() {
        let server = make_server();

        let input1 = ThoughtData {
            thought: "Branch thought 1".to_string(),
            thought_number: 1,
            total_thoughts: 2,
            next_thought_needed: true,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: Some(1),
            branch_id: Some("branch-a".to_string()),
            needs_more_thoughts: None,
        };
        server.process_thought(input1).unwrap();

        let input2 = ThoughtData {
            thought: "Branch thought 2".to_string(),
            thought_number: 2,
            total_thoughts: 2,
            next_thought_needed: false,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: Some(1),
            branch_id: Some("branch-a".to_string()),
            needs_more_thoughts: None,
        };
        let result = server.process_thought(input2).unwrap();

        let branches = result["branches"].as_array().unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0], "branch-a");
    }

    // -----------------------------------------------------------------------
    // Boolean string coercion tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_coerce_bool_true() {
        assert!(coerce_bool(&serde_json::Value::Bool(true)).unwrap());
        assert!(coerce_bool(&serde_json::Value::String("true".to_string())).unwrap());
        assert!(coerce_bool(&serde_json::Value::String("TRUE".to_string())).unwrap());
    }

    #[test]
    fn test_coerce_bool_false() {
        assert!(!coerce_bool(&serde_json::Value::Bool(false)).unwrap());
        assert!(!coerce_bool(&serde_json::Value::String("false".to_string())).unwrap());
        assert!(!coerce_bool(&serde_json::Value::String("FALSE".to_string())).unwrap());
    }

    #[test]
    fn test_coerce_bool_invalid() {
        let err = coerce_bool(&serde_json::Value::String("maybe".to_string())).unwrap_err();
        assert!(format!("{err}").contains("Cannot coerce"));
    }

    #[test]
    fn test_coerce_positive_i64_accepts_numbers_and_strings() {
        assert_eq!(
            coerce_positive_i64("thoughtNumber", &serde_json::json!(3)).unwrap(),
            3
        );
        assert_eq!(
            coerce_positive_i64("thoughtNumber", &serde_json::json!("3")).unwrap(),
            3
        );
    }

    #[test]
    fn test_coerce_positive_i64_rejects_zero_and_invalid_strings() {
        let zero = coerce_positive_i64("thoughtNumber", &serde_json::json!(0)).unwrap_err();
        assert!(format!("{zero}").contains("must be >= 1"));

        let invalid = coerce_positive_i64("thoughtNumber", &serde_json::json!("3.5")).unwrap_err();
        assert!(format!("{invalid}").contains("Invalid integer string"));
    }

    // -----------------------------------------------------------------------
    // Tool name test
    // -----------------------------------------------------------------------

    #[test]
    fn test_tool_name() {
        let tool = build_sequentialthinking_tool();
        assert_eq!(tool.name.as_ref(), "sequentialthinking");
    }

    #[test]
    fn test_get_tool() {
        let server = make_server();
        let tool = server.get_tool("sequentialthinking");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name.as_ref(), "sequentialthinking");

        let missing = server.get_tool("nonexistent");
        assert!(missing.is_none());
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_handle_very_long_thought() {
        let server = make_server();
        let input = make_thought(&"a".repeat(10000), 1, 1, false);
        let result = server.process_thought(input).unwrap();
        assert_eq!(result["thoughtNumber"], 1);
    }

    #[test]
    fn test_thought_number_one_total_one() {
        let server = make_server();
        let input = make_thought("Only thought", 1, 1, false);
        let result = server.process_thought(input).unwrap();
        assert_eq!(result["thoughtNumber"], 1);
        assert_eq!(result["totalThoughts"], 1);
    }

    // -----------------------------------------------------------------------
    // No boolean JSON Schema nodes test
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_boolean_json_schema_nodes() {
        let tool = build_sequentialthinking_tool();
        let schema_val = serde_json::Value::Object((*tool.input_schema).clone());
        if let Some(props) = schema_val["properties"].as_object() {
            for (key, val) in props {
                assert!(
                    !val.is_boolean(),
                    "property '{key}' in tool '{}' is a bare boolean: {val}",
                    tool.name
                );
            }
        }
    }
}
