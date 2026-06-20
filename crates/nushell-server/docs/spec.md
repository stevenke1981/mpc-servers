# Nushell MCP Server Spec

## Goal

Build `nushell-mcp`, a Rust stdio MCP server that lets trusted local MCP clients run Nushell commands through explicit, structured tools.

## Non-Goals

- Do not implement a remote HTTP/SSE MCP server in the first version.
- Do not sandbox Nushell beyond process-level timeout, output limits, and explicit command arguments.
- Do not auto-install Nushell.
- Do not emit logs to stdout while running in MCP stdio mode.

## Runtime Contract

- Binary name: `nushell-mcp`.
- MCP server name: `nushell-mcp`.
- Transport: stdio.
- Logging: stderr only in MCP mode.
- Nushell executable resolution:
  1. Tool-level `nu_path` argument when available.
  2. `NUSHELL_MCP_NU_PATH`.
  3. `nu` from `PATH`.
- Default timeout: 30 seconds.
- Maximum timeout: 120 seconds.
- Default per-stream output capture limit: 1 MiB.
- Maximum per-stream output capture limit: 4 MiB.

## Tools

### Nushell Core Tools

### `nu_version`

Checks Nushell availability by running `nu --version`.

Input:

```json
{
  "nu_path": "optional path to nu or nu.exe"
}
```

Output:

```json
{
  "ok": true,
  "exit_code": 0,
  "signal": null,
  "timed_out": false,
  "duration_ms": 42,
  "stdout": "0.100.0",
  "stderr": "",
  "stdout_truncated": false,
  "stderr_truncated": false,
  "command": {
    "executable": "nu",
    "args": ["--version"],
    "cwd": "D:\\nushell-mcp"
  },
  "error": null
}
```

### `nu_eval`

Runs inline Nushell code using `nu --no-config-file --commands <command>`.

Input:

```json
{
  "command": "ls | length",
  "cwd": "optional working directory",
  "stdin": "optional stdin",
  "timeout_ms": 30000,
  "max_output_bytes": 1048576,
  "nu_path": "optional path to nu or nu.exe"
}
```

### `nu_script`

Runs a Nushell script file using `nu --no-config-file <script_path> ...args`.

Input:

```json
{
  "script_path": "D:\\work\\task.nu",
  "args": ["a", "b"],
  "cwd": "optional working directory",
  "stdin": "optional stdin",
  "timeout_ms": 30000,
  "max_output_bytes": 1048576,
  "nu_path": "optional path to nu or nu.exe"
}
```

### Nushell Convenience Tools

Inspired by `git-bash-opencode-plugin`, these tools provide common read/search/list operations through Nushell so agents do not need to hand-write common shell snippets.

- `nu_grep`: search file contents with pattern, path, recursive, ignore-case, line-number, and max-lines options.
- `nu_find`: find files/directories by path, name substring, extension, type, depth, and max-results options.
- `nu_read`: read file head, tail, or full content with optional line numbers.
- `nu_ls`: list directory contents with optional hidden and long output.

### Git Workflow Tools

Inspired by `git-opencode-plugin`, these tools wrap common Git workflows with quiet output and CRLF-warning cleanup:

- `git_status`: working tree status, optionally porcelain.
- `git_diff`: unstaged, staged, or ref diff.
- `git_log`: recent commits, oneline or full.
- `git_tree`: commit graph with optional `--all` and ref.
- `git_branch`: list, create, or switch branches.
- `git_commit`: stage specific files or all files and commit using a temporary message file.
- `git_stash`: push, pop, list, or drop stashes.
- `git_precommit_review`: summarize staged diff and return a bounded review body.

## CLI Commands

- `nushell-mcp`: start stdio MCP server.
- `nushell-mcp --version`: print server version.
- `nushell-mcp health --json`: print JSON health report without starting MCP mode.
- `nushell-mcp update --json`: print a machine-readable update report that identifies the current binary, recommended install path, GitHub release URL, and the PowerShell updater command.

## Install and Update

- `install.ps1` installs or updates `nushell-mcp.exe`.
- Default install/update downloads a GitHub Release asset from `stevenke1981/nushell-mcp`.
- `install.ps1 -FromSource` builds from the current checkout and installs the local release binary.
- Stable install path: `%USERPROFILE%\.config\nushell-mcp\bin\nushell-mcp.exe`.
- If the stable executable is locked, install side-by-side as `nushell-mcp-<version>.exe` and report the actual path.
- The updater must emit JSON when called with `-Json`.

## OpenCode Plugin Packaging

The repo ships an OpenCode Git tools plugin under `opencode-plugins/nushell-mcp-git-tools`.

Human-facing docs must explain install/verify/uninstall. Agent-facing docs must explicitly tell agents when to prefer `gitStatus`, `gitDiff`, `gitLog`, `gitTree`, `gitBranch`, `gitCommit`, `gitStash`, and `gitPrecommitReview` over raw shell git.

## Error Handling

- Spawn failure returns `ok=false`, `exit_code=null`, and `error` with the OS error.
- Timeout kills the child process, returns `ok=false`, `timed_out=true`, and preserves captured output.
- Non-zero Nushell exit code returns `ok=false` and preserves stdout/stderr.
- Invalid tool input returns an MCP invalid-params error before spawning Nushell.

## Security Boundary

This server intentionally executes arbitrary Nushell code. It is suitable only for trusted local clients and trusted workspaces. The implementation must avoid additional shell interpolation by invoking `nu` directly with argument vectors.

## Verification Requirements

- `cargo fmt --check`
- `cargo test --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --release`
- Unit tests for argument building, timeout clamping, output truncation, non-zero exits, spawn errors, and timeout.
- MCP smoke test that starts the release binary over stdio, initializes it, lists tools, and calls `nu_version` using a fake Nushell executable.
- MCP smoke test must assert the full tool set and verify OpenCode-compatible JSON schemas.
- `install.ps1 -FromSource -Json` emits valid JSON and an installed binary path.
- `nushell-mcp update --json` emits valid JSON.
