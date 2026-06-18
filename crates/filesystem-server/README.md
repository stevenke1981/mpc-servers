# filesystem-server

Rust stdio MCP server port of `@modelcontextprotocol/server-filesystem`.

## Tools

- `read_file` deprecated alias for `read_text_file`
- `read_text_file`
- `read_media_file`
- `read_multiple_files`
- `write_file`
- `edit_file`
- `create_directory`
- `list_directory`
- `list_directory_with_sizes`
- `directory_tree`
- `move_file`
- `search_files`
- `get_file_info`
- `list_allowed_directories`

All path-taking tools validate paths against configured allowed directories before touching the filesystem. Validation canonicalizes existing paths, resolves the deepest existing ancestor for new candidate paths, rejects null bytes, prevents traversal and symlink escape, and compares path components case-insensitively on Windows.

## Usage

```powershell
cargo run -p filesystem-server -- D:\some\allowed\directory
```

Version probes exit before MCP stdio starts:

```powershell
cargo run -p filesystem-server -- --version
cargo run -p filesystem-server -- -V
cargo run -p filesystem-server -- version
```

## Client Config

OpenCode local config shape:

```json
{
  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["D:\\mpc-servers\\target\\debug\\filesystem-server.exe", "D:\\workspace"],
      "enabled": true,
      "timeout": 120000,
      "environment": {}
    }
  }
}
```

Codex config shape:

```toml
[mcp_servers.filesystem]
command = "D:/mpc-servers/target/debug/filesystem-server.exe"
args = ["D:/workspace"]
```

## Status

Command-line allowed directories are implemented. MCP Roots dynamic updates are not implemented yet; until that is done, start the server with explicit allowed directory arguments.

## Verification

```powershell
cargo fmt --check
cargo test -p filesystem-server --all-targets
cargo clippy --all-targets -- -D warnings
```
