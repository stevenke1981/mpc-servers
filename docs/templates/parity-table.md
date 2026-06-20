# <server-name> Parity

Parity document for the Rust implementation of `<upstream server/package>`.

## Status

- Rust crate: `<crate name>`
- Binary: `<binary name>`
- Upstream source/version: `<source>`
- Current status: `<implemented | partial | deferred>`
- Last verified: `<date or command evidence>`

## Feature Parity

| Upstream feature/tool | Rust feature/tool | Status | Verification | Notes |
|---|---|---|---|---|
| `<upstream tool>` | `<rust tool>` | implemented | `<unit test or smoke command>` | `<known difference>` |

Status values:

- `implemented`: behavior is present and verified by tests or SDK smoke.
- `partial`: behavior is present but has documented limits.
- `deferred`: intentionally not implemented yet, with a stated reason and follow-up task.
- `not applicable`: upstream behavior does not apply to this Rust server.

## Protocol Coverage

Use this table for MCP behavior beyond simple tool calls.

| Protocol feature | Status | Verification | Notes |
|---|---|---|---|
| tools/list | implemented | `scripts/tools-list-smoke.ps1` | No boolean JSON Schema nodes. |
| prompts/list | not applicable |  |  |
| resources/list | not applicable |  |  |
| resources/templates/list | not applicable |  |  |
| resources/subscribe | not applicable |  |  |
| roots/list | not applicable |  |  |
| logging/setLevel | not applicable |  |  |
| notifications/progress | not applicable |  |  |
| sampling/createMessage | not applicable |  |  |
| elicitation/create | deferred |  | Add reason when deferred. |

## Safety Parity

| Safety boundary | Status | Verification | Notes |
|---|---|---|---|
| path traversal protection | not applicable |  |  |
| symlink escape protection | not applicable |  |  |
| private network / SSRF policy | not applicable |  |  |
| process timeout | not applicable |  |  |
| shell injection protection | not applicable |  |  |
| persistence isolation | not applicable |  |  |

## Verification Commands

```powershell
cargo fmt --check -p <crate-name>
cargo test -p <crate-name>
cargo clippy -p <crate-name> --all-targets -- -D warnings
cargo build --release -p <crate-name>
.\scripts\tools-list-smoke.ps1 -Binary .\target\release\<binary>.exe -ExpectedToolCount <n> -ExpectedTools <tool1>,<tool2>
.\scripts\release-check.ps1 -Server <server-name> -SkipBuild
```

## Remaining Work

- `<follow-up task, or "None">`
