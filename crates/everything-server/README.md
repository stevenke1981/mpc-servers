# everything-server

Rust MCP compatibility testbed for the upstream `@modelcontextprotocol/server-everything` reference server.

This server is intended for client/protocol testing, not as a production automation server. It exposes the upstream tool inventory, prompts, resources, and resource templates over stdio.

Protocol coverage:

- Roots: `get-roots-list` calls client `roots/list` when the client advertises roots.
- Logging: `logging/setLevel` updates the current level and `toggle-simulated-logging`
  emits an MCP logging notification when enabled.
- Subscriptions: `resources/subscribe` / `resources/unsubscribe` are handled, and
  `toggle-subscriber-updates` emits update notifications for subscribed resources.
- Progress: `trigger-long-running-operation` emits progress notifications when the
  client supplies `_meta.progressToken`.
- Sampling: sampling tools call `sampling/createMessage` when the client advertises
  sampling; otherwise they return a deterministic fallback message.
- Elicitation: registered but explicitly deferred until this workspace enables the
  `rmcp` elicitation feature deliberately.

`scripts/everything-protocol-smoke.ps1` verifies the active protocol path with a
TypeScript SDK client that advertises roots and sampling, subscribes to a
resource, and asserts progress, logging, and resource update notifications.

`gzip-file-as-resource` supports `data:`, `http:`, and `https:` input. Fetching
is bounded by:

- `GZIP_MAX_FETCH_SIZE` (default `10485760`)
- `GZIP_MAX_FETCH_TIME_MILLIS` (default `30000`)
- `GZIP_ALLOWED_DOMAINS` (comma-separated allowlist; empty means all domains)

## Run

```powershell
cargo run -p everything-server -- --version
cargo run -p everything-server
```

## Verify

```powershell
cargo test -p everything-server
cargo clippy -p everything-server --all-targets -- -D warnings
cargo build -p everything-server
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -CallToolName echo -CallToolArgsJson '{"message":"hello"}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools gzip-file-as-resource -CallToolName gzip-file-as-resource -CallToolArgsJson '{"name":"smoke.txt.gz","data":"data:text/plain;base64,aGVsbG8=","outputType":"resource"}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools trigger-long-running-operation -CallToolName trigger-long-running-operation -CallToolArgsJson '{"duration":0,"steps":1}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools trigger-sampling-request -CallToolName trigger-sampling-request -CallToolArgsJson '{}'
.\scripts\everything-protocol-smoke.ps1 -Binary .\target\debug\everything-server.exe
.\scripts\prompts-resources-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedPromptCount 4 -ExpectedPrompts simple-prompt,args-prompt,completable-prompt,resource-prompt
```

## Parity

See `docs/parity/everything.md`.
