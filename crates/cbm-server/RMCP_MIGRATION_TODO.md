# CBM rmcp Migration and Hardening TODO

> Agent handoff document. Update checkboxes only after the listed acceptance test passes.

**Repository:** `stevenke1981/cbm-mcp`

**Binary:** `cbm` / `cbm.exe`

**MCP config key:** `cbm`

**MCP `serverInfo.name`:** `codebase-memory-mcp`

**Public tools:** 14
**SDK baseline:** `rmcp 1.7.0`

## Current state

The custom JSON-RPC/stdio implementation has been removed. CBM now uses the
official `rmcp` server, stdio transport, typed Schemars parameters, and an
official-client protocol test. The migration itself is complete; the remaining
work is cancellation depth, release hardening, and optional capabilities.

Release verification snapshot: `v0.2.3` was published from commit `7986d73`
with five platform archives, `SHA256SUMS.txt`, successful default/fallback
Windows installs, 14-tool OpenCode SDK validation, and `cbm connected`.

### Verified complete

- [x] Official `rmcp` stdio server and `ServerHandler`.
- [x] Tokio async executable entrypoint.
- [x] Typed `#[tool_router]` methods for all 14 tools.
- [x] `Deserialize + JsonSchema` inputs with unknown-field rejection.
- [x] SDK-owned initialize negotiation and tools capability.
- [x] Blocking graph work runs through `spawn_blocking`.
- [x] Domain failures return MCP tool errors instead of terminating the server.
- [x] Custom transport, framing, and manual JSON-RPC dispatcher removed.
- [x] In-process official rmcp client initialize/list/call test.
- [x] Compiled-process stdio initialize/list/call test.
- [x] Release archive smoke uses the extracted binary.
- [x] Stdout remains protocol-only in MCP mode.
- [x] OpenCode-sensitive boolean JSON Schema nodes are normalized in the actual
  `ServerHandler::list_tools()` and `get_tool()` responses.
- [x] Schema regression test rejects boolean `properties.*` and `items` nodes.
- [x] Release installers use checksums and do not compile by default.
- [x] Release installers support `GITHUB_TOKEN`/`GH_TOKEN` and fall back from
  GitHub API lookup to the public `/releases/latest` redirect.
- [x] MCP manifest points to release installers and stable installed paths, not
  `target/release`.
- [x] Windows installation can report the actual installed binary as JSON.
- [x] Windows locked executable fallback can configure agents to a side-by-side
  versioned `cbm-<version>.exe` path.

## P0 - Finish request cancellation correctly

This is the only remaining protocol-level migration blocker. Do not mark it
complete merely because closing the client stops the service.

- [ ] Add `tokio-util = "0.7"` and accept `CancellationToken` in all 14 router
  methods.
- [ ] Race each `spawn_blocking` join handle against `cancellation.cancelled()`
  with `tokio::select!`.
- [ ] Return a stable cancellation error and never emit a late success response.
- [ ] Add a protocol test that starts a deliberately slow tool request, sends
  `notifications/cancelled`, and proves bounded response/shutdown time.
- [ ] Keep the server usable after cancellation by calling a second lightweight
  tool in the same session.

Acceptance:

- Cancellation is observably different from an internal failure.
- The client receives no success result for the cancelled request.
- A later `list_projects` or `index_status` request succeeds.

## P0 - Add cooperative pipeline cancellation

Request-level cancellation does not stop a blocking worker by itself. Thread a
cooperative signal through graph work before claiming safe index cancellation.

- [ ] Introduce a small domain cancellation abstraction that is independent of
  rmcp types.
- [ ] Check cancellation between discovery, extraction, edge passes, semantic
  passes, persistence, and trace batches.
- [ ] Stop scheduling new Rayon or worker tasks after cancellation.
- [ ] Preserve the previous valid database when a full index is cancelled.
- [ ] Roll back the active bulk transaction on cancellation.
- [ ] Release watcher/pipeline busy guards on every cancellation path.
- [ ] Add a slow fixture that cancels full indexing and then reopens and queries
  the prior graph.
- [ ] Add equivalent coverage for incremental indexing and `ingest_traces`.

Acceptance:

- Cancelled indexing leaves no corrupt or half-published graph.
- Locks and watcher state are released.
- A subsequent index can start immediately.

## P0 - Publish and test the first post-migration release

The latest published tag may lag behind `main`. A source checkout using default
`./install.ps1` installs the latest GitHub Release, not untagged code.

- [x] Bump `Cargo.toml`, `Cargo.lock`, `packaging/mcp/manifest.json`, installer
  examples, and release metadata to the next patch version.
