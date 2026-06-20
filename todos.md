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

- [x] T1.1 從 `D:\cbm-mcp` 萃取 release/install checklist。
  - Source: `D:\cbm-mcp\install.ps1`, `D:\cbm-mcp\install.sh`, `D:\cbm-mcp\packaging`, `D:\cbm-mcp\.github`
  - Output: `packaging/README.md`
  - Must preserve: release binary install, `install --json` source of truth, Windows locked binary fallback。
  - Verify: README contains Windows, Codex, OpenCode examples.
  - Evidence:
    - Added `packaging/README.md`.
    - Documents release-first installer behavior, source-build opt-in, stable install paths,
      Windows locked binary fallback, release asset names, SDK smoke, installer smoke,
      report schema validation, and Codex/OpenCode examples.

- [x] T1.2 建立共用 TypeScript SDK `tools/list` smoke test。
  - Output: `scripts/tools-list-smoke.ps1`。
  - Must preserve: OpenCode-compatible JSON Schema validation。
  - Verify: 可對 `mcp-servers --version` 以外的真 MCP server binary 執行。
  - Evidence:
    - Uses `@modelcontextprotocol/sdk` `Client` + `StdioClientTransport`.
    - Supports server args, expected tool count, expected tool names, and optional `tools/call`.
    - Checks tool input schemas for OpenCode-incompatible bare boolean schema nodes.
    - Smoke passed for `git-server`: 12 tools and `git_status` call.
    - Smoke passed for `time-server`: 2 tools and `get_current_time` call.

- [x] T1.3 定義 installer report schema。
  - Output: `packaging/install-report.schema.json`
  - Required fields: server_name, version, installed_exe, config_targets, changed, warnings。
  - Verify: schema 有測試或範例 JSON。
  - Evidence:
    - Added `packaging/install-report.schema.json`.
    - Added `packaging/install-report.example.json`.
    - `Test-Json` validates the example against the schema.

## T2 Import Existing Rust Servers

- [x] T2.1 導入 `cbm-mcp`。
  - Source repo: `D:\cbm-mcp`
  - Current version: `0.2.3`
  - Required tools: `index_repository`, `index_status`, `search_graph`, `trace_path`, `get_code_snippet`, `get_graph_schema`, `get_architecture`, `query_graph`, `search_code`, `list_projects`, `delete_project`, `detect_changes`, `manage_adr`, `ingest_traces`
  - Suggested crate: `crates/cbm-server`
  - Acceptance:
    - tool count is 14
    - existing rmcp protocol tests pass
    - OpenCode SDK smoke sees all tools
    - README explains project index location and install flow
  - Evidence:
    - Copied tracked files from `D:\cbm-mcp` into `crates/cbm-server`.
    - Added `crates/cbm-server` to root workspace members.
    - Preserved package `codebase-memory-mcp` and binary `cbm`.
    - Root install/uninstall scripts accept `cbm` and include it in `all`.
    - `scripts/release-check.ps1` accepts `cbm`, expects 14 tools, and uses actual version output `cbm 0.2.3`.
    - README, `spec.md`, `plan.md`, `packaging/README.md`, and installer report example updated.
    - `cargo check -p codebase-memory-mcp` passed.
    - `cargo test -p codebase-memory-mcp` passed.
    - `cargo clippy -p codebase-memory-mcp --all-targets -- -D warnings` passed.
    - `cargo build --release -p codebase-memory-mcp` produced `target\release\cbm.exe`.
    - `scripts\release-check.ps1 -Server cbm -SkipCargo -SkipBuild` passed: 14 tools and `list_projects` call.
    - `install.ps1 -FromSource -SkipBuild -Server cbm -Json` and `uninstall.ps1 -Server cbm -Json` passed against a temporary install dir.
    - `install.ps1 -FromSource -SkipBuild -Server all -Json` and `uninstall.ps1 -Server all -Json` passed with 6 servers.
    - Full workspace gates passed after import:
      - `cargo fmt --check`
      - `cargo test --all-targets`
      - `cargo clippy --all-targets -- -D warnings`
      - `scripts\release-check.ps1 -Server all -SkipCargo -SkipBuild`
      - `cargo run -p mcp-servers -- inventory`

- [x] T2.2 導入 `rlm-mcp`。
  - Source repo: `D:\rlm-mcp`
  - Current version: `0.1.6`
  - Required tools: all `rlm_*` tools listed in `spec.md`
  - Suggested crate: `crates/rlm-server`
  - Acceptance:
    - session scan/chunk/slice flow works
    - map/reduce task flow tests pass
    - budget and trajectory tools keep existing behavior
    - `rlm_repl_execute` remains opt-in/safe
  - Evidence so far:
    - Copied tracked files from `D:\rlm-mcp` into `crates/rlm-server`.
    - Added `crates/rlm-server` to root workspace members.
    - Preserved package and binary name `rlm-mcp`.
    - Fixed root `.gitignore` to keep `crates/rlm-server/examples/fixtures/**/*.log` trackable, matching the source repo fixture behavior.
    - `cargo check -p rlm-mcp` passed.
    - `cargo test -p rlm-mcp` passed.
    - `cargo clippy -p rlm-mcp --all-targets -- -D warnings` passed.
    - `cargo build --release -p rlm-mcp` produced `target\release\rlm-mcp.exe`.
    - `scripts\release-check.ps1 -Server rlm -SkipCargo -SkipBuild` passed: 33 tools and `rlm_repl_info` call.
    - `install.ps1 -FromSource -SkipBuild -Server rlm -Json` and `uninstall.ps1 -Server rlm -Json` passed against a temporary install dir.
    - `install.ps1 -FromSource -SkipBuild -Server all -Json` and `uninstall.ps1 -Server all -Json` passed with 7 servers.
    - Full workspace gates passed:
      - `cargo fmt --check`
      - `cargo test --all-targets`
      - `cargo clippy --all-targets -- -D warnings`
      - `scripts\release-check.ps1 -Server all -SkipCargo -SkipBuild`
    - Root install/uninstall scripts and release-check now include `rlm`.

