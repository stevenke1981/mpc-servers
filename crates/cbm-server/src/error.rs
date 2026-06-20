use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// JSON-RPC 2.0 standard error codes (aligned with reference `mcp.c`).
pub const JSONRPC_PARSE_ERROR: i32 = -32700;
pub const JSONRPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSONRPC_INVALID_PARAMS: i32 = -32602;
pub const JSONRPC_INTERNAL_ERROR: i32 = -32603;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("query not allowed: {0}")]
    QueryNotAllowed(String),

    #[error("tree-sitter error: {0}")]
    TreeSitter(String),

    #[error("{0}")]
    Other(String),
}
