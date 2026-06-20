# memory Parity

Parity document for the Rust `memory` server line.

## Status

- Rust crates: `memory-core`, `memory-server`, `memory-cli`
- Binary: `memory-mcp-server`
- Imported source: `stevenke1981/memlong`
- Reference source: `stevenke1981/servers/src/memory`,
  `@modelcontextprotocol/server-memory@0.6.3`
- Current status: implemented as `memlong` replacement semantics.

This workspace keeps the imported Rust `memlong` behavior for the current
release. It does not claim TypeScript reference graph-tool parity yet.

## Tool Parity

| Upstream/reference tool | Rust tool | Status | Verification | Notes |
|---|---|---|---|---|
| `create_entities` | none | deferred | docs decision | Future compatibility bridge task. |
| `create_relations` | none | deferred | docs decision | Future compatibility bridge task. |
| `add_observations` | none | deferred | docs decision | Future compatibility bridge task. |
| `delete_entities` | none | deferred | docs decision | Future compatibility bridge task. |
| `delete_observations` | none | deferred | docs decision | Future compatibility bridge task. |
| `delete_relations` | none | deferred | docs decision | Future compatibility bridge task. |
| `read_graph` | none | deferred | docs decision | Future compatibility bridge task. |
| `search_nodes` | `search_memories` | replacement | SDK smoke | `memlong` searches durable extracted memories with hybrid retrieval semantics, not explicit graph nodes. |
| `open_nodes` | `get_memories` | replacement | SDK smoke | `memlong` retrieves memory records, not reference graph node documents. |
| none | `add_memory` | implemented | SDK smoke inventory | Adds durable memory content. |
| none | `delete_memory` | implemented | unit/integration tests | Deletes stored memories. |
| none | `consolidate_memories` | implemented | unit/integration tests | Consolidates stored memory records. |
| none | `get_memory_stats` | implemented | release-check SDK call | Used by release-check smoke. |
| none | `end_session` | implemented | SDK smoke inventory | Ends a memory session. |

## Decision

N4 decision for this release: keep `memlong` replacement semantics and do not
add the reference graph-tool bridge in the same release batch.

Reasoning:

- `memlong` is already a Rust implementation with persistent SQLite metadata,
  USearch vector index, and Tantivy BM25 index paths.
- The imported server has verified MCP inventory and release-check smoke.
- The TypeScript reference graph model is a different API surface; adding it
  should be a deliberate compatibility bridge with its own tests rather than a
  thin alias that misrepresents the storage model.

## Future Bridge Task

If a client requires `@modelcontextprotocol/server-memory` graph-tool parity,
create a dedicated bridge task:

1. Add the reference tool names to `memory-server`.
2. Define graph entities, relations, and observations storage in `memory-core`
   or a compatibility module.
3. Preserve existing `memlong` tools unchanged.
4. Add SDK smoke for graph inventory and at least one write/read flow:
   `create_entities` -> `create_relations` -> `read_graph`.
5. Update this parity document, root README, `spec.md`, and `todos.md`.

## Verification Commands

```powershell
cargo test -p memory-mcp-server
cargo test -p memory-core
cargo check -p memory-cli
cargo build --release -p memory-mcp-server
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\memory-mcp-server.exe -ServerEnv @{ MEMORY_DB_PATH="$env:TEMP\mpc-memory.db"; MEMORY_VECTOR_PATH="$env:TEMP\mpc-memory.usearch"; MEMORY_TANTIVY_PATH="$env:TEMP\mpc-memory-tantivy"; LLM_API_KEY="mock"; LLM_API_BASE="mock" } -ExpectedToolCount 7 -ExpectedTools add_memory,search_memories,get_memories,delete_memory,consolidate_memories,get_memory_stats,end_session -CallToolName get_memory_stats
.\scripts\release-check.ps1 -Server memory -SkipBuild
```