- [x] Run all required verification commands below.
- [x] Commit and push `main`.
- [x] Tag the exact tested commit and push the tag.
- [x] Wait for all GitHub Release archives and `SHA256SUMS.txt`.
- [x] On Windows, run default `./install.ps1` with no version and confirm the
  downloaded URL uses the new tag.
- [x] Force the GitHub API lookup to fail and confirm redirect fallback still
  installs the same release.
- [x] Keep OpenCode running during one update and confirm locked-binary fallback
  configures the actual side-by-side executable.
- [x] Verify the default installer log contains no Cargo compilation output.
- [x] Run the OpenCode SDK smoke against the installed binary.
- [x] Run `opencode --pure mcp list` and confirm `cbm connected`.
- [ ] Start a fresh Codex session and confirm CBM tools are registered.

Acceptance:

- A machine with no Rust toolchain can install and connect.
- The release binary exposes exactly 14 tools.
- OpenCode and Codex configs point to an existing installed executable.

## P1 - Strengthen public tool contracts

- [ ] Replace stringly typed closed fields with enums where compatibility allows:
  index mode, trace direction, search mode, and ADR mode.
- [ ] Lock defaults and enums in checked-in schema fixtures, not only property
  names and required fields.
- [ ] Add minimum/full/invalid input tests for each parameter family.
- [ ] Add response fixtures for one index tool, one read tool, one pagination
  tool, and one domain error.
- [ ] Add tests for unknown method, unknown tool, malformed JSON, invalid params,
  domain error, and post-error server reuse.
- [ ] Keep tool descriptions action-oriented and explicit about required index
  state, output limits, and read/write effects.

Acceptance:

- Tool name, description, required fields, defaults, enums, and representative
  response envelopes cannot drift silently.

## P1 - Service lifetime and watcher hardening

- [ ] Add a process test proving stdin EOF stops the watcher and exits promptly.
- [ ] Add a Ctrl+C process test on Windows and Unix CI.
- [ ] Debounce file events and serialize indexing per project.
- [ ] Test stdin close during watcher activity.
- [ ] Test cancellation during watcher-triggered incremental indexing.
- [ ] Confirm no watcher thread survives process exit.

Acceptance:

- Auto-sync cannot outlive the MCP process.
- Repeated events create bounded indexing work.

## P2 - Optional MCP resources

Resources remain intentionally disabled until fully implemented and tested.

- [ ] Write an ADR before enabling the resources capability.
- [ ] Start with read-only graph schema, architecture, and project-status URIs.
- [ ] Define URI templates, MIME types, size limits, project boundaries, and
  stale-index behavior.
- [ ] Add list/read/template tests before advertising resources.
- [ ] Never expose the SQLite file or unrestricted local filesystem paths.

Acceptance:

- Capability negotiation stays honest.
- Resource reads enforce project boundaries and output limits.

## P2 - Progress notifications

- [ ] Define stable indexing phases and progress units.
- [ ] Emit progress only when the request includes a progress token.
- [ ] Rate-limit notifications to avoid flooding stdio clients.
- [ ] Stop notifications immediately after cancellation or completion.
- [ ] Add official-client tests for ordering and terminal state.

## Documentation follow-up

- [x] Document side-by-side Windows updates and restart requirements.
- [x] Document GitHub API fallback and optional token use.
- [x] Add troubleshooting for `failed to get tools`, stdout pollution, stale
  command paths, invalid JSONC/TOML, permission failures, and locked binaries.
- [x] Update `PARITY_MATRIX.md` so protocol migration, cancellation, and graph
  feature parity are separate claims.
- [ ] Keep legacy `cbrlm` names only where migration compatibility requires them.

## Required verification

Run from the repository root:

```powershell
cargo fmt --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo tree -i rmcp
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-quality-gates.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-release-artifact.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File C:\Users\steven\.codex\skills\rust-rmcp-mcp-server\scripts\opencode_tools_list_smoke.ps1 -Binary .\target\release\cbm.exe
opencode --pure mcp list
```

Also verify:

- [x] `cargo tree -i rmcp` resolves exactly one intended rmcp version.
- [x] `cbm --version` matches the release tag.
- [x] `cbm install --dry-run --all --json` emits valid JSON on stdout.
- [x] Default installers contain no implicit source-build fallback.
- [x] `git status --short` contains no generated cache, config, or test database.

## Completion definition

The rmcp migration is complete when official SDK transport/router behavior,
OpenCode-compatible schemas, release-installed binaries, request cancellation,
cooperative index cancellation, and service shutdown all pass their acceptance
tests. Graph feature parity with the upstream project remains a separate track
owned by `PARITY_MATRIX.md` and must not be inferred from MCP protocol completion.
