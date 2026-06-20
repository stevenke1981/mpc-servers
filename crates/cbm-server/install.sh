#!/usr/bin/env bash
# Install codebase-memory-mcp for agents from GitHub Release by default.
#
# Usage:
#   ./install.sh
#   ./install.sh --from-source --all-agents

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${1:-}" == "--from-source" ]]; then
  shift
  exec "$SCRIPT_DIR/scripts/install.sh" "$@"
fi

case "$(uname -s)" in
  Linux) exec bash "$SCRIPT_DIR/packaging/linux/install.sh" "$@" ;;
  Darwin) exec bash "$SCRIPT_DIR/packaging/macos/install.sh" "$@" ;;
  *)
    echo "Unsupported OS for release install: $(uname -s)" >&2
    echo "Use scripts/install.sh for source installs." >&2
    exit 1
    ;;
esac
