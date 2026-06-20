param(
    [ValidateSet("all", "cbm", "everything", "filesystem", "fetch", "git", "memory", "nushell", "rlm", "time", "sequential-thinking")]
    [string[]]$Server = @("all"),

    [switch]$SkipCargo,

    [switch]$SkipBuild,

    [string]$ExpectedVersion = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$SmokeScript = Join-Path $RepoRoot "scripts\tools-list-smoke.ps1"
$PromptsResourcesSmokeScript = Join-Path $RepoRoot "scripts\prompts-resources-smoke.ps1"
$EverythingProtocolSmokeScript = Join-Path $RepoRoot "scripts\everything-protocol-smoke.ps1"

function Resolve-Servers {
    param([string[]]$Names)
    if ($Names -contains "all") {
        return @("cbm", "everything", "filesystem", "fetch", "git", "memory", "nushell", "rlm", "time", "sequential-thinking")
    }
    return $Names
}

function Get-ServerInfo {
    param([string]$Name)
    switch ($Name) {
        "cbm" {
            return @{
                Package = "codebase-memory-mcp"
                Binary = "cbm.exe"
                Version = "cbm 0.2.3"
                ToolCount = 14
                Tools = @("index_repository", "index_status", "search_graph", "query_graph", "list_projects")
                CallToolName = "list_projects"
                CallToolArgsJson = "{}"
            }
        }
        "filesystem" {
            return @{
                Package = "filesystem-server"
                Binary = "filesystem-server.exe"
                Version = "0.1.0"
                ToolCount = 14
                Tools = @("read_text_file", "write_file", "list_allowed_directories")
                CallToolName = "list_allowed_directories"
                CallToolArgsJson = "{}"
            }
        }
        "everything" {
            return @{
                Package = "everything-server"
                Binary = "everything-server.exe"
                Version = "0.1.0"
                ToolCount = 19
                Tools = @("echo", "get-sum", "get-structured-content", "get-tiny-image", "gzip-file-as-resource", "trigger-sampling-request")
                CallToolName = "echo"
                CallToolArgsJson = '{"message":"release smoke"}'
            }
        }
        "fetch" {
            return @{
                Package = "fetch-server"
                Binary = "fetch-server.exe"
                Version = "0.1.0"
                ToolCount = 1
                Tools = @("fetch")
                CallToolName = ""
                CallToolArgsJson = "{}"
            }
        }
        "git" {
            return @{
                Package = "git-server"
                Binary = "git-server.exe"
                Version = "0.1.0"
                ToolCount = 12
                Tools = @("git_status", "git_diff", "git_commit", "git_branch")
                CallToolName = "git_status"
                CallToolArgsJson = $null
            }
        }
        "memory" {
            return @{
                Package = "memory-mcp-server"
                Binary = "memory-mcp-server.exe"
                Version = "opencode-memory v0.1.0"
                ToolCount = 7
                Tools = @("add_memory", "search_memories", "get_memories", "delete_memory", "consolidate_memories", "get_memory_stats", "end_session")
                CallToolName = "get_memory_stats"
                CallToolArgsJson = "{}"
            }
        }
        "nushell" {
            return @{
                Package = "nushell-mcp"
                Binary = "nushell-mcp.exe"
                Version = "nushell-mcp 0.1.0"
                ToolCount = 15
                Tools = @("nu_version", "nu_eval", "nu_script", "git_status", "git_precommit_review", "nu_ls")
                CallToolName = "nu_version"
                CallToolArgsJson = $null
            }
        }
        "rlm" {
            return @{
                Package = "rlm-mcp"
                Binary = "rlm-mcp.exe"
                Version = "rlm-mcp 0.1.6"
                ToolCount = 33
                Tools = @("rlm_scan", "rlm_peek", "rlm_chunk", "rlm_reduce_merge", "rlm_repl_info")
                CallToolName = "rlm_repl_info"
                CallToolArgsJson = "{}"
            }
        }
        "time" {
            return @{
                Package = "time-server"
                Binary = "time-server.exe"
                Version = "0.1.0"
                ToolCount = 2
                Tools = @("get_current_time", "convert_time")
                CallToolName = "get_current_time"
                CallToolArgsJson = '{"timezone":"Asia/Taipei"}'
            }
        }
        "sequential-thinking" {
            return @{
                Package = "sequential-thinking-server"
                Binary = "sequential-thinking-server.exe"
                Version = "0.1.0"
                ToolCount = 1
                Tools = @("sequentialthinking")
                CallToolName = "sequentialthinking"
                CallToolArgsJson = '{"thought":"release smoke","thoughtNumber":1,"totalThoughts":1,"nextThoughtNeeded":false}'
            }
        }
        default {
            throw "Unknown server: $Name"
        }
    }
}

function Invoke-Step {
    param(
        [string]$StepName,
        [scriptblock]$Body
    )
    Write-Host ""
    Write-Host "==> $StepName" -ForegroundColor Cyan
    & $Body
}

Push-Location $RepoRoot
try {
    $selected = Resolve-Servers -Names $Server

    if (-not $SkipCargo) {
        Invoke-Step "cargo fmt --check" { cargo fmt --check }
        Invoke-Step "cargo test --all-targets" { cargo test --all-targets }
        Invoke-Step "cargo clippy --all-targets -- -D warnings" { cargo clippy --all-targets -- -D warnings }
    }

    if (-not $SkipBuild) {
        Invoke-Step "cargo build --release" { cargo build --release }
    }

    $results = @()
    foreach ($serverName in $selected) {
        $info = Get-ServerInfo -Name $serverName
        $binary = Join-Path $RepoRoot ("target\release\" + $info.Binary)
        if (-not (Test-Path -LiteralPath $binary)) {
            throw "Release binary not found: $binary"
        }

        Invoke-Step "$serverName --version" {
            $version = (& $binary --version).Trim()
            $expected = if ($ExpectedVersion) { $ExpectedVersion } else { $info.Version }
            if ($version -ne $expected) {
                throw "$serverName version mismatch: expected $expected, got $version"
            }
            Write-Host "$serverName version $version"
        }

        $tempPath = $null
        $serverArgs = @()
        $serverEnv = @{}
        $callArgs = $info.CallToolArgsJson
        $savedEnv = @{}
        try {
            if ($serverName -eq "filesystem") {
                $tempPath = Join-Path ([IO.Path]::GetTempPath()) ("mpc-release-fs-" + [guid]::NewGuid())
                New-Item -ItemType Directory -Force -Path $tempPath | Out-Null
                Set-Content -LiteralPath (Join-Path $tempPath "hello.txt") -Value "release smoke"
                $serverArgs = @($tempPath)
            } elseif ($serverName -eq "git") {
                $tempPath = Join-Path ([IO.Path]::GetTempPath()) ("mpc-release-git-" + [guid]::NewGuid())
                New-Item -ItemType Directory -Force -Path $tempPath | Out-Null
                git -C $tempPath init | Out-Null
                git -C $tempPath config user.email test@example.com | Out-Null
                git -C $tempPath config user.name "Test User" | Out-Null
                Set-Content -LiteralPath (Join-Path $tempPath "README.md") -Value "release smoke" -NoNewline
                git -C $tempPath add README.md | Out-Null
                git -C $tempPath commit -m "initial commit" | Out-Null
                $serverArgs = @("--repository", $tempPath)
                $callArgs = @{ repo_path = $tempPath } | ConvertTo-Json -Compress
            } elseif ($serverName -eq "memory") {
                $tempPath = Join-Path ([IO.Path]::GetTempPath()) ("mpc-release-memory-" + [guid]::NewGuid())
                New-Item -ItemType Directory -Force -Path $tempPath | Out-Null
                $serverEnv = @{
                    MEMORY_DB_PATH = Join-Path $tempPath "memory.db"
                    MEMORY_VECTOR_PATH = Join-Path $tempPath "vectors.usearch"
                    MEMORY_TANTIVY_PATH = Join-Path $tempPath "tantivy"
                    LLM_API_KEY = "mock"
                    LLM_API_BASE = "mock"
                }
            } elseif ($serverName -eq "nushell") {
                $tempPath = Join-Path ([IO.Path]::GetTempPath()) ("mpc-release-nu-" + [guid]::NewGuid())
                New-Item -ItemType Directory -Force -Path $tempPath | Out-Null
                $fakeNu = Join-Path $tempPath "fake-nu.cmd"
                Set-Content -LiteralPath $fakeNu -Value @'
@echo off
if "%~1"=="--version" (
  echo 0.100.0
  exit /b 0
)
echo unsupported fake nu call 1>&2
exit /b 2
'@
                $callArgs = @{ nu_path = $fakeNu } | ConvertTo-Json -Compress
            }

            Invoke-Step "$serverName MCP SDK smoke" {
                & $SmokeScript `
                    -Binary $binary `
                    -ServerArgs $serverArgs `
                    -ServerEnv $serverEnv `
                    -ExpectedToolCount $info.ToolCount `
                    -ExpectedTools $info.Tools `
                    -CallToolName $info.CallToolName `
                    -CallToolArgsJson $callArgs | Out-Host
            }

            if ($serverName -eq "everything") {
                Invoke-Step "$serverName gzip SDK smoke" {
                    & $SmokeScript `
                        -Binary $binary `
                        -ExpectedToolCount $info.ToolCount `
                        -ExpectedTools @("gzip-file-as-resource") `
                        -CallToolName "gzip-file-as-resource" `
                        -CallToolArgsJson '{"name":"release-smoke.txt.gz","data":"data:text/plain;base64,aGVsbG8gZnJvbSByZWxlYXNlIHNtb2tl","outputType":"resource"}' | Out-Host
                }

                Invoke-Step "$serverName protocol fallback SDK smoke" {
                    $protocolCalls = @(
                        @{ Name = "get-roots-list"; Args = "{}" },
                        @{ Name = "trigger-long-running-operation"; Args = '{"duration":0,"steps":1}' },
                        @{ Name = "toggle-simulated-logging"; Args = "{}" },
                        @{ Name = "toggle-subscriber-updates"; Args = "{}" },
                        @{ Name = "trigger-sampling-request"; Args = "{}" },
                        @{ Name = "trigger-elicitation-request"; Args = "{}" }
                    )
                    foreach ($call in $protocolCalls) {
                        & $SmokeScript `
                            -Binary $binary `
                            -ExpectedToolCount $info.ToolCount `
                            -ExpectedTools @($call.Name) `
                            -CallToolName $call.Name `
                            -CallToolArgsJson $call.Args | Out-Host
                    }
                }

                Invoke-Step "$serverName active protocol SDK smoke" {
                    & $EverythingProtocolSmokeScript `
                        -Binary $binary | Out-Host
                }

                Invoke-Step "$serverName prompts/resources SDK smoke" {
                    & $PromptsResourcesSmokeScript `
                        -Binary $binary `
                        -ExpectedPromptCount 4 `
                        -ExpectedPrompts @("simple-prompt", "args-prompt", "completable-prompt", "resource-prompt") `
                        -PromptName "resource-prompt" `
                        -PromptArgsJson '{"resourceType":"Text","resourceId":"2"}' `
                        -ExpectedResourceCount 3 `
                        -ExpectedResources @(
                            "demo://resource/static/document/instructions.md",
                            "demo://resource/static/document/features.md",
                            "demo://resource/static/document/startup.md"
                        ) `
                        -ExpectedResourceTemplateCount 2 `
                        -ExpectedResourceTemplates @(
                            "demo://resource/dynamic/text/{resourceId}",
                            "demo://resource/dynamic/blob/{resourceId}"
                        ) `
                        -ReadResourceUri "demo://resource/dynamic/text/2" | Out-Host
                }
            }
        } finally {
            foreach ($entry in $savedEnv.GetEnumerator()) {
                [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, "Process")
            }
            if ($tempPath) {
                Remove-Item -LiteralPath $tempPath -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        $results += [ordered]@{
            server = $serverName
            binary = $binary
            version = if ($ExpectedVersion) { $ExpectedVersion } else { $info.Version }
            tool_count = $info.ToolCount
        }
    }

    Invoke-Step "installer report schema example" {
        $ok = Test-Json `
            -Json (Get-Content -Raw -LiteralPath (Join-Path $RepoRoot "packaging\install-report.example.json")) `
            -Schema (Get-Content -Raw -LiteralPath (Join-Path $RepoRoot "packaging\install-report.schema.json"))
        if (-not $ok) {
            throw "install-report.example.json does not match schema"
        }
        Write-Host "install-report.example.json matches schema"
    }

    Write-Host ""
    Write-Host "Release check passed." -ForegroundColor Green
    $results | ConvertTo-Json -Depth 5
} finally {
    Pop-Location
}