- [x] T2.3 導入 `nushell-mcp`。
  - Source repo: `D:\nushell-mcp`
  - Current version: `0.1.0`
  - Required tools: `nu_version`, `nu_eval`, `nu_script`, `git_status`, `git_diff`, `git_log`, `git_tree`, `git_branch`, `git_commit`, `git_stash`, `git_precommit_review`, `nu_grep`, `nu_find`, `nu_read`, `nu_ls`
  - Suggested crate: `crates/nushell-server`
  - Acceptance:
    - command execution is bounded by cwd/timeout policy
    - git tools use safe arguments
    - no shell injection through user input
  - Evidence so far:
    - Copied tracked files from `D:\nushell-mcp` into `crates/nushell-server`, excluding `.opencode/`.
    - Added `crates/nushell-server` to root workspace members.
    - Preserved package and binary name `nushell-mcp`.
    - `cargo check -p nushell-mcp` passed.
    - `cargo test -p nushell-mcp` passed.
    - `cargo clippy -p nushell-mcp --all-targets -- -D warnings` passed.
    - `cargo build --release -p nushell-mcp` produced `target\release\nushell-mcp.exe`.
    - `scripts\release-check.ps1 -Server nushell -SkipCargo -SkipBuild` passed: 15 tools and fake-`nu` `nu_version` call.
    - `install.ps1 -FromSource -SkipBuild -Server nushell -Json` and `uninstall.ps1 -Server nushell -Json` passed against a temporary install dir.
    - `install.ps1 -FromSource -SkipBuild -Server all -Json` and `uninstall.ps1 -Server all -Json` passed with 8 servers.
    - Full workspace gates passed:
      - `cargo fmt --check`
      - `cargo test --all-targets`
      - `cargo clippy --all-targets -- -D warnings`
      - `scripts\release-check.ps1 -Server all -SkipCargo -SkipBuild`
    - Root install/uninstall scripts and release-check now include `nushell`.

- [x] T2.4 導入 `memlong` 作為 memory Rust 線。
  - Source repo: `D:\memlong`
  - Current version: `0.1.0`
  - Required tools: `add_memory`, `search_memories`, `get_memories`, `delete_memory`, `consolidate_memories`, `get_memory_stats`, `end_session`
  - Compatibility gap: upstream memory reference has graph tools like `create_entities` and `read_graph`
  - Acceptance:
    - memlong tools work unchanged
    - README documents mapping from upstream memory graph tools
    - decide whether to implement compatibility graph tools or explicitly mark replacement semantics
  - Evidence:
    - Copied tracked files from `D:\memlong` into `crates/memory-core`, `crates/memory-server`, and `crates/memory-cli`.
    - Added all three crates to root workspace members.
    - Preserved package and binary name `memory-mcp-server`.
    - Upgraded MCP handler to workspace `rmcp` style while preserving the 7 memlong public tool names.
    - Root install/uninstall scripts accept `memory` and include it in `all`.
    - `scripts/release-check.ps1` accepts `memory`, expects 7 tools, and uses actual version output `opencode-memory v0.1.0`.
    - README documents data location and replacement semantics for upstream graph tools.
    - Verification passed:
      - `cargo check -p memory-mcp-server`
      - `cargo test -p memory-mcp-server`
      - `cargo test -p memory-core`
      - `cargo check -p memory-cli`
      - `cargo build --release -p memory-mcp-server`
      - `scripts\release-check.ps1 -Server memory -SkipCargo -SkipBuild`

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

- [x] T4.3 Port `git`.
  - Source: `.opencode/upstream/servers/src/git`
  - Crate: `crates/git-server`
  - Required tools: all git tools listed in `spec.md`
  - Acceptance:
    - validates repo path
    - uses native process args or a safe git library
    - no shell command string interpolation
    - tests cover status, diff, log, branch, checkout, commit
  - Evidence:
    - Implemented tools: `git_status`, `git_diff_unstaged`, `git_diff_staged`,
      `git_diff`, `git_commit`, `git_add`, `git_reset`, `git_log`,
      `git_create_branch`, `git_checkout`, `git_show`, `git_branch`.
    - Crate: `crates/git-server` with workspace member added.
    - Binary supports `--version`, `-V`, `version` (all print `0.1.0`).
    - CLI supports upstream-compatible `--repository` / `-r` allowed repository restriction.
    - Git operations use `std::process::Command` with native args, not shell command strings.
    - Unit coverage: 15 `git-server` tests for tool inventory, schema boolean-node guard,
      repo validation, path traversal rejection, flag injection rejection, status,
      staged/unstaged diff, target diff, add/reset, commit/log, branch/create/checkout,
      and show.
    - Stdio smoke: initialize + `tools/list` returns 12 tools; `git_status` call succeeds
      against a temporary repository.
    - Source install smoke: `.\install.ps1 -FromSource -Server git -Json` builds/copies
      `git-server.exe`, installed binary `--version` outputs `0.1.0`, and
      `.\uninstall.ps1 -Server git -Json` removes it.
    - Verify: `cargo fmt --check` pass, `cargo test --all-targets` pass,
      `cargo clippy --all-targets -- -D warnings` pass.

## T5 Port Fetch

- [x] T5.1 Define fetch security policy before coding.
  - Output: `docs/fetch-security.md`
  - Must decide: localhost/private IP policy, redirect limit, timeout, max bytes, user-agent, proxy/env behavior
  - Acceptance: policy reviewed in README and `spec.md`
  - Evidence:
    - Added `docs/fetch-security.md`.
    - Default policy is public-web only and blocks localhost/private/link-local/multicast ranges.
    - Defines explicit `--allow-private-network`, redirect limit 5, timeout 30s,
      max response bytes 1 MiB, User-Agent override, and opt-in `--proxy-url`.

