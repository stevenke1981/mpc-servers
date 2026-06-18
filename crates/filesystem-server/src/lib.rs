mod allowed_directories;
mod path_validation;

pub use allowed_directories::AllowedDirectories;

use std::path::Path;
use std::sync::Arc;

use globset::{GlobBuilder, GlobMatcher};
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_json_schema(props: serde_json::Value) -> Arc<JsonObject> {
    let schema = serde_json::json!({
        "type": "object",
        "properties": props,
    });
    let obj = if let serde_json::Value::Object(o) = schema {
        o
    } else {
        unreachable!()
    };
    Arc::new(obj)
}

fn required_schema(props: serde_json::Value, required: &[&str]) -> Arc<JsonObject> {
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

fn extract_string<'a>(args: &'a JsonObject, key: &str) -> Result<&'a str, McpError> {
    args.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
        McpError::invalid_params(format!("Missing required argument: '{key}'"), None)
    })
}

fn extract_bool(args: &JsonObject, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn extract_string_array(args: &JsonObject, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn invalid_path_error(message: String) -> McpError {
    McpError::invalid_params(message, None)
}

fn read_only() -> ToolAnnotations {
    ToolAnnotations::new().read_only(true)
}

fn destructive_annotation(is_idempotent: bool) -> ToolAnnotations {
    ToolAnnotations::new()
        .read_only(false)
        .destructive(true)
        .idempotent(is_idempotent)
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let bytes_f = bytes as f64;
    let i = (bytes_f.log(1024.0).floor() as usize).min(UNITS.len() - 1);
    if i == 0 {
        format!("{} {}", bytes, UNITS[i])
    } else {
        format!("{:.2} {}", bytes_f / 1024f64.powi(i as i32), UNITS[i])
    }
}

/// MIME type mapping by file extension
fn mime_type_for_extension(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

fn classify_mime(mime: &str) -> &'static str {
    if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("audio/") {
        "audio"
    } else {
        "blob"
    }
}

fn build_glob_matcher(pattern: &str) -> Result<GlobMatcher, McpError> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|g| g.compile_matcher())
        .map_err(|e| {
            McpError::invalid_params(format!("Invalid glob pattern '{pattern}': {e}"), None)
        })
}

fn is_excluded(relative_path: &str, exclude_patterns: &[GlobMatcher]) -> bool {
    exclude_patterns
        .iter()
        .any(|matcher| matcher.is_match(relative_path))
}

fn generate_unified_diff(original: &str, modified: &str, filepath: &str) -> String {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(original, modified);

    // Count hunks and changes
    let mut hunks: Vec<String> = Vec::new();
    let mut current_hunk: Vec<(ChangeTag, String)> = Vec::new();
    let mut hunk_old_start = 1usize;
    let mut hunk_new_start = 1usize;

    let mut old_lineno = 1usize;
    let mut new_lineno = 1usize;

    for change in diff.iter_all_changes() {
        let tag = change.tag();
        let value = change.value().to_string();
        let is_eof = value.is_empty();

        match tag {
            ChangeTag::Equal => {
                if !current_hunk.is_empty() {
                    // Flush hunk
                    if current_hunk.len() > 3 {
                        // Trim context around the hunk
                        let mut trimmed: Vec<(ChangeTag, String)> = Vec::new();
                        trimmed.extend(current_hunk.drain(..2)); // keep first 2 context lines
                        trimmed.push((ChangeTag::Equal, "...\n".to_string()));
                        trimmed.extend(current_hunk.drain(current_hunk.len() - 2..)); // keep last 2
                        current_hunk = trimmed;
                    }
                    let hunk_text: String = current_hunk
                        .iter()
                        .map(|(t, v)| match t {
                            ChangeTag::Delete => format!("-{v}"),
                            ChangeTag::Insert => format!("+{v}"),
                            ChangeTag::Equal => format!(" {v}"),
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    let old_count = current_hunk
                        .iter()
                        .filter(|(t, _)| *t != ChangeTag::Insert)
                        .count();
                    let new_count = current_hunk
                        .iter()
                        .filter(|(t, _)| *t != ChangeTag::Delete)
                        .count();

                    hunks.push(format!(
                        "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n{hunk_text}",
                        old_start = hunk_old_start.saturating_sub(1).max(1),
                        new_start = hunk_new_start.saturating_sub(1).max(1),
                    ));
                    current_hunk.clear();
                }
                if !is_eof {
                    old_lineno += 1;
                    new_lineno += 1;
                }
                hunk_old_start = old_lineno;
                hunk_new_start = new_lineno;
            }
            ChangeTag::Delete => {
                if current_hunk.is_empty() {
                    // Include 2 context lines before
                    hunk_old_start = old_lineno.saturating_sub(2).max(1);
                    hunk_new_start = new_lineno.saturating_sub(2).max(1);
                }
                current_hunk.push((tag, value));
                if !is_eof {
                    old_lineno += 1;
                }
            }
            ChangeTag::Insert => {
                if current_hunk.is_empty() {
                    hunk_old_start = old_lineno.saturating_sub(2).max(1);
                    hunk_new_start = new_lineno.saturating_sub(2).max(1);
                }
                current_hunk.push((tag, value));
                if !is_eof {
                    new_lineno += 1;
                }
            }
        }
    }

    // Flush last hunk
    if !current_hunk.is_empty() {
        let hunk_text: String = current_hunk
            .iter()
            .map(|(t, v)| match t {
                ChangeTag::Delete => format!("-{v}"),
                ChangeTag::Insert => format!("+{v}"),
                ChangeTag::Equal => format!(" {v}"),
            })
            .collect::<Vec<_>>()
            .join("");

        let old_count = current_hunk
            .iter()
            .filter(|(t, _)| *t != ChangeTag::Insert)
            .count();
        let new_count = current_hunk
            .iter()
            .filter(|(t, _)| *t != ChangeTag::Delete)
            .count();

        hunks.push(format!(
            "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n{hunk_text}",
            old_start = hunk_old_start.max(1),
            new_start = hunk_new_start.max(1),
        ));
    }

    if hunks.is_empty() {
        return String::new();
    }

    let diff_body = hunks.join("");
    format!("--- {filepath}\n+++ {filepath}\n{diff_body}")
}

fn wrap_diff_in_backticks(diff: &str) -> String {
    let mut num_backticks = 3usize;
    let backtick_str = "`";
    while diff.contains(&backtick_str.repeat(num_backticks)) {
        num_backticks += 1;
    }
    let ticks = backtick_str.repeat(num_backticks);
    format!("{ticks}diff\n{diff}{ticks}\n\n")
}

// ---------------------------------------------------------------------------
// Tool builders
// ---------------------------------------------------------------------------

fn build_read_text_file_tool(name: &'static str, description: &'static str) -> Tool {
    Tool::new(
        name,
        description,
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file"
                },
                "tail": {
                    "type": "number",
                    "description": "If provided, returns only the last N lines of the file"
                },
                "head": {
                    "type": "number",
                    "description": "If provided, returns only the first N lines of the file"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_read_media_file_tool() -> Tool {
    Tool::new(
        "read_media_file",
        "Read an image or audio file. Returns the base64 encoded data and MIME type.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the media file"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_read_multiple_files_tool() -> Tool {
    Tool::new(
        "read_multiple_files",
        "Read the contents of multiple files simultaneously. Each file's content is \
         returned with its path as a reference.",
        required_schema(
            serde_json::json!({
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Array of file paths to read",
                    "minItems": 1
                }
            }),
            &["paths"],
        ),
    )
    .with_annotations(read_only())
}

fn build_write_file_tool() -> Tool {
    Tool::new(
        "write_file",
        "Create a new file or completely overwrite an existing file with new content. \
         Use with caution as it will overwrite existing files without warning.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "File path to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            }),
            &["path", "content"],
        ),
    )
    .with_annotations(destructive_annotation(true))
}

fn build_edit_file_tool() -> Tool {
    Tool::new(
        "edit_file",
        "Make line-based edits to a text file. Each edit replaces exact line sequences \
         with new content. Returns a git-style diff showing the changes made.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "oldText": {
                                "type": "string",
                                "description": "Text to search for - must match exactly"
                            },
                            "newText": {
                                "type": "string",
                                "description": "Text to replace with"
                            }
                        },
                        "required": ["oldText", "newText"]
                    },
                    "description": "List of edit operations to apply"
                },
                "dryRun": {
                    "type": "boolean",
                    "description": "Preview changes using git-style diff format without applying them",
                    "default": false
                }
            }),
            &["path", "edits"],
        ),
    )
    .with_annotations(destructive_annotation(false))
}

