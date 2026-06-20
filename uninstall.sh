#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.config/mpc-servers/bin"
JSON=0
SERVERS=("all")

usage() {
  cat <<'USAGE'
Usage:
  ./uninstall.sh [--server all|cbm|everything|filesystem|fetch|git|memory|nushell|rlm|time|sequential-thinking] [--install-dir DIR] [--json]

Options:
  --server       Server to uninstall. Can be repeated. Default: all
  --install-dir  Install directory. Default: ~/.config/mpc-servers/bin
  --json         Emit machine-readable JSON report
  -h, --help     Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --server)
      [[ ${SERVERS[*]} == "all" ]] && SERVERS=()
      SERVERS+=("${2:?missing value for --server}")
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:?missing value for --install-dir}"
      shift 2
      ;;
    --json)
      JSON=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

server_binary() {
  case "$1" in
    cbm) echo "cbm" ;;
    everything) echo "everything-server" ;;
    filesystem) echo "filesystem-server" ;;
    fetch) echo "fetch-server" ;;
    git) echo "git-server" ;;
    memory) echo "memory-mcp-server" ;;
    nushell) echo "nushell-mcp" ;;
    rlm) echo "rlm-mcp" ;;
    time) echo "time-server" ;;
    sequential-thinking) echo "sequential-thinking-server" ;;
    *) echo "Unknown server: $1" >&2; exit 1 ;;
  esac
}

resolve_servers() {
  for server in "${SERVERS[@]}"; do
    if [[ "$server" == "all" ]]; then
      printf '%s\n' cbm everything filesystem fetch git memory nushell rlm time sequential-thinking
      return
    fi
  done
  printf '%s\n' "${SERVERS[@]}"
}

json_escape() {
  python -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

reports=()
while IFS= read -r server; do
  binary="$(server_binary "$server")"
  removed=()
  if [[ -d "$INSTALL_DIR" ]]; then
    while IFS= read -r path; do
      [[ -n "$path" ]] || continue
      rm -f "$path"
      removed+=("$path")
    done < <(find "$INSTALL_DIR" -maxdepth 1 -type f \( -name "$binary" -o -name "${binary}-*" \) -print)
  fi

  removed_json="[]"
  if [[ ${#removed[@]} -gt 0 ]]; then
    removed_json="$(python -c 'import json,sys; print(json.dumps(sys.argv[1:]))' "${removed[@]}")"
  fi
  changed=false
  [[ ${#removed[@]} -gt 0 ]] && changed=true
  reports+=("{\"server_name\":$(json_escape "$server"),\"removed\":${removed_json},\"changed\":${changed},\"warnings\":[\"Agent configuration files were not modified.\"]}")
done < <(resolve_servers)

if [[ "$JSON" -eq 1 ]]; then
  printf '[%s]\n' "$(IFS=,; echo "${reports[*]}")"
else
  for report in "${reports[@]}"; do
    python -c 'import json,sys; r=json.loads(sys.argv[1]); print(("Removed " + r["server_name"] + ":\n  " + "\n  ".join(r["removed"])) if r["changed"] else ("Nothing to remove for " + r["server_name"] + "."))' "$report"
  done
  echo
  echo "Codex/OpenCode/Claude configuration files were not modified."
fi
