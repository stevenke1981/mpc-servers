# Rust MCP Servers Workspace

這個 repo 是把 `stevenke1981/servers.git` 的 MCP reference servers 逐步改寫成我們自己的 Rust MCP servers 的工作區，同時納入已經開發過、可以直接沿用的 Rust MCP 專案。

目前定位是「可版本化的 Rust workspace + 復用矩陣 + 後續 server crate 的落地起點」。

## Source Snapshot

本次盤點來源：

- `stevenke1981/servers.git`: `main@7b1170d`
- `stevenke1981/memlong.git`
- `stevenke1981/nushell-mcp.git`
- `stevenke1981/rlm-mcp.git`
- `stevenke1981/cbm-mcp.git`

`servers.git` 目前 reference servers：

| Server | 原始語言 | 上游版本 | Rust 策略 |
|---|---:|---:|---|
| `memory` | TypeScript | `0.6.3` | 直接以 `memlong` 作為 Rust 線 |
| `filesystem` | TypeScript | `0.6.3` | 需要 Rust port |
| `git` | Python | `0.6.2` | 需要 Rust port |
| `time` | Python | `0.6.2` | 需要 Rust port，適合第一個小型 port |
| `fetch` | Python | `0.6.3` | 需要 Rust port，先定安全策略 |
| `sequential-thinking` | TypeScript | `0.6.2` | 需要 Rust port，適合第二批 |
| `everything` | TypeScript | `2.0.0` | 作為 MCP protocol feature testbed |

## 可直接使用的既有專案

| 專案 | 判斷 | 原因 |
|---|---|---|
| `memlong` | 可直接使用，但需整理後納入 | 已是 Rust workspace，涵蓋 memory MCP server、CLI、核心資料層；可取代上游 `memory` reference server。 |
| `rlm-mcp` | 可直接使用 | 已是 Rust `rmcp` server，版本 `0.1.6`，適合保持獨立 server 線。 |
| `cbm-mcp` | 可直接使用 | 已是 Rust `rmcp` server，版本 `0.2.3`，而且 release/install/OpenCode/Codex 驗證流程最完整。 |
| `nushell-mcp` | 可直接使用 | 已是 Rust `rmcp` stdio server，適合作為本地 shell/server automation 能力；它不是 `servers.git` 的一對一 port。 |

## Workspace Layout

```text
crates/
  server-inventory/   # 來源、版本、復用策略的 typed inventory
  mcp-servers/        # workspace 管理 CLI
VERSIONING.md         # 版本號與 release 策略
```

目前 CLI：

```powershell
cargo run -p mcp-servers -- list
cargo run -p mcp-servers -- inventory
cargo run -p mcp-servers -- --version
```

## 開發原則

- 所有 server 都以 Rust crate 管理，優先使用官方 `rmcp` crate 與 stdio transport。
- stdout 在 MCP 模式下只輸出 protocol 訊息；logs/errors 走 stderr。
- 每個 server 都需要 `--version`、README、安裝方式、MCP smoke test。
- Windows install 不依賴 `target/release`，release 後預設下載 GitHub Release binary。
- OpenCode/Codex 相容性要用實際 `tools/list` schema smoke test 驗證。

## 建議落地順序

1. 整理 `cbm-mcp` 的 release/install 範式成 workspace 共用模板。
2. 將 `memlong` 作為 `memory` server 線導入，並評估 `rmcp` 版本對齊。
3. 先 port `time`，用它驗證 workspace crate、README、installer、release checklist。
4. port `sequential-thinking`，驗證 stateful tool 模式。
5. port `filesystem` 和 `git`，這兩個需要較完整的安全邊界與 Windows path/process 測試。
6. port `fetch`，先補 HTTP allowlist、redirect、content-size、SSRF 防護策略。
7. 用 `everything` 建立 MCP feature compatibility testbed。

## 驗證

```powershell
cargo fmt --check
cargo test --all-targets
cargo run -p mcp-servers -- inventory
```
