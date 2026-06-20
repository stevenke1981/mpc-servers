# Rust implementation checklist

> Derived from `D:\cbm\knowledge-graph\{architecture,specifications,functions,knowledge-graph}.md`
>
> Module mapping: [`MODULE_MAP.md`](MODULE_MAP.md) · Truth table: [`PARITY_MATRIX.md`](../PARITY_MATRIX.md)

Legend: `[x]` done · `[~]` partial/MVP · `[ ]` not started · `[-]` omitted/N/A

---

## 1. Platform & entry (`architecture.md` §2, `specifications.md` §2)

- [x] MCP stdio server (JSON-RPC 2.0, Content-Length)
- [x] CLI mode: `cli --json [--quiet] <tool> [args]`
- [~] Subcommands: `install`, `uninstall`, `update` (legacy naming cleanup pending)
- [ ] Subcommand: `config list|get|set|reset`
- [x] `hook-augment` for agent hooks
- [~] Flags: `--ui`, `--port` (default 9749), `--profile`
- [~] Graceful shutdown: SIGINT/Ctrl+C, watcher + HTTP stop
- [ ] Parent process watchdog (POSIX reference only)
- [x] Project naming from path hash slug (`cbm+` style)
- [x] Cache dir env (`CBM_CACHE_DIR` / compatibility aliases)

## 2. MCP protocol (`specifications.md` §3, `functions.md` §1.2)

- [x] `initialize` response
- [x] `tools/list` from checked-in JSON schemas
- [x] `tools/call` dispatch for 14 graph tools
- [~] JSON-RPC error object shapes vs reference
- [ ] Cancellation / shutdown mid-request behavior
- [x] Process-level MCP inspector smoke (`tests/mcp_process_test.rs`)
- [x] Schema drift CI: `tests/mcp_tool_schema_test.rs`

### Tool: `index_repository`

- [x] `repo_path` (required)
- [x] `project`, `mode`, `persistence`
- [x] `target_projects` (`mode=cross-repo-intelligence`; `["*"]` supported)
- [x] Response: success, counts, duration, artifact path

### Tool: `search_graph`

- [x] Core filters: query, label, name/qn/file patterns, relationship, direction
- [x] Pagination: limit, offset
- [ ] `semantic_query` / vector query if reference exposes
- [ ] BM25 + camelCase/snake_case tokenization
- [ ] Degree filters, entry-point filters, `total`/`has_more`

### Tool: `trace_path`

- [x] BFS inbound/outbound/both
- [x] Depth limit
- [ ] `data_flow`, `cross_service` modes
- [ ] Cycle handling metadata

### Tool: `get_code_snippet`

- [x] QN lookup + line slice from stored file content

### Tool: `get_graph_schema`

- [x] Honest `implemented_edge_types` list
- [~] Project-scoped counts

### Tool: `get_architecture`

- [~] Symbol/edge counts, community summary (components MVP)
- [ ] Reference-grade community labels

### Tool: `query_graph`

- [x] SELECT-only guard
- [x] Block PRAGMA mutation, attach, multi-statement writes
- [ ] Cypher subset parity (or documented SQL-only deviation)

### Tool: `search_code`

- [x] Full-text over file contents in store

### Tool: `list_projects` / `delete_project`

- [x] Basic CRUD

### Tool: `detect_changes`

- [~] Git HEAD + dirty file detection

### Tool: `manage_adr`

- [x] List/create/update ADR entries
- [x] `manage_adr` mode=sections (reference alias; no separate `manage_adr_sections` tool)

### Tool: `ingest_traces`

- [~] `RUNTIME_TRACE` edge ingest

### RLM tools (`rlm_*`)

