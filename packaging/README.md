# Packaging And Release Checklist

This workspace packages local stdio MCP servers for Codex, OpenCode, Claude, and
other MCP clients. The release path must make servers usable from stable binary
paths without requiring agents to compile from source.

## Release-First Installer Contract

- Default installers download GitHub Release assets.
- Source builds are opt-in only:
  - Windows: `.\install.ps1 -FromSource`
  - Linux/macOS: `./install.sh --from-source`
- Agent documentation and config examples must use the installer-reported
  `installed_exe` path, not `target/release`.
- Installer JSON output must validate against
  `packaging/install-report.schema.json`.
- Installers do not silently edit agent config files. They report paths and
  config targets so Codex/OpenCode/Claude config can use the actual executable.

## Installed Paths

Default user install location:

- Windows: `%USERPROFILE%\.config\mpc-servers\bin`
- Linux/macOS: `$HOME/.config/mpc-servers/bin`

Current binaries:

- `cbm`
- `everything-server`
- `filesystem-server`
- `fetch-server`
- `git-server`
- `memory-mcp-server`
- `nushell-mcp`
- `rlm-mcp`
- `time-server`
- `sequential-thinking-server`

## Windows Locked Binary Fallback

Windows agents may keep an installed `.exe` locked while a session is active.
The installer must handle that case without requiring users to kill the agent:

1. Try to copy to the stable binary path.
2. If the stable binary is locked, copy to a versioned side-by-side path.
3. Emit the actual `installed_exe` path in the JSON report.
4. Tell users and agents to configure the path from the report.

## Release Asset Names

Expected workspace release assets:

- `mpc-servers-windows-x86_64.zip`
- `mpc-servers-windows-aarch64.zip`
- `mpc-servers-linux-x86_64.tar.gz`
- `mpc-servers-linux-aarch64.tar.gz`
- `mpc-servers-macos-x86_64.tar.gz`
- `mpc-servers-macos-aarch64.tar.gz`

The GitHub Release workflow lives at `.github/workflows/release.yml` and
publishes these archives plus `SHA256SUMS.txt`.

The workflow uses GitHub-hosted x64 and arm64 runners for Windows, Linux, and
macOS instead of cross-compiling the full workspace.

## Pre-Tag Verification

Run these before tagging:

```powershell
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Check every implemented server:

```powershell
.\target\release\cbm.exe --version
.\target\release\everything-server.exe --version
.\target\release\filesystem-server.exe --version
.\target\release\fetch-server.exe --version
.\target\release\git-server.exe --version
.\target\release\memory-mcp-server.exe --version
.\target\release\nushell-mcp.exe --version
.\target\release\rlm-mcp.exe --version
.\target\release\time-server.exe --version
.\target\release\sequential-thinking-server.exe --version
```

Run SDK smoke tests against real binaries:

```powershell
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\cbm.exe -ExpectedToolCount 14 -ExpectedTools index_repository,index_status,search_graph,query_graph,list_projects
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\everything-server.exe -ExpectedToolCount 19 -ExpectedTools echo,get-sum,get-structured-content,get-tiny-image,gzip-file-as-resource,trigger-sampling-request
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\everything-server.exe -ExpectedToolCount 19 -ExpectedTools gzip-file-as-resource -CallToolName gzip-file-as-resource -CallToolArgsJson '{"name":"smoke.txt.gz","data":"data:text/plain;base64,aGVsbG8=","outputType":"resource"}'
.\scripts\everything-protocol-smoke.ps1 -Binary .\target\release\everything-server.exe
.\scripts\prompts-resources-smoke.ps1 -Binary .\target\release\everything-server.exe -ExpectedPromptCount 4 -ExpectedPrompts simple-prompt,args-prompt,completable-prompt,resource-prompt -PromptName resource-prompt -PromptArgsJson '{"resourceType":"Text","resourceId":"2"}' -ExpectedResourceCount 3 -ExpectedResources demo://resource/static/document/instructions.md,demo://resource/static/document/features.md,demo://resource/static/document/startup.md -ExpectedResourceTemplateCount 2 -ExpectedResourceTemplates 'demo://resource/dynamic/text/{resourceId}','demo://resource/dynamic/blob/{resourceId}' -ReadResourceUri demo://resource/dynamic/text/2
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\time-server.exe -ExpectedToolCount 2 -ExpectedTools get_current_time,convert_time
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\fetch-server.exe -ExpectedToolCount 1 -ExpectedTools fetch
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\git-server.exe -ExpectedToolCount 12 -ExpectedTools git_status,git_diff,git_commit,git_branch
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\memory-mcp-server.exe -ServerEnv @{ MEMORY_DB_PATH="$env:TEMP\mpc-memory.db"; MEMORY_VECTOR_PATH="$env:TEMP\mpc-memory.usearch"; MEMORY_TANTIVY_PATH="$env:TEMP\mpc-memory-tantivy"; LLM_API_KEY="mock"; LLM_API_BASE="mock" } -ExpectedToolCount 7 -ExpectedTools add_memory,search_memories,get_memories,delete_memory,consolidate_memories,get_memory_stats,end_session -CallToolName get_memory_stats
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\nushell-mcp.exe -ExpectedToolCount 15 -ExpectedTools nu_version,nu_eval,nu_script,git_status,git_precommit_review,nu_ls
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\rlm-mcp.exe -ExpectedToolCount 33 -ExpectedTools rlm_scan,rlm_peek,rlm_chunk,rlm_reduce_merge,rlm_repl_info
```

Run installer smoke:

```powershell
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("mpc-install-" + [guid]::NewGuid())
.\install.ps1 -FromSource -Server all -InstallDir $tmp -Json
.\uninstall.ps1 -Server all -InstallDir $tmp -Json
Remove-Item -LiteralPath $tmp -Recurse -Force
```

Validate the installer report schema:

```powershell
Test-Json `
  -Json (Get-Content -Raw .\packaging\install-report.example.json) `
  -Schema (Get-Content -Raw .\packaging\install-report.schema.json)
```

## Client Config Examples

Codex:

```toml
[mcp_servers.git]
command = "C:/Users/you/.config/mpc-servers/bin/git-server.exe"
args = ["--repository", "D:/workspace/repo"]
```

OpenCode:

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

## Release Checklist

1. Ensure all implemented servers support `--version`.
2. Run fmt, tests, clippy, and release build.
3. Run `scripts/tools-list-smoke.ps1` for each implemented server and
   `scripts/prompts-resources-smoke.ps1` for servers exposing prompts/resources.
4. Run source installer smoke and validate JSON reports.
5. Package release archives with all implemented binaries.
6. Publish checksums with release archives.
7. After GitHub Release assets exist, test default install without `-FromSource`.
8. Confirm README and package docs do not point agent configs at `target/release`.

For tag releases, `.github/workflows/release.yml` performs the archive and
checksum publishing steps after `scripts/release-check.ps1` passes.
