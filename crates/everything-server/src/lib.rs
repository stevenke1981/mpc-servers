use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use flate2::{write::GzEncoder, Compression};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        AnnotateAble, CallToolRequestParams, CallToolResult, Content, CreateMessageRequestParams,
        ErrorCode, GetPromptRequestParams, GetPromptResult, Implementation, JsonObject,
        ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListRootsResult,
        ListToolsResult, LoggingLevel, LoggingMessageNotificationParam, PaginatedRequestParams,
        ProgressNotificationParam, Prompt, PromptArgument, PromptMessage, PromptMessageRole,
        PromptsCapability, RawResource, RawResourceTemplate, ReadResourceRequestParams,
        ReadResourceResult, ResourceContents, ResourceUpdatedNotificationParam,
        ResourcesCapability, Root, SamplingMessage, ServerCapabilities, ServerInfo,
        SetLevelRequestParams, SubscribeRequestParams, Tool, ToolAnnotations, ToolsCapability,
        UnsubscribeRequestParams,
    },
    service::{NotificationContext, Peer, RequestContext, RoleServer},
    ErrorData as McpError,
};
use serde_json::{json, Value};
use url::Url;

pub const TOOL_NAMES: &[&str] = &[
    "echo",
    "get-annotated-message",
    "get-env",
    "get-resource-links",
    "get-resource-reference",
    "get-roots-list",
    "get-structured-content",
    "get-sum",
    "get-tiny-image",
    "gzip-file-as-resource",
    "toggle-simulated-logging",
    "toggle-subscriber-updates",
    "trigger-elicitation-request",
    "trigger-elicitation-request-async",
    "trigger-long-running-operation",
    "trigger-sampling-request",
    "trigger-sampling-request-async",
    "trigger-url-elicitation",
    "simulate-research-query",
];

pub const PROMPT_NAMES: &[&str] = &[
    "simple-prompt",
    "args-prompt",
    "completable-prompt",
    "resource-prompt",
];

const MCP_TINY_IMAGE: &str = "iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAIklEQVR4AWP4z8Dwn4GKgImaho0aNmjYoGGDho0bAADjKQQcS2w3VAAAAABJRU5ErkJggg==";
const TEXT_URI_BASE: &str = "demo://resource/dynamic/text";
const BLOB_URI_BASE: &str = "demo://resource/dynamic/blob";
const SESSION_URI_BASE: &str = "demo://resource/session";
const DEFAULT_GZIP_MAX_FETCH_SIZE: usize = 10 * 1024 * 1024;
const DEFAULT_GZIP_MAX_FETCH_TIME_MILLIS: u64 = 30_000;
const CLIENT_REQUEST_TIMEOUT_MILLIS: u64 = 800;

#[derive(Debug, Clone)]
struct SessionResource {
    name: String,
    mime_type: String,
    blob: String,
}

#[derive(Debug)]
pub struct EverythingServer {
    session_resources: Arc<Mutex<HashMap<String, SessionResource>>>,
    simulated_logging: Arc<Mutex<bool>>,
    subscriber_updates: Arc<Mutex<bool>>,
    subscribed_resources: Arc<Mutex<HashSet<String>>>,
    current_logging_level: Arc<Mutex<LoggingLevel>>,
    cached_roots: Arc<Mutex<Vec<Root>>>,
}

impl Default for EverythingServer {
    fn default() -> Self {
        Self {
            session_resources: Arc::default(),
            simulated_logging: Arc::default(),
            subscriber_updates: Arc::default(),
            subscribed_resources: Arc::default(),
            current_logging_level: Arc::new(Mutex::new(LoggingLevel::Info)),
            cached_roots: Arc::default(),
        }
    }
}

impl EverythingServer {
    pub fn tools(&self) -> Vec<Tool> {
        TOOL_NAMES.iter().map(|name| build_tool(name)).collect()
    }

    pub fn prompts(&self) -> Vec<Prompt> {
        build_prompts()
    }

    pub fn resources(&self) -> Vec<rmcp::model::Resource> {
        let mut resources = static_resources();
        let session = self.session_resources.lock().expect("session lock");
        resources.extend(session.iter().map(|(uri, resource)| {
            RawResource::new(uri, &resource.name)
                .with_description("Session-scoped resource created by gzip-file-as-resource")
                .with_mime_type(resource.mime_type.clone())
                .no_annotation()
        }));
        resources
    }
}

fn empty_obj() -> JsonObject {
    JsonObject::new()
}

fn schema(properties: Value, required: &[&str]) -> Arc<JsonObject> {
    let value = json!({
        "type": "object",
        "properties": properties,
        "required": required,
    });
    let Value::Object(obj) = value else {
        unreachable!()
    };
    Arc::new(obj)
}

fn empty_schema() -> Arc<JsonObject> {
    schema(json!({}), &[])
}

