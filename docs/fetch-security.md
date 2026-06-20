# Fetch Security Policy

`fetch-server` is a production MCP server with outbound network access. It must
be useful for current public web content while avoiding accidental access to
local services, cloud metadata endpoints, private networks, or unbounded
downloads.

## Default Network Boundary

Default policy is public-web only.

Allowed by default:

- `http://` URLs
- `https://` URLs
- Hosts that resolve only to public IP addresses

Blocked by default:

- `localhost` and `*.localhost`
- IPv4 loopback, private, link-local, broadcast, multicast, documentation,
  shared carrier-grade NAT, and benchmarking ranges
- IPv6 loopback, unspecified, multicast, unique-local, link-local, and
  documentation ranges
- Any hostname that resolves to at least one blocked address
- URL schemes other than HTTP/HTTPS

Private network access can be enabled only with an explicit server startup flag:

```powershell
fetch-server.exe --allow-private-network
```

Agent configs should not enable this flag unless the user intentionally wants the
model to access internal resources.

## Redirect Policy

- Redirects are followed manually by the server.
- Default redirect limit: 5.
- Every redirect target is revalidated against the same URL and network policy.
- Relative redirects are resolved against the current URL.
- Redirect loops fail once the limit is exceeded.

## Timeout And Size Policy

- Default request timeout: 30 seconds per HTTP request.
- Default maximum response bytes: 1 MiB.
- If `Content-Length` exceeds the byte limit, the request is rejected before the
  body is read.
- While streaming the body, the server stops and errors once the byte limit is
  exceeded.
- Tool output is truncated by characters using upstream-compatible `max_length`
  and `start_index` parameters.

## User-Agent

Default autonomous tool User-Agent:

```text
ModelContextProtocol/1.0 (Autonomous; +https://github.com/modelcontextprotocol/servers)
```

It can be overridden at startup:

```powershell
fetch-server.exe --user-agent "YourAgent/1.0"
```

## Proxy Policy

- Environment proxy variables are ignored by default.
- A proxy is used only when the server is started with `--proxy-url`.
- Redirect targets are still validated even when a proxy is configured.

Example:

```powershell
fetch-server.exe --proxy-url "http://proxy.example.com:8080"
```

## HTML Extraction

- HTML responses are simplified to readable plain text / Markdown-ish text unless
  the tool input sets `raw: true`.
- Non-HTML responses are returned as raw text with a content-type prefix.
- Unsupported or invalid UTF-8 bytes are decoded lossily.

## Tool Input

The `fetch` tool keeps upstream-compatible parameters:

- `url` required
- `max_length` optional, default `5000`, valid range `1..999999`
- `start_index` optional, default `0`
- `raw` optional, default `false`

