---
## Lesson #1 — 2026-06-14
**Trigger:** A requested Rust MCP server was briefly redirected to TypeScript because `cargo` was not on PATH.
**Rule:** Before switching implementation language due to a missing PATH command on Windows, check standard toolchain locations such as `%USERPROFILE%\.cargo\bin\cargo.exe` and verify with the absolute executable path.
**Source:** Rust Nushell MCP server planning

---
## Lesson #2 — 2026-06-14
**Trigger:** Running `cargo test`, `cargo clippy`, and `cargo build --release` in parallel produced build-directory lock noise during verification.
**Rule:** For final Rust verification, run cargo commands sequentially: `fmt --check`, `test --all-targets`, `clippy --all-targets -- -D warnings`, then `build --release`.
**Source:** Rust Nushell MCP server binary compilation

---
## Lesson #3 — 2026-06-15
**Trigger:** `cargo build --release` failed because the installed `target/release/nushell-mcp.exe` was still running and locked by Windows.
**Rule:** When release build cannot replace a running Windows binary, verify compilation with `cargo build --release --target-dir target/verify-release` unless the user approves stopping the active process.
**Source:** Git plugin CRLF warning display fix
