# Existing Rust Server Import Strategy

This document decides how existing Rust MCP projects are brought into the
`mpc-servers` workspace. It is intentionally implementation-oriented so another
agent can continue without reading prior chat history.

## Decision Summary

Use source vendor imports into this workspace, one server at a time, with a thin
workspace integration layer only where naming or packaging must change.

Do not use submodules for the current phase. Do not wrap external release
binaries as the primary implementation. Do not start with a monolithic workspace
merge of all four repos.

Import order:

1. `D:\cbm-mcp` -> `crates/cbm-server`
2. `D:\rlm-mcp` -> `crates/rlm-server`
3. `D:\nushell-mcp` -> `crates/nushell-server`
4. `D:\memlong` -> `crates/memory-server` plus any required internal support crates

## Why Source Vendor Import

Source vendor import means copying the source-controlled implementation into
this repository under `crates/`, excluding `.git`, `.codebase-memory`, `.opencode`,
`target`, local terminals, logs, and other generated artifacts.

Benefits:

- Keeps all shipped code under this repository's git history.
- Allows root installers to package one consistent release asset set.
- Lets `scripts/release-check.ps1` build and smoke-test the real server binary.
- Preserves existing Rust MCP tool implementations instead of rewriting stable
  behavior from scratch.
- Avoids requiring users or agents to have the sibling source repos present.

Costs:

- Upstream fixes must be manually ported or merged.
- Dependency versions may temporarily diverge between imported crates.
- Large crates such as `cbm` and `memlong` increase workspace build time.

## Rejected Options

### Git Submodule

Rejected for now because installer and release workflows should work from a
single checked-out repository without requiring submodule initialization. It also
makes agent handoff more fragile on Windows.

### Git Subtree

Acceptable later, but not necessary for this phase. A subtree is useful if we
need repeatable upstream sync history, but it adds operational overhead before
the first import is proven.

### Thin Wrapper Around External Binary

Rejected as the primary strategy because the goal is for this repository to
build and release usable MCP servers. A wrapper can be used only as a temporary
compatibility bridge when an imported crate cannot yet compile in this workspace.

### Rewrite From Scratch

Rejected for the existing Rust servers. `cbm-mcp`, `rlm-mcp`, `nushell-mcp`, and
`memlong` already contain working tool behavior that must be preserved.

## Per-Repo Strategy

### `cbm-mcp`

Chosen strategy: source vendor import into `crates/cbm-server`.

Rationale:

- Current source is already an rmcp stdio MCP server.
- Current version is `0.2.3`.
- It already contains `tools/list` schema normalization in `src/mcp/server.rs`.
- It already has tests around MCP contract and installer/release behavior.
- It is the best first import because the existing packaging checklist was
  derived from this project.

Required integration:

- Preserve all 14 public tool names.
- Keep `--version`, `-V`, and `version` behavior before stdio starts.
- Rename the workspace binary to `cbm-server` unless preserving `cbm` is
  explicitly required by compatibility tests.
- Prefer keeping the library name stable internally until tests pass.
- Add `cbm` to root installers, uninstallers, README, spec, `todos.md`, and
  `scripts/release-check.ps1`.
- Add an MCP SDK smoke expecting 14 tools.

Known risks:

- Source repo uses `schemars = 1`; current workspace dependencies include
  `schemars = 0.8` for earlier ports. Allow crate-local dependency versions
  first; only align workspace-wide after tests prove it is safe.
- The source repo has its own install/config logic. Root installers remain the
  workspace authority for `mpc-servers`; imported upstream install commands can
  remain available but must not contradict root README examples.

### `rlm-mcp`

Chosen strategy: source vendor import into `crates/rlm-server`.

Rationale:

- Current source is an rmcp stdio MCP server.
- Current version is `0.1.6`.
- It already has MCP contract tests and schema normalization.
- Tool behavior is broad and should be preserved directly.

Required integration:

