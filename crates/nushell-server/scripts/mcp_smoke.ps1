param(
    [string]$Binary = "$PSScriptRoot\..\target\release\nushell-mcp.exe",
    [string]$NuPath = ""
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Binary)) {
    throw "Binary not found: $Binary"
}

if (-not $NuPath) {
    $NuPath = $env:NUSHELL_MCP_NU_PATH
}

$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = (Resolve-Path -LiteralPath $Binary).Path
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$process = [System.Diagnostics.Process]::Start($psi)

function Send-JsonLine($Object) {
    $line = ($Object | ConvertTo-Json -Depth 20 -Compress)
    $process.StandardInput.WriteLine($line)
    $process.StandardInput.Flush()
}

function Read-JsonLine {
    $line = $process.StandardOutput.ReadLine()
    if (-not $line) {
        throw "Server closed stdout. stderr: $($process.StandardError.ReadToEnd())"
    }
    return $line | ConvertFrom-Json
}

try {
    Send-JsonLine @{
        jsonrpc = "2.0"
        id = 1
        method = "initialize"
        params = @{
            protocolVersion = "2025-11-25"
            capabilities = @{}
            clientInfo = @{ name = "manual-smoke"; version = "0.1.0" }
        }
    }
    $init = Read-JsonLine
    if ($init.result.serverInfo.name -ne "nushell-mcp") {
        throw "Unexpected server name: $($init | ConvertTo-Json -Depth 20)"
    }

    Send-JsonLine @{ jsonrpc = "2.0"; method = "notifications/initialized"; params = @{} }
    Send-JsonLine @{ jsonrpc = "2.0"; id = 2; method = "tools/list"; params = @{} }
    $tools = Read-JsonLine
    $names = @($tools.result.tools | ForEach-Object { $_.name } | Sort-Object)
    $expected = "git_branch,git_commit,git_diff,git_log,git_precommit_review,git_stash,git_status,git_tree,nu_eval,nu_find,nu_grep,nu_ls,nu_read,nu_script,nu_version"
    if (($names -join ",") -ne $expected) {
        throw "Unexpected tools: $($names -join ',')"
    }

    if ($NuPath) {
        Send-JsonLine @{
            jsonrpc = "2.0"
            id = 3
            method = "tools/call"
            params = @{
                name = "nu_version"
                arguments = @{ nu_path = $NuPath }
            }
        }
        $call = Read-JsonLine
        $call.result.content[0].text
    }

    "MCP smoke passed: $($names -join ', ')"
}
finally {
    if (-not $process.HasExited) {
        $process.Kill()
    }
}