fn build_create_directory_tool() -> Tool {
    Tool::new(
        "create_directory",
        "Create a new directory or ensure a directory exists. If the directory already \
         exists, this operation will succeed silently.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path of the directory to create"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(
        ToolAnnotations::new()
            .read_only(false)
            .destructive(false)
            .idempotent(true),
    )
}

fn build_list_directory_tool() -> Tool {
    Tool::new(
        "list_directory",
        "Get a detailed listing of all files and directories in a specified path. \
         Results clearly distinguish between files and directories with [FILE] and [DIR] prefixes.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path of the directory to list"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_list_directory_with_sizes_tool() -> Tool {
    Tool::new(
        "list_directory_with_sizes",
        "Get a detailed listing of all files and directories in a specified path, \
         including file sizes and a summary.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path of the directory to list"
                },
                "sortBy": {
                    "type": "string",
                    "enum": ["name", "size"],
                    "description": "Sort entries by name or size",
                    "default": "name"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_directory_tree_tool() -> Tool {
    Tool::new(
        "directory_tree",
        "Get a recursive tree view of files and directories as a JSON structure. \
         Each entry includes name, type (file/directory), and children for directories.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Root path for the directory tree"
                },
                "excludePatterns": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Patterns to exclude from the tree (glob format supported)"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_move_file_tool() -> Tool {
    Tool::new(
        "move_file",
        "Move or rename files and directories. Can move files between directories \
         and rename them in a single operation. Fails if the destination exists.",
        required_schema(
            serde_json::json!({
                "source": {
                    "type": "string",
                    "description": "Source file or directory path"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination file or directory path"
                }
            }),
            &["source", "destination"],
        ),
    )
    .with_annotations(destructive_annotation(false))
}

fn build_search_files_tool() -> Tool {
    Tool::new(
        "search_files",
        "Recursively search for files and directories matching a pattern. \
         Uses glob-style pattern matching. Returns full paths to all matching items.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Starting directory for the search"
                },
                "pattern": {
                    "type": "string",
                    "description": "Glob-style search pattern (e.g., '**/*.rs', '*.txt')"
                },
                "excludePatterns": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Patterns to exclude from results (glob format)"
                }
            }),
            &["path", "pattern"],
        ),
    )
    .with_annotations(read_only())
}

fn build_get_file_info_tool() -> Tool {
    Tool::new(
        "get_file_info",
        "Retrieve detailed metadata about a file or directory, including size, \
         creation time, last modified time, and type.",
        required_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory"
                }
            }),
            &["path"],
        ),
    )
    .with_annotations(read_only())
}

fn build_list_allowed_directories_tool() -> Tool {
    Tool::new(
        "list_allowed_directories",
        "Returns the list of directories that this server is allowed to access.",
        build_json_schema(serde_json::json!({})),
    )
    .with_annotations(read_only())
}

