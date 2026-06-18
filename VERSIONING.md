# Versioning Policy

This workspace uses SemVer for all Rust crates and release artifacts.

## Current Baseline

- Workspace version: `0.1.0`
- New crates inherit `workspace.package.version` until a crate needs an independent release cadence.
- Existing reusable servers keep their current upstream crate versions until imported or released from this workspace:
  - `memlong`: `0.1.0`
  - `nushell-mcp`: `0.1.0`
  - `rlm-mcp`: `0.1.6`
  - `codebase-memory-mcp` / `cbm`: `0.2.3`

## Tag Strategy

Use one of these two tag forms:

- Workspace release: `vX.Y.Z`
- Independent server release: `<server-name>-vX.Y.Z`

Prefer independent server tags once the workspace contains multiple production servers with different release speeds.

## Release Checklist

Before tagging a Rust MCP server:

1. Run `cargo fmt --check`.
2. Run `cargo test --all-targets`.
3. Run `cargo clippy --all-targets -- -D warnings`.
4. Run `cargo build --release`.
5. Confirm `--version` reports the tag version.
6. Smoke-test MCP `tools/list` with an OpenCode/TypeScript SDK path, not only an rmcp Rust client.
7. Verify installer output reports the actual installed binary path.
8. Confirm README install snippets do not point to `target/release`.
