#!/usr/bin/env bash
# codebase-memory-mcp search augmenter (Claude Code PreToolUse).
set +e
BIN="${CBM_BIN:-{{CBM_BIN}}}"
if [[ ! -x "$BIN" ]]; then exit 0; fi
"$BIN" hook-augment 2>/dev/null
exit 0