use std::path::PathBuf;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::git;
use crate::nu::{NuRunMode, NuRunOptions, get_nu_version, run_nushell};
use crate::nu_tools::{
    NuFindOptions, NuGrepOptions, NuLsOptions, NuReadOptions, NuToolCommon, nu_find, nu_grep,
    nu_ls, nu_read,
};

#[derive(Debug, Clone)]
pub struct NushellServer {
    tool_router: ToolRouter<Self>,
}

impl NushellServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for NushellServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuVersionInput {
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuEvalInput {
    /// Inline Nushell code to execute.
    pub command: String,
    /// Optional working directory for the command.
    pub cwd: Option<String>,
    /// Optional stdin passed to Nushell.
    pub stdin: Option<String>,
    /// Command timeout in milliseconds. Defaults to 30000 and is capped at 120000.
    pub timeout_ms: Option<u64>,
    /// Maximum bytes captured per stdout/stderr stream. Defaults to 1048576.
    pub max_output_bytes: Option<usize>,
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuScriptInput {
    /// Path to a .nu script file.
    pub script_path: String,
    /// Positional arguments passed to the script.
    pub args: Option<Vec<String>>,
    /// Optional working directory for the script process.
    pub cwd: Option<String>,
    /// Optional stdin passed to Nushell.
    pub stdin: Option<String>,
    /// Command timeout in milliseconds. Defaults to 30000 and is capped at 120000.
    pub timeout_ms: Option<u64>,
    /// Maximum bytes captured per stdout/stderr stream. Defaults to 1048576.
    pub max_output_bytes: Option<usize>,
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitCwdInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitStatusInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Return porcelain output for machine parsing.
    pub porcelain: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitDiffInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Show staged diff with `--cached`.
    pub staged: Option<bool>,
    /// Optional ref to compare against, e.g. `main` or `HEAD~1`.
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitLogInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Number of commits to return. Defaults to 10.
    pub count: Option<u64>,
    /// Use compact one-line output. Defaults to true.
    pub oneline: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitTreeInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Number of commits to return. Defaults to 20.
    pub count: Option<u64>,
    /// Include all local and remote branches.
    pub all: Option<bool>,
    /// Optional start ref, e.g. `main` or `HEAD~5`.
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitBranchInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Branch action: `list`, `create`, or `switch`.
    pub action: Option<String>,
    /// Required for `create` and `switch`.
    pub branch_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitCommitInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Commit message. Use a <=72 char subject, blank line, then body.
    pub message: String,
    /// Optional files to stage. Defaults to `git add -A`.
    pub files: Option<Vec<String>>,
    /// Amend the previous commit.
    pub amend: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitStashInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Stash action: `list`, `push`, `pop`, or `drop`.
    pub action: Option<String>,
    /// Optional message for `push`.
    pub message: Option<String>,
    /// Stash index for `pop` or `drop`. Defaults to 0.
    pub index: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GitPrecommitReviewInput {
    /// Optional Git working directory. Defaults to the MCP server process directory.
    pub cwd: Option<String>,
    /// Maximum staged diff characters to capture.
    pub max_chars: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuGrepInput {
    /// Regex pattern to search.
    pub pattern: String,
    /// File path, or directory when recursive is true. Defaults to current directory.
    pub path: Option<String>,
    /// Search recursively with Nushell glob.
    pub recursive: Option<bool>,
    /// Use case-insensitive regex matching.
    pub ignore_case: Option<bool>,
    /// Prefix matches with line numbers.
    pub line_number: Option<bool>,
    /// Maximum result lines. Defaults to 200.
    pub max_lines: Option<u64>,
    /// Optional working directory for Nushell.
    pub cwd: Option<String>,
    /// Command timeout in milliseconds. Defaults to 30000 and is capped at 120000.
    pub timeout_ms: Option<u64>,
    /// Maximum bytes captured per stdout/stderr stream. Defaults to 1048576.
    pub max_output_bytes: Option<usize>,
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuFindInput {
    /// Directory to search. Defaults to current directory.
    pub path: Option<String>,
    /// Optional filename regex/substring pattern applied to basename.
    pub name: Option<String>,
    /// Optional extension filter, with or without leading dot.
    pub extension: Option<String>,
    /// Entry type: `any`, `file`, or `directory`.
    pub entry_type: Option<String>,
    /// Search recursively with Nushell glob. Defaults to true.
    pub recursive: Option<bool>,
    /// Maximum results. Defaults to 200.
    pub max_results: Option<u64>,
    /// Optional working directory for Nushell.
    pub cwd: Option<String>,
    /// Command timeout in milliseconds. Defaults to 30000 and is capped at 120000.
    pub timeout_ms: Option<u64>,
    /// Maximum bytes captured per stdout/stderr stream. Defaults to 1048576.
    pub max_output_bytes: Option<usize>,
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuReadInput {
    /// File path to read.
    pub file: String,
    /// Read mode: `head`, `tail`, or `cat`.
    pub mode: Option<String>,
    /// Lines to return. Defaults to 50.
    pub lines: Option<u64>,
    /// Lines to skip before head/cat. Defaults to 0.
    pub offset: Option<u64>,
    /// Prefix returned lines with line numbers. Defaults to true.
    pub line_numbers: Option<bool>,
    /// Optional working directory for Nushell.
    pub cwd: Option<String>,
    /// Command timeout in milliseconds. Defaults to 30000 and is capped at 120000.
    pub timeout_ms: Option<u64>,
    /// Maximum bytes captured per stdout/stderr stream. Defaults to 1048576.
    pub max_output_bytes: Option<usize>,
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NuLsInput {
    /// Directory to list. Defaults to current directory.
    pub path: Option<String>,
    /// Include hidden files.
    pub all: Option<bool>,
    /// Keep full Nushell ls columns. Defaults to true.
    pub long: Option<bool>,
    /// Optional working directory for Nushell.
    pub cwd: Option<String>,
    /// Command timeout in milliseconds. Defaults to 30000 and is capped at 120000.
    pub timeout_ms: Option<u64>,
    /// Maximum bytes captured per stdout/stderr stream. Defaults to 1048576.
    pub max_output_bytes: Option<usize>,
    /// Optional Nushell executable path. Defaults to NUSHELL_MCP_NU_PATH or nu from PATH.
    pub nu_path: Option<String>,
}

#[tool_router(router = tool_router)]
impl NushellServer {
    #[tool(
        name = "nu_version",
        description = "Run `nu --version` using PATH, NUSHELL_MCP_NU_PATH, or an explicit nu_path."
    )]
    pub async fn nu_version(
        &self,
        Parameters(input): Parameters<NuVersionInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(get_nu_version(input.nu_path).await)
    }

    #[tool(
        name = "nu_eval",
        description = "Run inline Nushell code with `nu --no-config-file --commands <command>`."
    )]
    pub async fn nu_eval(
        &self,
        Parameters(input): Parameters<NuEvalInput>,
    ) -> Result<CallToolResult, McpError> {
        if input.command.trim().is_empty() {
            return Err(McpError::invalid_params("command must not be empty", None));
        }

        json_result(
            run_nushell(NuRunOptions {
                mode: NuRunMode::Eval {
                    command: input.command,
                },
                cwd: input.cwd.map(PathBuf::from),
                stdin: input.stdin,
                timeout_ms: input.timeout_ms,
                max_output_bytes: input.max_output_bytes,
                nu_path: input.nu_path,
            })
            .await,
        )
    }

    #[tool(
        name = "nu_script",
        description = "Run a Nushell script file with optional positional arguments."
    )]
    pub async fn nu_script(
        &self,
        Parameters(input): Parameters<NuScriptInput>,
    ) -> Result<CallToolResult, McpError> {
        if input.script_path.trim().is_empty() {
            return Err(McpError::invalid_params(
                "script_path must not be empty",
                None,
            ));
        }

        json_result(
            run_nushell(NuRunOptions {
                mode: NuRunMode::Script {
                    script_path: input.script_path,
                    args: input.args.unwrap_or_default(),
                },
                cwd: input.cwd.map(PathBuf::from),
                stdin: input.stdin,
                timeout_ms: input.timeout_ms,
                max_output_bytes: input.max_output_bytes,
                nu_path: input.nu_path,
            })
            .await,
        )
    }

    #[tool(
        name = "git_status",
        description = "Get current Git repository status, with optional porcelain output."
    )]
    pub async fn git_status(
        &self,
        Parameters(input): Parameters<GitStatusInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(
            git::git_status(
                input.cwd.map(PathBuf::from),
                input.porcelain.unwrap_or(false),
            )
            .await,
        )
    }

    #[tool(
        name = "git_diff",
        description = "Show Git diff for unstaged, staged, or a ref."
    )]
    pub async fn git_diff(
        &self,
        Parameters(input): Parameters<GitDiffInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(
            git::git_diff(
                input.cwd.map(PathBuf::from),
                input.staged.unwrap_or(false),
                input.reference,
            )
            .await,
        )
    }

    #[tool(name = "git_log", description = "Show recent Git commit history.")]
    pub async fn git_log(
        &self,
        Parameters(input): Parameters<GitLogInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(
            git::git_log(
                input.cwd.map(PathBuf::from),
                input.count,
                input.oneline.unwrap_or(true),
            )
            .await,
        )
    }

    #[tool(
        name = "git_tree",
        description = "Show Git commit graph with branch topology."
    )]
    pub async fn git_tree(
        &self,
        Parameters(input): Parameters<GitTreeInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(
            git::git_tree(
                input.cwd.map(PathBuf::from),
                input.count,
                input.all.unwrap_or(false),
                input.reference,
            )
            .await,
        )
    }

    #[tool(
        name = "git_branch",
        description = "List, create, or switch Git branches."
    )]
    pub async fn git_branch(
        &self,
        Parameters(input): Parameters<GitBranchInput>,
    ) -> Result<CallToolResult, McpError> {
        let action = input.action.unwrap_or_else(|| "list".to_owned());
        if !matches!(action.as_str(), "list" | "create" | "switch") {
            return Err(McpError::invalid_params(
                "action must be one of: list, create, switch",
                None,
            ));
        }
        json_result(git::git_branch(input.cwd.map(PathBuf::from), action, input.branch_name).await)
    }

    #[tool(
        name = "git_commit",
        description = "Stage files and create a Git commit using a temporary message file."
    )]
    pub async fn git_commit(
        &self,
        Parameters(input): Parameters<GitCommitInput>,
    ) -> Result<CallToolResult, McpError> {
        if input.message.trim().is_empty() {
            return Err(McpError::invalid_params("message must not be empty", None));
        }
        json_result(
            git::git_commit(
                input.cwd.map(PathBuf::from),
                input.message,
                input.files.unwrap_or_default(),
                input.amend.unwrap_or(false),
            )
            .await,
        )
    }

    #[tool(
        name = "git_stash",
        description = "Stash, pop, list, or drop Git stashes."
    )]
    pub async fn git_stash(
        &self,
        Parameters(input): Parameters<GitStashInput>,
    ) -> Result<CallToolResult, McpError> {
        let action = input.action.unwrap_or_else(|| "list".to_owned());
        if !matches!(action.as_str(), "list" | "push" | "pop" | "drop") {
            return Err(McpError::invalid_params(
                "action must be one of: list, push, pop, drop",
                None,
            ));
        }
        json_result(
            git::git_stash(
                input.cwd.map(PathBuf::from),
                action,
                input.message,
                input.index,
            )
            .await,
        )
    }

    #[tool(
        name = "git_precommit_review",
        description = "Review staged changes before commit with a bounded staged diff."
    )]
    pub async fn git_precommit_review(
        &self,
        Parameters(input): Parameters<GitPrecommitReviewInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(git::git_precommit_review(input.cwd.map(PathBuf::from), input.max_chars).await)
    }

    #[tool(
        name = "nu_grep",
        description = "Search file contents using Nushell open/lines/regex helpers."
    )]
    pub async fn nu_grep(
        &self,
        Parameters(input): Parameters<NuGrepInput>,
    ) -> Result<CallToolResult, McpError> {
        if input.pattern.trim().is_empty() {
            return Err(McpError::invalid_params("pattern must not be empty", None));
        }

        json_result(
            nu_grep(NuGrepOptions {
                pattern: input.pattern,
                path: input.path,
                recursive: input.recursive.unwrap_or(false),
                ignore_case: input.ignore_case.unwrap_or(false),
                line_number: input.line_number.unwrap_or(true),
                max_lines: input.max_lines,
                common: common_from(
                    input.cwd,
                    input.timeout_ms,
                    input.max_output_bytes,
                    input.nu_path,
                ),
            })
            .await,
        )
    }

    #[tool(
        name = "nu_find",
        description = "Find files or directories using Nushell glob/path helpers."
    )]
    pub async fn nu_find(
        &self,
        Parameters(input): Parameters<NuFindInput>,
    ) -> Result<CallToolResult, McpError> {
        let entry_type = input.entry_type.clone().unwrap_or_else(|| "any".to_owned());
        if !matches!(entry_type.as_str(), "any" | "file" | "directory") {
            return Err(McpError::invalid_params(
                "entry_type must be one of: any, file, directory",
                None,
            ));
        }

        json_result(
            nu_find(NuFindOptions {
                path: input.path,
                name: input.name,
                extension: input.extension,
                entry_type: Some(entry_type),
                recursive: input.recursive.unwrap_or(true),
                max_results: input.max_results,
                common: common_from(
                    input.cwd,
                    input.timeout_ms,
                    input.max_output_bytes,
                    input.nu_path,
                ),
            })
            .await,
        )
    }

    #[tool(
        name = "nu_read",
        description = "Read file head, tail, or first lines using Nushell."
    )]
    pub async fn nu_read(
        &self,
        Parameters(input): Parameters<NuReadInput>,
    ) -> Result<CallToolResult, McpError> {
        if input.file.trim().is_empty() {
            return Err(McpError::invalid_params("file must not be empty", None));
        }
        let mode = input.mode.unwrap_or_else(|| "head".to_owned());
        if !matches!(mode.as_str(), "head" | "tail" | "cat") {
            return Err(McpError::invalid_params(
                "mode must be one of: head, tail, cat",
                None,
            ));
        }

        json_result(
            nu_read(NuReadOptions {
                file: input.file,
                mode,
                lines: input.lines,
                offset: input.offset,
                line_numbers: input.line_numbers.unwrap_or(true),
                common: common_from(
                    input.cwd,
                    input.timeout_ms,
                    input.max_output_bytes,
                    input.nu_path,
                ),
            })
            .await,
        )
    }

    #[tool(
        name = "nu_ls",
        description = "List directory contents using Nushell ls."
    )]
    pub async fn nu_ls(
        &self,
        Parameters(input): Parameters<NuLsInput>,
    ) -> Result<CallToolResult, McpError> {
        json_result(
            nu_ls(NuLsOptions {
                path: input.path,
                all: input.all.unwrap_or(false),
                long: input.long.unwrap_or(true),
                common: common_from(
                    input.cwd,
                    input.timeout_ms,
                    input.max_output_bytes,
                    input.nu_path,
                ),
            })
            .await,
        )
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for NushellServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "nushell-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Runs trusted local Nushell commands through explicit MCP tool calls.".to_owned(),
            )
    }
}

fn json_result<T>(value: T) -> Result<CallToolResult, McpError>
where
    T: Serialize,
{
    let text = serde_json::to_string_pretty(&value)
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn common_from(
    cwd: Option<String>,
    timeout_ms: Option<u64>,
    max_output_bytes: Option<usize>,
    nu_path: Option<String>,
) -> NuToolCommon {
    NuToolCommon {
        cwd: cwd.map(PathBuf::from),
        timeout_ms,
        max_output_bytes,
        nu_path,
    }
}