fn build_tool(name: &str) -> Tool {
    let (description, input_schema, open_world) = match name {
        "echo" => (
            "Echoes back the input string",
            schema(
                json!({
                    "message": { "type": "string", "description": "Message to echo" }
                }),
                &["message"],
            ),
            false,
        ),
        "get-annotated-message" => (
            "Demonstrates annotated content patterns.",
            schema(
                json!({
                    "messageType": {
                        "type": "string",
                        "enum": ["error", "success", "debug"],
                        "description": "Type of message to demonstrate different annotation patterns"
                    },
                    "includeImage": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to include an example image"
                    }
                }),
                &["messageType"],
            ),
            false,
        ),
        "get-env" => (
            "Returns environment variables for MCP configuration debugging.",
            empty_schema(),
            false,
        ),
        "get-resource-links" => (
            "Returns up to ten resource links that reference different resource types.",
            schema(
                json!({
                    "count": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 10,
                        "default": 3,
                        "description": "Number of resource links to return"
                    }
                }),
                &[],
            ),
            false,
        ),
        "get-resource-reference" => (
            "Returns an embedded resource reference that can be used by MCP clients.",
            schema(
                json!({
                    "resourceType": {
                        "type": "string",
                        "enum": ["Text", "Blob"],
                        "default": "Text"
                    },
                    "resourceId": {
                        "type": "integer",
                        "minimum": 1,
                        "default": 1
                    }
                }),
                &[],
            ),
            false,
        ),
        "get-roots-list" => (
            "Lists MCP roots known to the server.",
            empty_schema(),
            false,
        ),
        "get-structured-content" => (
            "Returns structured weather content with a text fallback.",
            schema(
                json!({
                    "location": {
                        "type": "string",
                        "enum": ["New York", "Chicago", "Los Angeles"],
                        "description": "Choose city"
                    }
                }),
                &["location"],
            ),
            false,
        ),
        "get-sum" => (
            "Returns the sum of two numbers.",
            schema(
                json!({
                    "a": { "type": "number", "description": "First number" },
                    "b": { "type": "number", "description": "Second number" }
                }),
                &["a", "b"],
            ),
            false,
        ),
        "get-tiny-image" => ("Returns a tiny MCP logo image.", empty_schema(), false),
        "gzip-file-as-resource" => (
            "Compresses a single URL or data URI using gzip and exposes it as a session resource.",
            schema(
                json!({
                    "name": {
                        "type": "string",
                        "default": "README.md.gz",
                        "description": "Name of the output file"
                    },
                    "data": {
                        "type": "string",
                        "default": "data:text/plain,Hello%20from%20everything-server",
                        "description": "HTTP, HTTPS, or data URI content to compress"
                    },
                    "outputType": {
                        "type": "string",
                        "enum": ["resourceLink", "resource"],
                        "default": "resourceLink"
                    }
                }),
                &[],
            ),
            true,
        ),
        "trigger-long-running-operation" => (
            "Demonstrates a bounded long-running operation.",
            schema(
                json!({
                    "duration": { "type": "number", "default": 10, "minimum": 0, "maximum": 30 },
                    "steps": { "type": "integer", "default": 5, "minimum": 1, "maximum": 30 }
                }),
                &[],
            ),
            false,
        ),
        "toggle-simulated-logging" | "toggle-subscriber-updates" => (
            "Toggles a simulated compatibility-test feature.",
            empty_schema(),
            false,
        ),
        "trigger-elicitation-request"
        | "trigger-elicitation-request-async"
        | "trigger-sampling-request"
        | "trigger-sampling-request-async"
        | "trigger-url-elicitation"
        | "simulate-research-query" => (
            "Compatibility placeholder for client-assisted MCP protocol behavior.",
            empty_schema(),
            false,
        ),
        _ => ("Unknown tool", empty_schema(), false),
    };

    Tool::new(name.to_string(), description, input_schema).with_annotations(
        ToolAnnotations::new()
            .read_only(!matches!(name, "gzip-file-as-resource"))
            .destructive(false)
            .idempotent(true)
            .open_world(open_world),
    )
}

fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn get_args(params: &CallToolRequestParams) -> Result<&JsonObject, McpError> {
    params
        .arguments
        .as_ref()
        .ok_or_else(|| McpError::invalid_params("Missing arguments", None))
}

fn arg_str<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, McpError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        McpError::invalid_params(format!("Missing required argument: '{key}'"), None)
    })
}

fn arg_str_default<'a>(args: &'a JsonObject, key: &str, default: &'a str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn arg_f64(args: &JsonObject, key: &str) -> Result<f64, McpError> {
    args.get(key).and_then(Value::as_f64).ok_or_else(|| {
        McpError::invalid_params(format!("Missing required numeric argument: '{key}'"), None)
    })
}

fn arg_usize_default(
    args: &JsonObject,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, McpError> {
    let value = args
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(default as u64);
    let value = value as usize;
    if value < min || value > max {
        return Err(McpError::invalid_params(
            format!("'{key}' must be between {min} and {max}"),
            None,
        ));
    }
    Ok(value)
}

fn dynamic_text_uri(resource_id: u64) -> String {
    format!("{TEXT_URI_BASE}/{resource_id}")
}

fn dynamic_blob_uri(resource_id: u64) -> String {
    format!("{BLOB_URI_BASE}/{resource_id}")
}

fn dynamic_text_resource(resource_id: u64) -> ResourceContents {
    ResourceContents::TextResourceContents {
        uri: dynamic_text_uri(resource_id),
        mime_type: Some("text/plain".to_string()),
        text: format!("Resource {resource_id}: This is a plaintext resource created by the Rust everything server"),
        meta: None,
    }
}

fn dynamic_blob_resource(resource_id: u64) -> ResourceContents {
    let text = format!(
        "Resource {resource_id}: This is a base64 blob created by the Rust everything server"
    );
    ResourceContents::BlobResourceContents {
        uri: dynamic_blob_uri(resource_id),
        mime_type: Some("text/plain".to_string()),
        blob: BASE64_STANDARD.encode(text.as_bytes()),
        meta: None,
    }
}

fn resource_link(resource_id: u64, text: bool) -> RawResource {
    let uri = if text {
        dynamic_text_uri(resource_id)
    } else {
        dynamic_blob_uri(resource_id)
    };
    RawResource::new(
        uri,
        format!(
            "{} Resource {resource_id}",
            if text { "Text" } else { "Blob" }
        ),
    )
    .with_description(format!(
        "Resource {resource_id}: {} resource",
        if text { "plaintext" } else { "binary blob" }
    ))
    .with_mime_type(if text {
        "text/plain"
    } else {
        "application/octet-stream"
    })
}

fn parse_dynamic_resource_id(uri: &str, base: &str) -> Option<u64> {
    uri.strip_prefix(&format!("{base}/"))
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
}

fn static_documents() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "instructions.md",
            "text/markdown",
            "# Everything Reference Server\n\nRust compatibility testbed for MCP tools, prompts, resources, roots, logging, sampling, and elicitation.",
        ),
        (
            "features.md",
            "text/markdown",
            "# Features\n\nTools, prompts, resource templates, static resources, dynamic text/blob resources, and documented protocol feature fallbacks.",
        ),
        (
            "startup.md",
            "text/markdown",
            "# Startup\n\nRun `everything-server` over stdio. Use `--version`, `-V`, or `version` before stdio starts.",
        ),
    ]
}

