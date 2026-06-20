# Rust MCP Servers Workspace

`mpc-servers` is a Rust workspace for rewriting the MCP reference servers from
[`stevenke1981/servers`](https://github.com/stevenke1981/servers) into our own
Rust MCP servers, while reusing existing Rust projects where they are already a
good fit.

The workspace is git-managed, versioned, and designed for local stdio MCP usage
with Codex, OpenCode, Claude Desktop, and other MCP clients.

## Source Snapshot

- `stevenke1981/servers.git`: `main@7b1170d`
- `stevenke1981/memlong.git`
- `stevenke1981/nushell-mcp.git`
- `stevenke1981/rlm-mcp.git`
- `stevenke1981/cbm-mcp.git`

## Server Status

| Server | Source language | Upstream version | Rust status |
|---|---:|---:|---|
| `memory` | TypeScript | `0.6.3` | Imported from `memlong`: `crates/memory-server` + `crates/memory-core` |
| `cbm` | Rust | `0.2.3` | Imported: `crates/cbm-server` |
| `rlm` | Rust | `0.1.6` | Imported: `crates/rlm-server` |
| `nushell` | Rust | `0.1.0` | Imported: `crates/nushell-server` |
| `filesystem` | TypeScript | `0.6.3` | Ported: `crates/filesystem-server` |
| `git` | Python | `0.6.2` | Ported: `crates/git-server` |
| `time` | Python | `0.6.2` | Ported: `crates/time-server` |
| `fetch` | Python | `0.6.3` | Ported: `crates/fetch-server` |
| `sequential-thinking` | TypeScript | `0.6.2` | Ported: `crates/sequential-thinking-server` |
| `everything` | TypeScript | `2.0.0` | Ported testbed: `crates/everything-server` |

## Workspace Layout

```text
crates/
  cbm-server/                # Imported codebase-memory MCP server
  rlm-server/                # Imported RLM MCP server
  nushell-server/            # Imported Nushell MCP server
  memory-core/               # Imported memlong storage, retrieval, and consolidation core
  memory-server/             # Imported memlong MCP server
  memory-cli/                # Imported memlong debug/maintenance CLI
  filesystem-server/         # Filesystem MCP server with path safety and MCP Roots
  fetch-server/              # Bounded web fetch MCP server with SSRF protections
  git-server/                # Git MCP server with repository validation
  everything-server/         # MCP compatibility testbed for tools/prompts/resources
  sequential-thinking-server/# Stateful sequential thinking MCP server
  time-server/               # Timezone conversion/current time MCP server
  server-inventory/          # Typed source/version/reuse inventory
  mcp-servers/               # Workspace management CLI
install.ps1 / install.sh     # Release-first installers
uninstall.ps1 / uninstall.sh # Local uninstall helpers
VERSIONING.md                # Version and release policy
spec.md                      # Required parity and safety requirements
plan.md                      # Implementation plan
todos.md                     # Agent-ready task list
docs/templates/              # Copyable README/parity templates for future ports
.github/workflows/release.yml # GitHub Release asset workflow
```

## Build And Verify

```powershell
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -p mcp-servers -- inventory
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\git-server.exe -ExpectedToolCount 12
.\scripts\release-check.ps1
```

## Install

The default installer expects GitHub Release assets and does not compile from
source. Use the source option for local development until release assets exist.

Expected release asset names:

- `mpc-servers-windows-x86_64.zip`
- `mpc-servers-windows-aarch64.zip`
- `mpc-servers-linux-x86_64.tar.gz`
- `mpc-servers-linux-aarch64.tar.gz`
- `mpc-servers-macos-x86_64.tar.gz`
- `mpc-servers-macos-aarch64.tar.gz`

### Windows

```powershell
# Install all implemented MCP servers from GitHub Release
.\install.ps1

# Install one server from a specific release tag
.\install.ps1 -Server filesystem -Version v0.1.0

# Local development install from this checkout
.\install.ps1 -FromSource -Server all

# Machine-readable report
.\install.ps1 -FromSource -Json
```

### Linux/macOS

```bash
# Install all implemented MCP servers from GitHub Release
./install.sh

# Install one server from a specific release tag
./install.sh --server filesystem --version v0.1.0

# Local development install from this checkout
./install.sh --from-source --server all

# Machine-readable report
./install.sh --from-source --json
```

By default, binaries are installed to:

- Windows: `%USERPROFILE%\.config\mpc-servers\bin`
- Linux/macOS: `$HOME/.config/mpc-servers/bin`

Implemented server names accepted by the installers:

- `cbm`
- `everything`
- `filesystem`
- `fetch`
- `git`
- `memory`
- `nushell`
- `rlm`
- `time`
- `sequential-thinking`
- `all`

Installed binary names:

- `cbm`
- `filesystem-server`
- `fetch-server`
- `git-server`
- `everything-server`
- `memory-mcp-server`
- `nushell-mcp`
- `rlm-mcp`
- `time-server`
- `sequential-thinking-server`

## Client Configuration

Use the installed binary path from the install report. Do not point agents at
`target/release`.

Example Codex config:

```toml
[mcp_servers.cbm]
command = "C:/Users/you/.config/mpc-servers/bin/cbm.exe"
args = []

[mcp_servers.filesystem]
command = "C:/Users/you/.config/mpc-servers/bin/filesystem-server.exe"
args = ["D:/workspace"]

[mcp_servers.git]
command = "C:/Users/you/.config/mpc-servers/bin/git-server.exe"
args = ["--repository", "D:/workspace/repo"]

[mcp_servers.fetch]
command = "C:/Users/you/.config/mpc-servers/bin/fetch-server.exe"
args = []

[mcp_servers.memory]
command = "C:/Users/you/.config/mpc-servers/bin/memory-mcp-server.exe"
args = []

[mcp_servers.memory.env]
LLM_API_BASE = "http://localhost:8080/v1"
LLM_API_KEY = "local"
PROJECT_ROOT = "D:/workspace"

[mcp_servers.nushell]
command = "C:/Users/you/.config/mpc-servers/bin/nushell-mcp.exe"
args = []

[mcp_servers.rlm]
command = "C:/Users/you/.config/mpc-servers/bin/rlm-mcp.exe"
args = []

[mcp_servers.time]
command = "C:/Users/you/.config/mpc-servers/bin/time-server.exe"
args = []

[mcp_servers.sequential-thinking]
command = "C:/Users/you/.config/mpc-servers/bin/sequential-thinking-server.exe"
args = []
```

Example OpenCode config:

```json
{
  "mcp": {
    "cbm": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\cbm.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "filesystem": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\filesystem-server.exe", "D:\\workspace"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "git": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\git-server.exe", "--repository", "D:\\workspace\\repo"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "fetch": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\fetch-server.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "memory": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\memory-mcp-server.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {
        "LLM_API_BASE": "http://localhost:8080/v1",
        "LLM_API_KEY": "local",
        "PROJECT_ROOT": "D:\\workspace"
      }
    },
    "nushell": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\nushell-mcp.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "rlm": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\rlm-mcp.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    }
  }
}
```

## Memory Configuration And Compatibility

The `memory` server is the imported Rust `memlong` line. It exposes the long-term
memory tools `add_memory`, `search_memories`, `get_memories`, `delete_memory`,
`consolidate_memories`, `get_memory_stats`, and `end_session`.

Default data lives under `PROJECT_ROOT/.opencode/`:

- `MEMORY_DB_PATH`: SQLite metadata database
- `MEMORY_VECTOR_PATH`: USearch vector index
- `MEMORY_TANTIVY_PATH`: Tantivy BM25 index directory

If those variables are omitted, set `PROJECT_ROOT` to the workspace whose memory
should be isolated. Set `LLM_API_BASE`, `LLM_API_KEY`, `EXTRACTION_MODEL`,
`EMBEDDING_MODEL`, and `EMBEDDING_DIM` to match the local or hosted
OpenAI-compatible endpoint.

Compatibility note: the TypeScript reference `memory` server exposes knowledge
graph tools such as `create_entities`, `create_relations`, `read_graph`,
`search_nodes`, and `open_nodes`. This workspace currently preserves `memlong`
semantics instead of claiming graph-tool parity. Use `add_memory` and
`search_memories` for durable fact extraction and hybrid retrieval; graph-tool
compatibility remains a future bridge task. See
[docs/parity/memory.md](docs/parity/memory.md).

## Uninstall

```powershell
.\uninstall.ps1
.\uninstall.ps1 -Server filesystem
.\uninstall.ps1 -Json
```

```bash
./uninstall.sh
./uninstall.sh --server filesystem
./uninstall.sh --json
```

Uninstall scripts remove installed binaries from the install directory. They do
not edit Codex, OpenCode, or Claude configuration files.

## Development Rules

- All production servers use Rust crates and the official `rmcp` crate.
- MCP mode keeps stdout protocol-only; logs and diagnostics go to stderr.
- Every server binary must support `--version`, `-V`, and `version`.
- Public tool names must preserve upstream names.
- Path/process/network operations must keep explicit safety boundaries.
- OpenCode/Codex compatibility must be verified with real `tools/list` smoke tests.

Reusable MCP SDK smoke:

```powershell
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\time-server.exe -ExpectedToolCount 2 -ExpectedTools get_current_time,convert_time
.\scripts\everything-protocol-smoke.ps1 -Binary .\target\debug\everything-server.exe
.\scripts\prompts-resources-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedPromptCount 4 -ExpectedPrompts simple-prompt,args-prompt,completable-prompt,resource-prompt
```

Installer JSON reports must match [packaging/install-report.schema.json](packaging/install-report.schema.json).

Full release readiness check:

```powershell
.\scripts\release-check.ps1
```

GitHub Release asset workflow:

- [.github/workflows/release.yml](.github/workflows/release.yml)
- Produces the documented Windows/Linux/macOS archives and `SHA256SUMS.txt`.
- Runs `scripts/release-check.ps1` before packaging/upload.

Documentation templates for future server ports:

- [docs/templates/server-readme.md](docs/templates/server-readme.md)
- [docs/templates/parity-table.md](docs/templates/parity-table.md)

## Next Work

1. Decide whether to enable `rmcp` elicitation for `everything`; current parity is documented in [docs/parity/everything.md](docs/parity/everything.md).
2. Run final readiness, commit, and push.

---

# Rust MCP Servers Workspace（繁體中文）

`mpc-servers` 是一個 Rust workspace，用來把
[`stevenke1981/servers`](https://github.com/stevenke1981/servers) 的 MCP
reference servers 重新改寫成我們自己的 Rust MCP servers，同時沿用已經成熟的
Rust MCP 專案。

這個 workspace 使用 git 管理、版本化，目標是提供 Codex、OpenCode、Claude
Desktop 與其他 MCP client 可直接使用的本機 stdio MCP servers。

## 來源快照

- `stevenke1981/servers.git`: `main@7b1170d`
- `stevenke1981/memlong.git`
- `stevenke1981/nushell-mcp.git`
- `stevenke1981/rlm-mcp.git`
- `stevenke1981/cbm-mcp.git`

## Server 狀態

| Server | 原始語言 | 上游版本 | Rust 狀態 |
|---|---:|---:|---|
| `memory` | TypeScript | `0.6.3` | 已從 `memlong` 導入：`crates/memory-server` + `crates/memory-core` |
| `cbm` | Rust | `0.2.3` | 已導入：`crates/cbm-server` |
| `rlm` | Rust | `0.1.6` | 已導入：`crates/rlm-server` |
| `nushell` | Rust | `0.1.0` | 已導入：`crates/nushell-server` |
| `filesystem` | TypeScript | `0.6.3` | 已 port：`crates/filesystem-server` |
| `git` | Python | `0.6.2` | 已 port：`crates/git-server` |
| `time` | Python | `0.6.2` | 已 port：`crates/time-server` |
| `fetch` | Python | `0.6.3` | 已 port：`crates/fetch-server` |
| `sequential-thinking` | TypeScript | `0.6.2` | 已 port：`crates/sequential-thinking-server` |
| `everything` | TypeScript | `2.0.0` | 已 port testbed：`crates/everything-server` |

## 工作區結構

```text
crates/
  cbm-server/                # 已導入的 codebase-memory MCP server
  rlm-server/                # 已導入的 RLM MCP server
  nushell-server/            # 已導入的 Nushell MCP server
  memory-core/               # 已導入的 memlong storage/retrieval/consolidation core
  memory-server/             # 已導入的 memlong MCP server
  memory-cli/                # 已導入的 memlong debug/maintenance CLI
  filesystem-server/         # 含 path safety 與 MCP Roots 的 filesystem MCP server
  fetch-server/              # 含 SSRF 防護的 bounded web fetch MCP server
  git-server/                # 含 repository validation 的 Git MCP server
  everything-server/         # MCP tools/prompts/resources compatibility testbed
  sequential-thinking-server/# stateful sequential thinking MCP server
  time-server/               # timezone current/convert MCP server
  server-inventory/          # typed source/version/reuse inventory
  mcp-servers/               # workspace 管理 CLI
install.ps1 / install.sh     # release-first 安裝器
uninstall.ps1 / uninstall.sh # 本機解除安裝工具
VERSIONING.md                # 版本與 release 政策
spec.md                      # 必須保留的功能與安全規格
plan.md                      # 實作計畫
todos.md                     # 可交給代理直接開工的任務清單
docs/templates/              # 後續 port 可複製的 README/parity templates
.github/workflows/release.yml # GitHub Release asset workflow
```

## 建置與驗證

```powershell
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -p mcp-servers -- inventory
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\git-server.exe -ExpectedToolCount 12
.\scripts\release-check.ps1
```

## 安裝

預設安裝器會使用 GitHub Release binary，不會從 source 編譯。release assets 尚未
建立前，請使用 source 安裝模式做本機驗證。

預期 release asset 名稱：

- `mpc-servers-windows-x86_64.zip`
- `mpc-servers-windows-aarch64.zip`
- `mpc-servers-linux-x86_64.tar.gz`
- `mpc-servers-linux-aarch64.tar.gz`
- `mpc-servers-macos-x86_64.tar.gz`
- `mpc-servers-macos-aarch64.tar.gz`

### Windows

```powershell
# 從 GitHub Release 安裝所有已實作 MCP servers
.\install.ps1

# 從指定 release tag 安裝單一 server
.\install.ps1 -Server filesystem -Version v0.1.0

# 從目前 checkout 做本機開發安裝
.\install.ps1 -FromSource -Server all

# 輸出 machine-readable report
.\install.ps1 -FromSource -Json
```

### Linux/macOS

```bash
# 從 GitHub Release 安裝所有已實作 MCP servers
./install.sh

# 從指定 release tag 安裝單一 server
./install.sh --server filesystem --version v0.1.0

# 從目前 checkout 做本機開發安裝
./install.sh --from-source --server all

# 輸出 machine-readable report
./install.sh --from-source --json
```

預設安裝位置：

- Windows: `%USERPROFILE%\.config\mpc-servers\bin`
- Linux/macOS: `$HOME/.config/mpc-servers/bin`

installer 接受的 server 名稱：

- `cbm`
- `everything`
- `filesystem`
- `fetch`
- `git`
- `memory`
- `nushell`
- `rlm`
- `time`
- `sequential-thinking`
- `all`

安裝後的 binary 名稱：

- `cbm`
- `filesystem-server`
- `fetch-server`
- `git-server`
- `everything-server`
- `memory-mcp-server`
- `nushell-mcp`
- `rlm-mcp`
- `time-server`
- `sequential-thinking-server`

## Client 設定

請使用 install report 裡的實際 binary path。不要把 agent config 指到
`target/release`。

Codex config 範例：

```toml
[mcp_servers.cbm]
command = "C:/Users/you/.config/mpc-servers/bin/cbm.exe"
args = []

[mcp_servers.filesystem]
command = "C:/Users/you/.config/mpc-servers/bin/filesystem-server.exe"
args = ["D:/workspace"]

[mcp_servers.git]
command = "C:/Users/you/.config/mpc-servers/bin/git-server.exe"
args = ["--repository", "D:/workspace/repo"]

[mcp_servers.fetch]
command = "C:/Users/you/.config/mpc-servers/bin/fetch-server.exe"
args = []

[mcp_servers.memory]
command = "C:/Users/you/.config/mpc-servers/bin/memory-mcp-server.exe"
args = []

[mcp_servers.memory.env]
LLM_API_BASE = "http://localhost:8080/v1"
LLM_API_KEY = "local"
PROJECT_ROOT = "D:/workspace"

[mcp_servers.nushell]
command = "C:/Users/you/.config/mpc-servers/bin/nushell-mcp.exe"
args = []

[mcp_servers.rlm]
command = "C:/Users/you/.config/mpc-servers/bin/rlm-mcp.exe"
args = []

[mcp_servers.time]
command = "C:/Users/you/.config/mpc-servers/bin/time-server.exe"
args = []

[mcp_servers.sequential-thinking]
command = "C:/Users/you/.config/mpc-servers/bin/sequential-thinking-server.exe"
args = []
```

OpenCode config 範例：

```json
{
  "mcp": {
    "cbm": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\cbm.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "filesystem": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\filesystem-server.exe", "D:\\workspace"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "git": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\git-server.exe", "--repository", "D:\\workspace\\repo"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "fetch": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\fetch-server.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "memory": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\memory-mcp-server.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {
        "LLM_API_BASE": "http://localhost:8080/v1",
        "LLM_API_KEY": "local",
        "PROJECT_ROOT": "D:\\workspace"
      }
    },
    "nushell": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\nushell-mcp.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    },
    "rlm": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\rlm-mcp.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    }
  }
}
```

## Memory 設定與相容性

`memory` server 是已導入的 Rust `memlong` 線，提供 `add_memory`、
`search_memories`、`get_memories`、`delete_memory`、`consolidate_memories`、
`get_memory_stats`、`end_session` 等長期記憶工具。

預設資料會放在 `PROJECT_ROOT/.opencode/`：

- `MEMORY_DB_PATH`：SQLite metadata database
- `MEMORY_VECTOR_PATH`：USearch vector index
- `MEMORY_TANTIVY_PATH`：Tantivy BM25 index directory

如果不手動指定這些路徑，請至少設定 `PROJECT_ROOT`，讓不同 workspace 的記憶資料
互相隔離。`LLM_API_BASE`、`LLM_API_KEY`、`EXTRACTION_MODEL`、`EMBEDDING_MODEL`
與 `EMBEDDING_DIM` 需對應本機或遠端 OpenAI-compatible endpoint。

相容性說明：TypeScript reference `memory` server 提供 `create_entities`、
`create_relations`、`read_graph`、`search_nodes`、`open_nodes` 等 knowledge graph
tools。目前 workspace 保留的是 `memlong` 語意，不宣稱 graph-tool parity；請用
`add_memory` 與 `search_memories` 做 durable fact extraction 與 hybrid retrieval。
reference graph tools bridge 是後續工作。詳見
[docs/parity/memory.md](docs/parity/memory.md)。

## 解除安裝

```powershell
.\uninstall.ps1
.\uninstall.ps1 -Server filesystem
.\uninstall.ps1 -Json
```

```bash
./uninstall.sh
./uninstall.sh --server filesystem
./uninstall.sh --json
```

解除安裝腳本只會移除 install directory 裡的 binaries，不會修改 Codex、OpenCode
或 Claude 設定檔。

## 開發原則

- production servers 都使用 Rust crate 與官方 `rmcp` crate。
- MCP 模式下 stdout 只輸出 protocol；logs 與 diagnostics 走 stderr。
- 每個 server binary 都必須支援 `--version`、`-V`、`version`。
- 公開 tool name 必須保留上游名稱。
- path/process/network 操作必須有明確安全邊界。
- OpenCode/Codex 相容性必須用實際 `tools/list` smoke tests 驗證。

可重用 MCP SDK smoke：

```powershell
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\time-server.exe -ExpectedToolCount 2 -ExpectedTools get_current_time,convert_time
.\scripts\everything-protocol-smoke.ps1 -Binary .\target\debug\everything-server.exe
.\scripts\prompts-resources-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedPromptCount 4 -ExpectedPrompts simple-prompt,args-prompt,completable-prompt,resource-prompt
```

Installer JSON report 必須符合 [packaging/install-report.schema.json](packaging/install-report.schema.json)。

完整 release readiness check：

```powershell
.\scripts\release-check.ps1
```

GitHub Release asset workflow：

- [.github/workflows/release.yml](.github/workflows/release.yml)
- 會產生文件列出的 Windows/Linux/macOS archives 與 `SHA256SUMS.txt`。
- 打包與上傳前會先執行 `scripts/release-check.ps1`。

後續 server port 文件模板：

- [docs/templates/server-readme.md](docs/templates/server-readme.md)
- [docs/templates/parity-table.md](docs/templates/parity-table.md)

## 下一步

1. 決定是否為 `everything` 啟用 `rmcp` elicitation；目前 parity 見 [docs/parity/everything.md](docs/parity/everything.md)。
2. 執行最終 readiness、commit、push。
