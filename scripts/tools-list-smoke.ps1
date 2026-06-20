param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,

    [string[]]$ServerArgs = @(),

    [hashtable]$ServerEnv = @{},

    [string[]]$ExpectedTools = @(),

    [int]$ExpectedToolCount = 0,

    [string]$CallToolName = "",

    [string]$CallToolArgsJson = "{}",

    [string]$SdkVersion = "1.29.0",

    [int]$TimeoutMs = 120000
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Binary)) {
    throw "Binary not found: $Binary"
}

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js is required for the MCP SDK smoke test."
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm is required for installing @modelcontextprotocol/sdk."
}

try {
    $null = $CallToolArgsJson | ConvertFrom-Json -ErrorAction Stop
} catch {
    throw "CallToolArgsJson is not valid JSON: $($_.Exception.Message)"
}

$work = Join-Path $env:TEMP "mpc-servers-mcp-sdk-smoke-$SdkVersion"
New-Item -ItemType Directory -Force -Path $work | Out-Null

$pkg = Join-Path $work "package.json"
if (-not (Test-Path -LiteralPath $pkg)) {
    Set-Content -LiteralPath $pkg -Encoding UTF8 -Value '{"type":"module"}'
}

$sdkDir = Join-Path $work "node_modules\@modelcontextprotocol\sdk"
$sdkPackage = Join-Path $sdkDir "package.json"
if (-not (Test-Path -LiteralPath $sdkPackage)) {
    $mutexName = "Global\mpc-servers-mcp-sdk-smoke-" + ($SdkVersion -replace '[^A-Za-z0-9_.-]', '-')
    $mutex = [System.Threading.Mutex]::new($false, $mutexName)
    $hasLock = $false
    try {
        $hasLock = $mutex.WaitOne([TimeSpan]::FromMinutes(5))
        if (-not $hasLock) {
            throw "Timed out waiting for SDK install lock: $mutexName"
        }
        if (-not (Test-Path -LiteralPath $sdkPackage)) {
            npm --prefix $work install "@modelcontextprotocol/sdk@$SdkVersion" | Out-Host
        }
    } finally {
        if ($hasLock) {
            $mutex.ReleaseMutex()
        }
        $mutex.Dispose()
    }
}

$script = Join-Path $work "tools-list-smoke.mjs"
$js = @'
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const binary = process.env.MCP_BINARY;
const args = JSON.parse(process.env.MCP_SERVER_ARGS_JSON || "[]");
const serverEnv = JSON.parse(process.env.MCP_SERVER_ENV_JSON || "{}");
const expectedTools = JSON.parse(process.env.MCP_EXPECTED_TOOLS_JSON || "[]");
const expectedToolCount = Number(process.env.MCP_EXPECTED_TOOL_COUNT || "0");
const callToolName = process.env.MCP_CALL_TOOL_NAME || "";
const callToolArgs = JSON.parse(process.env.MCP_CALL_TOOL_ARGS_JSON || "{}");
const timeout = Number(process.env.MCP_TIMEOUT_MS || "120000");

function schemaNodeBooleanIssues(schema, path = "inputSchema") {
  const issues = [];

  function visitSchema(node, nodePath) {
    if (typeof node === "boolean") {
      issues.push(`${nodePath} is a bare boolean schema node`);
      return;
    }
    if (!node || typeof node !== "object" || Array.isArray(node)) return;

    for (const key of ["properties", "$defs", "definitions", "patternProperties", "dependentSchemas"]) {
      const map = node[key];
      if (map && typeof map === "object" && !Array.isArray(map)) {
        for (const [childKey, child] of Object.entries(map)) {
          visitSchema(child, `${nodePath}.${key}.${childKey}`);
        }
      }
    }

    for (const key of [
      "items", "additionalItems", "contains", "propertyNames", "unevaluatedItems",
      "unevaluatedProperties", "additionalProperties", "not", "if", "then", "else"
    ]) {
      if (Object.prototype.hasOwnProperty.call(node, key)) {
        visitSchema(node[key], `${nodePath}.${key}`);
      }
    }

    for (const key of ["allOf", "anyOf", "oneOf", "prefixItems"]) {
      const arr = node[key];
      if (Array.isArray(arr)) {
        arr.forEach((child, index) => visitSchema(child, `${nodePath}.${key}[${index}]`));
      }
    }
  }

  visitSchema(schema, path);
  return issues;
}