fn static_resources() -> Vec<rmcp::model::Resource> {
    static_documents()
        .into_iter()
        .map(|(name, mime, _)| {
            RawResource::new(
                format!("demo://resource/static/document/{name}"),
                name.to_string(),
            )
            .with_description(format!("Static document file exposed from /docs: {name}"))
            .with_mime_type(mime)
            .no_annotation()
        })
        .collect()
}

fn resource_templates() -> Vec<rmcp::model::ResourceTemplate> {
    vec![
        RawResourceTemplate::new(
            "demo://resource/dynamic/text/{resourceId}",
            "Dynamic Text Resource",
        )
        .with_description(
            "Plaintext dynamic resource fabricated from a positive integer resourceId.",
        )
        .with_mime_type("text/plain")
        .no_annotation(),
        RawResourceTemplate::new(
            "demo://resource/dynamic/blob/{resourceId}",
            "Dynamic Blob Resource",
        )
        .with_description("Base64 dynamic resource fabricated from a positive integer resourceId.")
        .with_mime_type("application/octet-stream")
        .no_annotation(),
    ]
}

fn build_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new("simple-prompt", Some("A prompt with no arguments"), None)
            .with_title("Simple Prompt"),
        Prompt::new(
            "args-prompt",
            Some("A prompt with two arguments, one required and one optional"),
            Some(vec![
                PromptArgument::new("city")
                    .with_description("Name of the city")
                    .with_required(true),
                PromptArgument::new("state")
                    .with_description("Name of the state")
                    .with_required(false),
            ]),
        )
        .with_title("Arguments Prompt"),
        Prompt::new(
            "completable-prompt",
            Some("First argument choice narrows values for second argument."),
            Some(vec![
                PromptArgument::new("department")
                    .with_description("Choose the department.")
                    .with_required(true),
                PromptArgument::new("name")
                    .with_description("Choose a team member to lead the selected department.")
                    .with_required(true),
            ]),
        )
        .with_title("Team Management"),
        Prompt::new(
            "resource-prompt",
            Some("A prompt that includes an embedded resource reference"),
            Some(vec![
                PromptArgument::new("resourceType")
                    .with_description("Type of resource to fetch: Text or Blob")
                    .with_required(true),
                PromptArgument::new("resourceId")
                    .with_description("ID of the resource to fetch")
                    .with_required(true),
            ]),
        )
        .with_title("Resource Prompt"),
    ]
}

fn prompt_arg<'a>(arguments: Option<&'a JsonObject>, key: &str) -> Option<&'a str> {
    arguments
        .and_then(|args| args.get(key))
        .and_then(Value::as_str)
}

fn prompt_arg_required<'a>(
    arguments: Option<&'a JsonObject>,
    key: &str,
) -> Result<&'a str, McpError> {
    prompt_arg(arguments, key).ok_or_else(|| {
        McpError::invalid_params(format!("Missing required prompt argument: '{key}'"), None)
    })
}

