#!/usr/bin/env bash
# Install cbm-mcp from GitHub Release (Linux x64 / arm64).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/stevenke1981/cbm-mcp/main/packaging/linux/install.sh | bash
#   CBM_VERSION=v0.2.3 ./packaging/linux/install.sh

set -euo pipefail

REPO="${CBM_REPO:-${CBRLM_REPO:-stevenke1981/cbm-mcp}}"
VERSION="${CBM_VERSION:-${CBRLM_VERSION:-latest}}"
INSTALL_DIR="${CBM_INSTALL_DIR:-${CBRLM_INSTALL_DIR:-$HOME/.local/bin}}"
CONFIG_DIR="${CBM_CONFIG_DIR:-${CBRLM_CONFIG_DIR:-$HOME/.config/cbm-mcp/bin}}"

arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) ARTIFACT="cbm-mcp-linux-x64" ;;
  aarch64|arm64) ARTIFACT="cbm-mcp-linux-arm64" ;;
  *)
    echo "Unsupported Linux architecture: $arch" >&2
    exit 1
    ;;
esac

if [ "$VERSION" = "latest" ]; then
  API="https://api.github.com/repos/${REPO}/releases/latest"
  token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
  if [ -n "$token" ]; then
    VERSION="$(curl -fsSL -H "User-Agent: cbm-mcp-installer" -H "Authorization: Bearer ${token}" "$API" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/' || true)"
  else
    VERSION="$(curl -fsSL -H "User-Agent: cbm-mcp-installer" "$API" | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/' || true)"
  fi
  if [ -z "$VERSION" ]; then
    latest_url="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest" || true)"
    VERSION="$(printf '%s\n' "$latest_url" | sed -E 's#^.*/releases/tag/([^/?#]+).*$#\1#')"
    if [ -z "$VERSION" ] || [ "$VERSION" = "$latest_url" ]; then
      echo "failed to resolve the latest GitHub Release for ${REPO}" >&2
      exit 1
    fi
  fi
fi

TAG="${VERSION#v}"
BASE="https://github.com/${REPO}/releases/download/${VERSION}"
ARCHIVE="${ARTIFACT}.tar.gz"
URL="${BASE}/${ARCHIVE}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading ${URL} ..."
curl -fsSL "$URL" -o "$TMP/${ARCHIVE}"

echo "Verifying checksum ..."
curl -fsSL "${BASE}/SHA256SUMS.txt" -o "$TMP/SHA256SUMS.txt"
expected="$(grep " ${ARCHIVE}$" "$TMP/SHA256SUMS.txt" | awk '{print $1}')"
if [ -z "$expected" ]; then
  echo "checksum for ${ARCHIVE} not found in SHA256SUMS.txt" >&2
  exit 1
fi
actual="$(sha256sum "$TMP/${ARCHIVE}" | awk '{print $1}')"
if [ "$actual" != "$expected" ]; then
  echo "checksum mismatch for ${ARCHIVE}" >&2
  exit 1
fi

tar -xzf "$TMP/${ARCHIVE}" -C "$TMP"

mkdir -p "$INSTALL_DIR" "$CONFIG_DIR"
install -m 755 "$TMP/cbm" "$CONFIG_DIR/cbm"
ln -sf "$CONFIG_DIR/cbm" "$INSTALL_DIR/cbm"

if ! echo ":$PATH:" | grep -q ":${INSTALL_DIR}:"; then
  echo ""
  echo "Add to PATH: export PATH=\"${INSTALL_DIR}:\$PATH\""
fi

echo "Configuring MCP agents..."
"$CONFIG_DIR/cbm" install --yes --all

echo ""
echo "Installed cbm ${VERSION} -> ${CONFIG_DIR}/cbm"
