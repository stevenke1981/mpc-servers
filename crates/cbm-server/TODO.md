# TODO - cbm-mcp full clone of codebase-memory-mcp

Official Rust MCP SDK migration plan: [`RMCP_MIGRATION_TODO.md`](RMCP_MIGRATION_TODO.md).

Goal: make `D:\cbm-mcp` a complete, independent Rust clone of the reference `codebase-memory-mcp`.

## Status snapshot (2026-06-12)

| Area | State | Doc |
|------|-------|-----|
| MCP server name | `codebase-memory-mcp` | Done |
| Graph tools (14) | Implemented (MVP/heuristic) | [`PARITY_MATRIX.md`](PARITY_MATRIX.md) |
| RLM tools | **Out of scope** → [`rlm-mcp`](../rlm-mcp) | [`SEPARATION.md`](../cbm/SEPARATION.md) |
| Agent packaging | Templates + installer (`cbm-mcp` / `codebase-memory-mcp` naming) | `packaging/mcp/` |
| Reference parity | **Not complete** — SQLite MVP, partial CALLS/semantic | P0–P3 below |

**Execution order:** this file = backlog · `PARITY_MATRIX.md` = public truth table · `CLONE_ROADMAP.md` = milestone map.

**Next P0 slices:** MCP tool schema lock per tool · dependency-edge invalidation · read-only store mode.

**Done recently:** artifact export/restore parity (`artifact.json`, schema_version, integrity gate) · incremental index · MCP JSON-RPC error codes.

Module inventory: [`docs/MODULE_MAP.md`](docs/MODULE_MAP.md) · Spec checklist: [`docs/IMPLEMENTATION_CHECKLIST.md`](docs/IMPLEMENTATION_CHECKLIST.md).

Reference sources:

- Local reference repo: `D:\_cbm-ref`
- Local reference docs: `D:\cbm\knowledge-graph\`
- Public target repo: `https://github.com/stevenke1981/cbm-mcp.git`
- This repo must stay independent from `D:\rlm-mcp`.
- RLM tools are out of scope for this repo.

Definition of done:

- The MCP server name remains `codebase-memory-mcp`.
- The tool surface matches the reference CBM graph server, excluding RLM tools.
- All supported tools have schema-compatible arguments and response shapes.
- Indexing, graph model, search, trace, semantic edges, ADR, installer, packaging, CI, and docs are verified against fixtures.
- Any intentional deviation from the reference is documented in `PARITY_MATRIX.md`.

## P0 - Baseline reference audit

- [x] Inventory reference repo modules from `D:\_cbm-ref`: `src/`, `internal/`, `pkg/`, `graph-ui/`, `scripts/`, `vendored/`, `test-infrastructure/`. → [`docs/MODULE_MAP.md`](docs/MODULE_MAP.md)
- [x] Convert `D:\cbm\knowledge-graph\architecture.md`, `specifications.md`, `functions.md`, and `knowledge-graph.md` into a Rust implementation checklist. → [`docs/IMPLEMENTATION_CHECKLIST.md`](docs/IMPLEMENTATION_CHECKLIST.md)
- [x] Produce a one-to-one module map:
  - reference C/core module
  - Rust module path
  - status
  - missing behavior
  - fixture/test proving parity
- [x] Reconcile `CLONE_ROADMAP.md`, `PARITY_MATRIX.md`, and this `TODO.md`; this file is the execution backlog, `PARITY_MATRIX.md` is the public truth table.
- [x] Add a test that compares advertised MCP tools and schemas against checked-in specs under `mcps/codebase-memory-mcp/tools/` (`tests/mcp_tool_schema_test.rs`).

Acceptance criteria:

- A new agent can identify every reference feature and its Rust status without opening the whole repo.
- CI fails when public tool schemas drift unexpectedly.

## P0 - MCP protocol and tool contract parity

- [ ] Keep MCP server name as `codebase-memory-mcp`.
- [~] Verify JSON-RPC 2.0 framing:
  - [x] `initialize`
  - [x] `tools/list`
  - [x] `tools/call`
  - [x] error objects (`-32700` parse, `-32601` method not found, tool `isError`)
  - [ ] cancellation/shutdown behavior where supported
- [ ] Match the reference tool set for graph CBM:
  - `index_repository`
  - `index_status`
  - `search_graph`
  - `trace_path`
  - `get_code_snippet`
  - `get_graph_schema`
  - `get_architecture`
  - `query_graph`
  - `search_code`
  - `list_projects`
  - `delete_project`
  - `detect_changes`
  - `manage_adr`
  - `ingest_traces`
