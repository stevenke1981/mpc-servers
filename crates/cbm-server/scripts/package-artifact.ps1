# Package a built codebase-memory-mcp binary into a release zip.
# Usage: .\scripts\package-artifact.ps1 <artifact-name> <binary-path>

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$ArtifactName,

    [Parameter(Mandatory = $true, Position = 1)]
    [string]$BinaryPath
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Dist = Join-Path $Root "dist"
$Stage = Join-Path $Dist "stage-$ArtifactName"

if (-not (Test-Path $BinaryPath)) {
    throw "Binary not found: $BinaryPath"
}

if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item $BinaryPath $Stage
$License = Join-Path $Root "LICENSE"
if (Test-Path $License) {
    Copy-Item $License $Stage
}
$Readme = Join-Path $Root "README.md"
if (Test-Path $Readme) {
    Copy-Item $Readme $Stage
}
$McpTemplates = Join-Path $Root "packaging\mcp"
if (Test-Path $McpTemplates) {
    Copy-Item $McpTemplates (Join-Path $Stage "mcp-templates") -Recurse
}
Set-Content -Path (Join-Path $Stage "RELEASE.txt") -Value "$ArtifactName"

New-Item -ItemType Directory -Force -Path $Dist | Out-Null
$ZipPath = Join-Path $Dist "$ArtifactName.zip"
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $ZipPath -Force
Remove-Item $Stage -Recurse -Force

$Hash = (Get-FileHash $ZipPath -Algorithm SHA256).Hash.ToLower()
Set-Content -Path (Join-Path $Dist "$ArtifactName.sha256") -Value "$Hash  $ArtifactName.zip" -NoNewline

Write-Host "Created $ZipPath"