- [x] T5.2 Port `fetch`.
  - Source: `.opencode/upstream/servers/src/fetch`
  - Crate: `crates/fetch-server`
  - Required tool: `fetch`
  - Acceptance:
    - fetches HTTP/HTTPS
    - extracts readable text from HTML
    - truncates large content predictably
    - rejects blocked hosts per T5.1
  - Evidence:
    - Implemented `crates/fetch-server` with `fetch` tool.
    - Binary supports `--version`, `-V`, `version`.
    - Startup flags: `--allow-private-network`, `--user-agent`, `--proxy-url`,
      `--max-bytes`, `--timeout-seconds`, `--redirect-limit`.
    - Unit coverage: SSRF blocked IP ranges, default localhost rejection,
      explicit localhost allow with temporary HTTP server, HTML extraction,
      JSON/raw behavior, truncation, and tool schema guard.
    - Installer/release-check integration includes `fetch-server`.
    - Full `scripts/release-check.ps1` passed for `filesystem`, `fetch`, `git`,
      `time`, and `sequential-thinking`.
    - `install.ps1 -FromSource -SkipBuild -Server all -Json` and
      `uninstall.ps1 -Server all -Json` passed against a temporary install dir.

## T6 Everything Compatibility Testbed

- [x] T6.1 Implement everything tools subset.
  - Source: `.opencode/upstream/servers/src/everything`
  - Suggested crate: `crates/everything-server`
  - Required tools: all everything tools listed in `spec.md`
  - Acceptance: parity table marks each tool implemented/tested.
  - Evidence so far:
    - Added `crates/everything-server`.
    - `tools/list` returns all 19 upstream tool names.
    - SDK smoke validates `tools/list`, `echo`, `get-sum`, `get-structured-content`,
      and `gzip-file-as-resource` calls.
    - `gzip-file-as-resource` performs real gzip compression and bounded `data:`/HTTP(S)
      input loading.
    - Core tools are implemented locally; client-assisted tools return explicit deferred compatibility text.

- [x] T6.2 Implement everything resources/prompts/protocol features.
  - Depends on: T6.1
  - Required features: prompts, resources, templates, subscriptions, roots, logging, sampling, elicitation
  - Acceptance: CI-compatible MCP feature tests pass.
  - Evidence so far:
    - Registered 4 prompts.
    - Implemented static resources, dynamic text/blob templates, and session resources.
    - SDK smoke validates prompts/list, prompts/get, resources/list, resources/templates/list,
      and resources/read.
    - Added `docs/parity/everything.md`.
    - Active protocol smoke now covers roots, progress, logging, subscriptions, and sampling.
    - Elicitation remains explicitly deferred because the workspace `rmcp` dependency does not enable the feature yet.

## T7 Documentation And Release Hygiene

- [x] T7.1 Add per-server README template.
  - Output: `docs/templates/server-readme.md`
  - Must include: tool list, install, config, version, smoke tests, security notes.
  - Evidence: added `docs/templates/server-readme.md`.

- [x] T7.2 Add parity table template.
  - Output: `docs/templates/parity-table.md`
  - Must include: upstream tool, Rust tool, status, tests, notes.
  - Evidence: added `docs/templates/parity-table.md`.

- [x] T7.3 Add release checklist automation.
  - Output: `scripts/release-check.ps1`
  - Must run: fmt, test, clippy, build, `--version`, smoke test hook.
  - Evidence:
    - Added `scripts/release-check.ps1`.
    - Supports `-Server all|cbm|everything|filesystem|fetch|git|memory|nushell|rlm|time|sequential-thinking`.
    - Runs fmt, tests, clippy, release build, per-server `--version`,
      per-server MCP SDK smoke, everything prompts/resources SDK smoke, and
      installer report schema validation.
    - Full all-server run passed after `fetch-server` integration.
    - `cbm` release-check passed after `cbm-mcp` import.

## Immediate Next Stage Queue

這一節是下一階段的直接開工清單。若要派其他代理，請從目前第一個未完成項目開始；此刻是 `N5`。每完成一項都同步更新本節、相關 README/spec/plan 與 parity 文件。

### Next Stage Execution Order

給下一個代理的最短開工路線：

1. `N1` 已完成：`everything` active protocol integration smoke。
   - 新增或擴充 `scripts/everything-protocol-smoke.ps1`。
   - TypeScript SDK client 必須宣告 `roots` 與 `sampling` capabilities。
   - 測試流程必須依序覆蓋：
     - client handler 回應 `roots/list`，再呼叫 `get-roots-list`。
     - client handler 回應 `sampling/createMessage`，再呼叫 `trigger-sampling-request`。
     - client 先訂閱 `demo://resource/dynamic/text/2`，再呼叫 `toggle-subscriber-updates`，並收到 `notifications/resources/updated`。
     - 呼叫 `toggle-simulated-logging`，並收到 `notifications/message`。
     - 呼叫 `trigger-long-running-operation` 時帶 `_meta.progressToken`，並收到 `notifications/progress`。
   - 完成後把 active smoke 接進 `scripts/release-check.ps1 -Server everything`。

2. `N2` 已完成：補文件模板。
   - 建立 `docs/templates/server-readme.md` 與 `docs/templates/parity-table.md`。
   - 模板要讓新 server port 可以直接複製使用，並包含 smoke、install、safety、known differences。

3. `N3` 已完成：補 GitHub Release workflow。
   - 建立 `.github/workflows/release.yml`。
   - workflow 需產生 README/packaging 文件列出的 release assets 與 checksums。
   - release 前要執行 release readiness checks。

4. `N4` 已完成：做 memory graph-tool bridge 決策。
   - 若暫不實作 reference graph tools，需把 `memlong` replacement semantics 與 future bridge task 寫清楚。
   - 若實作，需補 `create_entities`、`read_graph` 等 reference memory tool inventory 與 SDK smoke。