- [ ] For every tool, lock:
  - JSON schema
  - required/optional arguments
  - default values
  - pagination semantics
  - error messages/categories
  - response fields
- [ ] Decide whether ``manage_adr` mode=sections implemented (reference uses mode, not separate tool).
- [x] Add process-level MCP inspector smoke for `tools/list` and at least one `tools/call` (`tests/mcp_process_test.rs`).

Acceptance criteria:

- A generic MCP client can switch from reference CBM to Rust CBM without config or schema edits.
- `tools/list` output is snapshot-tested.

## P0 - Storage, schema, and artifact parity

- [ ] Match SQLite schema expectations:
  - `projects`
  - `files`
  - `symbols`
  - `edges`
  - `vectors`
  - `meta`
  - ADR storage
- [ ] Add migration/version metadata and compatibility checks.
- [x] Implement store bulk transaction API:
  - `begin_bulk_write` / `commit_bulk_write` / `rollback_bulk_write`
  - batch writes skip nested transactions during bulk mode
  - full index wrapped in single transaction (`pipeline::run_full`)
- [x] Add rollback tests for partial index failure (`tests/store_bulk_write_test.rs`).
- [~] Match `.codebase-memory/graph.db.zst` artifact behavior:
  - [x] export (zstd + `artifact.json` metadata)
  - [x] restore (decompress + integrity_check + atomic rename)
  - [x] skip restore when cache exists (`try_restore`)
  - [x] validate schema_version + original_size on import
  - [ ] `.gitattributes` merge driver setup (reference optional)
- [ ] Add read-only store mode for query operations.
- [ ] Add integrity check command or internal gate for release smoke.
- [ ] Confirm cache environment variables:
  - `CBM_CACHE_DIR`
  - legacy compatibility aliases, if retained

Acceptance criteria:

- Corrupt or partial indexes never replace a previous valid graph.
- Artifact export/import round-trips across machines.

## P0 - Index pipeline parity

- [ ] Match reference pipeline stages:
  - discover
  - definitions/symbol extract
  - structure nodes
  - imports
  - calls
  - inheritance/implements
  - decorators/attributes
  - routes/config
  - semantic vectors
  - semantic edges
  - LSP cross-file pass
  - git/history metadata
  - communities
  - post-processing summaries
- [x] Add a graph buffer staging layer equivalent to reference `graph_buffer` (`src/pipeline/graph_buffer.rs`; `finalize_graph_buffer` + `flush_to_store`).
- [ ] Add per-pass metrics:
  - files visited
  - files skipped
  - symbols emitted
  - edges emitted by type
  - duration
  - memory high-water mark
- [ ] Add pass-level failure isolation and diagnostics.
- [~] Add incremental indexing with changed-file invalidation by:
  - [x] git HEAD (`collect_incremental_paths` + watcher/`run_smart`)
  - [x] dirty file set (porcelain + HEAD diff merge)
  - [x] file hash (mtime_ns + size_bytes fingerprints in `files` table)
  - [ ] dependency edges where available
- [ ] Ensure `.gitignore` and `.cbmignore` behavior matches reference.

Acceptance criteria:

- Fixture repositories prove each pass emits expected nodes and edges.
- `get_architecture` and `get_graph_schema` reflect real emitted graph data.

## P0 - CALLS precision and LSP cross-file resolution

- [~] Implement hybrid CALLS resolution:
  - [x] Rust AST pass
  - [x] Python AST pass
  - [x] TypeScript/JavaScript AST pass
  - [x] Go AST pass
  - [x] Java AST pass
  - [~] C/C++ (C AST pass done; cpp shares query)
  - [x] C# (tree-sitter `invocation_expression`; requires `tree-sitter` ≥ 0.25 for grammar ABI 15)
  - [~] PHP (tree-sitter extract + `lsp_cross` + require/include + composer PSR-4; full `php_lsp.c` parity pending)
- [~] Add LSP-assisted cross-file resolution where the reference uses it (Python/JS/TS/Go/Java/PHP `lsp_cross.rs` MVP).
- [x] Add alias/import-aware call resolution (`SymbolRegistry` + `ImportMap`).
- [x] Add method vs free-function disambiguation (`CallTargetKind` + Method label in extract).
- [x] Add class/impl/trait/interface method resolution (`parent_class` + scoped `resolve_kind_scoped`).
- [x] Add negative fixtures (`tests/calls_negative_fixtures_test.rs` + existing ambiguous/nested tests):
  - [x] same symbol name in multiple files
  - [x] nested functions
  - [x] overloaded-like methods (class-scoped)
  - [x] imported aliases (Python/JS `as` + `symbol_aliases`)
  - [x] framework callback names (`console.log` noise)
- [x] Record confidence metadata in `properties_json` (`call_edge_properties_json`: callee, confidence, strategy, candidates, method, band).

Acceptance criteria:

- `CALLS` false positives are bounded and tested.
- Ambiguous calls do not fan out to every same-name symbol.

## P1 - Edge type parity

- [ ] Confirm and implement full reference edge vocabulary:
  - `CONTAINS`
  - `IMPORTS`
  - `CALLS`
  - `HTTP_CALLS`
  - `ASYNC_CALLS`
  - `INHERITS`
  - `IMPLEMENTS`
  - `DECORATES`
  - `TESTS`
  - `CONFIGURES`
  - `TYPE_REF`
  - `HTTP_ROUTE`
  - `SIMILAR_TO`
  - `SEMANTICALLY_RELATED`
  - `RUNTIME_TRACE`
- [ ] Implement `HTTP_CALLS`:
  - fetch/axios/request
  - reqwest/hyper
  - Python requests/httpx/aiohttp
  - Go net/http
  - Java HTTP clients
- [ ] Implement `ASYNC_CALLS` for common async/task systems.
- [ ] Implement type reference edges from annotations/signatures.
- [ ] Implement test-to-symbol edges.
- [ ] Implement config-to-code edges.

Acceptance criteria:

- `query_graph` can group all implemented edge types.
- `get_graph_schema` lists only live, emitted edge types.

## P1 - Search and query parity

- [ ] Match `search_graph` filters:
  - `query`
  - `semantic_query` / vector query if reference exposes it
  - `label`
  - `name_pattern`
  - `qn_pattern`
  - `file_pattern`
  - `relationship`
  - direction
  - degree filters
  - entry point filters
  - `include_connected`
  - `limit`
  - `offset`
  - `total`
  - `has_more`
- [ ] Add BM25 ranking with camelCase/snake_case tokenization.
- [ ] Normalize path and qualified-name matching across Windows/Linux.
- [ ] Decide and implement query language parity:
  - real Cypher subset, or
  - documented SQL-only deviation with tests.
- [ ] Add query guardrails:
  - read-only only
  - no PRAGMA mutation
  - no attach/detach
  - no multi-statement writes
- [ ] Add `trace_path` modes:
  - inbound
  - outbound
  - both
  - data-flow
  - cross-service
  - max depth
  - cycle handling

Acceptance criteria:

- Search ranking and filters match reference fixtures.
- Query behavior is safe and deterministic.

## P1 - Semantic system parity

- [ ] Recheck the 11 reference semantic signals:
  - TF-IDF
  - Random Indexing
  - MinHash structure
  - API signature vector
  - module proximity
  - Halstead complexity
  - type signature vector
  - decorator/attribute pattern vector
  - AST structural profile
  - approximate data flow
  - graph diffusion
- [ ] Tune weights and thresholds against reference examples.
- [ ] Store score breakdown in edge metadata.
- [ ] Add vector dimension and quantization compatibility tests.
- [ ] Add semantic search benchmarks for small, medium, and large repos.
- [ ] Ensure semantic pass is optional and does not degrade normal indexing latency unexpectedly.

Acceptance criteria:

- Semantic edges are explainable and reproducible.
- Reference similarity examples rank similarly in Rust.

## P1 - Community detection parity

- [ ] Replace connected-components MVP with Leiden/Louvain-grade community detection.
- [ ] Implement weighted graph construction for communities.
- [ ] Support resolution parameter.
- [ ] Persist `community_id` into symbol metadata or a dedicated table.
- [ ] Add deterministic seed mode for tests.
- [ ] Add graph-size performance tests.

Acceptance criteria:

- `get_architecture` reports meaningful communities.
- Community output is stable under deterministic test settings.

## P1 - Language coverage parity

- [ ] Inventory reference tree-sitter grammar coverage.
- [ ] Decide Rust crate strategy:
  - vendored grammars
  - generated bindings
  - feature-gated language packs
- [ ] Expand beyond current core languages toward the reference set.
- [ ] Add per-language smoke fixtures.
- [ ] Add fallback extractor for unsupported languages.
- [ ] Add fixture generator for simple definitions/imports/calls per language.

Acceptance criteria:

- Missing language support is visible in `PARITY_MATRIX.md`.
- Adding a grammar requires a fixture and tool smoke.

## P1 - Watcher, runtime, and shutdown parity

- [ ] Match watcher behavior:
  - refresh persisted projects
  - debounce/backoff
  - dirty signature
  - HEAD change detection
  - cancellation on shutdown
- [ ] Add graceful shutdown tests for:
  - MCP stdin close
  - Ctrl+C
  - HTTP server
  - background watcher
  - active pipeline cancellation
- [ ] Add runtime profile support equivalent to reference logs.
- [ ] Add memory budget enforcement and per-phase reporting.
- [ ] Confirm Windows-native behavior.

Acceptance criteria:

- No background threads survive process shutdown.
- Long indexing can be cancelled safely.

## P2 - HTTP graph UI parity

- [ ] Decide whether to port reference React/Three.js UI or maintain Rust lightweight UI.
- [ ] If full parity is required, port/reference:
  - search panel
  - node details
  - edge filters
  - project selector
  - schema/architecture views
  - error boundary
  - HTTP RPC client
- [ ] Add screenshot or Playwright smoke.
- [ ] Ensure UI is optional and does not affect MCP stdio by default.

Acceptance criteria:

- UI can inspect a real indexed graph.
- UI parity decision is documented.

## P2 - Agent installation and packaging parity

- [ ] Verify install/uninstall for:
  - OpenCode
  - Codex
  - Claude Code
  - Gemini CLI
  - Zed
  - Aider
  - generic `mcpServers`
- [ ] Preserve idempotent installs.
- [ ] Add config parsing for JSON, JSONC, TOML.
- [ ] Keep stable installed binary path separate from git clone build output.
- [ ] Add release archive smoke for:
  - checksum
  - extracted binary
  - CLI smoke
  - MCP initialize/tools/list
  - install dry-run
- [ ] Finish package manager wrappers:
  - Homebrew
  - Scoop
  - Winget
  - npm
  - PyPI
  - Go wrapper
  - Chocolatey
  - AUR

Acceptance criteria:

- A release artifact can be installed on a fresh Windows machine without pointing to `target/release`.
- Manual MCP templates match installer output.

## P2 - Reference wrapper parity

- [ ] Audit reference `pkg/go`.
- [ ] Audit reference `pkg/pypi`.
- [ ] Decide Rust repo ownership for wrappers:
  - same repo
  - generated packages
  - separate wrapper repos
- [ ] Add wrapper CI smoke.
- [ ] Add wrapper docs and version alignment.

Acceptance criteria:

- Downstream wrapper users can call the same MCP tools as reference users.

## P2 - Security and hardening parity

- [ ] Match URL/path safety rules from reference specs.
- [ ] Harden JSON-RPC request size and malformed input handling.
- [ ] Harden `query_graph` against mutation and injection.
- [ ] Add temp-file atomic writes for config/artifact updates.
- [ ] Add corrupted DB/artifact recovery behavior.
- [ ] Add Windows path canonicalization tests.
- [ ] Add dependency/license audit.

Acceptance criteria:

- Malformed configs, DBs, and MCP messages fail safely.
- Security-sensitive behavior is covered by tests.

## P3 - Performance benchmarks

- [ ] Add benchmark corpus sizes:
  - tiny fixture
  - medium repo
  - large monorepo
- [ ] Measure:
  - cold full index
  - incremental index
  - search latency
  - trace latency
  - query latency
  - memory peak
  - artifact export/import
- [ ] Compare against reference where runnable.
- [ ] Add regression thresholds for CI or nightly jobs.

Acceptance criteria:

- Performance claims in README are backed by repeatable measurements.

## Verification checklist before claiming full clone

- [ ] `cargo fmt --check`
- [ ] `cargo test --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo build --release`
- [ ] `.\scripts\smoke-quality-gates.ps1 -SkipBuild`
- [ ] `.\scripts\smoke-release-artifact.ps1 -SkipBuild`
- [x] Tool schema snapshot check (`cargo test mcp_tool_schema`)
- [ ] Reference fixture parity suite
- [ ] Large repo indexing benchmark
- [ ] Fresh-machine installer smoke
- [ ] `PARITY_MATRIX.md` has no unstated gaps

## Non-goals

- Do not add RLM tools here; use `D:\rlm-mcp`.
- Do not re-couple this repo to the legacy `D:\cbm\cbrlm` combined binary.
- Do not claim FoundationDB parity unless the project explicitly reverses the SQLite-only decision.
