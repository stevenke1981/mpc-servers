# Rust MCP Servers Implementation Plan

本計畫用來把 `stevenke1981/servers.git` 的 reference servers 與既有 Rust MCP 專案整理成同一個可 release 的 Rust workspace。規格來源以 `spec.md` 為準。

## 原則

- 先導入可直接使用的 Rust server，再 port reference servers。
- 每一批都要保留原本功能，並用 parity table 驗收。
- 每個 server 都以獨立 crate 實作，避免單一巨型 binary。
- Installer、版本號、README、schema smoke test 要和功能一起完成。

## Phase 0: Workspace Foundation

狀態：已完成。

已完成項目：

- 初始化 git repo。
- 建立 Rust workspace。
- 建立 `server-inventory` 與 `mcp-servers` 管理 CLI。
- 建立 `README.md` 與 `VERSIONING.md`。
- 建立本檔、`spec.md`、`todos.md` 作為代理交接文件。

驗收：

- `cargo fmt --check`
- `cargo test --all-targets`
- `cargo clippy --all-targets -- -D warnings`

## Phase 1: Release/Installer Template

目標：先從 `cbm-mcp` 抽出最可靠的 release 與 install 模式，避免每個 server 重做一次。

狀態：基礎文件、驗證工具與 release-check automation 已完成。

工作：

- 建立 `packaging/` 或 `crates/installer-support/` 共用模板。
- 定義 Windows stable path 與 locked binary fallback。
- 定義 `install --json` report schema。
- 建立 OpenCode/Codex config update 規則。
- 建立 TypeScript SDK `tools/list` smoke script。

目前完成：

- `packaging/README.md`：release/install checklist、Windows locked binary fallback、Codex/OpenCode 範例。
- `packaging/install-report.schema.json` 與 `packaging/install-report.example.json`。
- `scripts/tools-list-smoke.ps1`：使用 `@modelcontextprotocol/sdk` 執行 stdio `tools/list` 與選用 `tools/call`。
- `scripts/prompts-resources-smoke.ps1`：使用 `@modelcontextprotocol/sdk` 驗證 stdio `prompts/list`、`prompts/get`、`resources/list`、`resources/templates/list`、`resources/read`。
- `scripts/release-check.ps1`：整合 fmt、test、clippy、release build、`--version`、MCP SDK smoke 與 installer report schema check。

驗收：

- 可由任一 server crate 套用模板。
- README 不指向 `target/release` 作為正式安裝路徑。
- smoke script 能驗證 public tool schema。

## Phase 2: Import Existing Rust Servers

目標：導入已可直接使用的 Rust MCP 專案。

狀態：既有 Rust MCP server 匯入已完成第一輪，見 `docs/import-strategy.md`。

策略：採用逐一 source vendor import 到 `crates/` 的方式。先導入 `cbm-mcp`，
再導入 `rlm-mcp`、`nushell-mcp`、`memlong`。不使用 git submodule 作為第一階段
主策略，也不以外部 release binary wrapper 作為最終實作。

目前完成：

- `cbm-mcp` 已導入 `crates/cbm-server`。
- Package name 保留 `codebase-memory-mcp`，binary name 保留 `cbm`。
- Root install/uninstall scripts、release-check、README、spec、todos 與 packaging example 已納入 `cbm`。
- `cbm` package tests、clippy、release build、MCP SDK smoke、install/uninstall smoke 已通過。
- `rlm-mcp` 已導入 `crates/rlm-server`。
- Package name 與 binary name 保留 `rlm-mcp`。
- Root install/uninstall scripts、release-check、README、spec、todos 與 packaging example 已納入 `rlm`。
- `rlm` package tests、clippy、release build、MCP SDK smoke、install/uninstall smoke 已通過。
- `nushell-mcp` 已導入 `crates/nushell-server`。
- Package name 與 binary name 保留 `nushell-mcp`。
- Root install/uninstall scripts、release-check、README、spec、todos 與 packaging example 已納入 `nushell`。
- `nushell` package tests、clippy、release build、MCP SDK smoke、install/uninstall smoke 已通過。
- `memlong` 已導入 `crates/memory-core`、`crates/memory-server` 與 `crates/memory-cli`。
- Package name 與 binary name 保留 `memory-mcp-server`。
- Root install/uninstall scripts、release-check、README、spec、todos 與 packaging example 已納入 `memory`。
- `memory` core tests、MCP server tests、release build、MCP SDK smoke 已通過。

下一個目標：決定 memory graph-tool bridge 與 `everything` elicitation 後續策略，然後做最終 readiness、commit、push。

順序：

1. `cbm-mcp`
2. `rlm-mcp`
3. `nushell-mcp`
4. `memlong`

每個 server 的工作：

- 依 `docs/import-strategy.md` 做 source vendor import。
- 保留原 tool names。
- 對齊 `rmcp` 版本。
- 加上或保留 `--version`。
- 寫入 crate README。
- 補 parity table。
- 補 OpenCode/Codex config 範例。

