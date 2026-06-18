mod allowed_directories;
mod path_validation;

pub use allowed_directories::AllowedDirectories;

use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ErrorCode, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    ErrorData as McpError,
};

// ---------------------------------------------------------------------------
// MCP handler — minimal for T4.1 (no file operation tools yet)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct FilesystemServer {
    _allowed: AllowedDirectories,
}

impl FilesystemServer {
    pub fn new(allowed: AllowedDirectories) -> Self {
        Self { _allowed: allowed }
    }
}

impl ServerHandler for FilesystemServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        ServerInfo::new(caps).with_server_info(Implementation::new(
            "filesystem-server",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, _name: &str) -> Option<Tool> {
        // No tools exposed in T4.1.
        None
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(Vec::new()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        Err(McpError::new(
            ErrorCode::METHOD_NOT_FOUND,
            format!("Unknown tool: {}", request.name),
            None,
        ))
    }
}
