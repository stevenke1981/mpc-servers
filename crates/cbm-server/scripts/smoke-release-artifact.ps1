# Section 6.4 / 7.1 / 7.6 — verify packaged release archive end-to-end.
#
# Usage (local developer):
#   .\scripts\smoke-release-artifact.ps1
#   .\scripts\smoke-release-artifact.ps1 -SkipBuild
#
# Usage (CI matrix — archive already packaged):
#   .\scripts\smoke-release-artifact.ps1 -SkipBuild -SkipPackage `
#     -ArtifactName cbm-mcp-windows-x64 `
#     -ArchivePath dist\cbm-mcp-windows-x64.zip

param(
    [switch]$SkipBuild,
    [switch]$SkipPackage,
    [string]$ArtifactName = "cbm-mcp-windows-x64",
    [string]$BinaryPath = "",
    [string]$ArchivePath = "",
    [switch]$SkipMcpSmoke,
    [switch]$SkipInstallDryRun
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = Split-Path -Parent $Root
Set-Location $Root

if (-not $ArchivePath) {
    $ArchivePath = Join-Path $Root "dist\$ArtifactName.zip"
}

if (-not $SkipBuild) {
    Write-Host "==> cargo build --release" -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
}

if (-not $BinaryPath) {
    $BinaryPath = Join-Path $Root "target\release\cbm.exe"
}

if (-not $SkipPackage) {
    if (-not (Test-Path $BinaryPath)) {
        throw "release binary not found: $BinaryPath"
    }
    Write-Host "==> package artifact" -ForegroundColor Cyan
    & (Join-Path $Root "scripts\package-artifact.ps1") $ArtifactName $BinaryPath
}

$Zip = $ArchivePath
$HashFile = [System.IO.Path]::ChangeExtension($Zip, ".sha256")
if (-not (Test-Path $Zip)) { throw "archive missing: $Zip" }
if (-not (Test-Path $HashFile)) { throw "checksum file missing: $HashFile" }

Write-Host "==> verify checksum" -ForegroundColor Cyan
$expected = (Get-Content $HashFile -Raw).Split()[0].ToLower()
$actual = (Get-FileHash $Zip -Algorithm SHA256).Hash.ToLower()
if ($actual -ne $expected) {
    throw "checksum mismatch (expected $expected, got $actual)"
}

$Extract = Join-Path $env:TEMP "cbm-mcp-smoke-release"
if (Test-Path $Extract) { Remove-Item $Extract -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Extract | Out-Null
Expand-Archive -Path $Zip -DestinationPath $Extract -Force

$Extracted = Join-Path $Extract "cbm.exe"
if (-not (Test-Path $Extracted)) { throw "extracted binary missing" }

function Invoke-McpSmoke([string]$Binary) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Binary
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Environment["CBRLM_WATCHER"] = "0"
    $proc = [System.Diagnostics.Process]::Start($psi)

    try {
        $init = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}'
        $proc.StandardInput.WriteLine($init)
        $proc.StandardInput.Flush()
        $initResp = $proc.StandardOutput.ReadLine()
        if ($initResp -notmatch '"result"') { throw "MCP initialize failed: $initResp" }

        $list = '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
        $proc.StandardInput.WriteLine($list)
        $proc.StandardInput.Flush()
        $listResp = $proc.StandardOutput.ReadLine()
        if ($listResp -notmatch 'index_repository') { throw "MCP tools/list missing index_repository: $listResp" }
    } finally {
        try { $proc.StandardInput.Close() } catch {}
        if (-not $proc.WaitForExit(5000)) { $proc.Kill() }
    }
}

function Invoke-NativeCapture {
    param(
        [Parameter(Mandatory=$true)][string]$Binary,
        [Parameter(Mandatory=$true)][string[]]$Arguments
    )
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Binary
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Arguments = ($Arguments | ForEach-Object {
        $argument = [string]$_
        if ($argument -notmatch '[\s"]') {
            $argument
        } else {
            $escaped = [regex]::Replace($argument, '(\\*)"', '$1$1\"')
            $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
            '"' + $escaped + '"'
        }
    }) -join ' '
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    return [pscustomobject]@{
        ExitCode = $proc.ExitCode
        Stdout = $stdout
        Stderr = $stderr
    }
}

$SmokeCache = Join-Path $env:TEMP "cbm-mcp-smoke-cache"
if (Test-Path $SmokeCache) { Remove-Item $SmokeCache -Recurse -Force }
New-Item -ItemType Directory -Force -Path $SmokeCache | Out-Null
$env:CBRLM_CACHE_DIR = $SmokeCache
$env:CBRLM_WATCHER = "0"

Write-Host "==> smoke extracted binary" -ForegroundColor Cyan
& $Extracted --version
if ($LASTEXITCODE -ne 0) { throw "cbm --version failed" }

$indexJson = '{"repo_path":".","project":"smoke-artifact","mode":"fast","persistence":false}'
$indexResult = Invoke-NativeCapture -Binary $Extracted -Arguments @('cli','index_repository','--json','--quiet',$indexJson)
if ($indexResult.ExitCode -ne 0) { throw "index_repository from extracted binary failed: $($indexResult.Stderr)" }
if ($indexResult.Stdout -notmatch '"success":true') { throw "index did not succeed: $($indexResult.Stdout)" }

if (-not $SkipInstallDryRun) {
    Write-Host "==> smoke install dry-run" -ForegroundColor Cyan
    $dryResult = Invoke-NativeCapture -Binary $Extracted -Arguments @('install','--dry-run','--all','--json')
    $dryOut = "$($dryResult.Stdout)`n$($dryResult.Stderr)"
    if ($dryResult.ExitCode -ne 0) { throw "install --dry-run failed: $dryOut" }
    if ($dryOut -notmatch '\[dry-run\]') { throw "install dry-run produced no dry-run markers" }
    $null = $dryResult.Stdout | ConvertFrom-Json
}

if (-not $SkipMcpSmoke) {
    Write-Host "==> smoke MCP initialize + tools/list" -ForegroundColor Cyan
    Invoke-McpSmoke $Extracted
}

Write-Host "Release artifact smoke passed." -ForegroundColor Green
