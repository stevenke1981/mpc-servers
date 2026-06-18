# Agent-Ready TODOs

本檔是給其他代理直接開工用的任務清單。領任務前先讀 `README.md`、`spec.md`、`plan.md`、`VERSIONING.md`。每個任務完成後都要更新本檔狀態與相關 README/spec parity table。

狀態標記：

- `[ ]` 未開始
- `[~]` 進行中
- `[x]` 完成
- `[!]` blocked，需要在任務下方寫明原因

## 全域規則

- 工作目錄：`D:\mpc-servers`
- 不要提交 `.opencode/`、`.codebase-memory/`、`target/`。
- 使用 `apply_patch` 做精準修改。
- 保留原本專案 tool name；不要為了 Rust 命名風格改掉公開 MCP tool。
- 每個 server crate 必須支援 `--version`。
- 每個任務完成前至少跑：
  - `cargo fmt --check`
  - `cargo test --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
- 若任務涉及 MCP schema，還要跑 TypeScript/OpenCode SDK `tools/list` smoke test。

## T0 Workspace And Docs

- [x] T0.1 初始化 git repo 與 Rust workspace。
  - Evidence: commit `17a91ef chore: initialize rust mcp servers workspace`

- [x] T0.2 建立 server inventory CLI。
  - Files: `crates/server-inventory`, `crates/mcp-servers`
  - Verify: `cargo run -p mcp-servers -- inventory`

- [x] T0.3 增加 `plan.md`、`spec.md`、`todos.md`。
  - Verify: 文件存在且提到所有來源 server。

## T1 Common Release And Install Template

- [ ] T1.1 從 `D:\cbm-mcp` 萃取 release/install checklist。
  - Source: `D:\cbm-mcp\install.ps1`, `D:\cbm-mcp\install.sh`, `D:\cbm-mcp\packaging`, `D:\cbm-mcp\.github`
  - Output: `packaging/README.md`
  - Must preserve: release binary install, `install --json` source of truth, Windows locked binary fallback。
  - Verify: README contains Windows, Codex, OpenCode examples.

- [ ] T1.2 建立共用 TypeScript SDK `tools/list` smoke test。
  - Output: `scripts/tools-list-smoke.ps1` 或等效跨平台腳本。
  - Must preserve: OpenCode-compatible JSON Schema validation。
  - Verify: 可對 `mcp-servers --version` 以外的真 MCP server binary 執行。

- [ ] T1.3 定義 installer report schema。
  - Output: `packaging/install-report.schema.json`
  - Required fields: server_name, version, installed_exe, config_targets, changed, warnings。
  - Verify: schema 有測試或範例 JSON。

## T2 Import Existing Rust Servers

- [ ] T2.1 導入 `cbm-mcp`。
  - Source repo: `D:\cbm-mcp`
  - Current version: `0.2.3`
  - Required tools: `index_repository`, `index_status`, `search_graph`, `trace_path`, `get_code_snippet`, `get_graph_schema`, `get_architecture`, `query_graph`, `search_code`, `list_projects`, `delete_project`, `detect_changes`, `manage_adr`, `ingest_traces`
  - Suggested crate: `crates/cbm-server`
  - Acceptance:
    - tool count is 14
    - existing rmcp protocol tests pass
    - OpenCode SDK smoke sees all tools
    - README explains project index location and install flow

- [ ] T2.2 導入 `rlm-mcp`。
  - Source repo: `D:\rlm-mcp`
  - Current version: `0.1.6`
  - Required tools: all `rlm_*` tools listed in `spec.md`
  - Suggested crate: `crates/rlm-server`
  - Acceptance:
    - session scan/chunk/slice flow works
    - map/reduce task flow tests pass
    - budget and trajectory tools keep existing behavior
    - `rlm_repl_execute` remains opt-in/safe

- [ ] T2.3 導入 `nushell-mcp`。
  - Source repo: `D:\nushell-mcp`
  - Current version: `0.1.0`
  - Required tools: `nu_version`, `nu_eval`, `nu_script`, `git_status`, `git_diff`, `git_log`, `git_tree`, `git_branch`, `git_commit`, `git_stash`, `git_precommit_review`, `nu_grep`, `nu_find`, `nu_read`, `nu_ls`
  - Suggested crate: `crates/nushell-server`
  - Acceptance:
    - command execution is bounded by cwd/timeout policy
    - git tools use safe arguments
    - no shell injection through user input

- [ ] T2.4 導入 `memlong` 作為 memory Rust 線。
  - Source repo: `D:\memlong`
  - Current version: `0.1.0`
  - Required tools: `add_memory`, `search_memories`, `get_memories`, `delete_memory`, `consolidate_memories`, `get_memory_stats`, `end_session`
  - Compatibility gap: upstream memory reference has graph tools like `create_entities` and `read_graph`
  - Acceptance:
    - memlong tools work unchanged
    - README documents mapping from upstream memory graph tools
    - decide whether to implement compatibility graph tools or explicitly mark replacement semantics

## T3 Port Small Reference Servers

- [x] T3.1 Port `time`.
  - Source: `.opencode/upstream/servers/src/time`
  - Crate: `crates/time-server`
  - Required tools: `get_current_time`, `convert_time`
  - Acceptance:
    - IANA timezone names work
    - invalid timezone returns invalid params style MCP error
    - output includes timezone, datetime, day_of_week, is_dst
    - tests cover DST and multiple target timezones
  - Evidence: 16 tests, 0 clippy, 0 fmt, `--version` outputs `0.1.0`, stdio `tools/list` smoke returns `get_current_time` and `convert_time`.
    - `test_get_current_time_valid_tz` + `test_convert_time_basic` verify IANA names
    - `test_get_current_time_invalid_timezone` verifies invalid param error
    - `test_convert_time_nepal_fractional` covers DST
    - `test_convert_time_multi_target` covers array of target timezones

- [x] T3.2 Port `sequential-thinking`.
  - Source: `.opencode/upstream/servers/src/sequentialthinking`
  - Crate: `crates/sequential-thinking-server`
  - Required tool: `sequentialthinking`
  - Acceptance:
    - supports revision and branch fields
    - state is session-scoped (Mutex<Vec<ThoughtData>> + Mutex<HashMap<String, Vec<ThoughtData>>>)
    - tests cover thought history and branch behavior
  - Evidence: 17 tests, 0 clippy, 0 fmt, `--version`/`-V`/`version` all output `0.1.0`, stdio `tools/list` smoke returns `sequentialthinking`.
    - `test_accept_valid_basic_thought` + `test_track_multiple_thoughts_in_history` verify thought history
    - `test_branch_tracking` + `test_multiple_thoughts_in_same_branch` verify branch tracking
    - `test_revision_tracking` verifies revision fields
    - `test_auto_adjust_total_thoughts` verifies thoughtNumber > totalThoughts adjustment
    - `test_coerce_bool_true`/`test_coerce_bool_false` verify string "true"/"false" coercion
    - `test_coerce_positive_i64_accepts_numbers_and_strings` verifies upstream-style number coercion
    - `test_tool_name` verifies tool name is `sequentialthinking`
    - `test_no_boolean_json_schema_nodes` verifies no bare boolean in tool input schema

## T4 Port Filesystem And Git

- [x] T4.1 Port `filesystem` path safety core.
  - Source: `.opencode/upstream/servers/src/filesystem`
  - Crate: `crates/filesystem-server`
  - First deliverable: path validation module and tests
  - Acceptance:
    - rejects path traversal
    - rejects symlink escape
    - handles Windows case/canonicalization
    - supports command-line allowed directories
  - Evidence:
    - Crate: `crates/filesystem-server` with workspace member added
    - Path safety core: `AllowedDirectories` struct with `from_existing_dirs`,
      `validate_existing_path`, `validate_candidate_path`, `list_allowed_directories`
    - Component‑based comparison in `is_subpath` — prevents prefix sibling attacks
      (`/tmp/project` vs `/tmp/project2`);
    - `normalize_path` resolves `.`/`..` without touching filesystem
    - Null‑byte rejection in `has_null_bytes`
    - Symlink handling: `validate_existing_path` uses `canonicalize` (resolves symlinks);
      `validate_candidate_path` resolves deepest existing ancestor to detect symlink‑based escapes
    - Windows case‑insensitive component comparison in `is_subpath` (guarded by `cfg(windows)`)
    - Binary supports `--version`, `-V`, `version` (all print `0.1.0`)
    - Minimal MCP handler — no file operation tools exposed in T4.1
    - 37 unit tests: exact root, subpaths, prefix sibling, traversal, null bytes, inaccessible dirs,
      no valid dirs, non‑existent candidate, symlink outside, symlink inside, symlink parent outside,
      Windows case‑insensitive, `list_allowed_directories`, nested allowed dirs
    - Verify: `cargo fmt --check` pass, `cargo test --all-targets` pass (72 total),
      `cargo clippy --all-targets -- -D warnings` pass,
      `cargo run -p filesystem-server -- --version` → `0.1.0`,
      `cargo run -p filesystem-server -- -V` → `0.1.0`,
      `cargo run -p filesystem-server -- version` → `0.1.0`
    - Stdio smoke: initialize + `tools/list` returns 0 tools as expected for T4.1

- [x] T4.2 Port `filesystem` tools.
  - Depends on: T4.1
  - Required tools: all filesystem tools listed in `spec.md`
  - Acceptance:
    - read/write/edit/list/search/tree/info tools work
    - `edit_file` supports dry-run diff
    - `read_file` remains deprecated alias
    - Roots behavior documented and tested
  - Evidence:
    - Implemented tools: `read_file`, `read_text_file`, `read_media_file`, `read_multiple_files`,
      `write_file`, `edit_file`, `create_directory`, `list_directory`, `list_directory_with_sizes`,
      `directory_tree`, `move_file`, `search_files`, `get_file_info`, `list_allowed_directories`
    - Tool annotations set read-only/write/destructive/idempotent hints for filesystem operations.
    - Path operations call `AllowedDirectories` validation before filesystem access.
    - `edit_file` supports `dryRun` and returns a git-style diff.
    - Unit coverage: 76 filesystem-server tests, including tool inventory, deprecated `read_file`,
      dry-run edit, file moves, directory tree, search, media read, and schema boolean-node guard.
    - Stdio smoke: initialize + `tools/list` returns 14 tools; `read_text_file` call succeeds.
    - Roots smoke: server starts without CLI directories, requests `roots/list` after
      `notifications/initialized`, updates directories, and refreshes again after
      `notifications/roots/list_changed`.
    - Verify: `cargo fmt --check` pass, `cargo test --all-targets` pass,
      `cargo clippy --all-targets -- -D warnings` pass,
      `cargo run -p filesystem-server -- --version`/`-V`/`version` → `0.1.0`.

- [ ] T4.3 Port `git`.
  - Source: `.opencode/upstream/servers/src/git`
  - Suggested crate: `crates/git-server`
  - Required tools: all git tools listed in `spec.md`
  - Acceptance:
    - validates repo path
    - uses native process args or a safe git library
    - no shell command string interpolation
    - tests cover status, diff, log, branch, checkout, commit

## T5 Port Fetch

- [ ] T5.1 Define fetch security policy before coding.
  - Output: `docs/fetch-security.md`
  - Must decide: localhost/private IP policy, redirect limit, timeout, max bytes, user-agent, proxy/env behavior
  - Acceptance: policy reviewed in README and `spec.md`

- [ ] T5.2 Port `fetch`.
  - Source: `.opencode/upstream/servers/src/fetch`
  - Suggested crate: `crates/fetch-server`
  - Required tool: `fetch`
  - Acceptance:
    - fetches HTTP/HTTPS
    - extracts readable text from HTML
    - truncates large content predictably
    - rejects blocked hosts per T5.1

## T6 Everything Compatibility Testbed

- [ ] T6.1 Implement everything tools subset.
  - Source: `.opencode/upstream/servers/src/everything`
  - Suggested crate: `crates/everything-server`
  - Required tools: all everything tools listed in `spec.md`
  - Acceptance: parity table marks each tool implemented/tested.

- [ ] T6.2 Implement everything resources/prompts/protocol features.
  - Depends on: T6.1
  - Required features: prompts, resources, templates, subscriptions, roots, logging, sampling, elicitation
  - Acceptance: CI-compatible MCP feature tests pass.

## T7 Documentation And Release Hygiene

- [ ] T7.1 Add per-server README template.
  - Output: `docs/templates/server-readme.md`
  - Must include: tool list, install, config, version, smoke tests, security notes.

- [ ] T7.2 Add parity table template.
  - Output: `docs/templates/parity-table.md`
  - Must include: upstream tool, Rust tool, status, tests, notes.

- [ ] T7.3 Add release checklist automation.
  - Output: `scripts/release-check.ps1`
  - Must run: fmt, test, clippy, build, `--version`, smoke test hook.

## Suggested First Task For A New Agent

Pick T3.1 `time` unless assigned otherwise. It is the smallest production port and will validate the crate pattern before high-risk filesystem/git/fetch work.
