use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

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

const DEFAULT_CONTEXT_LINES: i64 = 3;

#[derive(Debug, Clone)]
pub struct GitServer {
    allowed_repository: Option<PathBuf>,
}

#[derive(Debug)]
pub struct GitError(String);

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GitError {}

type GitResult<T> = Result<T, GitError>;

impl GitServer {
    pub fn new(allowed_repository: Option<PathBuf>) -> Self {
        Self { allowed_repository }
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        Self::all_tools()
    }

    fn all_tools() -> Vec<Tool> {
        vec![
            build_git_status_tool(),
            build_git_diff_unstaged_tool(),
            build_git_diff_staged_tool(),
            build_git_diff_tool(),
            build_git_commit_tool(),
            build_git_add_tool(),
            build_git_reset_tool(),
            build_git_log_tool(),
            build_git_create_branch_tool(),
            build_git_checkout_tool(),
            build_git_show_tool(),
            build_git_branch_tool(),
        ]
    }

    fn resolve_repo(&self, repo_path: &str) -> Result<PathBuf, McpError> {
        validate_repo_path(Path::new(repo_path), self.allowed_repository.as_deref())
            .map_err(invalid_git_error)
    }
}

impl Default for GitServer {
    fn default() -> Self {
        Self::new(None)
    }
}

fn build_schema(props: serde_json::Value, required: &[&str]) -> Arc<JsonObject> {
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

fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn invalid_git_error(error: GitError) -> McpError {
    McpError::invalid_params(error.to_string(), None)
}

fn extract_string<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, McpError> {
    args.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
        McpError::invalid_params(format!("Missing required argument: '{key}'"), None)
    })
}

fn extract_string_array(args: &JsonObject, key: &str) -> Result<Vec<String>, McpError> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| {
            McpError::invalid_params(format!("Missing required argument: '{key}'"), None)
        })
}

fn extract_i64(args: &JsonObject, key: &str, default: i64) -> i64 {
    args.get(key).and_then(|v| v.as_i64()).unwrap_or(default)
}

fn read_only() -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(true)
        .destructive(false)
        .idempotent(true)
        .open_world(false)
}

fn write_annotation(destructive: bool, idempotent: bool) -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(destructive)
        .idempotent(idempotent)
        .open_world(false)
}

fn repo_path_schema(extra: serde_json::Value) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    props.insert(
        "repo_path".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "Path to the Git repository."
        }),
    );
    if let serde_json::Value::Object(extra) = extra {
        props.extend(extra);
    }
    serde_json::Value::Object(props)
}

fn build_git_status_tool() -> Tool {
    Tool::new(
        "git_status",
        "Shows the working tree status",
        build_schema(repo_path_schema(serde_json::json!({})), &["repo_path"]),
    )
    .with_annotations(read_only())
}

