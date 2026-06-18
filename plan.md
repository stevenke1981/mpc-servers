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

工作：

- 建立 `packaging/` 或 `crates/installer-support/` 共用模板。
- 定義 Windows stable path 與 locked binary fallback。
- 定義 `install --json` report schema。
- 建立 OpenCode/Codex config update 規則。
- 建立 TypeScript SDK `tools/list` smoke script。

驗收：

- 可由任一 server crate 套用模板。
- README 不指向 `target/release` 作為正式安裝路徑。
- smoke script 能驗證 public tool schema。

## Phase 2: Import Existing Rust Servers

目標：導入已可直接使用的 Rust MCP 專案。

順序：

1. `cbm-mcp`
2. `rlm-mcp`
3. `nushell-mcp`
4. `memlong`

每個 server 的工作：

- 決定是 git subtree、workspace crate copy、還是保留外部 repo 並用 wrapper 管理。
- 保留原 tool names。
- 對齊 `rmcp` 版本。
- 加上或保留 `--version`。
- 寫入 crate README。
- 補 parity table。
- 補 OpenCode/Codex config 範例。

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

## Phase 5: Fetch Port

目標：在安全策略清楚後實作網路 fetch。

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

## Phase 6: Everything Compatibility Testbed

目標：建立 Rust MCP compatibility server，用來測 prompts、resources、tools、roots、logging、sampling、elicitation 等 MCP feature。

完成標準：

- 覆蓋 `spec.md` 的 everything feature list。
- 可在 CI 中被 client smoke tests 使用。
- 不作為 production server 安裝入口。

## 持續維護

- 每完成一個 server，就更新 `spec.md` 的完成狀態與 `todos.md`。
- 每次 release 前執行 `VERSIONING.md` checklist。
- 每個 server 的 README 必須能讓新代理或新開發者不用讀聊天紀錄也能操作。
