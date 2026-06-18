# Rust MCP Servers Spec

本規格定義 `D:\mpc-servers` 要從 `stevenke1981/servers.git` 與既有 Rust MCP 專案保留下來的功能邊界。任何代理實作 server crate 時，必須先對照本檔，不可只做同名空殼。

## 目標

- 以 Rust 重新實作 MCP servers，優先使用 `rmcp` 與 stdio transport。
- 保留原本專案的相關功能、工具名稱、輸入語意、輸出語意與安全邊界。
- 將可直接使用的既有 Rust 專案整理進 workspace，而不是重寫已經可用的功能。
- 所有 crate、文件、版本號、installer、release 流程都由 git 管理。

## 非目標

- 不在第一階段實作 web/HTTP transport；除非 server 原本就是 compatibility testbed。
- 不為了統一而改掉既有公開 tool name。
- 不把 `.opencode/`、`.codebase-memory/`、`target/` 或 upstream clone 納入版本控制。

## 共通 MCP 規格

- 每個 production server 都必須提供 stdio transport。
- MCP 模式下 stdout 只輸出 JSON-RPC/MCP protocol 訊息；logs/errors 走 stderr。
- 每個 binary 必須支援 `--version`。
- 每個 server crate 必須有 README，包含功能、工具、安裝、範例 client config、驗證命令。
- 每個 server 必須有 `tools/list` smoke test，且要用 OpenCode/TypeScript SDK 路徑驗證 schema，不只用 Rust rmcp client。
- 公開 schema 不可輸出 OpenCode 會拒絕的 boolean JSON Schema node，例如工具欄位 schema 不可是裸 `true`。
- Windows process invocation 必須使用 native argument passing 模式，不依賴 shell-expanded JSON 字串。
- 安裝器 release 預設使用 GitHub Release binary，不預設從 source build。

## 版本規格

- Workspace 起始版本為 `0.1.0`。
- 既有專案導入時先保留原 crate/package version，再評估是否改採 workspace version。
- 可獨立 release 的 server 使用 `<server-name>-vX.Y.Z` tag。
- 全 workspace release 使用 `vX.Y.Z` tag。
- 詳細政策見 `VERSIONING.md`。

## 必須保留的上游功能

### `memory`

來源：

- Reference: `stevenke1981/servers/src/memory`，TypeScript package `@modelcontextprotocol/server-memory@0.6.3`
- Rust line: `stevenke1981/memlong`

上游 reference tool 必須有相容層或替代說明：

- `create_entities`
- `create_relations`
- `add_observations`
- `delete_entities`
- `delete_observations`
- `delete_relations`
- `read_graph`
- `search_nodes`
- `open_nodes`

`memlong` 既有 tool 必須保留：

- `add_memory`
- `search_memories`
- `get_memories`
- `delete_memory`
- `consolidate_memories`
- `get_memory_stats`
- `end_session`

驗收要求：

- Memory 資料必須可持久化。
- 搜尋必須保留 semantic、BM25、temporal 權重能力。
- 若不直接實作 reference graph tools，必須提供 bridge/compat crate 或 README 明確描述 replacement mapping。

### `filesystem`

來源：`stevenke1981/servers/src/filesystem`，TypeScript package `@modelcontextprotocol/server-filesystem@0.6.3`

必須保留的 tools：

- `read_file`，deprecated alias，行為等同 `read_text_file`
- `read_text_file`
- `read_media_file`
- `read_multiple_files`
- `write_file`
- `edit_file`
- `create_directory`
- `list_directory`
- `list_directory_with_sizes`
- `directory_tree`
- `move_file`
- `search_files`
- `get_file_info`
- `list_allowed_directories`

安全要求：

- 所有 path 操作必須限制在 allowed directories 或 MCP Roots 提供的 roots 內。
- 必須防 path traversal、symlink escape、case-insensitive Windows bypass。
- `write_file`、`edit_file`、`move_file` 必須標示 destructive/idempotent annotations。
- `edit_file` 必須支援 dry-run 並回傳 git-style diff。

### `git`

來源：`stevenke1981/servers/src/git`，Python package `mcp-server-git@0.6.2`

必須保留的 tools：

- `git_status`
- `git_diff_unstaged`
- `git_diff_staged`
- `git_diff`
- `git_commit`
- `git_add`
- `git_reset`
- `git_log`
- `git_create_branch`
- `git_checkout`
- `git_show`
- `git_branch`

安全要求：

- 必須驗證 `repo_path` 是 git repository。
- 不可把使用者輸入拼成 shell command。
- destructive 行為，例如 reset、checkout、commit，必須在 schema annotation 中反映風險。
- Windows 上要保留原生 process argument passing。

### `time`

來源：`stevenke1981/servers/src/time`，Python package `mcp-server-time@0.6.2`