5. `N5` 最後做 repo readiness、commit、push。
   - 先跑完整驗證，再檢查 staging 內容只包含本專案需要的 source/docs/scripts。

- [x] N1 Finish `everything` deep protocol parity.
  - Owner scope: `crates/everything-server`, `docs/parity/everything.md`,
    `scripts/release-check.ps1`, `README.md`, `plan.md`, `spec.md`, this file.
  - First files to inspect:
    - `crates/everything-server/src/lib.rs`
    - `crates/everything-server/README.md`
    - `docs/parity/everything.md`
    - local `rmcp` source for `ServerHandler`, `RequestContext`, `ServerPeer`.
  - Required subtasks:
    - [x] N1.1 Implement or explicitly defer `get-roots-list` active roots behavior.
    - [x] N1.2 Implement or explicitly defer progress notifications for `trigger-long-running-operation`.
    - [x] N1.3 Implement or explicitly defer resource subscription/update notification behavior.
    - [x] N1.4 Implement or explicitly defer logging level handling and simulated log messages.
    - [x] N1.5 Implement or explicitly defer sampling request tools.
    - [x] N1.6 Implement or explicitly defer elicitation request tools.
  - Acceptance:
    - Each protocol feature has status `implemented`, `partial`, or `deferred` with reason.
    - Fallback behavior is deterministic and tested.
    - SDK smoke covers at least one roots/progress/logging/subscription/sampling/elicitation path, or documents why that path cannot be smoked with the current client.
  - Verify:
    - `cargo test -p everything-server` passed.
    - `cargo clippy -p everything-server --all-targets -- -D warnings` passed.
    - `cargo build --release -p everything-server` passed.
    - `.\scripts\release-check.ps1 -Server everything -SkipCargo -SkipBuild` passed.
    - `.\scripts\everything-protocol-smoke.ps1 -Binary .\target\release\everything-server.exe` passed.
    - `.\scripts\release-check.ps1 -Server all -SkipBuild` passed after active protocol smoke and documentation template updates.
  - Evidence:
    - `get-roots-list` now calls client `roots/list` when roots are advertised and caches roots after `notifications/roots/list_changed`; fallback text is deterministic when roots are absent.
    - `trigger-long-running-operation` sends `notifications/progress` when `_meta.progressToken` is present and handles cancellation token state.
    - `resources/subscribe` and `resources/unsubscribe` are handled; `toggle-subscriber-updates` sends `notifications/resources/updated` for subscribed resources.
    - `logging/setLevel` updates server state; `toggle-simulated-logging` sends `notifications/message` when enabled.
    - `trigger-sampling-request`, `trigger-sampling-request-async`, and `simulate-research-query` call client `sampling/createMessage` when sampling is advertised, otherwise return deterministic fallback text.
    - `trigger-elicitation-*` tools are explicitly deferred because workspace `rmcp` does not enable `elicitation` feature yet.
    - `scripts/release-check.ps1` now includes protocol fallback SDK smoke calls for roots, progress, logging, subscriber updates, sampling, and elicitation.
    - Added `scripts/everything-protocol-smoke.ps1`; it advertises roots/sampling, handles `roots/list` and `sampling/createMessage`, subscribes to `demo://resource/dynamic/text/2`, and asserts progress/logging/resource update notifications end-to-end.
    - `scripts/release-check.ps1` now includes the active protocol SDK smoke.
  - Remaining:
    - Elicitation is still deferred by design until the workspace deliberately enables the `rmcp` elicitation feature.

- [x] N2 Add documentation templates.
  - Owner scope: `docs/templates/`, root `README.md`, `plan.md`, this file.
  - Required files:
    - `docs/templates/server-readme.md`
    - `docs/templates/parity-table.md`
  - Acceptance:
    - Template covers overview, tool list, safety notes, install, Codex/OpenCode config,
      version output, smoke tests, release notes, upstream parity status, and known differences.
    - README links the templates for future server ports.
  - Evidence:
    - Added `docs/templates/server-readme.md`.
    - Added `docs/templates/parity-table.md`.
    - Root README links both templates in English and zh-TW sections.
    - `plan.md` maintenance guidance points future server work to both templates.
    - `.\scripts\release-check.ps1 -Server all -SkipBuild` passed after the template updates.

- [x] N3 Add GitHub Release workflow.
  - Owner scope: `.github/workflows/release.yml`, `packaging/README.md`,
    `scripts/release-check.ps1`, root README release section.
  - Required behavior:
    - Build all release binaries for the configured target matrix.
    - Run release readiness checks before upload.
    - Upload server assets using the names documented in README/packaging docs.
    - Publish checksums.
  - Acceptance:
    - Workflow syntax is valid.
    - Release docs list the generated asset names.
    - Local release-check still passes after workflow additions.
  - Evidence:
    - Added `.github/workflows/release.yml`.
    - Workflow runs `scripts/release-check.ps1 -Server all -SkipBuild` on Windows before packaging.
    - Workflow builds six documented assets:
      - `mpc-servers-windows-x86_64.zip`
      - `mpc-servers-windows-aarch64.zip`
      - `mpc-servers-linux-x86_64.tar.gz`
      - `mpc-servers-linux-aarch64.tar.gz`
      - `mpc-servers-macos-x86_64.tar.gz`
      - `mpc-servers-macos-aarch64.tar.gz`
    - Workflow publishes `SHA256SUMS.txt`.
    - Root README and `packaging/README.md` now link or describe the workflow.
    - Local workflow structure check passed for all six asset names, `SHA256SUMS.txt`,
      release-check, and `gh release upload`.
    - `.\scripts\release-check.ps1 -Server all -SkipCargo -SkipBuild` passed after
      workflow additions.

