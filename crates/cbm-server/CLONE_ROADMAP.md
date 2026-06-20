# Full clone roadmap — codebase-memory-mcp

Target: feature-equivalent Rust rewrite of `D:\_cbm-ref` (DeusData/codebase-memory-mcp C core).

**Module map:** [`docs/MODULE_MAP.md`](docs/MODULE_MAP.md) · **Checklist:** [`docs/IMPLEMENTATION_CHECKLIST.md`](docs/IMPLEMENTATION_CHECKLIST.md)

## Architecture split (done)

- **cbm-mcp** (`D:\cbm-mcp`) — this repo; 14 graph MCP tools only
- **rlm-mcp** (`D:\rlm-mcp`) — standalone `codebase-memory-rlm-mcp` (scan/peek/chunk); no CBM coupling

## P0 — graph correctness

- [x] Reference module inventory (`docs/MODULE_MAP.md`, `docs/IMPLEMENTATION_CHECKLIST.md`)
- [~] Hybrid CALLS: `SymbolRegistry` + import map + AST + `lsp_cross` (Py/JS/Go/Java); method disambiguation pending
- [x] Store bulk transaction API + rollback tests
- [~] Honest `get_graph_schema` vs emitted edges (implemented; expand as new edge types land)

## P1 — reference pipeline

- [ ] Usages / TypeRef pass
- [ ] `HTTP_CALLS`, `ASYNC_CALLS`, cross-service edges
- [ ] Leiden/Louvain communities (replace connected-components)
- [ ] BM25 + camelCase tokenization in `search_graph`
- [ ] Cypher in `query_graph` (or document SQL-only deviation)
- [ ] `trace_path` data_flow / cross_service modes
- [ ] 159-language tree-sitter coverage (vendored grammars)
- [x] Graph buffer staging layer (`pipeline/graph_buffer.rs`)

## P2 — platform parity

- [ ] React/Three.js graph-ui (or ship reference UI variant)
- [ ] Go/PyPI/npm/Chocolatey/AUR wrappers
- [ ] Reference-grade semantic signal tuning
- [ ] Git history / cross-repo index modes

## Omitted by design

- FoundationDB backend (SQLite canonical)
- Foundation C runtime layer (Rust std + targeted crates)

## Verification

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
.\scripts\smoke-quality-gates.ps1
```

Reference specs: `docs/reference/` (from `knowledge-graph/`).