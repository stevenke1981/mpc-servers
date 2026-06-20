# git-server

Rust stdio MCP server for Git repository operations. This is a Rust port of
`mcp-server-git@0.6.2` from `stevenke1981/servers/src/git`.

## Tools

- `git_status`
- `git_diff_unstaged`
- `git_diff_staged`
- `git_diff`
- `git_commit`
- `git_add`
- `git_reset`
- `git_log`
- `git_create_branch`
- `git_checkout`
- `git_show`
- `git_branch`

## Security Model

- Every tool requires `repo_path` and validates that it resolves to a Git repository.
- If the server is started with `--repository <path>` or `-r <path>`, tool calls are
  limited to that repository path or its subdirectories.
- Git is invoked with native process arguments through `std::process::Command`.
  User input is never interpolated into shell command strings.
- Revision-like values that start with `-` are rejected before Git runs.
- `git_add` validates paths so relative traversal or absolute paths outside the
  repository cannot be staged.

## Run

```powershell
cargo run -p git-server -- --version
cargo run -p git-server -- --repository D:\workspace\repo
```

Without `--repository`, the server accepts any local Git repository path supplied
by the MCP tool call.

## Client Config

Use the installed binary path reported by `install.ps1 -Json` or `install.sh --json`.
Do not point agents at `target/release`.

```toml
[mcp_servers.git]
command = "C:/Users/you/.config/mpc-servers/bin/git-server.exe"
args = ["--repository", "D:/workspace/repo"]
```

```json
{
  "mcp": {
    "git": {
      "type": "local",
      "command": [
        "C:\\Users\\you\\.config\\mpc-servers\\bin\\git-server.exe",
        "--repository",
        "D:\\workspace\\repo"
      ],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    }
  }
}
```

## Verify

```powershell
cargo fmt --check
cargo test -p git-server
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -p git-server -- --version
```