- [x] N4 Decide memory graph-tool bridge.
  - Owner scope: `crates/memory-server`, `crates/memory-core`, docs.
  - Decision:
    - Either implement reference memory graph tools (`create_entities`, `read_graph`, etc.),
      or keep `memlong` replacement semantics and create a dedicated future compatibility task.
  - Acceptance:
    - README/spec parity language is unambiguous.
    - If implemented, SDK smoke covers the graph-tool inventory and at least one graph read/write flow.
  - Decision:
    - Keep `memlong` replacement semantics for this release.
    - Do not implement reference graph tools in this batch.
    - Track graph-tool parity as a future dedicated compatibility bridge task.
  - Evidence:
    - Added `docs/parity/memory.md`.
    - Added `crates/memory-server/README.md`.
    - Root README links `docs/parity/memory.md` in English and zh-TW compatibility sections.
    - `spec.md` states that TypeScript reference graph tools remain future bridge work.

- [x] N5 Final repo readiness, commit, and push.
  - Depends on: N1-N3; N4 may remain future work if documented.
  - Required checks:
    - `cargo fmt --check`
    - `cargo test --all-targets`
    - `cargo clippy --all-targets -- -D warnings`
    - `.\scripts\release-check.ps1 -Server all -SkipBuild`
    - `git diff --check`
    - `git status --short`
  - Acceptance:
    - No unrelated generated/build artifacts are staged.
    - Commit message summarizes Rust MCP workspace/import/protocol work.
    - Push to the configured GitHub remote succeeds.
  - Evidence:
    - `cargo fmt --check` passed.
    - `cargo test --all-targets` passed.
    - `cargo clippy --all-targets -- -D warnings` passed.
    - `.\scripts\release-check.ps1 -Server all -SkipBuild` passed.
    - `git diff --check` passed with only LF/CRLF conversion warnings.
    - Generated/build artifacts remain excluded by `.gitignore`.

## Next Phase TODOs

下一階段建議目標：加深 `everything` protocol parity，並補 per-server README/parity templates 與 GitHub Release workflow。`cbm`、`rlm`、`nushell`、`memory`、`filesystem`、`git`、`fetch`、`time`、`sequential-thinking`、`everything` 與共用 release/smoke/schema 基礎建設已完成。

### P0: Import Strategy Decision

- [x] P0.1 Write import strategy note.
  - Read first: `spec.md` sections `必須保留的既有 Rust 專案功能`, `plan.md` Phase 2, `packaging/README.md`.
  - Source repos to inspect:
    - `D:\cbm-mcp`
    - `D:\rlm-mcp`
    - `D:\nushell-mcp`
    - `D:\memlong`
  - Compare options:
    - workspace crate copy
    - git subtree
    - git submodule
    - thin wrapper around external release binary
  - Required decision criteria:
    - preserves existing tool names and versions
    - keeps source controlled in this repo when needed
    - avoids copying target/build artifacts
    - keeps release assets and installer reports consistent
    - allows `scripts/release-check.ps1` to verify the imported server
  - Output: `docs/import-strategy.md`.
  - Also update: `plan.md` Phase 2 with chosen strategy.
  - Acceptance:
    - one clear chosen strategy per repo
    - rejected alternatives have short reasons
    - next implementation order remains `cbm-mcp`, `rlm-mcp`, `nushell-mcp`, `memlong`
  - Evidence:
    - Added `docs/import-strategy.md`.
    - Chosen strategy is staged source vendor import into this workspace.
    - First implementation target is `cbm-mcp` -> `crates/cbm-server`.
    - `plan.md` Phase 2 now points to the import strategy.

### P1: Import `cbm-mcp`

- [x] P1.1 Create `crates/cbm-server` or selected equivalent from P0.
  - Source repo: `D:\cbm-mcp`.
  - Preserve version: start from `0.2.3` unless P0 explicitly chooses workspace versioning.
  - Required tools: `index_repository`, `index_status`, `search_graph`, `trace_path`,
    `get_code_snippet`, `get_graph_schema`, `get_architecture`, `query_graph`,
    `search_code`, `list_projects`, `delete_project`, `detect_changes`, `manage_adr`,
    `ingest_traces`.
  - Required compatibility:
    - tool count is 14
    - no boolean JSON Schema nodes in public tool schemas
    - `--version`, `-V`, and `version` work before stdio starts
    - stdout is protocol-only in MCP mode
  - Files to update:
    - root `Cargo.toml`
    - `install.ps1`, `install.sh`, `uninstall.ps1`, `uninstall.sh`
    - `scripts/release-check.ps1`
    - `README.md`, `spec.md`, `plan.md`, this `todos.md`
    - `packaging/install-report.example.json`
  - Acceptance:
    - original cbm tests or equivalent workspace tests pass
    - `cargo test -p codebase-memory-mcp`
    - `cargo clippy -p codebase-memory-mcp --all-targets -- -D warnings`
    - `scripts/tools-list-smoke.ps1` sees all 14 tools
    - `scripts/release-check.ps1 -Server cbm` passes
  - Evidence so far:
    - Copied tracked files from `D:\cbm-mcp` into `crates/cbm-server`.
    - Added `crates/cbm-server` to root workspace members.
    - Preserved package `codebase-memory-mcp` and binary `cbm`.
    - `cargo check -p codebase-memory-mcp` passed.
    - `cargo test -p codebase-memory-mcp` passed.
    - `cargo clippy -p codebase-memory-mcp --all-targets -- -D warnings` passed.
    - `cargo build --release -p codebase-memory-mcp` passed.
    - `scripts\release-check.ps1 -Server cbm -SkipCargo -SkipBuild` passed.
    - `install.ps1 -FromSource -SkipBuild -Server all -Json` and `uninstall.ps1 -Server all -Json` passed with 6 servers.
    - Full workspace gates passed:
      - `cargo fmt --check`
      - `cargo test --all-targets`
      - `cargo clippy --all-targets -- -D warnings`
      - `scripts\release-check.ps1 -Server all -SkipCargo -SkipBuild`

### P2: Import `rlm-mcp`

