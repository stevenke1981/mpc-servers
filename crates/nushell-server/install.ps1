param(
    [string]$Version = "latest",
    [switch]$FromSource,
    [switch]$SkipBuild,
    [switch]$Json,
    [string]$InstallDir = "$env:USERPROFILE\.config\nushell-mcp\bin"
)

$ErrorActionPreference = "Stop"
$Repo = "stevenke1981/nushell-mcp"
$Name = "nushell-mcp"
$ExeName = "nushell-mcp.exe"
$StablePath = Join-Path $InstallDir $ExeName

function Write-Report($Report) {
    if ($Json) {
        $Report | ConvertTo-Json -Depth 20
    } else {
        if ($Report.ok) {
            "Installed $($Report.name) $($Report.version)"
            "Path: $($Report.installedPath)"
            "Source: $($Report.source)"
            if ($Report.restartRequired) { "Restart MCP clients to use the new binary." }
        } else {
            throw $Report.error
        }
    }
}

function Resolve-LatestTag {
    $headers = @{ "User-Agent" = "nushell-mcp-installer" }
    $token = $env:GITHUB_TOKEN
    if (-not $token) { $token = $env:GH_TOKEN }
    if ($token) { $headers["Authorization"] = "Bearer $token" }

    try {
        $release = Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/$Repo/releases/latest"
        if ($release.tag_name) { return $release.tag_name }
    } catch {
        # Fall back to public redirect below.
    }

    $response = Invoke-WebRequest -MaximumRedirection 0 -ErrorAction SilentlyContinue -Uri "https://github.com/$Repo/releases/latest"
    $location = $response.Headers.Location
    if (-not $location) { throw "Could not resolve latest release tag for $Repo" }
    if ($location -match "/releases/tag/([^/]+)$") { return $Matches[1] }
    throw "Could not parse latest release redirect: $location"
}

function Copy-WithLockedFallback($Source, $VersionText) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    try {
        Copy-Item -LiteralPath $Source -Destination $StablePath -Force
        return @{ path = (Resolve-Path -LiteralPath $StablePath).Path; restartRequired = $true }
    } catch {
        $sideBySide = Join-Path $InstallDir "$Name-$VersionText.exe"
        Copy-Item -LiteralPath $Source -Destination $sideBySide -Force
        return @{ path = (Resolve-Path -LiteralPath $sideBySide).Path; restartRequired = $true }
    }
}

try {
    if ($FromSource) {
        if (-not $SkipBuild) {
            & "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release
            if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
        }
        $source = Join-Path $PSScriptRoot "target\release\$ExeName"
        if (-not (Test-Path -LiteralPath $source)) {
            throw "Source binary not found: $source"
        }
        $versionText = (& $source --version).Trim() -replace "^nushell-mcp\s+", ""
        $copy = Copy-WithLockedFallback -Source $source -VersionText $versionText
        Write-Report @{
            ok = $true
            name = $Name
            version = $versionText
            installedPath = $copy.path
            source = "from-source"
            restartRequired = $copy.restartRequired
        }
        exit 0
    }

    $tag = if ($Version -eq "latest") { Resolve-LatestTag } else { $Version }
    $assetBase = "$Name-$tag-x86_64-pc-windows-msvc.zip"
    $assetUrl = "https://github.com/$Repo/releases/download/$tag/$assetBase"
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "$Name-install-$([System.Guid]::NewGuid())"
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    $zipPath = Join-Path $tempRoot $assetBase
    Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath
    Expand-Archive -LiteralPath $zipPath -DestinationPath $tempRoot -Force
    $source = Get-ChildItem -LiteralPath $tempRoot -Recurse -Filter $ExeName | Select-Object -First 1
    if (-not $source) { throw "Release asset did not contain $ExeName" }
    $copy = Copy-WithLockedFallback -Source $source.FullName -VersionText $tag.TrimStart("v")
    Write-Report @{
        ok = $true
        name = $Name
        version = $tag
        installedPath = $copy.path
        source = $assetUrl
        restartRequired = $copy.restartRequired
    }
} catch {
    Write-Report @{
        ok = $false
        name = $Name
        version = $Version
        installedPath = $null
        source = if ($FromSource) { "from-source" } else { "github-release" }
        restartRequired = $false
        error = $_.Exception.Message
    }
    exit 1
}