fn build_prompt_messages(
    name: &str,
    arguments: Option<&JsonObject>,
) -> Result<Vec<PromptMessage>, McpError> {
    match name {
        "simple-prompt" => Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            "This is a simple prompt without arguments.",
        )]),
        "args-prompt" => {
            let city = prompt_arg_required(arguments, "city")?;
            let state = prompt_arg(arguments, "state");
            let location = state
                .map(|state| format!("{city}, {state}"))
                .unwrap_or_else(|| city.to_string());
            Ok(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                format!("What's weather in {location}?"),
            )])
        }
        "completable-prompt" => {
            let department = prompt_arg_required(arguments, "department")?;
            let name = prompt_arg_required(arguments, "name")?;
            Ok(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                format!("Please promote {name} to the head of the {department} team."),
            )])
        }
        "resource-prompt" => {
            let resource_type = prompt_arg_required(arguments, "resourceType")?;
            let resource_id = prompt_arg_required(arguments, "resourceId")?
                .parse::<u64>()
                .map_err(|_| {
                    McpError::invalid_params("resourceId must be a finite positive integer", None)
                })?;
            let resource = match resource_type {
                "Text" => dynamic_text_resource(resource_id),
                "Blob" => dynamic_blob_resource(resource_id),
                _ => {
                    return Err(McpError::invalid_params(
                        "resourceType must be Text or Blob",
                        None,
                    ));
                }
            };
            Ok(vec![
                PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!("This prompt includes the {resource_type} resource with id: {resource_id}. Please analyze the following resource:"),
                ),
                PromptMessage::new_resource(
                    PromptMessageRole::User,
                    match &resource {
                        ResourceContents::TextResourceContents { uri, .. }
                        | ResourceContents::BlobResourceContents { uri, .. } => uri.clone(),
                    },
                    Some("text/plain".to_string()),
                    match resource {
                        ResourceContents::TextResourceContents { text, .. } => Some(text),
                        ResourceContents::BlobResourceContents { .. } => None,
                    },
                    None,
                    None,
                    None,
                ),
            ])
        }
        _ => Err(McpError::new(
            ErrorCode::METHOD_NOT_FOUND,
            format!("Unknown prompt: {name}"),
            None,
        )),
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn allowed_gzip_domains() -> Vec<String> {
    std::env::var("GZIP_ALLOWED_DOMAINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
        .map(|domain| domain.to_ascii_lowercase())
        .collect()
}

fn validate_gzip_url(data: &str) -> Result<Url, McpError> {
    let url = Url::parse(data)
        .map_err(|e| McpError::invalid_params(format!("Invalid URL for gzip input: {e}"), None))?;
    match url.scheme() {
        "http" | "https" => {
            let allowed = allowed_gzip_domains();
            if !allowed.is_empty() {
                let host = url
                    .host_str()
                    .ok_or_else(|| McpError::invalid_params("URL is missing a host", None))?
                    .to_ascii_lowercase();
                let matched = allowed
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
                if !matched {
                    return Err(McpError::invalid_params(
                        format!("Domain {host} is not in GZIP_ALLOWED_DOMAINS"),
                        None,
                    ));
                }
            }
        }
        "data" => {}
        other => {
            return Err(McpError::invalid_params(
                format!(
                    "Unsupported URL protocol for gzip input: {other}. Only http, https, and data URLs are supported."
                ),
                None,
            ));
        }
    }
    Ok(url)
}

fn percent_decode_bytes(input: &str) -> Result<Vec<u8>, McpError> {
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(McpError::invalid_params(
                    "Invalid percent escape in data URI",
                    None,
                ));
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).map_err(|_| {
                McpError::invalid_params("Invalid percent escape in data URI", None)
            })?;
            let decoded = u8::from_str_radix(hex, 16).map_err(|_| {
                McpError::invalid_params("Invalid percent escape in data URI", None)
            })?;
            output.push(decoded);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    Ok(output)
}

fn data_uri_to_bytes(data: &str, max_bytes: usize) -> Result<Vec<u8>, McpError> {
    let payload = data
        .split_once(',')
        .map(|(_, payload)| payload)
        .ok_or_else(|| McpError::invalid_params("Invalid data URI payload", None))?;
    let header = &data[..data.find(',').unwrap_or(data.len())];
    let bytes = if header.ends_with(";base64") {
        BASE64_STANDARD
            .decode(payload)
            .map_err(|e| McpError::invalid_params(format!("Invalid base64 data URI: {e}"), None))
    } else {
        percent_decode_bytes(payload)
    }?;
    if bytes.len() > max_bytes {
        return Err(McpError::invalid_params(
            format!("Input exceeds GZIP_MAX_FETCH_SIZE of {max_bytes} bytes"),
            None,
        ));
    }
    Ok(bytes)
}

