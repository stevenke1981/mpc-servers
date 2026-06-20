use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use reqwest::{header, Client, Proxy, StatusCode};
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, ErrorCode, Implementation, JsonObject,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
        ToolAnnotations, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    ErrorData as McpError,
};
use scraper::{Html, Selector};
use url::Url;

pub const DEFAULT_USER_AGENT_AUTONOMOUS: &str =
    "ModelContextProtocol/1.0 (Autonomous; +https://github.com/modelcontextprotocol/servers)";
const DEFAULT_MAX_LENGTH: usize = 5000;
const MAX_TOOL_LENGTH: usize = 999_999;

#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub user_agent: String,
    pub allow_private_network: bool,
    pub redirect_limit: usize,
    pub timeout: Duration,
    pub max_response_bytes: u64,
    pub proxy_url: Option<String>,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            user_agent: DEFAULT_USER_AGENT_AUTONOMOUS.to_string(),
            allow_private_network: false,
            redirect_limit: 5,
            timeout: Duration::from_secs(30),
            max_response_bytes: 1024 * 1024,
            proxy_url: None,
        }
    }
}

#[derive(Debug)]
pub struct FetchError(String);

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FetchError {}

type FetchResult<T> = Result<T, FetchError>;

#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub url: String,
    pub content: String,
    pub prefix: String,
}

#[derive(Debug)]
pub struct FetchServer {
    config: FetchConfig,
    client: Client,
}

impl FetchServer {
    pub fn new(config: FetchConfig) -> FetchResult<Self> {
        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.timeout);

        if let Some(proxy_url) = &config.proxy_url {
            builder =
                builder
                    .proxy(Proxy::all(proxy_url).map_err(|e| {
                        FetchError(format!("Invalid proxy URL '{proxy_url}': {e}"))
                    })?);
        } else {
            builder = builder.no_proxy();
        }

        let client = builder
            .build()
            .map_err(|e| FetchError(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self { config, client })
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        vec![build_fetch_tool()]
    }
}

fn schema(props: serde_json::Value, required: &[&str]) -> Arc<JsonObject> {
    let schema = serde_json::json!({
        "type": "object",
        "properties": props,
        "required": required,
    });
    let obj = if let serde_json::Value::Object(o) = schema {
        o
    } else {
        unreachable!()
    };
    Arc::new(obj)
}

fn build_fetch_tool() -> Tool {
    Tool::new(
        "fetch",
        "Fetches a URL from the internet and optionally extracts its contents as markdown.",
        schema(
            serde_json::json!({
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": "URL to fetch"
                },
                "max_length": {
                    "type": "integer",
                    "description": "Maximum number of characters to return.",
                    "default": DEFAULT_MAX_LENGTH,
                    "minimum": 1,
                    "maximum": MAX_TOOL_LENGTH
                },
                "start_index": {
                    "type": "integer",
                    "description": "Return output starting at this character index.",
                    "default": 0,
                    "minimum": 0
                },
                "raw": {
                    "type": "boolean",
                    "description": "Return raw content without HTML simplification.",
                    "default": false
                }
            }),
            &["url"],
        ),
    )
    .with_annotations(
        ToolAnnotations::new()
            .read_only(true)
            .destructive(false)
            .idempotent(true)
            .open_world(true),
    )
}

fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn internal_error(error: FetchError) -> McpError {
    McpError::new(ErrorCode::INTERNAL_ERROR, error.to_string(), None)
}

fn extract_string<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, McpError> {
    args.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
        McpError::invalid_params(format!("Missing required argument: '{key}'"), None)
    })
}

fn extract_usize(args: &JsonObject, key: &str, default: usize) -> Result<usize, McpError> {
    match args.get(key) {
        None => Ok(default),
        Some(value) => {
            let Some(n) = value.as_u64() else {
                return Err(McpError::invalid_params(
                    format!("'{key}' must be a positive integer"),
                    None,
                ));
            };
            if n == 0 && key == "max_length" {
                return Err(McpError::invalid_params(
                    "'max_length' must be greater than 0",
                    None,
                ));
            }
            if n as usize > MAX_TOOL_LENGTH && key == "max_length" {
                return Err(McpError::invalid_params(
                    format!("'max_length' must be <= {MAX_TOOL_LENGTH}"),
                    None,
                ));
            }
            Ok(n as usize)
        }
    }
}

fn extract_bool(args: &JsonObject, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn is_blocked_hostname(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost" || host.ends_with(".localhost")
}

fn is_shared_cgnat(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_benchmark(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 198 && (18..=19).contains(&octets[1])
}

fn is_ipv6_documentation(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] == 0x2001) && (ip.segments()[1] == 0x0db8)
}

pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_multicast()
                || ip.is_unspecified()
                || ip.is_documentation()
                || is_shared_cgnat(ip)
                || is_benchmark(ip)
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || is_ipv6_unique_local(&ip)
                || is_ipv6_unicast_link_local(&ip)
                || is_ipv6_documentation(ip)
        }
    }
}

fn is_ipv6_unique_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn is_ipv6_unicast_link_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

async fn validate_url(url: &Url, config: &FetchConfig) -> FetchResult<()> {
    match url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(FetchError(format!(
                "Unsupported URL scheme '{scheme}'. Only http and https are allowed."
            )));
        }
    }

    let host = url
        .host_str()
        .ok_or_else(|| FetchError("URL must include a host".to_string()))?;

    if config.allow_private_network {
        return Ok(());
    }

    if is_blocked_hostname(host) {
        return Err(FetchError(format!(
            "Blocked host '{host}' by fetch security policy"
        )));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(FetchError(format!(
                "Blocked IP address '{ip}' by fetch security policy"
            )));
        }
        return Ok(());
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| FetchError("URL must include a valid port".to_string()))?;
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| FetchError(format!("Failed to resolve host '{host}': {e}")))?
        .collect::<Vec<_>>();

    if addrs.is_empty() {
        return Err(FetchError(format!("Host '{host}' did not resolve")));
    }

    if let Some(blocked) = addrs
        .iter()
        .map(|addr| addr.ip())
        .find(|ip| is_blocked_ip(*ip))
    {
        return Err(FetchError(format!(
            "Blocked host '{host}' because it resolves to '{blocked}'"
        )));
    }

    Ok(())
}

fn is_redirect_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

fn extract_content_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector = Selector::parse("article, main, body").expect("static selector");
    let mut text = String::new();

    for node in document.select(&selector) {
        let chunk = node.text().collect::<Vec<_>>().join(" ");
        if chunk.trim().len() > text.trim().len() {
            text = chunk;
        }
    }

    if text.trim().is_empty() {
        text = document.root_element().text().collect::<Vec<_>>().join(" ");
    }

    let simplified = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if simplified.is_empty() {
        "<error>Page failed to be simplified from HTML</error>".to_string()
    } else {
        simplified
    }
}

fn is_html(content_type: &str, body: &str) -> bool {
    content_type.to_ascii_lowercase().contains("text/html")
        || body
            .get(..body.len().min(100))
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("<html")
        || content_type.trim().is_empty()
}

pub async fn fetch_url(
    client: &Client,
    config: &FetchConfig,
    url: &str,
    force_raw: bool,
) -> FetchResult<FetchedPage> {
    let mut current = Url::parse(url).map_err(|e| FetchError(format!("Invalid URL: {e}")))?;

    for redirect_count in 0..=config.redirect_limit {
        validate_url(&current, config).await?;

        let response = client
            .get(current.clone())
            .header(header::USER_AGENT, config.user_agent.clone())
            .send()
            .await
            .map_err(|e| FetchError(format!("Failed to fetch {current}: {e}")))?;

        if is_redirect_status(response.status()) {
            if redirect_count == config.redirect_limit {
                return Err(FetchError(format!(
                    "Redirect limit exceeded while fetching {url}"
                )));
            }
            let location = response
                .headers()
                .get(header::LOCATION)
                .ok_or_else(|| FetchError("Redirect response missing Location header".to_string()))?
                .to_str()
                .map_err(|e| FetchError(format!("Invalid redirect Location header: {e}")))?;
            current = current
                .join(location)
                .map_err(|e| FetchError(format!("Invalid redirect URL '{location}': {e}")))?;
            continue;
        }

        if response.status().as_u16() >= 400 {
            return Err(FetchError(format!(
                "Failed to fetch {current} - status code {}",
                response.status()
            )));
        }

        if let Some(length) = response.content_length() {
            if length > config.max_response_bytes {
                return Err(FetchError(format!(
                    "Response exceeds max bytes limit: {length} > {}",
                    config.max_response_bytes
                )));
            }
        }

        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();

        let mut bytes = Vec::new();
        let mut response = response;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| FetchError(format!("Failed reading response body: {e}")))?
        {
            bytes.extend_from_slice(&chunk);
            if bytes.len() as u64 > config.max_response_bytes {
                return Err(FetchError(format!(
                    "Response exceeds max bytes limit: {} > {}",
                    bytes.len(),
                    config.max_response_bytes
                )));
            }
        }

        let raw = String::from_utf8_lossy(&bytes).to_string();
        if is_html(&content_type, &raw) && !force_raw {
            return Ok(FetchedPage {
                url: current.to_string(),
                content: extract_content_from_html(&raw),
                prefix: String::new(),
            });
        }

        let prefix = format!(
            "Content type {content_type} cannot be simplified to markdown, but here is the raw content:\n"
        );
        return Ok(FetchedPage {
            url: current.to_string(),
            content: raw,
            prefix,
        });
    }

    Err(FetchError("Redirect loop ended unexpectedly".to_string()))
}

