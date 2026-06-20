# Reference → Rust module map

> Generated: 2026-06-12 · Reference: `D:\_cbm-ref` · Target: `D:\cbm-mcp`
>
> Public status table: [`PARITY_MATRIX.md`](../PARITY_MATRIX.md) · Execution backlog: [`TODO.md`](../TODO.md)

## How to read this document

| Status | Meaning |
|--------|---------|
| **Done** | Rust module exists; behavior matches reference for supported scope |
| **Partial** | Module exists; precision, coverage, or API gaps documented |
| **MVP** | Works for agent workflows; heuristic or simplified |
| **Stub** | Placeholder or spec-only; not wired |
| **Missing** | No Rust equivalent yet |
| **Omitted** | Intentionally not ported (e.g. FoundationDB, RLM) |
| **N/A** | Out of scope for cbm-mcp (e.g. RLM → `rlm-mcp`) |

---

## Top-level layout

| Reference path | Role | Rust path | Status | Notes |
|----------------|------|-----------|--------|-------|
| `src/main.c` | Entry, signals, threads | `src/main.rs` | Partial | Ctrl+C shutdown; no parent watchdog |
| `src/cli/` | CLI dispatch, hook-augment | `src/cli/`, `src/hooks/` | Partial | `--json --quiet` done; `config` subcmd partial |
| `src/mcp/` | JSON-RPC MCP server | `src/mcp/` | Partial | 14 tools; process-level smoke in `tests/mcp_process_test.rs` |
| `src/pipeline/` | Index passes + registry | `src/pipeline/` | Partial | ~8 passes vs ~20 reference passes |
| `src/store/` | SQLite graph CRUD | `src/store/` | Partial | Bulk tx done; no read-only query mode |
| `src/graph_buffer/` | In-memory staging | `pipeline/graph_buffer.rs` | **Partial** | MVP staging + edge dedup + single flush; no parallel worker merge |
| `src/discover/` | File discovery + language | `src/discover.rs` | Partial | `.gitignore`/`.cbmignore`; 7 langs vs 159 grammars |
| `src/semantic/` | 11-signal similarity | `src/semantic/` | MVP | All signals wired; weights not reference-tuned |
| `src/simhash/` | MinHash structures | `src/semantic/signals.rs` | Partial | Embedded in semantic module |
| `src/cypher/` | Cypher subset queries | `src/mcp/tools.rs` (`query_graph`) | Partial | SQL-only; read-only guard |
| `src/foundation/` | Alloc, hash, logging, platform | `src/error.rs`, `src/runtime/` | Partial | Rust std + tracing; no mimalloc/arena |
| `src/watcher/` | Git poll reindex | `src/watcher.rs` | Done | Backoff + dirty signature |
| `src/traces/` | Runtime trace ingest | `src/mcp/tools.rs` (`ingest_traces`) | MVP | |
| `src/ui/` + `graph-ui/` | HTTP + React 3D UI | `src/http/` | MVP | Embedded HTML UI; no React/Three.js port |
| `internal/cbm/` | Tree-sitter + extract + LSP | `src/pipeline/*`, `internal/` N/A | Partial | See [internal/cbm](#internalcbm-tree-sitter--extractors) |
| `pkg/*` | Go/Python/npm/Homebrew… | `src/install/` | Partial | Installer templates; legacy naming cleaned names remain |
| `scripts/` | Build, bench, security | `scripts/` | Partial | Quality-gate smokes; no benchmark CI |
| `test-infrastructure/` | Docker bench env | `tests/fixtures/` | Partial | Fixture repos; no docker bench |
| `vendored/` | sqlite3, yyjson, tre, mongoose… | `Cargo.toml` deps | Partial | Rust crates replace most vendored C |
| `tools/` | Grammar generators | — | **Missing** | `generate-lang-code.py` not ported |

**RLM** (`rlm_*` tools, session persistence): **N/A** → standalone [`rlm-mcp`](https://github.com/stevenke1981/rlm-mcp).

---

## `src/` core modules

### Entry (`src/main.c` → `src/main.rs`, `src/lib.rs`)

| Reference concern | Rust | Status |
|-------------------|------|--------|
| MCP stdio default mode | `main.rs` | Done |
| CLI subcommand | `cli/mod.rs` | Done |
| install/uninstall/update | `install/mod.rs` | Partial |
| `--ui`, `--port`, `--profile` | `http/mod.rs`, `runtime/profile.rs` | Partial |
| Signal shutdown cascade | `main.rs`, `watcher.rs` | MVP |
| Watcher + HTTP background threads | `watcher.rs`, `http/mod.rs` | Done |

### MCP (`src/mcp/` → `src/mcp/`)

| Reference file / API | Rust module | Status |
|----------------------|-------------|--------|
| `mcp.c` JSON-RPC framing | `mcp/transport.rs` | Partial |
| `tools/list` from specs | `mcp/tool_specs.rs` | Done |
| Tool handlers (14 graph) | `mcp/tools.rs` | Partial |
| Store cache / evict idle | `mcp/server.rs` | Partial |
| `target_projects` arg | `index_repository` + `cross_repo.rs` | **Partial** — `mode=cross-repo-intelligence` MVP |
| `manage_adr_sections` | docs only | **Alias** — `manage_adr` mode=sections (no separate tool) |

Checked-in tool schemas: `mcps/codebase-memory-mcp/tools/*.json` (snapshot-tested).

### Store (`src/store/` + `internal/cbm/sqlite_writer.c` → `src/store/`)

| Reference API | Rust | Status | Fixture |
|---------------|------|--------|---------|
| Schema (projects, files, symbols, edges, vectors, meta, ADR) | `store/schema.rs` | Partial | `tests/store_*` |
| `cbm_store_begin_bulk` / `end_bulk` | `begin_bulk_write` / `commit` / `rollback` | Done | `tests/store_bulk_write_test.rs` |
| `cbm_store_open_path_query` (read-only) | — | **Missing** | — |
| `cbm_store_check_integrity` | — | **Missing** | — |
| `cbm_store_dump_to_file` | `persistence/mod.rs` | Partial | artifact round-trip manual |
| BFS / search / vector search | `store/mod.rs` | Partial | smoke tests |
| `cbm_leiden` / `cbm_louvain` | `pipeline/communities.rs` (components) | MVP | — |
| Migration version metadata | — | **Missing** | — |

### Graph buffer (`src/graph_buffer/`)

| Reference | Rust | Status |
|-----------|------|--------|
| In-memory node/edge arena during index | `GraphBuffer` in `pipeline/mod.rs` | **Partial** |
| Bulk dump buffer → SQLite | `flush_to_store` inside bulk transaction | **Partial** |

**Next slice:** add staging layer before SQLite flush (P0 in `TODO.md`).

### Discover (`src/discover/` → `src/discover.rs`)

| Reference | Rust | Status |
|-----------|------|--------|
| Directory walk | `discover()` | Done |
| `.gitignore` / `.cbmignore` | Yes | Done |
| Language detection (`language.c`, 159 grammars) | `language_for_path` (7 core langs) | Partial |
| `IndexMode` full/moderate/fast | Yes | Done |

### Pipeline (`src/pipeline/` → `src/pipeline/`)

Reference passes (37 files) mapped to Rust:

| Reference pass | File | Rust module | Status | Missing behavior |
|----------------|------|-------------|--------|------------------|
| Discover | (in `pipeline.c`) | `discover.rs` + `pipeline/mod.rs` | Done | — |
| Structure | `pass_definitions.c` (structure nodes) | `pipeline/structure.rs` | Done | Package-level nodes |
| Definitions / extract | `pass_definitions.c` | `pipeline/extract.rs` | Partial | 7 langs; no registry |
| Imports | `extract_imports.c` | `pipeline/imports.rs` | Partial | Regex heuristic |
| Calls | `pass_calls.c`, `extract_calls.c` | `pipeline/calls.rs`, `calls_ast.rs`, `registry.rs`, `import_map.rs` | Partial | AST + import map; LSP subprocess pending |
| LSP cross-file | `pass_lsp_cross.c` | `pipeline/lsp_cross.rs` | **Partial** | Python/JS/TS/Go/Java imported-type methods; C/PHP pending |
| Usages / TypeRef | `pass_usages.c`, `extract_usages.c` | — | **Missing** | `TYPE_REF` edges |
| Inherits / Implements | (semantic + extract) | `pipeline/inheritance.rs` | Partial | Regex |
| Decorates | `extract_semantic.c` patterns | `pipeline/inheritance.rs` | Partial | |
| Semantic vectors | `pass_semantic.c` | `semantic/mod.rs` | MVP | |
| Semantic edges | `pass_semantic_edges.c`, `pass_similarity.c` | `semantic/mod.rs` | MVP | |
| Routes | `pass_route_nodes.c` | `pipeline/routes.rs` | MVP | Framework-limited |
| HTTP calls | (service patterns) | — | **Missing** | `HTTP_CALLS` |
| Async calls | — | — | **Missing** | `ASYNC_CALLS` |
| Tests | `pass_tests.c` | — | **Missing** | `TESTS` edges |
| Config / env | `pass_configures.c`, `pass_envscan.c` | — | **Missing** | `CONFIGURES` |
| K8s | `pass_k8s.c`, `extract_k8s.c` | — | **Missing** | |
| Git history | `pass_githistory.c`, `pass_gitdiff.c` | `git.rs` (HEAD/dirty only) | Partial | |
| Cross-repo | `pass_cross_repo.c` | — | **Missing** | |
| Communities | Leiden in store | `pipeline/communities.rs` | MVP | Connected components only |
| Enrichment / complexity | `pass_enrichment.c`, `pass_complexity.c` | — | **Missing** | Halstead in semantic only |
| Compile commands | `pass_compile_commands.c` | — | **Missing** | C/C++ LSP aid |
| Parallel workers | `pass_parallel.c`, `worker_pool.c` | `rayon` in pipeline | Partial | |
| Incremental | `pipeline_incremental.c` | `pipeline/mod.rs` `run_incremental` | Partial | File-hash invalidation |
| Registry | `registry.c` | `pipeline/registry.rs` | **Partial** | Import-aware resolve; no LSP subprocess yet |
| FQN / path alias | `fqn.c`, `path_alias.c` | `project.rs`, `symbol_id.rs` | Partial | Relative import resolve |
| Artifact | `artifact.c` | `persistence/mod.rs` | Partial | Checksum/version gate |

### Semantic (`src/semantic/` → `src/semantic/`)

| Reference signal | Rust (`signals.rs`, `ri.rs`, `corpus.rs`) | Status |
|------------------|-------------------------------------------|--------|
| TF-IDF | Yes | MVP |
| Random Indexing | `ri.rs` | MVP |
| MinHash structure | Yes | Partial |
| API signature vector | Yes | MVP |
| Module proximity | Yes | MVP |
| Halstead complexity | Yes | MVP |
| Type signature vector | Yes | MVP |
| Decorator pattern vector | Yes | MVP |
| AST structural profile | Yes | MVP |
| Approximate data flow | Yes | MVP |
| Graph diffusion | Yes | MVP |

### Cypher (`src/cypher/` → `query_graph` handler)

| Reference | Rust | Status |
|-----------|------|--------|
| Cypher subset parser | SQL passthrough + guards | Partial |
| Read-only enforcement | Yes | Done |
| PRAGMA/attach block | Yes | Done |

### Watcher (`src/watcher/` → `src/watcher.rs`)

| Reference | Rust | Status |
|-----------|------|--------|
| Poll git HEAD | Yes | Done |
| Debounce / backoff | Yes | Done |
| Pipeline try-lock | `test_lock.rs` pattern | Partial |
| Cancel on shutdown | Yes | MVP |

### Traces (`src/traces/` → `ingest_traces`)

| Reference | Rust | Status |
|-----------|------|--------|
| `RUNTIME_TRACE` edges | `mcp/tools.rs` | MVP |

### UI (`src/ui/` + `graph-ui/` → `src/http/`)

| Reference | Rust | Status |
|-----------|------|--------|
| Mongoose HTTP server | `axum` / embedded server | Done |
| React + Three.js graph | `http/ui.html` lightweight | MVP |
| Search panel, edge filters | Partial in HTML UI | MVP |

### Foundation (`src/foundation/` → Rust std + crates)

| Reference (38 files) | Rust replacement | Status |
|----------------------|------------------|--------|
| mimalloc / arena | `std` allocator | Omitted |
| hash_table, string | `HashMap`, `String` | Done |
| logging | `tracing` | Done |
| platform (Windows/POSIX) | `std::path`, `canonicalize` tests needed | Partial |

---

## `internal/cbm/` (tree-sitter + extractors)

Embedded tree-sitter runtime + per-language grammars. Rust uses `tree-sitter` crate with a small grammar set.

### Runtime

| Reference | Rust | Status |
|-----------|------|--------|
| `ts_runtime.c`, parser/scanner/tree | `tree-sitter` crate | Partial |
| `lang_specs.c` | `discover.rs` + `extract.rs` | Partial |
| `type_registry.c`, `type_rep.c` | — | **Missing** |
| `arena.c`, `ac.c` (Aho-Corasick) | — | **Missing** / std |

### Extractors

| Reference | Rust | Status |
|-----------|------|--------|
| `extract_defs.c` | `pipeline/extract.rs` | Partial |
| `extract_imports.c` | `pipeline/imports.rs` | Partial |
| `extract_calls.c` | `pipeline/calls.rs`, `calls_ast.rs` | Partial |
| `extract_unified.c` | — | **Missing** |
| `extract_usages.c` | — | **Missing** |
| `extract_type_refs.c` | — | **Missing** |
| `extract_type_assigns.c` | — | **Missing** |
| `extract_semantic.c` | `semantic/` | Partial |
| `extract_channels.c` | — | **Missing** |
| `extract_env_accesses.c` | — | **Missing** |
| `extract_k8s.c` | — | **Missing** |

### LSP helpers (`*_lsp.c`, `lsp_all.c`)

| Language | Reference | Rust | Status |
|----------|-----------|------|--------|
| Rust | `rust_lsp.c` | `calls_ast.rs` (AST only) | Partial |
| Python | `py_lsp.c` | — | **Missing** |
| TypeScript | `ts_lsp.c` | — | **Missing** |
| Go | `go_lsp.c` | — | **Missing** |
| Java | `java_lsp.c` | — | **Missing** |
| C/C++ | `c_lsp.c` | — | **Missing** |
| C# | `cs_lsp.c` | — | **Missing** |
| PHP | `php_lsp.c` | — | **Missing** |
| Kotlin | `kotlin_lsp.c` | — | **Missing** |

**Next slice (P0):** hybrid CALLS + LSP cross-file (`pass_lsp_cross.c` parity).

### Grammars (`grammar_*.c`)

| Metric | Reference | Rust |
|--------|-----------|------|
| Grammar count | **~159** (`grammar_*.c`) | **7** (Rust, Python, JS/TS, Go, Java, C, C++) |
| Strategy | Vendored tree-sitter grammars | `tree-sitter-*` crates; expand via feature packs |

### Storage helpers

| Reference | Rust | Status |
|-----------|------|--------|
| `sqlite_writer.c` | `store/mod.rs` | Partial |
| `zstd_store.c` | `persistence/mod.rs` | Partial |
| `lz4_store.c` (bulk load compression) | — | **Missing** |
| `wasm_store.c` | — | Omitted |

### Stdlib seed data (`*_stdlib_data.c`, `rust_crates_seed.c`)

| Reference | Rust | Status |
|-----------|------|--------|
| Built-in stdlib call targets | — | **Missing** | Improves external CALLS resolution |

---

## Packaging & distribution (`pkg/`)

| Reference | Rust | Status |
|-----------|------|--------|
| `pkg/go/` | — | **Missing** (P2) |
| `pkg/pypi/` | — | **Missing** (P2) |
| `pkg/npm/` | — | **Missing** (P2) |
| `pkg/homebrew`, `scoop`, `winget`, `chocolatey`, `aur` | `packaging/` stubs | Partial |
| `install.ps1` / `install.sh` | `src/install/mod.rs` | Partial |

Agent templates: `packaging/mcp/templates/` (OpenCode, Codex, Claude, dual-server with rlm-mcp).

---

## Agent integration

| Reference | Rust | Status |
|-----------|------|--------|
| `hook_augment.c` | `src/hooks/mod.rs` | Done |
| `src/agent/` patterns | `src/agent/mod.rs` | Partial |
| MCP name `codebase-memory-mcp` | Yes | Done |

---

## Test & verification mapping

| Reference | Rust | Status |
|-----------|------|--------|
| `tests/` C unit tests | `tests/*.rs`, `src/**` `#[cfg(test)]` | Partial |
| `test-infrastructure/` docker bench | `tests/fixtures/` | Partial |
| `scripts/benchmark-*.sh` | — | **Missing** |
| MCP schema snapshot | `tests/mcp_tool_schema_test.rs` | Done |
| CALLS fixtures | cross-lang tests | Partial |
| Bulk rollback | `tests/store_bulk_write_test.rs` | Done |

---

## Priority gaps (execution order)

Aligned with [`TODO.md`](../TODO.md) P0:

1. **Registry + LSP cross-file CALLS** — `registry.c`, `pass_lsp_cross.c`, `*_lsp.c`
2. **Graph buffer staging** — `src/graph_buffer/`
3. **Usages / TypeRef pass** — `pass_usages.c`, `extract_type_refs.c`
4. **`target_projects`** — wire `index_repository` handler to spec
5. ~~**`manage_adr_sections`**~~ — `manage_adr` mode=sections (done)
6. **Read-only store + integrity gate** — release smoke
7. **Grammar expansion** — track per-language in `PARITY_MATRIX.md`

---

## Related docs

- Spec copies: [`docs/reference/`](reference/)
- Rust checklist derived from specs: [`IMPLEMENTATION_CHECKLIST.md`](IMPLEMENTATION_CHECKLIST.md)
- Milestones: [`CLONE_ROADMAP.md`](../CLONE_ROADMAP.md)