- [-] N/A — [`rlm-mcp`](https://github.com/stevenke1981/rlm-mcp)

## 3. Storage (`architecture.md` §2.4, `knowledge-graph.md` §2, `functions.md` §1.4)

### Tables

- [~] `projects`, `files`, `symbols`, `edges`, `vectors`, `meta`
- [~] ADR storage
- [ ] Schema migration version + compatibility checks

### Store API

- [x] Open/create per-project DB
- [x] Upsert nodes, edges, files, vectors
- [x] BFS, search, architecture queries
- [x] Bulk write: `begin_bulk_write` / `commit` / `rollback`
- [x] Rollback tests (`tests/store_bulk_write_test.rs`)
- [ ] Read-only open for query-only operations
- [ ] `check_integrity` gate for release smoke
- [ ] Drop/create indexes around bulk load (reference optimization)

### Artifacts

- [~] `.codebase-memory/graph.db.zst` export
- [~] Restore on cold start when cache miss
- [ ] Skip restore when valid cache exists (verify parity)
- [ ] Checksum / version validation on import

## 4. Index pipeline (`architecture.md` §2.3, `functions.md` §1.3)

### Orchestration

- [x] `run_full` wrapped in single bulk transaction
- [~] `run_incremental` (hash-based; limited invalidation)
- [ ] Pipeline try-lock for watcher (non-blocking reindex)
- [ ] Per-pass metrics: files, symbols, edges by type, duration, memory
- [ ] Pass-level failure isolation
- [ ] Cancel token propagation

### Passes

| Pass | Checklist |
|------|-----------|
| Discover | [x] walk tree · [x] gitignore/cbmignore · [~] language detect |
| Structure | [x] Project/Folder/File nodes · [ ] Package nodes |
| Extract / definitions | [~] tree-sitter 7 langs · [ ] symbol registry |
| Imports | [~] regex per language · [ ] import graph reachability |
| Calls | [~] Rust AST · [~] regex fallback · [x] registry resolve · [~] LSP cross-file (Py/JS/Go MVP) |
| Usages / TypeRef | [ ] `pass_usages` parity |
| Inherits / Implements | [~] regex patterns |
| Decorates | [~] attribute patterns |
| Semantic vectors | [~] 11 signals (MVP weights) |
| Semantic edges | [~] SIMILAR_TO / SEMANTICALLY_RELATED thresholds |
| Routes | [~] HTTP_ROUTE framework patterns |
| HTTP_CALLS | [ ] |
| ASYNC_CALLS | [ ] |
| Tests | [ ] TESTS edges |
| Config / env / K8s | [ ] |
| Git history / cross-repo | [~] HEAD/dirty only · [ ] history passes |
| Communities | [~] connected components · [ ] Leiden/Louvain |
| Post summaries | [~] via get_architecture |

### Graph buffer

- [ ] In-memory staging (`graph_buffer.c` equivalent) before SQLite flush

### Registry & FQN (`registry.c`, `fqn.c`)

- [ ] `cbm_registry_add` / `resolve` / `fuzzy_resolve`
- [ ] Import reachability for call targets
- [ ] Confidence band in edge `properties_json`
- [~] QN format: `file::label::name@Lline`

## 5. CALLS & LSP (`architecture.md`, `TODO.md` P0)

- [~] Rust AST same-file calls
- [ ] Python LSP-assisted resolution
- [ ] TypeScript/JavaScript LSP
- [ ] Go LSP
- [x] Java LSP cross-file MVP (`lsp_cross.rs` + `import_map` + pipeline fixtures)
- [ ] C/C++ LSP (+ compile_commands)
- [ ] C# LSP
- [ ] PHP LSP
- [ ] Method vs free-function disambiguation
- [ ] Alias/import-aware resolution
- [ ] Negative fixtures: duplicate names, nested fns, overloads, aliases

## 6. Edge vocabulary (`specifications.md`, `PARITY_MATRIX.md`)

- [x] CONTAINS, IMPORTS, CALLS
- [~] INHERITS, IMPLEMENTS, DECORATES, HTTP_ROUTE
- [~] SIMILAR_TO, SEMANTICALLY_RELATED (optional semantic pass)
- [x] RUNTIME_TRACE (ingest)
- [ ] HTTP_CALLS, ASYNC_CALLS, TYPE_REF, TESTS, CONFIGURES

## 7. Semantic system (`architecture.md` §semantic)

- [~] All 11 signals implemented (`src/semantic/signals.rs`)
- [ ] Reference-tuned weights and thresholds
- [ ] Score breakdown in edge metadata
- [ ] Vector dimension / quantization compatibility tests
- [ ] Optional pass — does not block fast index mode

## 8. Language coverage (`internal/cbm/grammar_*.c`)

- [~] 7 core languages
- [ ] Inventory 159 reference grammars
- [ ] Feature-gated language packs strategy
- [ ] Fallback extractor for unsupported langs
- [ ] Per-language smoke fixture per new grammar

## 9. Watcher (`architecture.md` §2.4)

- [x] Background poll thread
- [x] Git HEAD change detection
- [x] Debounce / exponential backoff
- [~] Dirty signature
- [~] Pipeline busy skip + retry
- [ ] Cancellation on process shutdown tests

## 10. HTTP UI (`graph-ui/`, `src/ui/`)

- [x] Optional HTTP server on port 9749
- [~] Lightweight embedded UI (`http/ui.html`)
- [ ] React + Three.js full graph-ui port (P2 decision)
- [ ] Playwright / screenshot smoke

## 11. Agent install & packaging (`pkg/`, `install.ps1`)

- [~] OpenCode, Codex, Claude, Gemini, Zed, Aider templates
- [~] Idempotent install/uninstall
- [ ] JSON / JSONC / TOML config parsing parity
- [ ] Release archive smoke on fresh Windows machine
- [ ] Homebrew, Scoop, Winget, npm, PyPI, Go, Chocolatey, AUR wrappers (P2)

## 12. Security & hardening (`specifications.md`)

- [~] URL/path safety in install/download paths
- [~] `query_graph` injection guards
- [ ] JSON-RPC request size limits
- [ ] Atomic temp-file writes for config/artifacts
- [ ] Corrupt DB recovery behavior
- [ ] Windows path canonicalization tests

## 13. Performance & CI (`scripts/benchmark-*.sh`)

- [ ] Tiny / medium / large benchmark corpora
- [ ] Cold full index, incremental, search, trace latency
- [ ] Memory peak reporting (`runtime/budget.rs` partial)
- [ ] Regression thresholds in CI or nightly

## 14. Verification gates (`TODO.md` final checklist)

- [x] `cargo test` (incl. `mcp_tool_schema`, bulk write)
- [x] `cargo clippy -D warnings`
- [x] `cargo build --release`
- [x] `scripts/smoke-quality-gates.ps1`
- [x] `scripts/smoke-release-artifact.ps1`
- [ ] `cargo fmt --check` in all CI paths
- [ ] Reference fixture parity suite
- [ ] Large repo indexing benchmark
- [ ] Fresh-machine installer smoke
- [ ] `PARITY_MATRIX.md` has no unstated gaps

---

## Next implementation slices (from checklist density)

1. **P0** — Registry + hybrid LSP CALLS (`§5`)
2. **P0** — Graph buffer staging (`§4`)
3. **P0** — Wire `target_projects`; decide `manage_adr_sections` (`§2`)
4. **P0** — Read-only store + integrity gate (`§3`)
5. **P1** — Usages/TypeRef, HTTP_CALLS, Leiden communities (`§4`, `§6`, `§7`)