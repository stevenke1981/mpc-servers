param(
    [ValidateSet("all", "filesystem", "time", "sequential-thinking")]
    [string[]]$Server = @("all"),
    [string]$InstallDir = $(Join-Path $HOME ".config\mpc-servers\bin"),
    [switch]$Json
)

$ErrorActionPreference = "Stop"

$ServerMap = @{
    "filesystem" = "filesystem-server.exe"
    "time" = "time-server.exe"
    "sequential-thinking" = "sequential-thinking-server.exe"
}

function Resolve-Servers {
    param([string[]]$Names)
    if ($Names -contains "all") {
        return @("filesystem", "time", "sequential-thinking")
    }
    return $Names
}

$reports = @()
$selectedServers = Resolve-Servers -Names $Server

foreach ($name in $selectedServers) {
    $binary = $ServerMap[$name]
    if (-not $binary) {
        throw "Unknown server: $name"
    }

    $removed = @()
    if (Test-Path -LiteralPath $InstallDir) {
        $base = [IO.Path]::GetFileNameWithoutExtension($binary)
        $targets = @()
        $stable = Join-Path $InstallDir $binary
        if (Test-Path -LiteralPath $stable) {
            $targets += Get-Item -LiteralPath $stable
        }
        $targets += Get-ChildItem -LiteralPath $InstallDir -File -Filter "$base-*.exe" -ErrorAction SilentlyContinue

        foreach ($target in $targets | Sort-Object FullName -Unique) {
            Remove-Item -LiteralPath $target.FullName -Force
            $removed += $target.FullName
        }
    }

    $reports += [ordered]@{
        server_name = $name
        removed = $removed
        changed = ($removed.Count -gt 0)
        warnings = @("Agent configuration files were not modified.")
    }
}

if ($Json) {
    ConvertTo-Json -InputObject @($reports) -Depth 5
} else {
    foreach ($report in $reports) {
        if ($report.changed) {
            Write-Host "Removed $($report.server_name):"
            foreach ($path in $report.removed) {
                Write-Host "  $path"
            }
        } else {
            Write-Host "Nothing to remove for $($report.server_name)."
        }
    }
    Write-Host ""
    Write-Host "Codex/OpenCode/Claude configuration files were not modified."
}