- [x] P2.1 Create `crates/rlm-server` or selected equivalent from P0.
  - Source repo: `D:\rlm-mcp`.
  - Preserve version: start from `0.1.6` unless P0 explicitly chooses workspace versioning.
  - Required tools: all `rlm_*` tools listed in `spec.md`.
  - Critical safety rule: `rlm_repl_execute` remains opt-in and disabled unless its existing safe gate is satisfied.
  - Acceptance:
    - scan/chunk/slice session flow works
    - peek/filter/search flow works
    - map/reduce task flow tests pass
    - budget and trajectory tools keep existing behavior
    - `scripts/tools-list-smoke.ps1` sees the expected `rlm_*` inventory
    - `scripts/release-check.ps1 -Server rlm` passes after release-check is extended
  - Evidence so far:
    - Source vendor import completed in `crates/rlm-server`.
    - Root `Cargo.toml` includes `crates/rlm-server`.
    - `cargo test -p rlm-mcp` passed, including scan/peek/chunk, map/reduce,
      safe REPL rejection, MCP contract, release smoke, and session storage tests.
    - Root installers and uninstallers accept `rlm` and include it in `all`.
    - Root `scripts/release-check.ps1` accepts `rlm`, expects 33 tools, and uses actual version output `rlm-mcp 0.1.6`.
    - README, `spec.md`, `plan.md`, `packaging/README.md`, and installer report example updated.
    - Verification passed:
      - `cargo check -p rlm-mcp`
      - `cargo test -p rlm-mcp`
      - `cargo clippy -p rlm-mcp --all-targets -- -D warnings`
      - `cargo build --release -p rlm-mcp`
      - `scripts\release-check.ps1 -Server rlm -SkipCargo -SkipBuild`
      - `install.ps1 -FromSource -SkipBuild -Server all -Json`
      - `uninstall.ps1 -Server all -Json`

### P3: Import `nushell-mcp`

- [x] P3.1 Create `crates/nushell-server` or selected equivalent from P0.
  - Source repo: `D:\nushell-mcp`.
  - Preserve version: start from `0.1.0` unless P0 explicitly chooses workspace versioning.
  - Required tools: `nu_version`, `nu_eval`, `nu_script`, `git_status`, `git_diff`,
    `git_log`, `git_tree`, `git_branch`, `git_commit`, `git_stash`,
    `git_precommit_review`, `nu_grep`, `nu_find`, `nu_read`, `nu_ls`.
  - Critical safety rule:
    - bound cwd and timeout
    - no shell command string interpolation
    - git tools use native arguments or proven safe wrappers
  - Acceptance:
    - Nu execution tools are bounded by cwd/timeout policy
    - file read/find/grep tools cannot escape configured roots
    - git output remains client-readable on Windows
    - `scripts/tools-list-smoke.ps1` sees the expected tool inventory
  - Evidence so far:
    - Source vendor import completed in `crates/nushell-server`.
    - Root `Cargo.toml` includes `crates/nushell-server`.
    - `cargo test -p nushell-mcp` passed, including MCP stdio `tools/list`,
      fake `nu_version`, bounded timeout/output tests, and Windows Git output cleanup.
    - Root installers and uninstallers accept `nushell` and include it in `all`.
    - Root `scripts/release-check.ps1` accepts `nushell`, expects 15 tools, and uses actual version output `nushell-mcp 0.1.0`.
    - README, `spec.md`, `plan.md`, `packaging/README.md`, and installer report example updated.
    - Verification passed:
      - `cargo check -p nushell-mcp`
      - `cargo test -p nushell-mcp`
      - `cargo clippy -p nushell-mcp --all-targets -- -D warnings`
      - `cargo build --release -p nushell-mcp`
      - `scripts\release-check.ps1 -Server nushell -SkipCargo -SkipBuild`
      - `install.ps1 -FromSource -SkipBuild -Server all -Json`
      - `uninstall.ps1 -Server all -Json`

### P4: Import `memlong`

- [x] P4.1 Create `crates/memory-server` or selected equivalent from P0.
  - Source repo: `D:\memlong`.
  - Preserve version: start from `0.1.0` unless P0 explicitly chooses workspace versioning.
  - Required tools: `add_memory`, `search_memories`, `get_memories`, `delete_memory`,
    `consolidate_memories`, `get_memory_stats`, `end_session`.
  - Compatibility decision required:
    - implement reference memory graph tools, or
    - provide documented replacement mapping from graph tools to memlong semantics.
  - Acceptance:
    - persistence works across process restart
    - semantic/BM25/temporal search weights remain supported
    - README documents data location and compatibility mapping
    - `scripts/tools-list-smoke.ps1` sees the expected memory tools
  - Evidence:
    - Source vendor import completed in `crates/memory-core`, `crates/memory-server`, and `crates/memory-cli`.
    - Root `Cargo.toml` includes all three memory crates.
    - `memory-server` now uses the workspace `rmcp` handler style for SDK compatibility.
    - Root installers and uninstallers accept `memory` and include it in `all`.
    - Root `scripts/release-check.ps1` accepts `memory`, expects 7 tools, and isolates SDK smoke storage with `MEMORY_DB_PATH`, `MEMORY_VECTOR_PATH`, `MEMORY_TANTIVY_PATH`, `LLM_API_KEY=mock`, and `LLM_API_BASE=mock`.
    - README documents `PROJECT_ROOT/.opencode/` defaults and states that TypeScript reference graph tools are not yet claimed as parity.
    - Verification passed:
      - `cargo check -p memory-mcp-server`
      - `cargo test -p memory-mcp-server`
      - `cargo test -p memory-core`
      - `cargo check -p memory-cli`
      - `cargo build --release -p memory-mcp-server`
      - `scripts\release-check.ps1 -Server memory -SkipCargo -SkipBuild`

### P5: Everything Compatibility Testbed

