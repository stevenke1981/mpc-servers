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
| `memory` | TypeScript | `0.6.3` | Use `memlong` as the Rust line |
| `filesystem` | TypeScript | `0.6.3` | Ported: `crates/filesystem-server` |
| `git` | Python | `0.6.2` | Pending Rust port |
| `time` | Python | `0.6.2` | Ported: `crates/time-server` |
| `fetch` | Python | `0.6.3` | Pending Rust port; security policy first |
| `sequential-thinking` | TypeScript | `0.6.2` | Ported: `crates/sequential-thinking-server` |
| `everything` | TypeScript | `2.0.0` | Planned MCP protocol feature testbed |

## Workspace Layout

```text
crates/
  filesystem-server/         # Filesystem MCP server with path safety and MCP Roots
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
```

## Build And Verify

```powershell
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -p mcp-servers -- inventory
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

- `filesystem`
- `time`
- `sequential-thinking`
- `all`

Installed binary names:

- `filesystem-server`
- `time-server`
- `sequential-thinking-server`

## Client Configuration

Use the installed binary path from the install report. Do not point agents at
`target/release`.

Example Codex config:

```toml
[mcp_servers.filesystem]
command = "C:/Users/you/.config/mpc-servers/bin/filesystem-server.exe"
args = ["D:/workspace"]

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
    "filesystem": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\filesystem-server.exe", "D:\\workspace"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    }
  }
}
```

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

## Next Work

1. Port `git` with strict repository boundary validation and native process args.
2. Define fetch SSRF/redirect/timeout/max-bytes policy before coding `fetch`.
3. Import or wrap `cbm-mcp`, `rlm-mcp`, `nushell-mcp`, and `memlong`.
4. Build common release packaging around these root install scripts.

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
| `memory` | TypeScript | `0.6.3` | 以 `memlong` 作為 Rust 線 |
| `filesystem` | TypeScript | `0.6.3` | 已 port：`crates/filesystem-server` |
| `git` | Python | `0.6.2` | 待 Rust port |
| `time` | Python | `0.6.2` | 已 port：`crates/time-server` |
| `fetch` | Python | `0.6.3` | 待 Rust port，需先定安全策略 |
| `sequential-thinking` | TypeScript | `0.6.2` | 已 port：`crates/sequential-thinking-server` |
| `everything` | TypeScript | `2.0.0` | 預計作為 MCP protocol feature testbed |

## 工作區結構

```text
crates/
  filesystem-server/         # 含 path safety 與 MCP Roots 的 filesystem MCP server
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
```

## 建置與驗證

```powershell
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -p mcp-servers -- inventory
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

- `filesystem`
- `time`
- `sequential-thinking`
- `all`

安裝後的 binary 名稱：

- `filesystem-server`
- `time-server`
- `sequential-thinking-server`

## Client 設定

請使用 install report 裡的實際 binary path。不要把 agent config 指到
`target/release`。

Codex config 範例：

```toml
[mcp_servers.filesystem]
command = "C:/Users/you/.config/mpc-servers/bin/filesystem-server.exe"
args = ["D:/workspace"]

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
    "filesystem": {
      "type": "local",
      "command": ["C:\\Users\\you\\.config\\mpc-servers\\bin\\filesystem-server.exe", "D:\\workspace"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    }
  }
}
```

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

## 下一步

1. port `git`，保留 repository 邊界驗證與 native process args。
2. 實作 `fetch` 前先定 SSRF、redirect、timeout、max-bytes 策略。
3. 導入或包裝 `cbm-mcp`、`rlm-mcp`、`nushell-mcp`、`memlong`。
4. 以本次 root install scripts 為基礎建立共用 release packaging。
