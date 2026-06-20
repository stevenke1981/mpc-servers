# Deferred Packaging Channels

codebase-memory-mcp currently ships via **install scripts**, **GitHub Releases** (multi-platform binaries + `SHA256SUMS.txt`), **Homebrew**, **Scoop**, and **Winget**.

The reference `cbm-mcp` project also lists these channels. They are **intentionally deferred** for the Rust rewrite until wrapper maintenance is automated:

| Channel | Status | Notes |
|---------|--------|-------|
| Go wrapper | Deferred | Thin `main` calling `codebase-memory-mcp` binary |
| Python / PyPI | Deferred | `pip install codebase-memory-mcp` shim package |
| npm | Deferred | `@codebase-memory-mcp/cli` postinstall binary fetch |
| Chocolatey | Deferred | Windows package manager |
| AUR (Arch) | Deferred | `codebase-memory-mcp-bin` PKGBUILD |
| Glama MCP registry | Deferred | Metadata publish after stable API |

Supported today:

- `packaging/windows/install.ps1` — checksum-verified download
- `packaging/linux/install.sh` — checksum-verified download
- `packaging/macos/install.sh`
- `packaging/homebrew/codebase-memory-mcp.rb`
- `packaging/scoop/codebase-memory-mcp.json`
- `packaging/winget/codebase-memory-mcp.yaml`

Release hashes are generated in `.github/workflows/release.yml` as `SHA256SUMS.txt`.
