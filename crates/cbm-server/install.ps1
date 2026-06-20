# Install codebase-memory-mcp for agents from GitHub Release by default (Windows).
#
# Usage:
#   .\install.ps1
#   .\install.ps1 -Version v0.1.0
#   .\install.ps1 -FromSource -SkipBuild -AllAgents

param(
    [string]$Version = $(if ($env:CBM_VERSION) { $env:CBM_VERSION } elseif ($env:CBRLM_VERSION) { $env:CBRLM_VERSION } else { "latest" }),
    [switch]$FromSource,
    [switch]$SkipBuild,
    [switch]$AllAgents
)

$ErrorActionPreference = "Stop"

if ($FromSource) {
    $Script = Join-Path $PSScriptRoot "scripts\install.ps1"
    & $Script -SkipBuild:$SkipBuild -AllAgents:$AllAgents
    exit $LASTEXITCODE
}

$Script = Join-Path $PSScriptRoot "packaging\windows\install.ps1"
& $Script -Version $Version