// ---------------------------------------------------------------------------
// Tool implementation functions
// ---------------------------------------------------------------------------

/// Read file content as UTF-8 text, optionally limited to head/tail lines.
async fn read_text_file_impl(
    path: &Path,
    tail: Option<usize>,
    head: Option<usize>,
) -> Result<CallToolResult, McpError> {
    if tail.is_some() && head.is_some() {
        return Err(McpError::invalid_params(
            "Cannot specify both head and tail parameters simultaneously",
            None,
        ));
    }

    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {e}"),
            None,
        )
    })?;

    let result = if let Some(n) = tail {
        let lines: Vec<&str> = content.lines().collect();
        let start = lines.len().saturating_sub(n);
        lines[start..].join("\n")
    } else if let Some(n) = head {
        let lines: Vec<&str> = content.lines().collect();
        lines.into_iter().take(n).collect::<Vec<_>>().join("\n")
    } else {
        content
    };

    Ok(text_result(result))
}

/// Read a binary file and return base64-encoded content with MIME type.
async fn read_media_file_impl(path: &Path) -> Result<CallToolResult, McpError> {
    use base64::Engine;
    let data = tokio::fs::read(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {e}"),
            None,
        )
    })?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = mime_type_for_extension(&ext);
    let type_label = classify_mime(mime);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    let content_item = serde_json::json!({
        "type": type_label,
        "data": b64,
        "mimeType": mime
    });

    let text = serde_json::to_string_pretty(&content_item).unwrap_or_default();
    Ok(text_result(text))
}

/// Read multiple files, returning combined text with path headers.
async fn read_multiple_files_impl(
    paths: &[String],
    allowed: &AllowedDirectories,
) -> Result<CallToolResult, McpError> {
    let mut results: Vec<String> = Vec::new();

    for file_path_str in paths {
        let path = Path::new(file_path_str);
        let result = match allowed.validate_path(path) {
            Ok(valid_path) => match tokio::fs::read_to_string(&valid_path).await {
                Ok(content) => format!("{file_path_str}:\n{content}\n"),
                Err(e) => format!("{file_path_str}: Error - {e}"),
            },
            Err(e) => format!("{file_path_str}: Error - {e}"),
        };
        results.push(result);
    }

    Ok(text_result(results.join("\n---\n")))
}

/// Write content to a file with atomic rename for safety.
async fn write_file_impl(path: &Path, content: &str) -> Result<CallToolResult, McpError> {
    use std::io::Write;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create parent directory: {e}"),
                None,
            )
        })?;
    }

    // Try direct write first
    match tokio::fs::write(path, content).await {
        Ok(()) => {
            let text = format!("Successfully wrote to {}", path.display());
            Ok(text_result(text))
        }
        Err(direct_write_error) => {
            // Fallback: write to temp file and rename (atomic on most filesystems)
            let pid = std::process::id();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let temp_path = path.with_extension(format!(".{pid}-{now:016x}.tmp"));

            let write_result = (|| -> Result<(), String> {
                let mut file = std::fs::File::create(&temp_path)
                    .map_err(|e| format!("Failed to create temp file: {e}"))?;
                file.write_all(content.as_bytes())
                    .map_err(|e| format!("Failed to write temp file: {e}"))?;
                file.sync_all()
                    .map_err(|e| format!("Failed to sync: {e}"))?;
                std::fs::rename(&temp_path, path)
                    .map_err(|e| format!("Failed to rename temp file: {e}"))?;
                Ok(())
            })();

            match write_result {
                Ok(()) => {
                    let text = format!("Successfully wrote to {}", path.display());
                    Ok(text_result(text))
                }
                Err(msg) => {
                    let _ = std::fs::remove_file(&temp_path);
                    Err(McpError::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to write file after direct write failed ({direct_write_error}): {msg}"),
                        None,
                    ))
                }
            }
        }
    }
}

/// Apply edits to a file. Supports dry-run mode that returns diff without writing.
async fn edit_file_impl(
    path: &Path,
    edits: &[serde_json::Value],
    dry_run: bool,
) -> Result<CallToolResult, McpError> {
    let content = tokio::fs::read_to_string(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to read file: {e}"),
            None,
        )
    })?;

    let normalized_content = content.replace("\r\n", "\n");
    let mut modified = normalized_content.clone();

    for edit_val in edits {
        let old_text = edit_val
            .get("oldText")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_params("Each edit must have an 'oldText' field", None)
            })?;
        let new_text = edit_val
            .get("newText")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_params("Each edit must have a 'newText' field", None)
            })?;

        let normalized_old = old_text.replace("\r\n", "\n");
        let normalized_new = new_text.replace("\r\n", "\n");

        // Try exact match first
        if modified.contains(&normalized_old) {
            modified = modified.replacen(&normalized_old, &normalized_new, 1);
            continue;
        }

        // Try whitespace-normalized line matching
        let old_lines: Vec<&str> = normalized_old.split('\n').collect();
        let content_lines: Vec<&str> = modified.split('\n').collect();

        let mut match_found = false;
        'outer: for i in 0..=content_lines.len().saturating_sub(old_lines.len()) {
            let is_match = old_lines.iter().enumerate().all(|(j, old_line)| {
                let trimmed_old = old_line.trim();
                content_lines
                    .get(i + j)
                    .map(|l| l.trim() == trimmed_old)
                    .unwrap_or(false)
            });

            if is_match {
                // Preserve indentation from the first matched line
                let original_indent: &str = &content_lines[i]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();

                let new_lines: Vec<String> = normalized_new
                    .split('\n')
                    .enumerate()
                    .map(|(j, line)| {
                        if j == 0 {
                            // Use original indent for first line
                            format!("{original_indent}{}", line.trim_start())
                        } else {
                            // Preserve relative indentation for subsequent lines
                            let old_indent: usize = old_lines
                                .get(j)
                                .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                                .unwrap_or(0);
                            let new_indent: usize =
                                line.chars().take_while(|c| c.is_whitespace()).count();
                            if old_indent > 0 && new_indent > 0 {
                                let relative =
                                    (new_indent as i32 - old_indent as i32).max(0) as usize;
                                format!(
                                    "{}{}{}",
                                    original_indent,
                                    " ".repeat(relative),
                                    line.trim_start()
                                )
                            } else {
                                line.to_string()
                            }
                        }
                    })
                    .collect();

                let mut new_content_lines: Vec<&str> = content_lines.to_vec();
                new_content_lines
                    .splice(i..i + old_lines.len(), new_lines.iter().map(|s| s.as_str()));
                modified = new_content_lines.join("\n");
                match_found = true;
                break 'outer;
            }
        }

        if !match_found {
            return Err(McpError::invalid_params(
                format!("Could not find exact match for edit:\n{old_text}"),
                None,
            ));
        }
    }

    // Generate unified diff
    let diff = generate_unified_diff(&normalized_content, &modified, &path.to_string_lossy());
    let formatted = wrap_diff_in_backticks(&diff);

    if !dry_run {
        // Write the modified content
        tokio::fs::write(path, &modified).await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to write file: {e}"),
                None,
            )
        })?;
    }

    Ok(text_result(formatted))
}

