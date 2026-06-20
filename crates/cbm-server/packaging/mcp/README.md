# cbm MCP package

Handoff templates for agents wiring **cbm** (graph index only).

Agent config key: `cbm`
MCP `serverInfo.name`: `codebase-memory-mcp`
Transport: stdio
Binary: `cbm` / `cbm.exe` or absolute path to the release binary

Protocol handling, capability negotiation, tool routing, and stdio framing are
provided by the official Rust MCP SDK (`rmcp 1.7.0`).

## Fast path

```powershell
.\install.ps1
```

The checkout installer downloads the latest GitHub Release binary, verifies checksums, installs agent MCP config, and writes hooks. It does not compile Rust. Use `.\install.ps1 -FromSource` only for local unreleased development.

## Manual config

| Template | Target |
|----------|--------|
| `generic-mcp.json` | Claude-style `mcpServers`, Gemini CLI, Zed |
| `codex-config.toml` | Codex `config.toml` snippet |
| `opencode.json` | OpenCode `opencode.json` snippet |
| `claude-settings.json` | Claude Code / Desktop settings |
| `manifest.json` | Machine-readable package summary |
| `dual-servers.example.json` | Optional second server: `rlm-mcp` |

Replace `{{CBM_BINARY}}` with an absolute binary path.

## Environment

```json
{
  "CBM_PROJECT_PREFIX": "cbm+",
  "CBM_AGENT": "generic"
}
```

Legacy aliases `CBRLM_*` are still accepted by the binary.

## Tool contract (14 graph tools)

`index_repository`, `index_status`, `search_graph`, `trace_path`, `get_code_snippet`, `get_graph_schema`, `get_architecture`, `query_graph`, `search_code`, `list_projects`, `delete_project`, `detect_changes`, `manage_adr`, `ingest_traces`

Use graph tools before broad file search when a project is indexed.

## RLM (separate project)

RLM session tools (`rlm_scan`, `rlm_peek`, `rlm_chunk`, `rlm_workflow`, ...) live in **[rlm-mcp](https://github.com/stevenke1981/rlm-mcp)** as MCP server **`rlm-mcp`**.

- Not bundled with this binary
- No code dependency between repos
- Optional: enable both servers in the same agent (see `dual-servers.example.json`)
