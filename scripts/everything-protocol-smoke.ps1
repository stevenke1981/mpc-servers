param(
    [Parameter(Mandatory = $true)]
    [string]$Binary,

    [string[]]$ServerArgs = @(),

    [hashtable]$ServerEnv = @{},

    [string]$SdkVersion = "1.29.0",

    [int]$TimeoutMs = 120000
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Binary)) {
    throw "Binary not found: $Binary"
}

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js is required for the MCP SDK protocol smoke test."
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm is required for installing @modelcontextprotocol/sdk."
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

$script = Join-Path $work "everything-protocol-smoke.mjs"
$js = @'
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import {
  CreateMessageRequestSchema,
  ListRootsRequestSchema,
  LoggingMessageNotificationSchema,
  ProgressNotificationSchema,
  ResourceUpdatedNotificationSchema,
} from "@modelcontextprotocol/sdk/types.js";

const binary = process.env.MCP_BINARY;
const args = JSON.parse(process.env.MCP_SERVER_ARGS_JSON || "[]");
const serverEnv = JSON.parse(process.env.MCP_SERVER_ENV_JSON || "{}");
const timeout = Number(process.env.MCP_TIMEOUT_MS || "120000");
const rootUri = process.env.MCP_ROOT_URI || "file:///D:/mpc-servers";
const subscribedUri = "demo://resource/dynamic/text/2";
const progressToken = "mpc-servers-everything-active-smoke";

const events = {
  rootsRequests: [],
  samplingRequests: [],
  progress: [],
  logs: [],
  resourceUpdated: [],
};

function firstText(result) {
  const text = result?.content?.find((item) => item?.type === "text")?.text;
  return typeof text === "string" ? text : "";
}

function requireEvent(name, predicate, details) {
  if (!predicate()) {
    throw new Error(`${name} was not observed. ${details()}`);
  }
}

const transport = new StdioClientTransport({
  command: binary,
  args,
  stderr: "pipe",
  env: { ...process.env, ...serverEnv },
});

const client = new Client(
  { name: "mpc-servers-everything-protocol-smoke", version: "1" },
  { capabilities: { roots: { listChanged: true }, sampling: {} } },
);

client.setRequestHandler(ListRootsRequestSchema, async (request) => {
  events.rootsRequests.push(request.params ?? {});
  return {
    roots: [
      {
        uri: rootUri,
        name: "mpc-servers workspace",
      },
    ],
  };
});

client.setRequestHandler(CreateMessageRequestSchema, async (request) => {
  events.samplingRequests.push({
    messageCount: Array.isArray(request.params?.messages) ? request.params.messages.length : 0,
    maxTokens: request.params?.maxTokens ?? null,
  });
  return {
    model: "mpc-servers-smoke-model",
    role: "assistant",
    content: {
      type: "text",
      text: "sampling ok from active protocol smoke",
    },
    stopReason: "endTurn",
  };
});

client.setNotificationHandler(ProgressNotificationSchema, async (notification) => {
  events.progress.push(notification.params);
});

client.setNotificationHandler(LoggingMessageNotificationSchema, async (notification) => {
  events.logs.push(notification.params);
});

client.setNotificationHandler(ResourceUpdatedNotificationSchema, async (notification) => {
  events.resourceUpdated.push(notification.params);
});

try {
  await client.connect(transport);
  const capabilities = client.getServerCapabilities();

  const rootsResult = await client.callTool({ name: "get-roots-list", arguments: {} }, undefined, { timeout });
  if (rootsResult.isError) {
    throw new Error("get-roots-list returned isError=true");
  }
  requireEvent(
    "roots/list request",
    () => events.rootsRequests.length >= 1 && firstText(rootsResult).includes(rootUri),
    () => JSON.stringify({ rootsRequests: events.rootsRequests, text: firstText(rootsResult) }, null, 2),
  );

  const samplingResult = await client.callTool({ name: "trigger-sampling-request", arguments: {} }, undefined, { timeout });
  if (samplingResult.isError) {
    throw new Error("trigger-sampling-request returned isError=true");
  }
  requireEvent(
    "sampling/createMessage request",
    () => events.samplingRequests.length >= 1 && firstText(samplingResult).includes("sampling ok from active protocol smoke"),
    () => JSON.stringify({ samplingRequests: events.samplingRequests, text: firstText(samplingResult) }, null, 2),
  );

  await client.subscribeResource({ uri: subscribedUri }, { timeout });
  const updateResult = await client.callTool({ name: "toggle-subscriber-updates", arguments: {} }, undefined, { timeout });
  if (updateResult.isError) {
    throw new Error("toggle-subscriber-updates returned isError=true");
  }
  requireEvent(
    "notifications/resources/updated",
    () => events.resourceUpdated.some((event) => event.uri === subscribedUri),
    () => JSON.stringify(events.resourceUpdated, null, 2),
  );

  await client.setLoggingLevel("debug", { timeout });
  const loggingResult = await client.callTool({ name: "toggle-simulated-logging", arguments: {} }, undefined, { timeout });
  if (loggingResult.isError) {
    throw new Error("toggle-simulated-logging returned isError=true");
  }
  requireEvent(
    "notifications/message",
    () => events.logs.length >= 1 && events.logs.some((event) => event.level === "debug"),
    () => JSON.stringify(events.logs, null, 2),
  );

  const longRunningResult = await client.callTool(
    {
      name: "trigger-long-running-operation",
      arguments: { duration: 0, steps: 2 },
      _meta: { progressToken },
    },
    undefined,
    { timeout },
  );
  if (longRunningResult.isError) {
    throw new Error("trigger-long-running-operation returned isError=true");
  }
  requireEvent(
    "notifications/progress",
    () => events.progress.filter((event) => event.progressToken === progressToken).length >= 2,
    () => JSON.stringify(events.progress, null, 2),
  );

  console.log(JSON.stringify({
    ok: true,
    binary,
    args,
    envKeys: Object.keys(serverEnv).sort(),
    serverCapabilities: capabilities,
    rootsRequestCount: events.rootsRequests.length,
    samplingRequestCount: events.samplingRequests.length,
    progressNotificationCount: events.progress.length,
    loggingNotificationCount: events.logs.length,
    resourceUpdatedNotificationCount: events.resourceUpdated.length,
    subscribedUri,
  }, null, 2));
} catch (error) {
  console.error("MCP SDK everything active protocol smoke failed");
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
$env:MCP_TIMEOUT_MS = [string]$TimeoutMs
$env:MCP_ROOT_URI = ("file:///" + ((Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path -replace "\\", "/"))

node $script
if ($LASTEXITCODE -ne 0) {
    throw "MCP SDK everything active protocol smoke failed"
}
