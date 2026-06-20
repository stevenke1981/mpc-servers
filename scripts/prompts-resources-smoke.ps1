param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,

    [string[]]$ServerArgs = @(),

    [hashtable]$ServerEnv = @{},

    [string[]]$ExpectedPrompts = @(),

    [int]$ExpectedPromptCount = 0,

    [string]$PromptName = "",

    [string]$PromptArgsJson = "{}",

    [string[]]$ExpectedResources = @(),

    [int]$ExpectedResourceCount = 0,

    [string[]]$ExpectedResourceTemplates = @(),

    [int]$ExpectedResourceTemplateCount = 0,

    [string]$ReadResourceUri = "",

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
    $null = $PromptArgsJson | ConvertFrom-Json -ErrorAction Stop
} catch {
    throw "PromptArgsJson is not valid JSON: $($_.Exception.Message)"
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

$script = Join-Path $work "prompts-resources-smoke.mjs"
$js = @'
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const binary = process.env.MCP_BINARY;
const args = JSON.parse(process.env.MCP_SERVER_ARGS_JSON || "[]");
const serverEnv = JSON.parse(process.env.MCP_SERVER_ENV_JSON || "{}");
const expectedPrompts = JSON.parse(process.env.MCP_EXPECTED_PROMPTS_JSON || "[]");
const expectedPromptCount = Number(process.env.MCP_EXPECTED_PROMPT_COUNT || "0");
const promptName = process.env.MCP_PROMPT_NAME || "";
const promptArgs = JSON.parse(process.env.MCP_PROMPT_ARGS_JSON || "{}");
const expectedResources = JSON.parse(process.env.MCP_EXPECTED_RESOURCES_JSON || "[]");
const expectedResourceCount = Number(process.env.MCP_EXPECTED_RESOURCE_COUNT || "0");
const expectedResourceTemplates = JSON.parse(process.env.MCP_EXPECTED_RESOURCE_TEMPLATES_JSON || "[]");
const expectedResourceTemplateCount = Number(process.env.MCP_EXPECTED_RESOURCE_TEMPLATE_COUNT || "0");
const readResourceUri = process.env.MCP_READ_RESOURCE_URI || "";
const timeout = Number(process.env.MCP_TIMEOUT_MS || "120000");

function requireNames(kind, actual, expected) {
  const missing = expected.filter((name) => !actual.includes(name));
  if (missing.length > 0) {
    throw new Error(`Missing expected ${kind}: ${missing.join(", ")}`);
  }
}

const transport = new StdioClientTransport({
  command: binary,
  args,
  stderr: "pipe",
  env: { ...process.env, ...serverEnv },
});
const client = new Client({ name: "mpc-servers-prompts-resources-smoke", version: "1" });

try {
  await client.connect(transport);
  const capabilities = client.getServerCapabilities();

  const promptResult = await client.listPrompts(undefined, { timeout });
  const promptNames = promptResult.prompts.map((prompt) => prompt.name);
  if (expectedPromptCount > 0 && promptNames.length !== expectedPromptCount) {
    throw new Error(`Expected ${expectedPromptCount} prompts, got ${promptNames.length}: ${promptNames.join(", ")}`);
  }
  requireNames("prompts", promptNames, expectedPrompts);

  let getPromptSummary = null;
  if (promptName) {
    const result = await client.getPrompt({ name: promptName, arguments: promptArgs }, undefined, { timeout });
    getPromptSummary = {
      description: result.description ?? null,
      messageCount: Array.isArray(result.messages) ? result.messages.length : 0,
    };
    if (!Array.isArray(result.messages) || result.messages.length === 0) {
      throw new Error(`Prompt ${promptName} returned no messages`);
    }
  }

  const resourceResult = await client.listResources(undefined, { timeout });
  const resourceUris = resourceResult.resources.map((resource) => resource.uri);
  if (expectedResourceCount > 0 && resourceUris.length !== expectedResourceCount) {
    throw new Error(`Expected ${expectedResourceCount} resources, got ${resourceUris.length}: ${resourceUris.join(", ")}`);
  }
  requireNames("resources", resourceUris, expectedResources);

  const templateResult = await client.listResourceTemplates(undefined, { timeout });
  const templateUris = templateResult.resourceTemplates.map((template) => template.uriTemplate);
  if (expectedResourceTemplateCount > 0 && templateUris.length !== expectedResourceTemplateCount) {
    throw new Error(`Expected ${expectedResourceTemplateCount} resource templates, got ${templateUris.length}: ${templateUris.join(", ")}`);
  }
  requireNames("resource templates", templateUris, expectedResourceTemplates);

  let readResourceSummary = null;
  if (readResourceUri) {
    const result = await client.readResource({ uri: readResourceUri }, undefined, { timeout });
    readResourceSummary = {
      contentCount: Array.isArray(result.contents) ? result.contents.length : 0,
      mimeTypes: Array.isArray(result.contents)
        ? result.contents.map((content) => content.mimeType ?? null)
        : [],
    };
    if (!Array.isArray(result.contents) || result.contents.length === 0) {
      throw new Error(`Resource ${readResourceUri} returned no contents`);
    }
  }

  console.log(JSON.stringify({
    ok: true,
    binary,
    args,
    envKeys: Object.keys(serverEnv).sort(),
    serverCapabilities: capabilities,
    promptCount: promptNames.length,
    prompts: promptNames,
    gotPrompt: promptName || null,
    getPrompt: getPromptSummary,
    resourceCount: resourceUris.length,
    resources: resourceUris,
    resourceTemplateCount: templateUris.length,
    resourceTemplates: templateUris,
    readResource: readResourceUri || null,
    readResourceResult: readResourceSummary,
  }, null, 2));
} catch (error) {
  console.error("MCP SDK prompts/resources smoke failed");
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
$env:MCP_EXPECTED_PROMPTS_JSON = ConvertTo-Json -InputObject @($ExpectedPrompts) -Compress
$env:MCP_EXPECTED_PROMPT_COUNT = [string]$ExpectedPromptCount
$env:MCP_PROMPT_NAME = $PromptName
$env:MCP_PROMPT_ARGS_JSON = $PromptArgsJson
$env:MCP_EXPECTED_RESOURCES_JSON = ConvertTo-Json -InputObject @($ExpectedResources) -Compress
$env:MCP_EXPECTED_RESOURCE_COUNT = [string]$ExpectedResourceCount
$env:MCP_EXPECTED_RESOURCE_TEMPLATES_JSON = ConvertTo-Json -InputObject @($ExpectedResourceTemplates) -Compress
$env:MCP_EXPECTED_RESOURCE_TEMPLATE_COUNT = [string]$ExpectedResourceTemplateCount
$env:MCP_READ_RESOURCE_URI = $ReadResourceUri
$env:MCP_TIMEOUT_MS = [string]$TimeoutMs

node $script
if ($LASTEXITCODE -ne 0) {
    throw "MCP SDK prompts/resources smoke failed"
}