async fn fetch_http_bounded(
    url: Url,
    max_bytes: usize,
    timeout_millis: u64,
) -> Result<Vec<u8>, McpError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_millis))
        .build()
        .map_err(|e| McpError::invalid_params(format!("Failed to build HTTP client: {e}"), None))?;
    let mut response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| McpError::invalid_params(format!("Failed to fetch {url}: {e}"), None))?;

    if let Some(content_length) = response.content_length() {
        if content_length > max_bytes as u64 {
            return Err(McpError::invalid_params(
                format!("Content-Length for {url} exceeds max of {max_bytes}: {content_length}"),
                None,
            ));
        }
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| McpError::invalid_params(format!("Failed reading {url}: {e}"), None))?
    {
        if body.len() + chunk.len() > max_bytes {
            return Err(McpError::invalid_params(
                format!("Response from {url} exceeds {max_bytes} bytes"),
                None,
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn load_gzip_input_bytes(data: &str) -> Result<Vec<u8>, McpError> {
    let max_bytes = env_usize("GZIP_MAX_FETCH_SIZE", DEFAULT_GZIP_MAX_FETCH_SIZE);
    let timeout_millis = env_u64(
        "GZIP_MAX_FETCH_TIME_MILLIS",
        DEFAULT_GZIP_MAX_FETCH_TIME_MILLIS,
    );
    let url = validate_gzip_url(data)?;
    match url.scheme() {
        "data" => data_uri_to_bytes(data, max_bytes),
        "http" | "https" => fetch_http_bounded(url, max_bytes, timeout_millis).await,
        _ => unreachable!("validate_gzip_url allows only data/http/https"),
    }
}

fn gzip_bytes(input: &[u8]) -> Result<Vec<u8>, McpError> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(input)
        .map_err(|e| McpError::invalid_params(format!("Failed to gzip input: {e}"), None))?;
    encoder
        .finish()
        .map_err(|e| McpError::invalid_params(format!("Failed to finish gzip stream: {e}"), None))
}

fn client_supports_roots(peer: &Peer<RoleServer>) -> bool {
    peer.peer_info()
        .and_then(|info| info.capabilities.roots.as_ref())
        .is_some()
}

fn client_supports_sampling(peer: &Peer<RoleServer>) -> bool {
    peer.peer_info()
        .and_then(|info| info.capabilities.sampling.as_ref())
        .is_some()
}

async fn request_client_roots(peer: &Peer<RoleServer>) -> Result<ListRootsResult, String> {
    tokio::time::timeout(
        Duration::from_millis(CLIENT_REQUEST_TIMEOUT_MILLIS),
        peer.list_roots(),
    )
    .await
    .map_err(|_| "roots/list request timed out".to_string())?
    .map_err(|e| format!("roots/list request failed: {e}"))
}

fn format_roots(roots: &[Root]) -> String {
    if roots.is_empty() {
        return "Client roots list is empty.".to_string();
    }
    let value = serde_json::to_value(roots).unwrap_or_else(|_| json!([]));
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "[]".to_string())
}

async fn request_sampling_message(peer: &Peer<RoleServer>) -> Result<String, String> {
    let params = CreateMessageRequestParams::new(
        vec![SamplingMessage::user_text(
            "Reply with a short sentence confirming sampling support.",
        )],
        100,
    )
    .with_system_prompt("You are responding to an MCP everything-server compatibility test.");
    let result = tokio::time::timeout(
        Duration::from_millis(CLIENT_REQUEST_TIMEOUT_MILLIS),
        peer.create_message(params),
    )
    .await
    .map_err(|_| "sampling/createMessage request timed out".to_string())?
    .map_err(|e| format!("sampling/createMessage request failed: {e}"))?;
    serde_json::to_string_pretty(&result)
        .map_err(|e| format!("sampling/createMessage result serialization failed: {e}"))
}

async fn notify_progress_step(
    peer: &Peer<RoleServer>,
    token: rmcp::model::ProgressToken,
    progress: f64,
    total: f64,
    message: impl Into<String>,
) -> Result<(), String> {
    peer.notify_progress(
        ProgressNotificationParam::new(token, progress)
            .with_total(total)
            .with_message(message),
    )
    .await
    .map_err(|e| format!("progress notification failed: {e}"))
}

async fn notify_log_message(
    peer: &Peer<RoleServer>,
    level: LoggingLevel,
    data: Value,
) -> Result<(), String> {
    peer.notify_logging_message(
        LoggingMessageNotificationParam::new(level, data).with_logger("everything-server"),
    )
    .await
    .map_err(|e| format!("logging notification failed: {e}"))
}

async fn notify_resource_updated(peer: &Peer<RoleServer>, uri: String) -> Result<(), String> {
    peer.notify_resource_updated(ResourceUpdatedNotificationParam::new(uri))
        .await
        .map_err(|e| format!("resource update notification failed: {e}"))
}

impl ServerHandler for EverythingServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability {
            list_changed: Some(true),
        });
        caps.prompts = Some(PromptsCapability {
            list_changed: Some(true),
        });
        caps.resources = Some(ResourcesCapability {
            subscribe: Some(true),
            list_changed: Some(true),
        });
        caps.logging = Some(empty_obj());
        ServerInfo::new(caps).with_server_info(Implementation::new(
            "mcp-servers/everything",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        TOOL_NAMES.contains(&name).then(|| build_tool(name))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(self.tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "echo" => {
                let args = get_args(&request)?;
                Ok(text_result(format!("Echo: {}", arg_str(args, "message")?)))
            }
            "get-annotated-message" => {
                let args = get_args(&request)?;
                let message_type = arg_str(args, "messageType")?;
                let include_image = args
                    .get("includeImage")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let text = match message_type {
                    "error" => "Error: Operation failed",
                    "success" => "Operation completed successfully",
                    "debug" => "Debug: Cache hit ratio 0.95, latency 150ms",
                    _ => {
                        return Err(McpError::invalid_params(
                            "messageType must be error, success, or debug",
                            None,
                        ));
                    }
                };
                let mut content = vec![Content::text(text)];
                if include_image {
                    content.push(Content::image(MCP_TINY_IMAGE, "image/png"));
                }
                Ok(CallToolResult::success(content))
            }
            "get-env" => {
                let env: serde_json::Map<String, Value> = std::env::vars()
                    .map(|(key, value)| (key, Value::String(value)))
                    .collect();
                Ok(text_result(
                    serde_json::to_string_pretty(&Value::Object(env)).unwrap_or_default(),
                ))
            }
            "get-resource-links" => {
                let empty = JsonObject::new();
                let args = request.arguments.as_ref().unwrap_or(&empty);
                let count = arg_usize_default(args, "count", 3, 1, 10)?;
                let mut content = vec![Content::text(format!(
                    "Here are {count} resource links to resources available in this server:"
                ))];
                for id in 1..=count as u64 {
                    let is_text = id % 2 == 0;
                    content.push(Content::resource_link(resource_link(id, is_text)));
                }
                Ok(CallToolResult::success(content))
            }
            "get-resource-reference" => {
                let empty = JsonObject::new();
                let args = request.arguments.as_ref().unwrap_or(&empty);
                let resource_type = arg_str_default(args, "resourceType", "Text");
                let resource_id = arg_usize_default(args, "resourceId", 1, 1, usize::MAX)? as u64;
                let resource = match resource_type {
                    "Text" => dynamic_text_resource(resource_id),
                    "Blob" => dynamic_blob_resource(resource_id),
                    _ => {
                        return Err(McpError::invalid_params(
                            "resourceType must be Text or Blob",
                            None,
                        ));
                    }
                };
                Ok(CallToolResult::success(vec![
                    Content::text(format!("Returning resource reference for Resource {resource_id}:")),
                    Content::resource(resource),
                    Content::text(format!(
                        "You can access this resource using the URI: {}",
                        if resource_type == "Text" {
                            dynamic_text_uri(resource_id)
                        } else {
                            dynamic_blob_uri(resource_id)
                        }
                    )),
                ]))
            }
            "get-roots-list" => {
                if client_supports_roots(&context.peer) {
                    match request_client_roots(&context.peer).await {
                        Ok(result) => {
                            *self.cached_roots.lock().expect("roots lock") = result.roots.clone();
                            Ok(text_result(format!(
                                "Client roots synchronized:\n{}",
                                format_roots(&result.roots)
                            )))
                        }
                        Err(error) => {
                            let cached = self.cached_roots.lock().expect("roots lock").clone();
                            Ok(text_result(format!(
                                "Client advertises roots, but active synchronization failed: {error}\nCached roots:\n{}",
                                format_roots(&cached)
                            )))
                        }
                    }
                } else {
                    let cached = self.cached_roots.lock().expect("roots lock").clone();
                    Ok(text_result(format!(
                        "Client did not advertise roots capability. Cached roots:\n{}",
                        format_roots(&cached)
                    )))
                }
            }
            "get-structured-content" => {
                let args = get_args(&request)?;
                let location = arg_str(args, "location")?;
                let weather = match location {
                    "New York" => json!({"temperature":33,"conditions":"Cloudy","humidity":82}),
                    "Chicago" => {
                        json!({"temperature":36,"conditions":"Light rain / drizzle","humidity":82})
                    }
                    "Los Angeles" => {
                        json!({"temperature":73,"conditions":"Sunny / Clear","humidity":48})
                    }
                    _ => {
                        return Err(McpError::invalid_params(
                            "location must be New York, Chicago, or Los Angeles",
                            None,
                        ));
                    }
                };
                Ok(CallToolResult::structured(weather))
            }
            "get-sum" => {
                let args = get_args(&request)?;
                let a = arg_f64(args, "a")?;
                let b = arg_f64(args, "b")?;
                let sum = a + b;
                Ok(text_result(format!("The sum of {a} and {b} is {sum}.")))
            }
            "get-tiny-image" => Ok(CallToolResult::success(vec![
                Content::text("Here's the image you requested:"),
                Content::image(MCP_TINY_IMAGE, "image/png"),
                Content::text("The image above is the MCP logo."),
            ])),
            "gzip-file-as-resource" => {
                let empty = JsonObject::new();
                let args = request.arguments.as_ref().unwrap_or(&empty);
                let name = arg_str_default(args, "name", "README.md.gz");
                let data = arg_str_default(args, "data", "data:text/plain,Hello%20from%20everything-server");
                let output_type = arg_str_default(args, "outputType", "resourceLink");
                let bytes = load_gzip_input_bytes(data).await?;
                let uri = format!("{SESSION_URI_BASE}/{name}");
                let compressed = gzip_bytes(&bytes)?;
                let blob = BASE64_STANDARD.encode(compressed);
                let resource = SessionResource {
                    name: name.to_string(),
                    mime_type: "application/gzip".to_string(),
                    blob: blob.clone(),
                };
                self.session_resources
                    .lock()
                    .expect("session lock")
                    .insert(uri.clone(), resource.clone());
                if output_type == "resource" {
                    Ok(CallToolResult::success(vec![Content::resource(
                        ResourceContents::BlobResourceContents {
                            uri,
                            mime_type: Some(resource.mime_type),
                            blob,
                            meta: None,
                        },
                    )]))
                } else if output_type == "resourceLink" {
                    Ok(CallToolResult::success(vec![Content::resource_link(
                        RawResource::new(uri, name)
                            .with_mime_type("application/gzip")
                            .with_description("Session resource created by gzip-file-as-resource"),
                    )]))
                } else {
                    Err(McpError::invalid_params(
                        "outputType must be resourceLink or resource",
                        None,
                    ))
                }
            }
            "trigger-long-running-operation" => {
                let empty = JsonObject::new();
                let args = request.arguments.as_ref().unwrap_or(&empty);
                let duration = args.get("duration").and_then(Value::as_f64).unwrap_or(10.0);
                let steps = arg_usize_default(args, "steps", 5, 1, 30)?;
                let bounded = duration.clamp(0.0, 30.0);
                let progress_token = context.meta.get_progress_token();
                let mut sent_progress = 0_usize;
                for step in 1..=steps {
                    if let Some(token) = progress_token.clone() {
                        if notify_progress_step(
                            &context.peer,
                            token,
                            step as f64,
                            steps as f64,
                            format!("Completed step {step} of {steps}"),
                        )
                        .await
                        .is_ok()
                        {
                            sent_progress += 1;
                        }
                    }
                    if bounded > 0.0 {
                        let millis = ((bounded * 100.0) / steps as f64).max(1.0) as u64;
                        tokio::time::sleep(Duration::from_millis(millis)).await;
                    }
                    if context.ct.is_cancelled() {
                        return Ok(text_result(format!(
                            "Long running operation cancelled after step {step} of {steps}."
                        )));
                    }
                }
                Ok(text_result(format!(
                    "Long running operation completed. Duration: {duration} seconds, Steps: {steps}, Progress notifications sent: {sent_progress}."
                )))
            }
            "toggle-simulated-logging" => {
                let enabled = {
                    let mut flag = self.simulated_logging.lock().expect("logging lock");
                    *flag = !*flag;
                    *flag
                };
                let level = *self
                    .current_logging_level
                    .lock()
                    .expect("logging level lock");
                let notify_status = if enabled {
                    match notify_log_message(
                        &context.peer,
                        level,
                        json!({
                            "event": "simulated_logging_enabled",
                            "message": "everything-server simulated logging toggled on"
                        }),
                    )
                    .await
                    {
                        Ok(()) => "logging notification sent".to_string(),
                        Err(error) => error,
                    }
                } else {
                    "logging disabled; no notification sent".to_string()
                };
                Ok(text_result(format!(
                    "Simulated logging is now {}. {notify_status}.",
                    if enabled { "enabled" } else { "disabled" }
                )))
            }
            "toggle-subscriber-updates" => {
                let enabled = {
                    let mut flag = self.subscriber_updates.lock().expect("subscriber lock");
                    *flag = !*flag;
                    *flag
                };
                let subscribed: Vec<String> = self
                    .subscribed_resources
                    .lock()
                    .expect("subscription lock")
                    .iter()
                    .cloned()
                    .collect();
                let mut notified = 0_usize;
                let mut errors = Vec::new();
                if enabled {
                    for uri in subscribed.iter().cloned() {
                        match notify_resource_updated(&context.peer, uri).await {
                            Ok(()) => notified += 1,
                            Err(error) => errors.push(error),
                        }
                    }
                }
                let detail = if subscribed.is_empty() {
                    "No subscribed resources are registered for this client session.".to_string()
                } else if errors.is_empty() {
                    format!("Resource update notifications sent: {notified}.")
                } else {
                    format!(
                        "Resource update notifications sent: {notified}; errors: {}.",
                        errors.join("; ")
                    )
                };
                Ok(text_result(format!(
                    "Simulated subscriber updates are now {}. {detail}",
                    if enabled { "enabled" } else { "disabled" }
                )))
            }
            "trigger-sampling-request"
            | "trigger-sampling-request-async"
            | "simulate-research-query" => {
                if client_supports_sampling(&context.peer) {
                    match request_sampling_message(&context.peer).await {
                        Ok(result) => Ok(text_result(format!(
                            "{} completed via sampling/createMessage:\n{result}",
                            request.name
                        ))),
                        Err(error) => Ok(text_result(format!(
                            "{} attempted sampling/createMessage but fell back: {error}",
                            request.name
                        ))),
                    }
                } else {
                    Ok(text_result(format!(
                        "{} requires client sampling capability; the connected client did not advertise it.",
                        request.name
                    )))
                }
            }
            "trigger-elicitation-request"
            | "trigger-elicitation-request-async"
            | "trigger-url-elicitation" => Ok(text_result(format!(
                "{} requires rmcp elicitation feature support and a client that advertises elicitation. It is explicitly deferred in this Rust build.",
                request.name
            ))),
            _ => Err(McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(build_prompts()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult::new(build_prompt_messages(
            request.name.as_str(),
            request.arguments.as_ref(),
        )?))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(self.resources()))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(
            resource_templates(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Ok(ReadResourceResult::new(
            self.read_resource_contents(request.uri.as_str())?,
        ))
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        self.read_resource_contents(&request.uri)?;
        self.subscribed_resources
            .lock()
            .expect("subscription lock")
            .insert(request.uri);
        Ok(())
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        self.subscribed_resources
            .lock()
            .expect("subscription lock")
            .remove(&request.uri);
        Ok(())
    }

    async fn set_level(
        &self,
        request: SetLevelRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        *self
            .current_logging_level
            .lock()
            .expect("logging level lock") = request.level;
        Ok(())
    }

    async fn on_roots_list_changed(&self, context: NotificationContext<RoleServer>) {
        if !client_supports_roots(&context.peer) {
            return;
        }
        if let Ok(result) = request_client_roots(&context.peer).await {
            *self.cached_roots.lock().expect("roots lock") = result.roots;
        }
    }
}

impl EverythingServer {
    fn read_resource_contents(&self, uri: &str) -> Result<Vec<ResourceContents>, McpError> {
        if let Some(id) = parse_dynamic_resource_id(uri, TEXT_URI_BASE) {
            return Ok(vec![dynamic_text_resource(id)]);
        }
        if let Some(id) = parse_dynamic_resource_id(uri, BLOB_URI_BASE) {
            return Ok(vec![dynamic_blob_resource(id)]);
        }
        if let Some((_, mime, content)) = static_documents()
            .into_iter()
            .find(|(name, _, _)| uri == format!("demo://resource/static/document/{name}"))
        {
            return Ok(vec![ResourceContents::TextResourceContents {
                uri: uri.to_string(),
                mime_type: Some(mime.to_string()),
                text: content.to_string(),
                meta: None,
            }]);
        }
        if let Some(resource) = self
            .session_resources
            .lock()
            .expect("session lock")
            .get(uri)
            .cloned()
        {
            return Ok(vec![ResourceContents::BlobResourceContents {
                uri: uri.to_string(),
                mime_type: Some(resource.mime_type),
                blob: resource.blob,
                meta: None,
            }]);
        }
        Err(McpError::invalid_params(
            format!("Unknown resource: {uri}"),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn tool_inventory_has_everything_names() {
        let server = EverythingServer::default();
        let tools = server.tools();
        assert_eq!(tools.len(), 19);
        for name in TOOL_NAMES {
            assert!(
                tools.iter().any(|tool| tool.name == *name),
                "missing {name}"
            );
        }
    }

    #[test]
    fn prompt_inventory_has_four_prompts() {
        let server = EverythingServer::default();
        let prompts = server.prompts();
        assert_eq!(prompts.len(), 4);
        for name in PROMPT_NAMES {
            assert!(
                prompts.iter().any(|prompt| prompt.name == *name),
                "missing {name}"
            );
        }
    }

    #[test]
    fn every_prompt_get_builds_messages() {
        let simple = build_prompt_messages("simple-prompt", None).unwrap();
        assert_eq!(simple.len(), 1);

        let mut args = JsonObject::new();
        args.insert("city".to_string(), json!("Portland"));
        args.insert("state".to_string(), json!("Oregon"));
        let with_args = build_prompt_messages("args-prompt", Some(&args)).unwrap();
        assert_eq!(with_args.len(), 1);

        let mut completable = JsonObject::new();
        completable.insert("department".to_string(), json!("Engineering"));
        completable.insert("name".to_string(), json!("Ada"));
        let completable_messages =
            build_prompt_messages("completable-prompt", Some(&completable)).unwrap();
        assert_eq!(completable_messages.len(), 1);

        let mut resource = JsonObject::new();
        resource.insert("resourceType".to_string(), json!("Text"));
        resource.insert("resourceId".to_string(), json!("7"));
        let resource_messages = build_prompt_messages("resource-prompt", Some(&resource)).unwrap();
        assert_eq!(resource_messages.len(), 2);
    }

    #[test]
    fn prompt_get_rejects_missing_required_args() {
        let err = build_prompt_messages("args-prompt", None).unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn resource_templates_are_available() {
        let templates = resource_templates();
        assert_eq!(templates.len(), 2);
        assert!(templates
            .iter()
            .any(|template| template.uri_template.contains("/text/")));
        assert!(templates
            .iter()
            .any(|template| template.uri_template.contains("/blob/")));
    }

    #[test]
    fn resource_read_supports_dynamic_text_blob_and_static_docs() {
        let server = EverythingServer::default();

        let text = server
            .read_resource_contents("demo://resource/dynamic/text/3")
            .unwrap();
        assert!(matches!(
            &text[0],
            ResourceContents::TextResourceContents { text, .. } if text.contains("Resource 3")
        ));

        let blob = server
            .read_resource_contents("demo://resource/dynamic/blob/4")
            .unwrap();
        assert!(matches!(
            &blob[0],
            ResourceContents::BlobResourceContents { blob, .. } if !blob.is_empty()
        ));

        let static_doc = server
            .read_resource_contents("demo://resource/static/document/instructions.md")
            .unwrap();
        assert!(matches!(
            &static_doc[0],
            ResourceContents::TextResourceContents { mime_type, text, .. }
                if mime_type.as_deref() == Some("text/markdown") && text.contains("Everything")
        ));
    }

    #[test]
    fn resource_read_rejects_unknown_uri() {
        let server = EverythingServer::default();
        let err = server
            .read_resource_contents("demo://resource/does-not-exist")
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn tool_schemas_have_no_boolean_property_nodes() {
        for tool in EverythingServer::default().tools() {
            let schema_val = Value::Object((*tool.input_schema).clone());
            if let Some(props) = schema_val["properties"].as_object() {
                for (key, val) in props {
                    assert!(
                        !val.is_boolean(),
                        "property '{key}' in tool '{}' is a bare boolean schema node",
                        tool.name
                    );
                }
            }
        }
    }

    #[test]
    fn structured_weather_matches_upstream_values() {
        let weather = match "Chicago" {
            "New York" => json!({"temperature":33,"conditions":"Cloudy","humidity":82}),
            "Chicago" => {
                json!({"temperature":36,"conditions":"Light rain / drizzle","humidity":82})
            }
            _ => unreachable!(),
        };
        assert_eq!(weather["temperature"], 36);
        assert_eq!(weather["conditions"], "Light rain / drizzle");
        assert_eq!(weather["humidity"], 82);
    }

    #[test]
    fn data_uri_decodes_plain_text() {
        let bytes = data_uri_to_bytes("data:text/plain,hello%20world", DEFAULT_GZIP_MAX_FETCH_SIZE)
            .unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn data_uri_respects_max_bytes() {
        let err = data_uri_to_bytes("data:text/plain,hello%20world", 4).unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn gzip_bytes_round_trips() {
        let compressed = gzip_bytes(b"hello gzip").unwrap();
        assert_eq!(&compressed[..2], &[0x1f, 0x8b]);

        let mut decoder = GzDecoder::new(compressed.as_slice());
        let mut decoded = String::new();
        decoder.read_to_string(&mut decoded).unwrap();
        assert_eq!(decoded, "hello gzip");
    }

    #[tokio::test]
    async fn load_gzip_input_fetches_http_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\nhello from http",
                )
                .unwrap();
        });

        let bytes = load_gzip_input_bytes(&format!("http://{addr}/data.txt"))
            .await
            .unwrap();
        handle.join().unwrap();
        assert_eq!(bytes, b"hello from http");
    }
}