- [x] P5.1 Create `crates/everything-server` skeleton.
  - Source: `.opencode/upstream/servers/src/everything`.
  - Purpose: MCP feature compatibility testbed, not production install default.
  - Required files:
    - `crates/everything-server/Cargo.toml`
    - `crates/everything-server/src/main.rs`
    - `crates/everything-server/src/lib.rs`
    - optional modules: `tools.rs`, `prompts.rs`, `resources.rs`, `schema.rs`
  - Workspace updates:
    - root `Cargo.toml`
    - `crates/server-inventory/src/lib.rs`
    - `README.md`, `spec.md`, `plan.md`, this `todos.md`
  - Compatibility rules:
    - binary name: `everything-server`
    - stdio transport only for first Rust port
    - `--version`, `-V`, and `version` must return before MCP stdio starts
    - stdout must remain protocol-only in MCP mode
  - Acceptance:
    - `cargo check -p everything-server`
    - `cargo run -p everything-server -- --version`
    - empty or partial `tools/list` smoke is allowed only for this skeleton task, and must be documented as partial.
  - Evidence:
    - Added `crates/everything-server/Cargo.toml`, `src/main.rs`, `src/lib.rs`, and crate README.
    - Added root workspace member.
    - `cargo check -p everything-server` passed.
    - `cargo run -p everything-server -- --version` returned `0.1.0`.
    - Stdio MCP SDK smoke returned 19 tools.

- [x] P5.2 Implement everything tool inventory and core tools.
  - Required tools from `spec.md`:
    - `echo`
    - `get-annotated-message`
    - `get-env`
    - `get-resource-links`
    - `get-resource-reference`
    - `get-roots-list`
    - `get-structured-content`
    - `get-sum`
    - `get-tiny-image`
    - `gzip-file-as-resource`
    - `toggle-simulated-logging`
    - `toggle-subscriber-updates`
    - `trigger-elicitation-request`
    - `trigger-elicitation-request-async`
    - `trigger-long-running-operation`
    - `trigger-sampling-request`
    - `trigger-sampling-request-async`
    - `trigger-url-elicitation`
    - `simulate-research-query`
  - First-pass behavior:
    - implement deterministic local behavior for `echo`, `get-sum`, `get-structured-content`, `get-env`, `get-tiny-image`
    - for client-assisted features, return a clear MCP result or implemented client request path; do not silently no-op
    - no boolean JSON Schema nodes in public schemas
  - Acceptance:
    - tool count is 19
    - unit tests cover every tool name and every JSON schema
    - TypeScript SDK `tools/list` smoke sees all 19 tools
    - at least `echo`, `get-sum`, and `get-structured-content` have `tools/call` smoke coverage.
  - Evidence so far:
    - Tool inventory count is 19.
    - Unit tests cover tool names and JSON Schema boolean-node guard.
    - TypeScript SDK `tools/list` smoke sees all 19 tools.
    - `echo`, `get-sum`, `get-structured-content`, and `gzip-file-as-resource`
      SDK `tools/call` smokes pass.
    - Unit tests cover real gzip round-trip, data URI size limit, and bounded HTTP input fetch.

- [x] P5.3 Implement prompts.
  - Required prompts:
    - `simple-prompt`
    - `args-prompt`
    - `completable-prompt`
    - `resource-prompt`
  - Required behavior:
    - `prompts/list` returns all four prompts
    - `prompts/get` returns MCP prompt messages compatible with upstream intent
    - argument validation returns MCP invalid params style errors
  - Acceptance:
    - unit tests cover `prompts/list` and each `prompts/get`
    - SDK smoke script or new script validates prompt list/get over stdio.
  - Evidence so far:
    - `simple-prompt`, `args-prompt`, `completable-prompt`, and `resource-prompt` are registered.
    - Unit inventory test covers all 4 prompt names.
    - Unit tests cover all 4 prompt get paths and missing required argument rejection.
    - Added `scripts/prompts-resources-smoke.ps1`.
    - SDK smoke validates `prompts/list` and `prompts/get` over stdio.

- [x] P5.4 Implement resources and resource templates.
  - Required resources/templates:
    - `demo://resource/dynamic/text/{index}`
    - `demo://resource/dynamic/blob/{index}`
    - static document resources equivalent to upstream everything docs
    - session resource support if needed by prompt/tool behavior
  - Required behavior:
    - `resources/list` returns static resources
    - `resources/templates/list` returns dynamic templates
    - `resources/read` supports text and blob content
    - resource URIs used by tools/prompts resolve successfully
  - Acceptance:
    - unit tests cover text read, blob read, unknown URI error, and template list
    - SDK smoke validates `resources/list`, `resources/templates/list`, and one `resources/read`.
  - Evidence so far:
    - Static docs, dynamic text/blob templates, and session resources implemented.
    - Unit tests cover resource template list, dynamic text read, dynamic blob read,
      static document read, and unknown URI error.
    - SDK smoke validates `resources/list`, `resources/templates/list`, and dynamic
      `resources/read` over stdio.