const transport = new StdioClientTransport({
  command: binary,
  args,
  stderr: "pipe",
  env: { ...process.env, ...serverEnv },
});
const client = new Client({ name: "mpc-servers-tools-list-smoke", version: "1" });

try {
  await client.connect(transport);
  const capabilities = client.getServerCapabilities();
  const listResult = await client.listTools(undefined, { timeout });
  const toolNames = listResult.tools.map((tool) => tool.name);

  if (expectedToolCount > 0 && toolNames.length !== expectedToolCount) {
    throw new Error(`Expected ${expectedToolCount} tools, got ${toolNames.length}: ${toolNames.join(", ")}`);
  }

  const missing = expectedTools.filter((name) => !toolNames.includes(name));
  if (missing.length > 0) {
    throw new Error(`Missing expected tools: ${missing.join(", ")}`);
  }

  const schemaIssues = [];
  for (const tool of listResult.tools) {
    schemaIssues.push(...schemaNodeBooleanIssues(tool.inputSchema, `tools.${tool.name}.inputSchema`));
  }
  if (schemaIssues.length > 0) {
    throw new Error(`Boolean JSON Schema nodes found:\n${schemaIssues.join("\n")}`);
  }

  let callResultSummary = null;
  if (callToolName) {
    const callResult = await client.callTool({ name: callToolName, arguments: callToolArgs }, undefined, { timeout });
    callResultSummary = {
      isError: Boolean(callResult.isError),
      contentCount: Array.isArray(callResult.content) ? callResult.content.length : 0,
      structuredContent: callResult.structuredContent ?? null,
    };
    if (callResult.isError) {
      throw new Error(`Tool call returned isError=true for ${callToolName}`);
    }
  }

  console.log(JSON.stringify({
    ok: true,
    binary,
    args,
    envKeys: Object.keys(serverEnv).sort(),
    serverCapabilities: capabilities,
    toolCount: toolNames.length,
    tools: toolNames,
    calledTool: callToolName || null,
    callResult: callResultSummary,
  }, null, 2));
} catch (error) {
  console.error("MCP SDK tools/list smoke failed");
  console.error("name:", error?.name);
  console.error("message:", error?.message);
  if (error?.issues) console.error("issues:", JSON.stringify(error.issues, null, 2));
  process.exitCode = 1;
} finally {
  await client.close().catch(() => {});
}
'@
Set-Content -LiteralPath $script -Encoding UTF8 -Value $js

$env:MCP_BINARY = (Resolve-Path -LiteralPath $Binary).Path
$env:MCP_SERVER_ARGS_JSON = ConvertTo-Json -InputObject @($ServerArgs) -Compress
$env:MCP_SERVER_ENV_JSON = ConvertTo-Json -InputObject $ServerEnv -Compress
$env:MCP_EXPECTED_TOOLS_JSON = ConvertTo-Json -InputObject @($ExpectedTools) -Compress
$env:MCP_EXPECTED_TOOL_COUNT = [string]$ExpectedToolCount
$env:MCP_CALL_TOOL_NAME = $CallToolName
$env:MCP_CALL_TOOL_ARGS_JSON = $CallToolArgsJson
$env:MCP_TIMEOUT_MS = [string]$TimeoutMs

node $script
if ($LASTEXITCODE -ne 0) {
    throw "MCP SDK tools/list smoke failed"
}