/// Create a directory (mkdir -p).
async fn create_directory_impl(path: &Path) -> Result<CallToolResult, McpError> {
    tokio::fs::create_dir_all(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to create directory: {e}"),
            None,
        )
    })?;

    let text = format!("Successfully created directory {}", path.display());
    Ok(text_result(text))
}

/// List directory contents with [FILE] / [DIR] prefixes.
async fn list_directory_impl(path: &Path) -> Result<CallToolResult, McpError> {
    let mut entries = tokio::fs::read_dir(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to list directory: {e}"),
            None,
        )
    })?;

    let mut formatted: Vec<String> = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Error reading entry: {e}"),
            None,
        )
    })? {
        let file_type = entry.file_type().await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Error getting file type: {e}"),
                None,
            )
        })?;
        let prefix = if file_type.is_dir() {
            "[DIR]"
        } else {
            "[FILE]"
        };
        formatted.push(format!("{prefix} {}", entry.file_name().to_string_lossy()));
    }

    formatted.sort();
    Ok(text_result(formatted.join("\n")))
}

/// List directory with file sizes and summary.
async fn list_directory_with_sizes_impl(
    path: &Path,
    sort_by: &str,
) -> Result<CallToolResult, McpError> {
    let mut entries = tokio::fs::read_dir(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to list directory: {e}"),
            None,
        )
    })?;

    struct EntryInfo {
        name: String,
        is_dir: bool,
        size: u64,
    }

    let mut detailed: Vec<EntryInfo> = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Error reading entry: {e}"),
            None,
        )
    })? {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry
            .file_type()
            .await
            .map(|ft| ft.is_dir())
            .unwrap_or(false);

        let size = if is_dir {
            0
        } else {
            match entry.metadata().await {
                Ok(meta) => meta.len(),
                Err(_) => 0,
            }
        };

        detailed.push(EntryInfo { name, is_dir, size });
    }

    // Sort
    match sort_by {
        "size" => detailed.sort_by_key(|entry| std::cmp::Reverse(entry.size)),
        _ => detailed.sort_by_key(|entry| entry.name.to_lowercase()),
    }

    // Format
    let mut output: Vec<String> = Vec::new();
    let mut total_files = 0u64;
    let mut total_dirs = 0u64;
    let mut total_size = 0u64;

    for entry in &detailed {
        let prefix = if entry.is_dir { "[DIR]" } else { "[FILE]" };
        let name_padded = format!("{:<30}", entry.name);
        if entry.is_dir {
            output.push(format!("{prefix} {name_padded}"));
            total_dirs += 1;
        } else {
            let size_str = format_file_size(entry.size);
            let size_padded = format!("{:>10}", size_str);
            output.push(format!("{prefix} {name_padded} {size_padded}"));
            total_files += 1;
            total_size += entry.size;
        }
    }

    output.push(String::new());
    output.push(format!(
        "Total: {total_files} files, {total_dirs} directories"
    ));
    output.push(format!("Combined size: {}", format_file_size(total_size)));

    Ok(text_result(output.join("\n")))
}

/// Build a recursive directory tree as JSON.
async fn directory_tree_impl(
    path: &Path,
    exclude_patterns: &[String],
    allowed: &AllowedDirectories,
) -> Result<CallToolResult, McpError> {
    let matchers: Vec<GlobMatcher> = exclude_patterns
        .iter()
        .map(|p| build_glob_matcher(p))
        .collect::<Result<Vec<_>, _>>()?;

    async fn build_tree(
        current_path: &Path,
        root_path: &Path,
        matchers: &[GlobMatcher],
        allowed: &AllowedDirectories,
    ) -> Result<Vec<serde_json::Value>, McpError> {
        let mut entries = tokio::fs::read_dir(current_path).await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read directory: {e}"),
                None,
            )
        })?;

        let mut result: Vec<serde_json::Value> = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Error reading entry: {e}"),
                None,
            )
        })? {
            let name = entry.file_name().to_string_lossy().to_string();

            // Check exclusion patterns
            if let Ok(rel) = entry.path().strip_prefix(root_path) {
                let rel_str = rel.to_string_lossy();
                if is_excluded(&rel_str, matchers) {
                    continue;
                }
            }

            let file_type = entry.file_type().await.map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Error getting file type: {e}"),
                    None,
                )
            })?;

            if file_type.is_dir() {
                let children =
                    Box::pin(build_tree(&entry.path(), root_path, matchers, allowed)).await?;
                result.push(serde_json::json!({
                    "name": name,
                    "type": "directory",
                    "children": children
                }));
            } else {
                // Validate the file is within allowed dirs
                if allowed.validate_path(&entry.path()).is_ok() {
                    result.push(serde_json::json!({
                        "name": name,
                        "type": "file"
                    }));
                }
            }
        }

        Ok(result)
    }

    let tree = build_tree(path, path, &matchers, allowed).await?;
    let text = serde_json::to_string_pretty(&tree).unwrap_or_else(|_| "[]".to_string());
    Ok(text_result(text))
}

