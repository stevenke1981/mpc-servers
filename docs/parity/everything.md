# Everything Server Parity

Source: `.opencode/upstream/servers/src/everything`

Upstream package: `@modelcontextprotocol/server-everything@2.0.0`

Rust crate: `crates/everything-server`

Status: usable compatibility testbed. Core tools, prompts, resources, gzip, and the
rmcp-supported roots/logging/subscription/progress/sampling paths are implemented and
covered by both deterministic fallback smoke tests and an active TypeScript SDK
integration client. Elicitation remains explicitly deferred.

## Implemented

| Feature | Rust status | Evidence |
|---|---|---|
| stdio transport | implemented | `cargo run -p everything-server -- --version`; SDK smoke over `target/debug/everything-server.exe` |
| `tools/list` | implemented | 19 tools returned by `scripts/tools-list-smoke.ps1` |
| `echo` | implemented | SDK `tools/call` smoke |
| `get-sum` | implemented | SDK `tools/call` smoke |
| `get-structured-content` | implemented | SDK `tools/call` smoke preserves upstream weather values and structured content |
| `get-tiny-image` | implemented | returns text/image/text content |
| `get-env` | implemented | returns process environment as JSON text |
| `get-resource-links` | implemented | returns text plus dynamic text/blob resource links |
| `get-resource-reference` | implemented | embeds dynamic text/blob resources |
| `gzip-file-as-resource` | implemented | compresses `data:`, `http:`, and `https:` input into `application/gzip`; bounded by `GZIP_MAX_FETCH_SIZE`, `GZIP_MAX_FETCH_TIME_MILLIS`, and optional `GZIP_ALLOWED_DOMAINS`; unit tests cover gzip round-trip and HTTP fetch; SDK smoke calls the tool |
| prompts | implemented | 4 prompt names registered; unit tests cover all prompt get paths; SDK prompts/resources smoke validates `prompts/list` and `prompts/get` |
| resources | implemented | static docs, dynamic text/blob templates, session resources; unit tests cover text/blob/static/unknown reads; SDK prompts/resources smoke validates `resources/read` |
| resource templates | implemented | `demo://resource/dynamic/text/{resourceId}` and `demo://resource/dynamic/blob/{resourceId}`; SDK prompts/resources smoke validates `resources/templates/list` |
| logging capability | implemented | capability advertised; `logging/setLevel` updates server state; `toggle-simulated-logging` sends `notifications/message` when enabled; active SDK protocol smoke observes a debug log notification |
| subscriptions | implemented | `resources/subscribe` and `resources/unsubscribe` are handled; subscribed resources receive `notifications/resources/updated` when `toggle-subscriber-updates` is enabled; active SDK protocol smoke subscribes to `demo://resource/dynamic/text/2` and observes the update notification |
| roots | implemented | `get-roots-list` calls client `roots/list` when the client advertises roots and caches roots after `notifications/roots/list_changed`; active SDK protocol smoke advertises roots and observes a `roots/list` request; fallback smoke covers clients without roots capability |
| sampling tools | implemented | `trigger-sampling-request`, `trigger-sampling-request-async`, and `simulate-research-query` call client `sampling/createMessage` when sampling is advertised; active SDK protocol smoke advertises sampling and observes `sampling/createMessage`; fallback smoke covers clients without sampling capability |
| elicitation tools | deferred | tools are registered and return explicit deferred compatibility text because the workspace does not enable `rmcp` elicitation feature for all servers yet |
| long-running operation | implemented | bounded delay and completion text implemented; sends `notifications/progress` when the tool call includes `_meta.progressToken`; active SDK protocol smoke observes two progress notifications for a token; fallback smoke covers calls without a progress token |

## Tool Inventory

- `echo`
- `get-annotated-message`
- `get-env`
- `get-resource-links`
- `get-resource-reference`
- `get-roots-list`
- `get-structured-content`
- `get-sum`
- `get-tiny-image`
- `gzip-file-as-resource`
- `toggle-simulated-logging`
- `toggle-subscriber-updates`
- `trigger-elicitation-request`
- `trigger-elicitation-request-async`
- `trigger-long-running-operation`
- `trigger-sampling-request`
- `trigger-sampling-request-async`
- `trigger-url-elicitation`
- `simulate-research-query`

## Verification

```powershell
cargo check -p everything-server
cargo test -p everything-server
cargo clippy -p everything-server --all-targets -- -D warnings
cargo build -p everything-server
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools echo,get-sum,get-structured-content,get-tiny-image,gzip-file-as-resource,trigger-sampling-request -CallToolName echo -CallToolArgsJson '{"message":"hello"}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -CallToolName get-sum -CallToolArgsJson '{"a":2,"b":3}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -CallToolName get-structured-content -CallToolArgsJson '{"location":"Chicago"}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools gzip-file-as-resource -CallToolName gzip-file-as-resource -CallToolArgsJson '{"name":"smoke.txt.gz","data":"data:text/plain;base64,aGVsbG8=","outputType":"resource"}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools get-roots-list -CallToolName get-roots-list -CallToolArgsJson '{}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools trigger-long-running-operation -CallToolName trigger-long-running-operation -CallToolArgsJson '{"duration":0,"steps":1}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools toggle-simulated-logging -CallToolName toggle-simulated-logging -CallToolArgsJson '{}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools toggle-subscriber-updates -CallToolName toggle-subscriber-updates -CallToolArgsJson '{}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools trigger-sampling-request -CallToolName trigger-sampling-request -CallToolArgsJson '{}'
.\scripts\tools-list-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedToolCount 19 -ExpectedTools trigger-elicitation-request -CallToolName trigger-elicitation-request -CallToolArgsJson '{}'
.\scripts\everything-protocol-smoke.ps1 -Binary .\target\debug\everything-server.exe
.\scripts\prompts-resources-smoke.ps1 -Binary .\target\debug\everything-server.exe -ExpectedPromptCount 4 -ExpectedPrompts simple-prompt,args-prompt,completable-prompt,resource-prompt -PromptName resource-prompt -PromptArgsJson '{"resourceType":"Text","resourceId":"2"}' -ExpectedResourceCount 3 -ExpectedResources demo://resource/static/document/instructions.md,demo://resource/static/document/features.md,demo://resource/static/document/startup.md -ExpectedResourceTemplateCount 2 -ExpectedResourceTemplates 'demo://resource/dynamic/text/{resourceId}','demo://resource/dynamic/blob/{resourceId}' -ReadResourceUri demo://resource/dynamic/text/2
.\scripts\release-check.ps1 -Server all -SkipBuild
```

## Remaining Work

- Decide whether to enable `rmcp` `elicitation` feature workspace-wide, or isolate elicitation behind a dedicated feature so `trigger-elicitation-*` can call `elicitation/create`.