- Preserve all `rlm_*` public tool names from `spec.md`.
- Keep session persistence, chunk/slice/peek behavior, map/reduce, budget, and
  trajectory tools.
- Keep `rlm_repl_execute` opt-in and safe; do not enable command execution by
  default.
- Extend release-check only after a minimal scan/chunk/slice smoke fixture exists.

Known risks:

- Broad tool surface means a narrow `tools/list` check is not enough.
- Provider-backed task execution can be slow or environment-dependent, so tests
  must keep mock/dry-run paths for CI.

### `nushell-mcp`

Chosen strategy: source vendor import into `crates/nushell-server`.

Rationale:

- Current source is a small Rust rmcp server and should be easy to integrate.
- It has Windows-conscious Nu and Git command handling.
- Current version is `0.1.0`.

Required integration:

- Preserve all Nu and Git tool names from `spec.md`.
- Keep cwd and timeout bounds.
- Keep native process argument passing.
- Keep Windows-readable Git output behavior.

Known risks:

- It uses Rust edition 2024 while the workspace package defaults to 2021. Keep
  crate-local edition first.
- The host machine must have `nu` available for full runtime tests; unit tests
  should keep fake Nu fixtures where possible.

### `memlong`

Chosen strategy: staged source vendor import, completed for the Rust MCP path.

Imported targets:

- `crates/memory-server` for the MCP server binary.
- `crates/memory-core` for storage, extraction, retrieval, and consolidation.
- `crates/memory-cli` for local debug and maintenance workflows.

Rationale:

- `memlong` is the Rust memory line and preserves the user's existing memory
  workflow better than reimplementing the TypeScript reference server first.
- It is a workspace with core, MCP server, and CLI crates, so it should be
  imported after smaller single-crate servers.

Required integration:

- Preserve `add_memory`, `search_memories`, `get_memories`, `delete_memory`,
  `consolidate_memories`, `get_memory_stats`, and `end_session`.
- Document whether TypeScript reference graph tools are implemented as a bridge
  or replaced by memlong semantics.
- Preserve persistence and semantic/BM25/temporal search weighting.

Current integration notes:

- Public MCP server package and binary remain `memory-mcp-server`.
- The MCP server was upgraded from source `rmcp = 0.1` to the workspace
  `rmcp = 1.x` handler style so the TypeScript MCP SDK can validate
  `tools/list` and `tools/call`.
- Root installers, uninstallers, and `scripts/release-check.ps1` now accept
  `memory`.
- TypeScript reference graph tools are not claimed as implemented; README
  documents the current memlong replacement semantics and leaves graph-tool
  bridge work for a later compatibility pass.

Known risks:

- Source repo uses older `rmcp = 0.1` and `schemars = 0.8`, unlike the other
  imported servers.
- It has multiple internal crates and heavier storage dependencies.
- Compatibility with reference memory graph tools needs an explicit design
  decision before claiming parity.

## Common Import Checklist

For each imported server:

1. Copy only source-controlled files needed to build and test the crate.
2. Exclude `.git`, `.codebase-memory`, `.opencode`, `target`, logs, terminal
   transcripts, and generated indexes.
3. Add the crate to root `Cargo.toml`.
4. Ensure the binary supports `--version`, `-V`, and `version`.
5. Ensure MCP mode keeps stdout protocol-only.
6. Run package-level tests and clippy first.
7. Add the server to root install/uninstall scripts.
8. Add the server to `scripts/release-check.ps1`.
9. Run `scripts/tools-list-smoke.ps1` against the real binary.
10. Update README, `spec.md`, `plan.md`, and `todos.md`.

## Next Implementation Target

Start `everything` after the imported Rust servers are green.

Concrete next task:

1. Inspect `.opencode/upstream/servers/src/everything`.
2. Create `crates/everything-server`.
3. Implement a protocol compatibility testbed for tools, prompts, resources,
   resource templates, subscriptions, roots, logging, sampling, and elicitation.
4. Add parity tables and CI-compatible smoke tests before including it in
   release-check.
