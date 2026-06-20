# nushell-mcp

`nushell-mcp` is a Rust stdio MCP server that exposes trusted local Nushell execution through structured MCP tools.

## Tools

Core Nushell tools:

- `nu_version`: runs `nu --version`.
- `nu_eval`: runs inline code with `nu --no-config-file --commands <command>`.
- `nu_script`: runs a `.nu` script file with optional positional arguments.

Nushell convenience tools inspired by `git-bash-opencode-plugin`:

- `nu_grep`: search file contents with pattern/path/recursive/ignore-case options.
- `nu_find`: find files or directories with name, extension, type, and recursion filters.
- `nu_read`: read file head, tail, or first lines with optional line numbers.
- `nu_ls`: list directory contents.

Git workflow tools inspired by `git-opencode-plugin`:

- `git_status`: working tree status, optionally porcelain.
- `git_diff`: unstaged, staged, or ref diff.
- `git_log`: recent commit history.
- `git_tree`: commit graph with branch topology.
- `git_branch`: list, create, or switch branches.
- `git_commit`: stage files and commit through a temporary message file.
- `git_stash`: push, pop, list, or drop stashes.
- `git_precommit_review`: summarize staged changes before committing.

Every tool returns JSON text with:

- `ok`
- `exit_code`
- `signal`
- `timed_out`
- `duration_ms`
- `stdout`
- `stderr`
- `stdout_truncated`
- `stderr_truncated`
- `command`
- `error`

## Build

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release
```

The release binary is written to:

```text
D:\nushell-mcp\target\release\nushell-mcp.exe
```

Nushell must be on `PATH` as `nu`, or configured with:

```powershell
$env:NUSHELL_MCP_NU_PATH = "C:\Path\To\nu.exe"
```

Quick checks:

```powershell
.\target\release\nushell-mcp.exe --version
.\target\release\nushell-mcp.exe health --json
powershell -ExecutionPolicy Bypass -File .\scripts\mcp_smoke.ps1
```

## OpenCode

Add this server to `opencode.json` or `opencode.jsonc`:

```json
{
  "mcp": {
    "nushell": {
      "type": "local",
      "command": ["D:\\nushell-mcp\\target\\release\\nushell-mcp.exe"],
      "enabled": true,
      "timeout": 120000,
      "environment": {
        "NUSHELL_MCP_NU_PATH": "C:\\Path\\To\\nu.exe"
      }
    }
  }
}
```

If `nu` is already on `PATH`, omit `environment`.

## Codex

Add this to `config.toml`:

```toml
[mcp_servers.nushell]
command = "D:/nushell-mcp/target/release/nushell-mcp.exe"
args = []

[mcp_servers.nushell.env]
NUSHELL_MCP_NU_PATH = "C:/Path/To/nu.exe"
```

If `nu` is already on `PATH`, omit the `env` block.

## Security

This server intentionally executes arbitrary Nushell and Git commands with the permissions of the MCP host process. Enable it only for trusted local clients and trusted workspaces.

The implementation invokes `nu` directly with argument vectors, not through another shell. It also enforces bounded timeouts and per-stream output capture limits, but it is not a sandbox.

## Reference Plugins

The expanded tool set borrows workflow ideas from:

- `https://github.com/stevenke1981/git-opencode-plugin.git`
- `https://github.com/stevenke1981/git-bash-opencode-plugin.git`

This project remains an MCP server. It does not copy OpenCode plugin hooks such as message transforms, compaction guidance, or `shell.env`.