/// Move or rename a file/directory.
async fn move_file_impl(source: &Path, destination: &Path) -> Result<CallToolResult, McpError> {
    // Check if destination exists
    if destination.exists() {
        return Err(McpError::invalid_params(
            format!("Destination already exists: {}", destination.display()),
            None,
        ));
    }

    // Ensure parent directory of destination exists
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create parent directory: {e}"),
                None,
            )
        })?;
    }

    tokio::fs::rename(source, destination).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to move file: {e}"),
            None,
        )
    })?;

    let text = format!(
        "Successfully moved {} to {}",
        source.display(),
        destination.display()
    );
    Ok(text_result(text))
}

/// Recursively search files matching a glob pattern.
async fn search_files_impl(
    path: &Path,
    pattern: &str,
    exclude_patterns: &[String],
    allowed: &AllowedDirectories,
) -> Result<CallToolResult, McpError> {
    let matcher = build_glob_matcher(pattern)?;
    let exclude_matchers: Vec<GlobMatcher> = exclude_patterns
        .iter()
        .map(|p| build_glob_matcher(p))
        .collect::<Result<Vec<_>, _>>()?;

    let mut results: Vec<String> = Vec::new();

    async fn walk_search(
        current_path: &Path,
        root_path: &Path,
        matcher: &GlobMatcher,
        exclude_matchers: &[GlobMatcher],
        allowed: &AllowedDirectories,
        results: &mut Vec<String>,
    ) -> Result<(), McpError> {
        let mut entries = tokio::fs::read_dir(current_path).await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read directory: {e}"),
                None,
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            McpError::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Error reading entry: {e}"),
                None,
            )
        })? {
            let full_path = entry.path();

            // Validate path is within allowed dirs
            if allowed.validate_path(&full_path).is_err() {
                continue;
            }

            let rel_path = full_path
                .strip_prefix(root_path)
                .unwrap_or(&full_path)
                .to_string_lossy()
                .to_string();

            // Check exclusion
            if is_excluded(&rel_path, exclude_matchers) {
                continue;
            }

            // Check pattern match
            if matcher.is_match(&rel_path) {
                results.push(full_path.to_string_lossy().to_string());
            }

            let file_type = entry.file_type().await.map_err(|e| {
                McpError::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Error getting file type: {e}"),
                    None,
                )
            })?;

            if file_type.is_dir() {
                Box::pin(walk_search(
                    &full_path,
                    root_path,
                    matcher,
                    exclude_matchers,
                    allowed,
                    results,
                ))
                .await?;
            }
        }

        Ok(())
    }

    walk_search(
        path,
        path,
        &matcher,
        &exclude_matchers,
        allowed,
        &mut results,
    )
    .await?;

    let text = if results.is_empty() {
        "No matches found".to_string()
    } else {
        results.join("\n")
    };

    Ok(text_result(text))
}

/// Get file metadata.
async fn get_file_info_impl(path: &Path) -> Result<CallToolResult, McpError> {
    let metadata = tokio::fs::metadata(path).await.map_err(|e| {
        McpError::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to get file info: {e}"),
            None,
        )
    })?;

    let file_type = if metadata.is_dir() {
        "directory"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };

    let created = metadata
        .created()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let modified = metadata
        .modified()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let accessed = metadata
        .accessed()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        })
        .unwrap_or(0);

    // Format permissions similar to Unix-style.
    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::MetadataExt;
        format!("{:o}", metadata.mode())
    };
    #[cfg(not(unix))]
    let permissions = {
        let perms = metadata.permissions();
        format!("{:03o}", if perms.readonly() { 0o444 } else { 0o644 })
    };

    let lines = [
        format!("size: {}", metadata.len()),
        format!("created: {created}"),
        format!("modified: {modified}"),
        format!("accessed: {accessed}"),
        format!("isDirectory: {}", metadata.is_dir()),
        format!("isFile: {}", metadata.is_file()),
        format!("type: {file_type}"),
        format!("permissions: {permissions}"),
    ];

    Ok(text_result(lines.join("\n")))
}

/// List configured allowed directories.
fn list_allowed_directories_impl(allowed: &AllowedDirectories) -> CallToolResult {
    let dirs = allowed.list_allowed_directories();
    let lines: Vec<String> = dirs
        .iter()
        .map(|d| d.to_string_lossy().to_string())
        .collect();
    let text = if lines.is_empty() {
        "Allowed directories:\n(empty - no directories configured)".to_string()
    } else {
        let mut output = "Allowed directories:\n".to_string();
        output.push_str(&lines.join("\n"));
        output
    };
    text_result(text)
}

// ---------------------------------------------------------------------------
// MCP handler
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct FilesystemServer {
    allowed: AllowedDirectories,
}

impl FilesystemServer {
    pub fn new(allowed: AllowedDirectories) -> Self {
        Self { allowed }
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        Self::all_tools()
    }

