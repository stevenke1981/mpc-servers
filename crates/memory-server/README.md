# memory-mcp-server

Rust MCP server imported from `memlong`.

## Status

- Crates: `memory-core`, `memory-server`, `memory-cli`
- Binary: `memory-mcp-server`
- Version: `0.1.0`
- Transport: stdio
- Install name: `memory`
- Parity: see `docs/parity/memory.md`

## Tools

| Tool | Status | Notes |
|---|---|---|
| `add_memory` | implemented | Adds durable memory content. |
| `search_memories` | implemented | Hybrid retrieval over persisted memories. |
| `get_memories` | implemented | Retrieves stored memory records. |
| `delete_memory` | implemented | Deletes memory records. |
| `consolidate_memories` | implemented | Consolidates memory content. |
| `get_memory_stats` | implemented | Reports storage/index stats. |
| `end_session` | implemented | Ends a memory session. |

## Configuration

Default data lives under `PROJECT_ROOT/.opencode/` unless explicit paths are
provided.

- `MEMORY_DB_PATH`: SQLite metadata database
- `MEMORY_VECTOR_PATH`: USearch vector index
- `MEMORY_TANTIVY_PATH`: Tantivy BM25 index directory
- `LLM_API_BASE`: OpenAI-compatible endpoint
- `LLM_API_KEY`: API key or local placeholder
- `EXTRACTION_MODEL`: extraction model override
- `EMBEDDING_MODEL`: embedding model override
- `EMBEDDING_DIM`: embedding dimension override

## Compatibility

The TypeScript reference `memory` server exposes graph tools such as
`create_entities`, `create_relations`, `read_graph`, `search_nodes`, and
`open_nodes`. This Rust workspace currently preserves `memlong` replacement
semantics instead of claiming graph-tool parity.

Use `add_memory` and `search_memories` for durable fact extraction and hybrid
retrieval. Add a dedicated compatibility bridge before advertising reference
graph-tool parity.

## Install

```powershell
.\install.ps1 -Server memory
.\install.ps1 -FromSource -Server memory -Json
.\uninstall.ps1 -Server memory -Json
```

Use the `installed_exe` path from the JSON report in agent configs. Do not point
agents at `target/release`.

## Verify

```powershell
cargo test -p memory-mcp-server
cargo test -p memory-core
cargo check -p memory-cli
cargo build --release -p memory-mcp-server
.\scripts\release-check.ps1 -Server memory -SkipBuild
```