必須保留的 tools：

- `get_current_time`
- `convert_time`

驗收要求：

- 支援 IANA timezone name。
- invalid timezone 必須回傳 MCP invalid params style error。
- 輸出包含 timezone、datetime、day_of_week、is_dst。
- `convert_time` 必須支援一個 source timezone 與多個 target timezone。

### `fetch`

來源：`stevenke1981/servers/src/fetch`，Python package `mcp-server-fetch@0.6.3`

必須保留的 tools：

- `fetch`

安全與行為要求：

- 支援 URL fetch、HTML-to-text/Markdown-ish extraction、content truncation。
- 必須設定 request timeout、max response bytes、redirect 限制。
- 必須先定義 SSRF 防護策略，例如禁止 localhost/private IP，或預設 deny 並允許顯式 allowlist。
- User-Agent 與 proxy 行為要在 README 中說明。

### `sequential-thinking`

來源：`stevenke1981/servers/src/sequentialthinking`，TypeScript package `@modelcontextprotocol/server-sequential-thinking@0.6.2`

必須保留的 tools：

- `sequentialthinking`

驗收要求：

- 保留 thought、thoughtNumber、totalThoughts、nextThoughtNeeded、isRevision、revisesThought、branchFromThought、branchId、needsMoreThoughts 等語意。
- 回傳目前 thought history 與 branch 狀態。
- 狀態必須是 session-scoped，不可跨 client 汙染。

### `everything`

來源：`stevenke1981/servers/src/everything`，TypeScript package `@modelcontextprotocol/server-everything@2.0.0`

定位：不要當 production tool server；作為 MCP compatibility testbed。

必須保留或重建的 feature coverage：

- Tools: `echo`, `get-annotated-message`, `get-env`, `get-resource-links`, `get-resource-reference`, `get-roots-list`, `get-structured-content`, `get-sum`, `get-tiny-image`, `gzip-file-as-resource`, `toggle-simulated-logging`, `toggle-subscriber-updates`, `trigger-elicitation-request`, `trigger-elicitation-request-async`, `trigger-long-running-operation`, `trigger-sampling-request`, `trigger-sampling-request-async`, `trigger-url-elicitation`, `simulate-research-query`
- Prompts
- Resources
- Resource templates
- Subscriptions/resource update notifications
- Roots
- Logging
- Sampling
- Elicitation

## 必須保留的既有 Rust 專案功能

### `rlm-mcp`

來源：`stevenke1981/rlm-mcp.git`，目前版本 `0.1.6`

必須保留的 tools：

- `rlm_workflow`
- `rlm_scan`
- `rlm_env_info`
- `rlm_peek`
- `rlm_slice`
- `rlm_repl_info`
- `rlm_repl_execute`
- `rlm_transform`
- `rlm_artifact_write`
- `rlm_artifact_read`
- `rlm_chunk`
- `rlm_map_plan`
- `rlm_map_claim`
- `rlm_map_complete`
- `rlm_reduce_schema`
- `rlm_reduce_merge`
- `rlm_session_list`
- `rlm_session_delete`
- `rlm_session_cleanup`
- `rlm_session_export`
- `rlm_session_import`
- `rlm_task_create`
- `rlm_task_list`
- `rlm_task_result`
- `rlm_task_reduce`
- `rlm_task_cancel`
- `rlm_budget_configure`
- `rlm_budget_status`
- `rlm_trajectory_get`
- `rlm_trajectory_final`
- `rlm_benchmark_list`
- `rlm_benchmark_run`
- `rlm_tools_reference`

### `cbm-mcp`

來源：`stevenke1981/cbm-mcp.git`，目前版本 `0.2.3`

必須保留的 tools：

- `index_repository`
- `index_status`
- `search_graph`
- `trace_path`
- `get_code_snippet`
- `get_graph_schema`
- `get_architecture`
- `query_graph`
- `search_code`
- `list_projects`
- `delete_project`
- `detect_changes`
- `manage_adr`
- `ingest_traces`

### `nushell-mcp`

來源：`stevenke1981/nushell-mcp.git`，目前版本 `0.1.0`

必須保留的 tools：

- `nu_version`
- `nu_eval`
- `nu_script`
- `git_status`
- `git_diff`
- `git_log`
- `git_tree`
- `git_branch`
- `git_commit`
- `git_stash`
- `git_precommit_review`
- `nu_grep`
- `nu_find`
- `nu_read`
- `nu_ls`

## 驗收標準

- `cargo fmt --check` 通過。
- `cargo test --all-targets` 通過。
- `cargo clippy --all-targets -- -D warnings` 通過。
- 每個新增 server crate 都要有 tool inventory test。
- 每個 port 完成時，需附來源 parity table。
- README、`plan.md`、`todos.md` 必須同步更新。