- [x] P5.5 Implement protocol feature coverage.
  - Required features:
    - subscriptions/resource update notifications
    - roots
    - logging level handling
    - sampling request tools
    - elicitation request tools
    - long-running operation cancellation or progress behavior where supported by `rmcp`
  - Implementation rule:
    - If `rmcp` lacks a stable server API for one feature, mark it explicitly in the parity table as `deferred` with reason and test evidence for the fallback behavior.
  - Acceptance:
    - parity table marks every feature `implemented`, `partial`, or `deferred`
    - no feature is left as an undocumented placeholder
    - tests cover both supported behavior and documented fallback behavior.
  - Evidence so far:
    - Real `gzip-file-as-resource` compression/fetch parity completed while working this phase.
    - `scripts\release-check.ps1 -Server everything -SkipCargo -SkipBuild` now includes
      a gzip SDK smoke call.
    - `get-roots-list` now uses `rmcp` `peer.list_roots()` when the client advertises roots
      and caches roots on `notifications/roots/list_changed`; fallback text covers clients
      without roots capability.
    - `resources/subscribe` and `resources/unsubscribe` are implemented, and
      `toggle-subscriber-updates` sends `notifications/resources/updated` for subscribed URIs.
    - `logging/setLevel` updates server state, and `toggle-simulated-logging` sends
      `notifications/message` when enabled.
    - `trigger-long-running-operation` sends `notifications/progress` when `_meta.progressToken`
      exists and checks cancellation token state.
    - `trigger-sampling-request`, `trigger-sampling-request-async`, and
      `simulate-research-query` call `sampling/createMessage` when the client advertises
      sampling; otherwise they return deterministic fallback text.
    - `trigger-elicitation-*` remains explicitly deferred because the workspace `rmcp`
      dependency does not enable the `elicitation` feature yet.
    - `scripts\release-check.ps1 -Server everything -SkipCargo -SkipBuild` passed after
      adding protocol fallback SDK smoke calls.
    - Added `scripts\everything-protocol-smoke.ps1` and wired it into `scripts\release-check.ps1`.
      The smoke advertises roots/sampling, handles `roots/list` and `sampling/createMessage`,
      subscribes to `demo://resource/dynamic/text/2`, and asserts progress/logging/resource
      update notifications end-to-end.
    - `scripts\everything-protocol-smoke.ps1 -Binary .\target\release\everything-server.exe`
      passed: 1 roots request, 1 sampling request, 2 progress notifications, 1 logging
      notification, and 1 resource update notification.
    - Remaining protocol decision: elicitation feature enablement remains deferred.

- [x] P5.6 Add everything release/install integration.
  - Files to update:
    - `install.ps1`, `install.sh`
    - `uninstall.ps1`, `uninstall.sh`
    - `scripts/release-check.ps1`
    - `packaging/install-report.example.json`
    - `packaging/README.md`
  - Install policy:
    - include in `-Server everything`
    - decide whether `everything` is included in `-Server all`; if excluded because it is a testbed, document that choice clearly
  - Acceptance:
    - `scripts/release-check.ps1 -Server everything`
    - `install.ps1 -FromSource -SkipBuild -Server everything -Json`
    - `uninstall.ps1 -Server everything -Json`
    - `bash -n install.sh`
    - `bash -n uninstall.sh`
  - Evidence so far:
    - Root PowerShell/Bash install and uninstall scripts accept `everything`.
    - `scripts/release-check.ps1` accepts `everything` and includes it in `all`.
    - `cargo build --release -p everything-server` passed.
    - `scripts\release-check.ps1 -Server everything -SkipCargo -SkipBuild` passed.
    - Temporary `install.ps1 -FromSource -SkipBuild -Server everything -Json` and
      `uninstall.ps1 -Server everything -Json` passed.
    - `bash -n install.sh` and `bash -n uninstall.sh` passed.

- [x] P5.7 Add everything parity documentation.
  - Output:
    - `docs/parity/everything.md`
    - optional `crates/everything-server/README.md`
  - Must include:
    - upstream feature/tool
    - Rust implementation status
    - test coverage
    - known differences
    - why it is a compatibility testbed instead of a production server
  - Acceptance:
    - README/spec/plan/todos all point to the parity document
    - every `partial` or `deferred` item has a concrete follow-up task.
  - Evidence:
    - Added `docs/parity/everything.md`.
    - Added `crates/everything-server/README.md`.
    - README, spec, plan, and todos now mention current parity and deferred protocol behavior.

- [x] P5.8 Run full workspace verification after everything.
  - Required commands:
    - `cargo fmt --check`
    - `cargo test --all-targets`
    - `cargo clippy --all-targets -- -D warnings`
    - `scripts/release-check.ps1 -Server everything`
    - `scripts/release-check.ps1 -Server all -SkipBuild`
    - `git diff --check`
  - Acceptance:
    - update this file with evidence
    - do not mark P5 complete unless SDK smoke covers tools, prompts, resources, and documented protocol feature behavior.
  - Evidence:
    - `cargo fmt --check` passed.
    - `cargo test --all-targets` passed.
    - `cargo clippy --all-targets -- -D warnings` passed.
    - `scripts\release-check.ps1 -Server all -SkipBuild` passed, including `everything`.
    - `git diff --check` passed.
    - `cargo run -p mcp-servers -- inventory` lists `everything` as `ImplementedPort`.
    - Extra SDK `tools/call` smokes passed for `get-sum`, `get-structured-content`,
      and `gzip-file-as-resource`.
    - Added SDK prompts/resources smoke coverage for `everything`.
    - P5 protocol behavior is complete for the `rmcp`-supported feature set; elicitation remains explicitly deferred.

### P6: Documentation Templates And Release Workflow

- [x] P6.1 Add per-server README template.
  - Output: `docs/templates/server-readme.md`.
  - Must include: overview, tool list, safety notes, install, Codex/OpenCode config,
    `--version`, smoke tests, release notes.
  - Evidence:
    - Added `docs/templates/server-readme.md` with status, tools, prompts/resources,
      safety notes, install, Codex/OpenCode config, verification, known differences,
      and release notes sections.

- [x] P6.2 Add parity table template.
  - Output: `docs/templates/parity-table.md`.
  - Must include: upstream feature/tool, Rust feature/tool, status, tests, notes.
  - Evidence:
    - Added `docs/templates/parity-table.md` with feature parity, protocol coverage,
      safety parity, verification commands, status definitions, and remaining work.

- [ ] P6.3 Add GitHub Release workflow.
  - Output: `.github/workflows/release.yml`.
  - Must produce assets listed in `README.md`.
  - Must run `scripts/release-check.ps1` or equivalent before upload.
  - Must publish checksums.

## Suggested First Task For A New Agent

Pick `N5` next: run final readiness checks, review the staged files, commit the Rust MCP workspace, and push to GitHub.
