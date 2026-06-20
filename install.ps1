param(
    [ValidateSet("all", "cbm", "everything", "filesystem", "fetch", "git", "memory", "nushell", "rlm", "time", "sequential-thinking")]
    [string[]]$Server = @("all"),
    [string]$Version = $(if ($env:MPC_SERVERS_VERSION) { $env:MPC_SERVERS_VERSION } else { "latest" }),
    [string]$InstallDir = $(Join-Path $HOME ".config\mpc-servers\bin"),
    [switch]$FromSource,
    [switch]$SkipBuild,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

$Repo = "stevenke1981/mpc-servers"
$InstallRoot = (Resolve-Path -LiteralPath (New-Item -ItemType Directory -Force -Path $InstallDir)).Path
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

$ServerMap = @{
    "cbm" = @{ Package = "codebase-memory-mcp"; Binary = "cbm.exe" }
    "everything" = @{ Package = "everything-server"; Binary = "everything-server.exe" }
    "filesystem" = @{ Package = "filesystem-server"; Binary = "filesystem-server.exe" }
    "fetch" = @{ Package = "fetch-server"; Binary = "fetch-server.exe" }
    "git" = @{ Package = "git-server"; Binary = "git-server.exe" }
    "memory" = @{ Package = "memory-mcp-server"; Binary = "memory-mcp-server.exe" }
    "nushell" = @{ Package = "nushell-mcp"; Binary = "nushell-mcp.exe" }
    "rlm" = @{ Package = "rlm-mcp"; Binary = "rlm-mcp.exe" }
    "time" = @{ Package = "time-server"; Binary = "time-server.exe" }
    "sequential-thinking" = @{ Package = "sequential-thinking-server"; Binary = "sequential-thinking-server.exe" }
}

function Resolve-Servers {
    param([string[]]$Names)
    if ($Names -contains "all") {
        return @("cbm", "everything", "filesystem", "fetch", "git", "memory", "nushell", "rlm", "time", "sequential-thinking")
    }
    return $Names
}

function Get-TargetTriple {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        "Arm64" { return "windows-aarch64" }
        default { return "windows-x86_64" }
    }
}

function Get-ReleaseUrl {
    param([string]$Version, [string]$Asset)
    if ($Version -eq "latest") {
        return "https://github.com/$Repo/releases/latest/download/$Asset"
    }
    return "https://github.com/$Repo/releases/download/$Version/$Asset"
}

function Copy-WithLockedFallback {
    param(
        [string]$SourcePath,
        [string]$BinaryName,
        [string]$VersionLabel
    )
    $stablePath = Join-Path $InstallRoot $BinaryName
    $changed = $true
    try {
        Copy-Item -LiteralPath $SourcePath -Destination $stablePath -Force
        return @{ Path = $stablePath; Changed = $changed; Warning = $null }
    } catch {
        $safeVersion = ($VersionLabel -replace '[^A-Za-z0-9_.-]', '-')
        $stamp = Get-Date -Format "yyyyMMddHHmmss"
        $fallbackName = [IO.Path]::GetFileNameWithoutExtension($BinaryName) + "-$safeVersion-$stamp.exe"
        $fallbackPath = Join-Path $InstallRoot $fallbackName
        Copy-Item -LiteralPath $SourcePath -Destination $fallbackPath -Force
        return @{
            Path = $fallbackPath
            Changed = $changed
            Warning = "Stable binary was locked; installed side-by-side binary instead: $fallbackPath"
        }
    }
}

function Download-ReleaseBundle {
    $triple = Get-TargetTriple
    $asset = "mpc-servers-$triple.zip"
    $url = Get-ReleaseUrl -Version $Version -Asset $asset
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("mpc-servers-install-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    $zip = Join-Path $tmp $asset
    try {
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    } catch {
        throw "Failed to download $url. Release assets may not exist yet; use -FromSource for local development installs."
    }
    Expand-Archive -LiteralPath $zip -DestinationPath $tmp -Force
    return $tmp
}

function Find-BinaryInBundle {
    param([string]$BundleDir, [string]$BinaryName)
    $match = Get-ChildItem -LiteralPath $BundleDir -Recurse -File -Filter $BinaryName | Select-Object -First 1
    if (-not $match) {
        throw "Release bundle did not contain $BinaryName"
    }
    return $match.FullName
}

$selectedServers = Resolve-Servers -Names $Server
$bundleDir = $null
if (-not $FromSource) {
    $bundleDir = Download-ReleaseBundle
}

$reports = @()
foreach ($name in $selectedServers) {
    $entry = $ServerMap[$name]
    if (-not $entry) {
        throw "Unknown server: $name"
    }

    if ($FromSource) {
        if (-not $SkipBuild) {
            Push-Location $ScriptRoot
            try {
                & cargo build --release -p $entry.Package
                if ($LASTEXITCODE -ne 0) {
                    throw "cargo build failed for $($entry.Package)"
                }
            } finally {
                Pop-Location
            }
        }
        $sourcePath = Join-Path $ScriptRoot "target\release\$($entry.Binary)"
        if (-not (Test-Path -LiteralPath $sourcePath)) {
            throw "Built binary not found: $sourcePath"
        }
    } else {
        $sourcePath = Find-BinaryInBundle -BundleDir $bundleDir -BinaryName $entry.Binary
    }

    $installed = Copy-WithLockedFallback -SourcePath $sourcePath -BinaryName $entry.Binary -VersionLabel $Version
    $versionOutput = & $installed.Path --version
    $warnings = @()
    if ($installed.Warning) { $warnings += $installed.Warning }

    $reports += [ordered]@{
        server_name = $name
        version = $versionOutput.Trim()
        installed_exe = $installed.Path
        config_targets = @("codex", "opencode", "claude")
        changed = $installed.Changed
        warnings = $warnings
    }
}

if ($Json) {
    ConvertTo-Json -InputObject @($reports) -Depth 5
} else {
    foreach ($report in $reports) {
        Write-Host "Installed $($report.server_name) $($report.version): $($report.installed_exe)"
        foreach ($warning in $report.warnings) {
            Write-Warning $warning
        }
    }
    Write-Host ""
    Write-Host "Use the installed_exe path in Codex/OpenCode/Claude config. Do not point agents at target\release."
}