fn slice_content(content: &str, start_index: usize, max_length: usize) -> (String, Option<usize>) {
    let chars = content.chars().collect::<Vec<_>>();
    if start_index >= chars.len() {
        return (
            "<error>No more content available.</error>".to_string(),
            None,
        );
    }

    let end = (start_index + max_length).min(chars.len());
    let mut sliced = chars[start_index..end].iter().collect::<String>();
    let next = if end < chars.len() { Some(end) } else { None };
    if let Some(next_start) = next {
        sliced.push_str(&format!(
            "\n\n<error>Content truncated. Call the fetch tool with a start_index of {next_start} to get more content.</error>"
        ));
    }
    (sliced, next)
}

impl ServerHandler for FetchServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        ServerInfo::new(caps).with_server_info(Implementation::new(
            "fetch-server",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        match name {
            "fetch" => Some(build_fetch_tool()),
            _ => None,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![build_fetch_tool()]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request
            .arguments
            .as_ref()
            .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;

        match request.name.as_ref() {
            "fetch" => {
                let url = extract_string(args, "url")?;
                let max_length = extract_usize(args, "max_length", DEFAULT_MAX_LENGTH)?;
                let start_index = extract_usize(args, "start_index", 0)?;
                let raw = extract_bool(args, "raw", false);
                let page = fetch_url(&self.client, &self.config, url, raw)
                    .await
                    .map_err(internal_error)?;
                let (content, _) = slice_content(&page.content, start_index, max_length);
                Ok(text_result(format!(
                    "{}Contents of {}:\n{}",
                    page.prefix, page.url, content
                )))
            }
            _ => Err(McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    async fn serve_once(status: &str, content_type: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_string();
        let content_type = content_type.to_string();
        let body = body.to_string();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        format!(
            "http://{addr}/page-{}",
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    }

    #[test]
    fn test_blocked_ip_ranges() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
    }

    #[tokio::test]
    async fn test_default_policy_blocks_localhost() {
        let config = FetchConfig::default();
        let url = Url::parse("http://127.0.0.1:8080/").unwrap();
        let err = validate_url(&url, &config).await.unwrap_err();
        assert!(err.to_string().contains("Blocked IP address"));
    }

    #[tokio::test]
    async fn test_fetch_localhost_when_explicitly_allowed() {
        let mut config = FetchConfig {
            allow_private_network: true,
            ..FetchConfig::default()
        };
        config.timeout = Duration::from_secs(5);
        let server = FetchServer::new(config.clone()).unwrap();
        let url = serve_once(
            "200 OK",
            "text/html",
            "<html><body><article><h1>Hello</h1><p>Smoke page</p></article></body></html>",
        )
        .await;
        let page = fetch_url(&server.client, &config, &url, false)
            .await
            .unwrap();
        assert!(page.content.contains("Hello"));
        assert!(page.content.contains("Smoke page"));
    }

    #[tokio::test]
    async fn test_fetch_raw_json() {
        let config = FetchConfig {
            allow_private_network: true,
            ..FetchConfig::default()
        };
        let server = FetchServer::new(config.clone()).unwrap();
        let url = serve_once("200 OK", "application/json", "{\"key\":\"value\"}").await;
        let page = fetch_url(&server.client, &config, &url, false)
            .await
            .unwrap();
        assert_eq!(page.content, "{\"key\":\"value\"}");
        assert!(page.prefix.contains("Content type application/json"));
    }

    #[test]
    fn test_slice_content_truncates_with_next_index() {
        let (content, next) = slice_content("abcdef", 1, 3);
        assert!(content.starts_with("bcd"));
        assert!(content.contains("start_index of 4"));
        assert_eq!(next, Some(4));
    }

    #[test]
    fn test_slice_content_no_more_content() {
        let (content, next) = slice_content("abc", 3, 2);
        assert!(content.contains("No more content"));
        assert_eq!(next, None);
    }

    #[test]
    fn test_tool_inventory_and_schema() {
        let server = FetchServer::new(FetchConfig::default()).unwrap();
        let tools = server.get_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "fetch");
        let schema_val = serde_json::Value::Object((*tools[0].input_schema).clone());
        for (key, val) in schema_val["properties"].as_object().unwrap() {
            assert!(
                !val.is_boolean(),
                "property '{key}' is a bare boolean schema node"
            );
        }
    }
}