    fn all_tools() -> Vec<Tool> {
        vec![
            build_read_text_file_tool(
                "read_file",
                "Read the complete contents of a file as text. DEPRECATED: Use read_text_file instead.",
            ),
            build_read_text_file_tool(
                "read_text_file",
                "Read the complete contents of a file from the file system as text. \
                 Handles various text encodings and provides detailed error messages \
                 if the file cannot be read.",
            ),
            build_read_media_file_tool(),
            build_read_multiple_files_tool(),
            build_write_file_tool(),
            build_edit_file_tool(),
            build_create_directory_tool(),
            build_list_directory_tool(),
            build_list_directory_with_sizes_tool(),
            build_directory_tree_tool(),
            build_move_file_tool(),
            build_search_files_tool(),
            build_get_file_info_tool(),
            build_list_allowed_directories_tool(),
        ]
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

    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::all_tools()
            .into_iter()
            .find(|t| t.name.as_ref() == name)
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
        let args = request.arguments.as_ref();

        match request.name.as_ref() {
            "read_file" | "read_text_file" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let tail = args
                    .get("tail")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                let head = args
                    .get("head")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize);
                let valid_path = self
                    .allowed
                    .validate_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                read_text_file_impl(&valid_path, tail, head).await
            }

            "read_media_file" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let valid_path = self
                    .allowed
                    .validate_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                read_media_file_impl(&valid_path).await
            }

            "read_multiple_files" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let paths: Vec<String> = args
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .ok_or_else(|| {
                        McpError::invalid_params("Missing or invalid 'paths' argument", None)
                    })?;
                if paths.is_empty() {
                    return Err(McpError::invalid_params(
                        "'paths' must contain at least one file path",
                        None,
                    ));
                }
                read_multiple_files_impl(&paths, &self.allowed).await
            }

            "write_file" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let content = extract_string(args, "content")?;
                let valid_path = self
                    .allowed
                    .validate_candidate_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                write_file_impl(&valid_path, content).await
            }

            "edit_file" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let dry_run = extract_bool(args, "dryRun", false);
                let edits = args
                    .get("edits")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .ok_or_else(|| {
                        McpError::invalid_params("Missing or invalid 'edits' argument", None)
                    })?;
                if edits.is_empty() {
                    return Err(McpError::invalid_params(
                        "'edits' must contain at least one edit operation",
                        None,
                    ));
                }
                let valid_path = self
                    .allowed
                    .validate_existing_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                edit_file_impl(&valid_path, &edits, dry_run).await
            }

            "create_directory" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let valid_path = self
                    .allowed
                    .validate_candidate_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                create_directory_impl(&valid_path).await
            }

            "list_directory" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let valid_path = self
                    .allowed
                    .validate_existing_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                list_directory_impl(&valid_path).await
            }

            "list_directory_with_sizes" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let sort_by = args
                    .get("sortBy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("name");
                let valid_path = self
                    .allowed
                    .validate_existing_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                list_directory_with_sizes_impl(&valid_path, sort_by).await
            }

            "directory_tree" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let exclude_patterns = extract_string_array(args, "excludePatterns");
                let valid_path = self
                    .allowed
                    .validate_existing_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                directory_tree_impl(&valid_path, &exclude_patterns, &self.allowed).await
            }

            "move_file" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let source = extract_string(args, "source")?;
                let destination = extract_string(args, "destination")?;
                let valid_source = self
                    .allowed
                    .validate_existing_path(Path::new(source))
                    .map_err(invalid_path_error)?;
                let valid_dest = self
                    .allowed
                    .validate_candidate_path(Path::new(destination))
                    .map_err(invalid_path_error)?;
                move_file_impl(&valid_source, &valid_dest).await
            }

            "search_files" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let pattern = extract_string(args, "pattern")?;
                let exclude_patterns = extract_string_array(args, "excludePatterns");
                let valid_path = self
                    .allowed
                    .validate_existing_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                search_files_impl(&valid_path, pattern, &exclude_patterns, &self.allowed).await
            }

            "get_file_info" => {
                let args =
                    args.ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                let path_str = extract_string(args, "path")?;
                let valid_path = self
                    .allowed
                    .validate_path(Path::new(path_str))
                    .map_err(invalid_path_error)?;
                get_file_info_impl(&valid_path).await
            }

            "list_allowed_directories" => Ok(list_allowed_directories_impl(&self.allowed)),

            _ => Err(McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    trait RawTextContentTestExt {
        fn contains(&self, needle: &str) -> bool;
    }

    impl RawTextContentTestExt for rmcp::model::RawTextContent {
        fn contains(&self, needle: &str) -> bool {
            self.text.contains(needle)
        }
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let pid = std::process::id();
            let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("fs-tools-test-{pid}-{seq}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn join(&self, rel: &str) -> PathBuf {
            self.path.join(rel)
        }

        fn mkdir(&self, rel: &str) -> PathBuf {
            let p = self.path.join(rel);
            fs::create_dir_all(&p).unwrap();
            p
        }

        fn write(&self, rel: &str, content: &str) -> PathBuf {
            let p = self.path.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut f = File::create(&p).unwrap();
            f.write_all(content.as_bytes()).unwrap();
            p
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn make_allowed(td: &TestDir) -> AllowedDirectories {
        let dir = td.mkdir("allowed");
        AllowedDirectories::from_existing_dirs(&[&dir]).unwrap()
    }

    // -----------------------------------------------------------------------
    // Tool schema tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_tool_names() {
        let tools = FilesystemServer::all_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"read_text_file"));
        assert!(names.contains(&"read_media_file"));
        assert!(names.contains(&"read_multiple_files"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"create_directory"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"list_directory_with_sizes"));
        assert!(names.contains(&"directory_tree"));
        assert!(names.contains(&"move_file"));
        assert!(names.contains(&"search_files"));
        assert!(names.contains(&"get_file_info"));
        assert!(names.contains(&"list_allowed_directories"));
        assert_eq!(names.len(), 14);
    }

    #[test]
    fn test_tool_name_read_file_deprecated() {
        let tool = build_read_text_file_tool("read_file", "deprecated");
        assert_eq!(tool.name.as_ref(), "read_file");
    }

    #[test]
    fn test_no_boolean_json_schema_nodes() {
        for tool in FilesystemServer::all_tools() {
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

    // -----------------------------------------------------------------------
    // read_text_file tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_read_text_file_whole_file() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/test.txt", "line1\nline2\nline3\n");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = read_text_file_impl(&valid, None, None).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert_eq!(text.text, "line1\nline2\nline3\n");
    }

    #[tokio::test]
    async fn test_read_text_file_head() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/test.txt", "line1\nline2\nline3\nline4\nline5\n");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = read_text_file_impl(&valid, None, Some(2)).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert_eq!(text.text, "line1\nline2");
    }

    #[tokio::test]
    async fn test_read_text_file_tail() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/test.txt", "line1\nline2\nline3\nline4\nline5\n");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = read_text_file_impl(&valid, Some(2), None).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert_eq!(text.text, "line4\nline5");
    }

    #[tokio::test]
    async fn test_read_text_file_head_and_tail_rejected() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/test.txt", "content\n");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = read_text_file_impl(&valid, Some(2), Some(2)).await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // write_file tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_write_file_new_file() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.join("allowed/new_file.txt");
        let valid = allowed.validate_candidate_path(&file).unwrap();
        let result = write_file_impl(&valid, "Hello, World!").await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("Successfully wrote to"));

        let content = fs::read_to_string(&valid).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file_overwrite_existing() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/existing.txt", "old content\n");
        let valid = allowed.validate_existing_path(&file).unwrap();
        write_file_impl(&valid, "new content\n").await.unwrap();
        let content = fs::read_to_string(&valid).unwrap();
        assert_eq!(content, "new content\n");
    }

    // -----------------------------------------------------------------------
    // edit_file tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_edit_file_exact_match() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/edit.txt", "hello\nworld\nfoo\n");
        let valid = allowed.validate_existing_path(&file).unwrap();

        let edits = vec![serde_json::json!({
            "oldText": "world",
            "newText": "earth"
        })];

        let result = edit_file_impl(&valid, &edits, false).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("diff"));

        let content = fs::read_to_string(&valid).unwrap();
        assert_eq!(content, "hello\nearth\nfoo\n");
    }

    #[tokio::test]
    async fn test_edit_file_dry_run() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/dry_run.txt", "original\ncontent\n");
        let valid = allowed.validate_existing_path(&file).unwrap();

        let edits = vec![serde_json::json!({
            "oldText": "original",
            "newText": "modified"
        })];

        // dryRun = true
        let result = edit_file_impl(&valid, &edits, true).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("diff"));

        // File should remain unchanged
        let content = fs::read_to_string(&valid).unwrap();
        assert_eq!(content, "original\ncontent\n");
    }

    #[tokio::test]
    async fn test_edit_file_match_not_found() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/notfound.txt", "some content\n");
        let valid = allowed.validate_existing_path(&file).unwrap();

        let edits = vec![serde_json::json!({
            "oldText": "nonexistent text",
            "newText": "replacement"
        })];

        let result = edit_file_impl(&valid, &edits, false).await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // create_directory tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_create_directory_new() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let dir = td.join("allowed/newdir");
        let valid = allowed.validate_candidate_path(&dir).unwrap();
        let result = create_directory_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("Successfully created directory"));
        assert!(dir.exists());
    }

    #[tokio::test]
    async fn test_create_directory_already_exists() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let dir = td.mkdir("allowed/existing");
        let valid = allowed.validate_existing_path(&dir).unwrap();
        let result = create_directory_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("Successfully created directory"));
    }

    // -----------------------------------------------------------------------
    // list_directory tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_directory() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let dir = td.mkdir("allowed/sub");
        td.write("allowed/a.txt", "a");
        td.write("allowed/b.txt", "b");
        let valid = allowed
            .validate_existing_path(dir.parent().unwrap())
            .unwrap();
        let result = list_directory_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("[FILE] a.txt"));
        assert!(text.contains("[FILE] b.txt"));
        assert!(text.contains("[DIR] sub"));
    }

    // -----------------------------------------------------------------------
    // list_directory_with_sizes tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_directory_with_sizes() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        td.write("allowed/small.txt", "small");
        td.write("allowed/big.txt", "this is a larger file content here");
        let dir = td.path.join("allowed");
        let valid = allowed.validate_existing_path(&dir).unwrap();
        let result = list_directory_with_sizes_impl(&valid, "name")
            .await
            .unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("[FILE]"));
        assert!(text.contains("Total:"));
        assert!(text.contains("Combined size:"));
    }

    // -----------------------------------------------------------------------
    // move_file tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_move_file_rename() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let src = td.write("allowed/source.txt", "content");
        let dst = td.join("allowed/dest.txt");
        let valid_src = allowed.validate_existing_path(&src).unwrap();
        let valid_dst = allowed.validate_candidate_path(&dst).unwrap();
        let result = move_file_impl(&valid_src, &valid_dst).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("Successfully moved"));
        assert!(!src.exists());
        assert!(dst.exists());
    }

    #[tokio::test]
    async fn test_move_file_destination_exists() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let src = td.write("allowed/src.txt", "src");
        let dst = td.write("allowed/dst.txt", "dst");
        let valid_src = allowed.validate_existing_path(&src).unwrap();
        let valid_dst = allowed.validate_existing_path(&dst).unwrap();
        let result = move_file_impl(&valid_src, &valid_dst).await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // search_files tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_search_files_find_rs() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        td.write("allowed/main.rs", "fn main() {}");
        td.write("allowed/lib.rs", "pub fn lib() {}");
        td.write("allowed/readme.md", "# readme");
        let dir = td.path.join("allowed");
        let valid = allowed.validate_existing_path(&dir).unwrap();
        let result = search_files_impl(&valid, "*.rs", &[], &allowed)
            .await
            .unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("main.rs"));
        assert!(text.contains("lib.rs"));
        assert!(!text.contains("readme.md"));
    }

    #[tokio::test]
    async fn test_search_files_no_matches() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        td.write("allowed/file.txt", "text");
        let dir = td.path.join("allowed");
        let valid = allowed.validate_existing_path(&dir).unwrap();
        let result = search_files_impl(&valid, "*.rs", &[], &allowed)
            .await
            .unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert_eq!(text.text, "No matches found");
    }

    // -----------------------------------------------------------------------
    // get_file_info tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_get_file_info_file() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/info.txt", "content");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = get_file_info_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("size:"));
        assert!(text.contains("isFile: true"));
        assert!(text.contains("type: file"));
    }

    #[tokio::test]
    async fn test_get_file_info_directory() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let dir = td.mkdir("allowed/subdir");
        let valid = allowed.validate_existing_path(&dir).unwrap();
        let result = get_file_info_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("isDirectory: true"));
        assert!(text.contains("type: directory"));
    }

    // -----------------------------------------------------------------------
    // list_allowed_directories tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_allowed_directories_handler() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let result = list_allowed_directories_impl(&allowed);
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("Allowed directories:"));
    }

    // -----------------------------------------------------------------------
    // directory_tree tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_directory_tree_basic() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        td.write("allowed/a.txt", "a");
        td.mkdir("allowed/sub");
        td.write("allowed/sub/b.txt", "b");
        let dir = td.path.join("allowed");
        let valid = allowed.validate_existing_path(&dir).unwrap();
        let result = directory_tree_impl(&valid, &[], &allowed).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("a.txt"));
        assert!(text.contains("sub"));
        assert!(text.contains("b.txt"));
    }

    // -----------------------------------------------------------------------
    // read_media_file tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_read_media_file_png() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        // Write a minimal PNG (just a valid enough binary for base64 encoding)
        let file = td.write("allowed/image.png", "fake-png-content");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = read_media_file_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("image/png"));
    }

    #[tokio::test]
    async fn test_read_media_file_unknown_extension() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let file = td.write("allowed/file.xyz", "binary data");
        let valid = allowed.validate_existing_path(&file).unwrap();
        let result = read_media_file_impl(&valid).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("application/octet-stream"));
        assert!(text.contains("blob"));
    }

    // -----------------------------------------------------------------------
    // read_multiple_files tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_read_multiple_files_basic() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let _f1 = td.write("allowed/f1.txt", "file one");
        let _f2 = td.write("allowed/f2.txt", "file two");

        let paths = vec![
            td.join("allowed/f1.txt").to_string_lossy().to_string(),
            td.join("allowed/f2.txt").to_string_lossy().to_string(),
        ];

        let result = read_multiple_files_impl(&paths, &allowed).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("f1.txt"));
        assert!(text.contains("file one"));
        assert!(text.contains("f2.txt"));
        assert!(text.contains("file two"));
    }

    #[tokio::test]
    async fn test_read_multiple_files_partial_failure() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let _f1 = td.write("allowed/exists.txt", "exists");

        let paths = vec![
            td.join("allowed/exists.txt").to_string_lossy().to_string(),
            td.join("allowed/nonexistent.txt")
                .to_string_lossy()
                .to_string(),
        ];

        let result = read_multiple_files_impl(&paths, &allowed).await.unwrap();
        let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
        assert!(text.contains("exists.txt"));
        assert!(text.contains("exists"));
        assert!(text.contains("nonexistent.txt"));
        assert!(text.contains("Error"));
    }

    // -----------------------------------------------------------------------
    // Server handler integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_server_get_tool() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let server = FilesystemServer::new(allowed);

        assert!(server.get_tool("read_text_file").is_some());
        assert!(server.get_tool("read_file").is_some());
        assert!(server.get_tool("write_file").is_some());
        assert!(server.get_tool("edit_file").is_some());
        assert!(server.get_tool("list_allowed_directories").is_some());
        assert!(server.get_tool("nonexistent_tool").is_none());
    }

    #[test]
    fn test_server_list_tools_count() {
        let td = TestDir::new();
        let allowed = make_allowed(&td);
        let server = FilesystemServer::new(allowed);
        let tools = server.get_tools();
        assert_eq!(tools.len(), 14);
    }

    // -----------------------------------------------------------------------
    // format_file_size tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_file_size_zero() {
        assert_eq!(format_file_size(0), "0 B");
    }

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(format_file_size(500), "500 B");
    }

    #[test]
    fn test_format_file_size_kb() {
        let result = format_file_size(2048);
        assert!(result.contains("KB"));
    }

    #[test]
    fn test_format_file_size_mb() {
        let result = format_file_size(2_097_152);
        assert!(result.contains("MB"));
    }

    // -----------------------------------------------------------------------
    // generate_unified_diff tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_unified_diff_add_line() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        let diff = generate_unified_diff(old, new, "test.txt");
        assert!(diff.contains("--- test.txt"));
        assert!(diff.contains("+++ test.txt"));
        assert!(diff.contains("+line3"));
    }

    #[test]
    fn test_generate_unified_diff_no_changes() {
        let content = "same\ncontent\n";
        let diff = generate_unified_diff(content, content, "test.txt");
        assert!(diff.is_empty() || !diff.contains("---"));
    }
}
