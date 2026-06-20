# <server-name>

Rust stdio MCP server for `<upstream-or-purpose>`.

## Status

- Source: `<upstream repo/package and version>`
- Rust crate: `<crate name>`
- Binary: `<binary name>`
- Version: `<version>`
- Transport: stdio
- Install name: `<install.ps1 -Server value>`
- Parity: see `<docs/parity/server.md>`

## Tools

| Tool | Status | Notes |
|---|---|---|
| `<tool_name>` | implemented | `<short behavior or known difference>` |

## Prompts And Resources

Use this section only when the server exposes prompts, resources, or resource
templates. Remove it for tool-only servers.

| Feature | Status | Notes |
|---|---|---|
| prompts | not applicable |  |
| resources | not applicable |  |
| resource templates | not applicable |  |

## Safety Notes

- Filesystem access: `<allowed roots, path validation, or not applicable>`
- Network access: `<timeouts, redirects, SSRF policy, or not applicable>`
- Process execution: `<native args, shell policy, timeout, or not applicable>`
- Destructive behavior: `<tool annotations and confirmation model>`
- Data storage: `<database/index paths and isolation rules>`

## Install

Default install uses GitHub Release assets. Source builds are for local
development only.

```powershell
.\install.ps1 -Server <server-name>
.\install.ps1 -FromSource -Server <server-name> -Json
.\uninstall.ps1 -Server <server-name> -Json
```

```bash
./install.sh --server <server-name>
./install.sh --from-source --server <server-name> --json
./uninstall.sh --server <server-name> --json
```

Use the `installed_exe` path from the JSON report in agent configs. Do not point
agents at `target/release`.

## Codex Config

```toml
[mcp_servers.<server-name>]
command = "C:/Users/you/.config/mpc-servers/bin/<binary>.exe"
args = []

[mcp_servers.<server-name>.env]
# Add required environment variables here.
```

## OpenCode Config

```json
{
  "mcp": {
    "<server-name>": {
      "type": "local",
      "command": [
        "C:\\Users\\you\\.config\\mpc-servers\\bin\\<binary>.exe"
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
cargo fmt --check -p <crate-name>
cargo test -p <crate-name>
cargo clippy -p <crate-name> --all-targets -- -D warnings
cargo build --release -p <crate-name>
.\target\release\<binary>.exe --version
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\<binary>.exe -ExpectedToolCount <n> -ExpectedTools <tool1>,<tool2>
.\scripts\release-check.ps1 -Server <server-name> -SkipBuild
```

Add specialized smoke commands here when the server exposes prompts, resources,
protocol callbacks, filesystem roots, network fetch, or process execution.

## Known Differences

- `<difference from upstream or "None known">`

## Release Notes

- Release asset: `<workspace archive containing binary>`
- Checksums: `SHA256SUMS.txt`
- Version tag: `<vX.Y.Z or server-vX.Y.Z>`