Memory compatibility note:

- `memory` 目前採用 `memlong` replacement semantics，提供 7 個長期記憶 tools。
- TypeScript reference `memory` graph tools 尚未實作 parity bridge；若 client 需要
  `create_entities` / `read_graph` 等工具，需在後續階段補 compatibility layer。

驗收：

- 原 repo 既有測試通過。
- workspace `cargo test --all-targets` 通過。
- `tools/list` tool count 與 `spec.md` 對應。
- 既有 release tag 與 workspace version 策略不衝突。

## Phase 3: First Reference Ports

目標：用小 server 打通完整 Rust port 範式。

順序：

1. `time`
2. `sequential-thinking`

`time` 完成標準：

- `get_current_time`
- `convert_time`
- IANA timezone 驗證。
- invalid timezone error。
- snapshot/contract tests 覆蓋輸出欄位。

`sequential-thinking` 完成標準：

- `sequentialthinking`
- session-scoped thought state。
- revision/branch fields 行為相容。
- contract tests 覆蓋 thought history 與 branch 狀態。

## Phase 4: File And Git Ports

目標：實作風險較高、需完整安全邊界的本地能力。

狀態：`filesystem` 與 `git` 已完成初版 production port。

順序：

1. `filesystem`
2. `git`

`filesystem` 必須先完成：

- allowed directories model。
- MCP Roots support。
- path canonicalization。
- symlink escape 防護。
- Windows case/path 測試。
- dry-run edit diff。

`git` 必須先完成：

- repo boundary validation。
- native process invocation。
- no shell string command。
- destructive tools annotations。
- branch/log/diff/commit test fixtures。

目前完成：

- `crates/git-server` 保留 12 個 upstream git tools。
- CLI 支援 `--repository` / `-r` 限制 repository 邊界。
- 單元測試覆蓋 status、diff、log、branch、checkout、commit、show、path traversal 與 flag injection。

## Phase 5: Fetch Port

目標：在安全策略清楚後實作網路 fetch。

狀態：已完成初版 production port。

前置決策：

- SSRF 防護策略。
- private IP / localhost policy。
- redirect policy。
- max response bytes。
- timeout。
- content type handling。

完成標準：

- `fetch`
- HTML to text extraction。
- truncation message。
- network error mapping。
- README security notes。

目前完成：

- `docs/fetch-security.md` 定義 public-web default、SSRF 防護、redirect、timeout、max bytes、User-Agent 與 proxy 策略。
- `crates/fetch-server` 實作 `fetch` tool。
- CLI 支援 `--allow-private-network`、`--user-agent`、`--proxy-url`、`--max-bytes`、`--timeout-seconds`、`--redirect-limit`。
- installer、uninstaller、release-check、README、spec 與 packaging 文件已納入 `fetch`。

驗收：

- `cargo test -p fetch-server`
- `cargo clippy -p fetch-server --all-targets -- -D warnings`
- `scripts/tools-list-smoke.ps1` 對 `fetch-server` 回傳 1 個 tool。
- `scripts/release-check.ps1` 全 server 通過。
- `install.ps1 -FromSource -SkipBuild -Server all -Json` 與 `uninstall.ps1 -Server all -Json` 對 5 個已實作 server 通過。

## Phase 6: Everything Compatibility Testbed

目標：建立 Rust MCP compatibility server，用來測 prompts、resources、tools、roots、logging、sampling、elicitation 等 MCP feature。

狀態：初版可用。`crates/everything-server` 已提供 stdio transport、19 個 tools、4 個 prompts、static/dynamic/session resources 與 resource templates。`gzip-file-as-resource` 已支援真 gzip 壓縮與 bounded `data:`/HTTP(S) 載入。TypeScript MCP SDK tools smoke、prompts/resources smoke 與 active protocol smoke 已通過。`rmcp` 支援的 roots sync、subscriptions/resource update notifications、logging level/message、sampling request 與 progress notification path 已接上 deterministic fallback，並由 `scripts/everything-protocol-smoke.ps1` 驗證 active client capability path；elicitation 仍明確 deferred，詳見 `docs/parity/everything.md`。

完成標準：

- 覆蓋 `spec.md` 的 everything feature list。
- 可在 CI 中被 client smoke tests 使用。
- 不作為 production server 安裝入口。

## 持續維護

- 每完成一個 server，就更新 `spec.md` 的完成狀態與 `todos.md`。
- 每次 release 前執行 `VERSIONING.md` checklist。
- 每個 server 的 README 必須能讓新代理或新開發者不用讀聊天紀錄也能操作。
- 新增或重整 server 時，使用 `docs/templates/server-readme.md` 與
  `docs/templates/parity-table.md`。
- GitHub Release assets 由 `.github/workflows/release.yml` 產生，並包含
  README/packaging 文件列出的 archives 與 `SHA256SUMS.txt`。
