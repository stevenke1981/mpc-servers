#!/usr/bin/env bash
set -euo pipefail

REPO="stevenke1981/mpc-servers"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="${MPC_SERVERS_VERSION:-latest}"
INSTALL_DIR="${HOME}/.config/mpc-servers/bin"
FROM_SOURCE=0
SKIP_BUILD=0
JSON=0
SERVERS=("all")

usage() {
  cat <<'USAGE'
Usage:
  ./install.sh [--server all|cbm|everything|filesystem|fetch|git|memory|nushell|rlm|time|sequential-thinking] [--version latest|vX.Y.Z]
  ./install.sh --from-source [--server all] [--skip-build] [--json]

Options:
  --server       Server to install. Can be repeated. Default: all
  --version      GitHub Release tag or latest. Default: latest
  --install-dir  Install directory. Default: ~/.config/mpc-servers/bin
  --from-source  Build/copy from this checkout instead of downloading a release asset
  --skip-build   With --from-source, copy existing target/release binaries
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
    --version)
      VERSION="${2:?missing value for --version}"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:?missing value for --install-dir}"
      shift 2
      ;;
    --from-source)
      FROM_SOURCE=1
      shift
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
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

server_package() {
  case "$1" in
    cbm) echo "codebase-memory-mcp" ;;
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

target_triple() {
  local os arch
  case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="macos" ;;
    *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
  esac
  echo "${os}-${arch}"
}

release_url() {
  local asset="$1"
  if [[ "$VERSION" == "latest" ]]; then
    echo "https://github.com/${REPO}/releases/latest/download/${asset}"
  else
    echo "https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
  fi
}

json_escape() {
  python -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

download_bundle() {
  local triple asset tmp url
  triple="$(target_triple)"
  asset="mpc-servers-${triple}.tar.gz"
  url="$(release_url "$asset")"
  tmp="$(mktemp -d)"
  if ! curl -fsSL "$url" -o "${tmp}/${asset}"; then
    echo "Failed to download ${url}. Release assets may not exist yet; use --from-source for local development installs." >&2
    exit 1
  fi
  tar -xzf "${tmp}/${asset}" -C "$tmp"
  echo "$tmp"
}

find_binary_in_bundle() {
  local bundle_dir="$1" binary="$2"
  local found
  found="$(find "$bundle_dir" -type f -name "$binary" -print -quit)"
  if [[ -z "$found" ]]; then
    echo "Release bundle did not contain ${binary}" >&2
    exit 1
  fi
  echo "$found"
}

mkdir -p "$INSTALL_DIR"
BUNDLE_DIR=""
if [[ "$FROM_SOURCE" -eq 0 ]]; then
  BUNDLE_DIR="$(download_bundle)"
fi

reports=()
while IFS= read -r server; do
  package="$(server_package "$server")"
  binary="$(server_binary "$server")"

  if [[ "$FROM_SOURCE" -eq 1 ]]; then
    if [[ "$SKIP_BUILD" -eq 0 ]]; then
      (cd "$SCRIPT_DIR" && cargo build --release -p "$package")
    fi
    source_path="${SCRIPT_DIR}/target/release/${binary}"
    [[ -f "$source_path" ]] || { echo "Built binary not found: ${source_path}" >&2; exit 1; }
  else
    source_path="$(find_binary_in_bundle "$BUNDLE_DIR" "$binary")"
  fi

  target_path="${INSTALL_DIR}/${binary}"
  cp "$source_path" "$target_path"
  chmod +x "$target_path"
  version_output="$("$target_path" --version | tr -d '\r')"

  reports+=("{\"server_name\":$(json_escape "$server"),\"version\":$(json_escape "$version_output"),\"installed_exe\":$(json_escape "$target_path"),\"config_targets\":[\"codex\",\"opencode\",\"claude\"],\"changed\":true,\"warnings\":[]}")
done < <(resolve_servers)

if [[ "$JSON" -eq 1 ]]; then
  printf '[%s]\n' "$(IFS=,; echo "${reports[*]}")"
else
  for report in "${reports[@]}"; do
    python -c 'import json,sys; r=json.loads(sys.argv[1]); print(f"Installed {r[\"server_name\"]} {r[\"version\"]}: {r[\"installed_exe\"]}")' "$report"
  done
  echo
  echo "Use the installed_exe path in Codex/OpenCode/Claude config. Do not point agents at target/release."
fi