fn build_git_diff_unstaged_tool() -> Tool {
    Tool::new(
        "git_diff_unstaged",
        "Shows changes in the working directory that are not yet staged",
        build_schema(
            repo_path_schema(serde_json::json!({
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines to show in diff output.",
                    "default": DEFAULT_CONTEXT_LINES
                }
            })),
            &["repo_path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_git_diff_staged_tool() -> Tool {
    Tool::new(
        "git_diff_staged",
        "Shows changes that are staged for commit",
        build_schema(
            repo_path_schema(serde_json::json!({
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines to show in diff output.",
                    "default": DEFAULT_CONTEXT_LINES
                }
            })),
            &["repo_path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_git_diff_tool() -> Tool {
    Tool::new(
        "git_diff",
        "Shows differences between branches or commits",
        build_schema(
            repo_path_schema(serde_json::json!({
                "target": {
                    "type": "string",
                    "description": "Git branch, commit, or revision to diff against."
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines to show in diff output.",
                    "default": DEFAULT_CONTEXT_LINES
                }
            })),
            &["repo_path", "target"],
        ),
    )
    .with_annotations(read_only())
}

fn build_git_commit_tool() -> Tool {
    Tool::new(
        "git_commit",
        "Records changes to the repository",
        build_schema(
            repo_path_schema(serde_json::json!({
                "message": {
                    "type": "string",
                    "description": "Commit message."
                }
            })),
            &["repo_path", "message"],
        ),
    )
    .with_annotations(write_annotation(false, false))
}

fn build_git_add_tool() -> Tool {
    Tool::new(
        "git_add",
        "Adds file contents to the staging area",
        build_schema(
            repo_path_schema(serde_json::json!({
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files to stage. Use [\".\"] to stage all repository changes.",
                    "minItems": 1
                }
            })),
            &["repo_path", "files"],
        ),
    )
    .with_annotations(write_annotation(false, true))
}

fn build_git_reset_tool() -> Tool {
    Tool::new(
        "git_reset",
        "Unstages all staged changes",
        build_schema(repo_path_schema(serde_json::json!({})), &["repo_path"]),
    )
    .with_annotations(write_annotation(true, true))
}

fn build_git_log_tool() -> Tool {
    Tool::new(
        "git_log",
        "Shows the commit logs",
        build_schema(
            repo_path_schema(serde_json::json!({
                "max_count": {
                    "type": "integer",
                    "description": "Maximum number of commits to return.",
                    "default": 10
                },
                "start_timestamp": {
                    "type": "string",
                    "description": "Start timestamp for filtering commits."
                },
                "end_timestamp": {
                    "type": "string",
                    "description": "End timestamp for filtering commits."
                }
            })),
            &["repo_path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_git_create_branch_tool() -> Tool {
    Tool::new(
        "git_create_branch",
        "Creates a new branch from an optional base branch",
        build_schema(
            repo_path_schema(serde_json::json!({
                "branch_name": {
                    "type": "string",
                    "description": "Name of the branch to create."
                },
                "base_branch": {
                    "type": "string",
                    "description": "Optional base branch or revision."
                }
            })),
            &["repo_path", "branch_name"],
        ),
    )
    .with_annotations(write_annotation(false, false))
}

fn build_git_checkout_tool() -> Tool {
    Tool::new(
        "git_checkout",
        "Switches branches",
        build_schema(
            repo_path_schema(serde_json::json!({
                "branch_name": {
                    "type": "string",
                    "description": "Branch name to check out."
                }
            })),
            &["repo_path", "branch_name"],
        ),
    )
    .with_annotations(write_annotation(false, false))
}

fn build_git_show_tool() -> Tool {
    Tool::new(
        "git_show",
        "Shows the contents of a commit",
        build_schema(
            repo_path_schema(serde_json::json!({
                "revision": {
                    "type": "string",
                    "description": "Commit, branch, tag, or revision to show."
                }
            })),
            &["repo_path", "revision"],
        ),
    )
    .with_annotations(read_only())
}

fn build_git_branch_tool() -> Tool {
    Tool::new(
        "git_branch",
        "List Git branches",
        build_schema(
            repo_path_schema(serde_json::json!({
                "branch_type": {
                    "type": "string",
                    "enum": ["local", "remote", "all"],
                    "description": "Whether to list local branches, remote branches, or all branches.",
                    "default": "local"
                },
                "contains": {
                    "type": "string",
                    "description": "Only list branches containing this commit."
                },
                "not_contains": {
                    "type": "string",
                    "description": "Only list branches not containing this commit."
                }
            })),
            &["repo_path", "branch_type"],
        ),
    )
    .with_annotations(read_only())
}

fn run_git<I, S>(repo_path: &Path, args: I) -> GitResult<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .map_err(|e| GitError(format!("Failed to run git: {e}")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\r', '\n'])
            .to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if stderr.is_empty() { stdout } else { stderr };
        Err(GitError(if message.is_empty() {
            format!("git exited with status {}", output.status)
        } else {
            message
        }))
    }
}

fn reject_flag_like(value: &str, label: &str) -> GitResult<()> {
    if value.starts_with('-') {
        Err(GitError(format!(
            "Invalid {label}: '{value}' - cannot start with '-'"
        )))
    } else {
        Ok(())
    }
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

#[cfg(windows)]
fn path_is_within(path: &Path, parent: &Path) -> bool {
    let path = path.to_string_lossy().to_lowercase();
    let parent = parent.to_string_lossy().to_lowercase();
    path == parent || path.starts_with(&(parent.trim_end_matches('\\').to_string() + "\\"))
}

#[cfg(not(windows))]
fn path_is_within(path: &Path, parent: &Path) -> bool {
    path == parent || path.starts_with(parent)
}

pub fn validate_repo_path(
    repo_path: &Path,
    allowed_repository: Option<&Path>,
) -> GitResult<PathBuf> {
    let resolved_repo = repo_path
        .canonicalize()
        .map_err(|e| GitError(format!("Invalid path '{}': {e}", repo_path.display())))?;

    if let Some(allowed_repository) = allowed_repository {
        let resolved_allowed = allowed_repository.canonicalize().map_err(|e| {
            GitError(format!(
                "Invalid allowed repository '{}': {e}",
                allowed_repository.display()
            ))
        })?;

        if !path_is_within(&resolved_repo, &resolved_allowed) {
            return Err(GitError(format!(
                "Repository path '{}' is outside the allowed repository '{}'",
                repo_path.display(),
                allowed_repository.display()
            )));
        }
    }

    let top_level = run_git(&resolved_repo, ["rev-parse", "--show-toplevel"]).map_err(|_| {
        GitError(format!(
            "'{}' is not a valid Git repository",
            repo_path.display()
        ))
    })?;
    PathBuf::from(top_level.trim())
        .canonicalize()
        .map_err(|e| GitError(format!("Invalid Git repository root: {e}")))
}

fn validate_repo_file(repo_root: &Path, file: &str) -> GitResult<()> {
    if file.is_empty() {
        return Err(GitError("File path must not be empty".to_string()));
    }
    if file == "." {
        return Ok(());
    }

    let candidate = if Path::new(file).is_absolute() {
        PathBuf::from(file)
    } else {
        repo_root.join(file)
    };
    let candidate = normalize_lexically(&candidate);
    if !path_is_within(&candidate, repo_root) {
        return Err(GitError(format!(
            "Path '{file}' is outside the repository '{}'",
            repo_root.display()
        )));
    }
    Ok(())
}

pub fn git_status(repo_root: &Path) -> GitResult<String> {
    run_git(repo_root, ["status"])
}

pub fn git_diff_unstaged(repo_root: &Path, context_lines: i64) -> GitResult<String> {
    run_git(
        repo_root,
        [
            "diff".to_string(),
            format!("--unified={}", context_lines.max(0)),
        ],
    )
}

pub fn git_diff_staged(repo_root: &Path, context_lines: i64) -> GitResult<String> {
    run_git(
        repo_root,
        [
            "diff".to_string(),
            format!("--unified={}", context_lines.max(0)),
            "--cached".to_string(),
        ],
    )
}

pub fn git_diff(repo_root: &Path, target: &str, context_lines: i64) -> GitResult<String> {
    reject_flag_like(target, "target")?;
    run_git(repo_root, ["rev-parse", "--verify", target])?;
    run_git(
        repo_root,
        [
            "diff".to_string(),
            format!("--unified={}", context_lines.max(0)),
            target.to_string(),
        ],
    )
}

pub fn git_commit(repo_root: &Path, message: &str) -> GitResult<String> {
    if message.trim().is_empty() {
        return Err(GitError("Commit message must not be empty".to_string()));
    }
    run_git(repo_root, ["commit", "-m", message])?;
    let hash = run_git(repo_root, ["rev-parse", "HEAD"])?;
    Ok(format!("Changes committed successfully with hash {hash}"))
}

pub fn git_add(repo_root: &Path, files: &[String]) -> GitResult<String> {
    if files.is_empty() {
        return Err(GitError("Files must contain at least one path".to_string()));
    }

    if files == [String::from(".")] {
        run_git(repo_root, ["add", "."])?;
    } else {
        for file in files {
            validate_repo_file(repo_root, file)?;
        }
        let mut args = vec!["add".to_string(), "--".to_string()];
        args.extend(files.iter().cloned());
        run_git(repo_root, args)?;
    }
    Ok("Files staged successfully".to_string())
}

pub fn git_reset(repo_root: &Path) -> GitResult<String> {
    run_git(repo_root, ["reset"])?;
    Ok("All staged changes reset".to_string())
}

pub fn git_log(
    repo_root: &Path,
    max_count: i64,
    start_timestamp: Option<&str>,
    end_timestamp: Option<&str>,
) -> GitResult<Vec<String>> {
    if let Some(value) = start_timestamp {
        reject_flag_like(value, "start_timestamp")?;
    }
    if let Some(value) = end_timestamp {
        reject_flag_like(value, "end_timestamp")?;
    }

    let mut args = vec![
        "log".to_string(),
        format!("--max-count={}", max_count.max(1)),
        "--format=Commit: %H%nAuthor: %an%nDate: %ad%nMessage: %s%n".to_string(),
    ];
    if let Some(value) = start_timestamp {
        args.push("--since".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = end_timestamp {
        args.push("--until".to_string());
        args.push(value.to_string());
    }

    let output = run_git(repo_root, args)?;
    if output.trim().is_empty() {
        return Ok(Vec::new());
    }

    Ok(output
        .split("\n\n")
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| {
            if chunk.ends_with('\n') {
                chunk.to_string()
            } else {
                format!("{chunk}\n")
            }
        })
        .collect())
}

pub fn git_create_branch(
    repo_root: &Path,
    branch_name: &str,
    base_branch: Option<&str>,
) -> GitResult<String> {
    reject_flag_like(branch_name, "branch name")?;
    run_git(repo_root, ["check-ref-format", "--branch", branch_name])?;

    let base_name = if let Some(base_branch) = base_branch {
        reject_flag_like(base_branch, "base branch")?;
        run_git(repo_root, ["rev-parse", "--verify", base_branch])?;
        run_git(repo_root, ["branch", branch_name, base_branch])?;
        base_branch.to_string()
    } else {
        let active = run_git(repo_root, ["rev-parse", "--abbrev-ref", "HEAD"])?;
        run_git(repo_root, ["branch", branch_name])?;
        active
    };

    Ok(format!("Created branch '{branch_name}' from '{base_name}'"))
}

pub fn git_checkout(repo_root: &Path, branch_name: &str) -> GitResult<String> {
    reject_flag_like(branch_name, "branch name")?;
    run_git(repo_root, ["rev-parse", "--verify", branch_name])?;
    run_git(repo_root, ["checkout", branch_name])?;
    Ok(format!("Switched to branch '{branch_name}'"))
}

pub fn git_show(repo_root: &Path, revision: &str) -> GitResult<String> {
    reject_flag_like(revision, "revision")?;
    run_git(repo_root, ["rev-parse", "--verify", revision])?;
    run_git(
        repo_root,
        [
            "show",
            "--format=Commit: %H%nAuthor: %an%nDate: %ad%nMessage: %B",
            "--patch",
            revision,
        ],
    )
}

pub fn git_branch(
    repo_root: &Path,
    branch_type: &str,
    contains: Option<&str>,
    not_contains: Option<&str>,
) -> GitResult<String> {
    if let Some(value) = contains {
        reject_flag_like(value, "contains value")?;
    }
    if let Some(value) = not_contains {
        reject_flag_like(value, "not_contains value")?;
    }

    let mut args = vec!["branch".to_string()];
    match branch_type {
        "local" => {}
        "remote" => args.push("-r".to_string()),
        "all" => args.push("-a".to_string()),
        other => return Ok(format!("Invalid branch type: {other}")),
    }
    if let Some(value) = contains {
        args.push("--contains".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = not_contains {
        args.push("--no-contains".to_string());
        args.push(value.to_string());
    }

    run_git(repo_root, args)
}

impl ServerHandler for GitServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        ServerInfo::new(caps)
            .with_server_info(Implementation::new("git-server", env!("CARGO_PKG_VERSION")))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::all_tools()
            .into_iter()
            .find(|tool| tool.name.as_ref() == name)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(Self::all_tools()))
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
        let repo_path = extract_string(args, "repo_path")?;
        let repo_root = self.resolve_repo(repo_path)?;

        match request.name.as_ref() {
            "git_status" => Ok(text_result(format!(
                "Repository status:\n{}",
                git_status(&repo_root).map_err(invalid_git_error)?
            ))),
            "git_diff_unstaged" => {
                let context = extract_i64(args, "context_lines", DEFAULT_CONTEXT_LINES);
                Ok(text_result(format!(
                    "Unstaged changes:\n{}",
                    git_diff_unstaged(&repo_root, context).map_err(invalid_git_error)?
                )))
            }
            "git_diff_staged" => {
                let context = extract_i64(args, "context_lines", DEFAULT_CONTEXT_LINES);
                Ok(text_result(format!(
                    "Staged changes:\n{}",
                    git_diff_staged(&repo_root, context).map_err(invalid_git_error)?
                )))
            }
            "git_diff" => {
                let target = extract_string(args, "target")?;
                let context = extract_i64(args, "context_lines", DEFAULT_CONTEXT_LINES);
                Ok(text_result(format!(
                    "Diff with {target}:\n{}",
                    git_diff(&repo_root, target, context).map_err(invalid_git_error)?
                )))
            }
            "git_commit" => {
                let message = extract_string(args, "message")?;
                Ok(text_result(
                    git_commit(&repo_root, message).map_err(invalid_git_error)?,
                ))
            }
            "git_add" => {
                let files = extract_string_array(args, "files")?;
                Ok(text_result(
                    git_add(&repo_root, &files).map_err(invalid_git_error)?,
                ))
            }
            "git_reset" => Ok(text_result(
                git_reset(&repo_root).map_err(invalid_git_error)?,
            )),
            "git_log" => {
                let max_count = extract_i64(args, "max_count", 10);
                let start = args.get("start_timestamp").and_then(|v| v.as_str());
                let end = args.get("end_timestamp").and_then(|v| v.as_str());
                let log = git_log(&repo_root, max_count, start, end).map_err(invalid_git_error)?;
                Ok(text_result(format!("Commit history:\n{}", log.join("\n"))))
            }
            "git_create_branch" => {
                let branch_name = extract_string(args, "branch_name")?;
                let base_branch = args.get("base_branch").and_then(|v| v.as_str());
                Ok(text_result(
                    git_create_branch(&repo_root, branch_name, base_branch)
                        .map_err(invalid_git_error)?,
                ))
            }
            "git_checkout" => {
                let branch_name = extract_string(args, "branch_name")?;
                Ok(text_result(
                    git_checkout(&repo_root, branch_name).map_err(invalid_git_error)?,
                ))
            }
            "git_show" => {
                let revision = extract_string(args, "revision")?;
                Ok(text_result(
                    git_show(&repo_root, revision).map_err(invalid_git_error)?,
                ))
            }
            "git_branch" => {
                let branch_type = extract_string(args, "branch_type")?;
                let contains = args.get("contains").and_then(|v| v.as_str());
                let not_contains = args.get("not_contains").and_then(|v| v.as_str());
                Ok(text_result(
                    git_branch(&repo_root, branch_type, contains, not_contains)
                        .map_err(invalid_git_error)?,
                ))
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
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestRepo {
        path: PathBuf,
    }

    impl TestRepo {
        fn new() -> Self {
            let pid = std::process::id();
            let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("git-server-test-{pid}-{seq}"));
            fs::create_dir_all(&path).unwrap();
            run_no_repo(&path, ["git", "init"]).unwrap();
            run_git(&path, ["config", "user.email", "test@example.com"]).unwrap();
            run_git(&path, ["config", "user.name", "Test User"]).unwrap();
            fs::write(path.join("test.txt"), "test\n").unwrap();
            run_git(&path, ["add", "test.txt"]).unwrap();
            run_git(&path, ["commit", "-m", "initial commit"]).unwrap();
            Self { path }
        }

        fn root(&self) -> PathBuf {
            validate_repo_path(&self.path, None).unwrap()
        }

        fn write(&self, rel: &str, content: &str) {
            fs::write(self.path.join(rel), content).unwrap();
        }
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run_no_repo<I, S>(cwd: &Path, args: I) -> GitResult<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut iter = args.into_iter();
        let program = iter.next().unwrap();
        let output = Command::new(program)
            .current_dir(cwd)
            .args(iter)
            .output()
            .map_err(|e| GitError(format!("failed command: {e}")))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(GitError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    #[test]
    fn test_tool_names_and_count() {
        let tools = GitServer::default().get_tools();
        let names: Vec<_> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        assert_eq!(tools.len(), 12);
        assert_eq!(
            names,
            vec![
                "git_status",
                "git_diff_unstaged",
                "git_diff_staged",
                "git_diff",
                "git_commit",
                "git_add",
                "git_reset",
                "git_log",
                "git_create_branch",
                "git_checkout",
                "git_show",
                "git_branch",
            ]
        );
    }

    #[test]
    fn test_no_boolean_json_schema_nodes() {
        for tool in GitServer::default().get_tools() {
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

    #[test]
    fn test_validate_repo_path_rejects_outside_allowed() {
        let repo = TestRepo::new();
        let outside = std::env::temp_dir().join(format!(
            "git-server-outside-{}",
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&outside).unwrap();
        let err = validate_repo_path(&outside, Some(&repo.path)).unwrap_err();
        assert!(err.to_string().contains("outside the allowed repository"));
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn test_validate_repo_path_rejects_non_repo() {
        let dir = std::env::temp_dir().join(format!(
            "git-server-not-repo-{}",
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let err = validate_repo_path(&dir, None).unwrap_err();
        assert!(err.to_string().contains("not a valid Git repository"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_git_status() {
        let repo = TestRepo::new();
        let status = git_status(&repo.root()).unwrap();
        assert!(status.contains("On branch") || status.to_lowercase().contains("branch"));
    }

    #[test]
    fn test_git_diff_unstaged() {
        let repo = TestRepo::new();
        repo.write("test.txt", "modified content\n");
        let diff = git_diff_unstaged(&repo.root(), DEFAULT_CONTEXT_LINES).unwrap();
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("modified content"));
    }

    #[test]
    fn test_git_diff_staged() {
        let repo = TestRepo::new();
        repo.write("staged.txt", "staged content\n");
        git_add(&repo.root(), &[String::from("staged.txt")]).unwrap();
        let diff = git_diff_staged(&repo.root(), DEFAULT_CONTEXT_LINES).unwrap();
        assert!(diff.contains("staged.txt"));
        assert!(diff.contains("staged content"));
    }

    #[test]
    fn test_git_diff_target() {
        let repo = TestRepo::new();
        let root = repo.root();
        let default_branch = run_git(&root, ["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        run_git(&root, ["checkout", "-b", "feature-diff"]).unwrap();
        repo.write("test.txt", "feature changes\n");
        git_add(&root, &[String::from("test.txt")]).unwrap();
        git_commit(&root, "feature commit").unwrap();
        let diff = git_diff(&root, &default_branch, DEFAULT_CONTEXT_LINES).unwrap();
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("feature changes"));
    }

    #[test]
    fn test_git_add_rejects_path_traversal() {
        let repo = TestRepo::new();
        let outside = repo.path.parent().unwrap().join("outside-git-server.txt");
        fs::write(&outside, "secret").unwrap();
        let err = git_add(&repo.root(), &[String::from("../outside-git-server.txt")]).unwrap_err();
        assert!(err.to_string().contains("outside the repository"));
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn test_git_add_and_reset() {
        let repo = TestRepo::new();
        repo.write("reset_test.txt", "content to reset\n");
        let root = repo.root();
        assert_eq!(
            git_add(&root, &[String::from("reset_test.txt")]).unwrap(),
            "Files staged successfully"
        );
        assert!(git_diff_staged(&root, DEFAULT_CONTEXT_LINES)
            .unwrap()
            .contains("reset_test.txt"));
        assert_eq!(git_reset(&root).unwrap(), "All staged changes reset");
        assert!(!git_diff_staged(&root, DEFAULT_CONTEXT_LINES)
            .unwrap()
            .contains("reset_test.txt"));
    }

    #[test]
    fn test_git_commit_and_log() {
        let repo = TestRepo::new();
        let root = repo.root();
        repo.write("commit_test.txt", "content\n");
        git_add(&root, &[String::from("commit_test.txt")]).unwrap();
        let result = git_commit(&root, "test commit message").unwrap();
        assert!(result.contains("Changes committed successfully with hash"));
        let log = git_log(&root, 2, None, None).unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].contains("test commit message"));
    }

    #[test]
    fn test_git_create_branch_and_checkout() {
        let repo = TestRepo::new();
        let root = repo.root();
        let result = git_create_branch(&root, "new-feature-branch", None).unwrap();
        assert!(result.contains("Created branch 'new-feature-branch'"));
        let result = git_checkout(&root, "new-feature-branch").unwrap();
        assert!(result.contains("Switched to branch 'new-feature-branch'"));
        let current = run_git(&root, ["rev-parse", "--abbrev-ref", "HEAD"]).unwrap();
        assert_eq!(current, "new-feature-branch");
    }

    #[test]
    fn test_git_branch() {
        let repo = TestRepo::new();
        let root = repo.root();
        git_create_branch(&root, "new-branch-local", None).unwrap();
        let result = git_branch(&root, "local", None, None).unwrap();
        assert!(result.contains("new-branch-local"));
        assert_eq!(git_branch(&root, "remote", None, None).unwrap().trim(), "");
        assert!(git_branch(&root, "invalid", None, None)
            .unwrap()
            .contains("Invalid branch type"));
    }

    #[test]
    fn test_git_show() {
        let repo = TestRepo::new();
        let root = repo.root();
        repo.write("show_test.txt", "show content\n");
        git_add(&root, &[String::from("show_test.txt")]).unwrap();
        git_commit(&root, "show test commit").unwrap();
        let head = run_git(&root, ["rev-parse", "HEAD"]).unwrap();
        let result = git_show(&root, &head).unwrap();
        assert!(result.contains("Commit:"));
        assert!(result.contains("show test commit"));
        assert!(result.contains("show_test.txt"));
    }

    #[test]
    fn test_rejects_flag_injection() {
        let repo = TestRepo::new();
        let root = repo.root();
        assert!(git_diff(&root, "--output=/tmp/evil", DEFAULT_CONTEXT_LINES).is_err());
        assert!(git_checkout(&root, "--orphan=evil").is_err());
        assert!(git_show(&root, "-p").is_err());
        assert!(git_create_branch(&root, "-f", None).is_err());
        assert!(git_log(&root, 10, Some("--exec=evil"), None).is_err());
        assert!(git_branch(&root, "local", Some("--exec=evil"), None).is_err());
    }
}
