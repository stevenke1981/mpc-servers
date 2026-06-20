//! Process-level MCP stdio smoke: spawn binary, initialize, tools/list snapshot, tools/call.

mod support;

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn expected_tool_names_from_specs() -> BTreeSet<String> {
    let dir = repo_root().join("mcps/codebase-memory-mcp/tools");
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(dir).expect("mcps tool specs directory missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("rlm_"))
        {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&raw).unwrap();
        names.insert(
            value
                .get("name")
                .and_then(|v| v.as_str())
                .expect("spec missing name")
                .to_string(),
        );
    }
    names
}

fn write_mcp_frame(stdin: &mut ChildStdin, body: &str) {
    writeln!(stdin, "{body}").expect("write JSON-RPC line");
    stdin.flush().expect("flush stdin");
}

fn read_mcp_frame(reader: &mut impl BufRead) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).expect("read JSON-RPC line");
    line.trim_end().to_string()
}

fn spawn_mcp_process(cache_dir: &Path) -> Child {
    let bin = cargo_bin("cbm");
    Command::new(bin)
        .env("CBM_WATCHER", "0")
        .env("CBRLM_WATCHER", "0")
        .env("CBM_CACHE_DIR", cache_dir)
        .env("CBRLM_CACHE_DIR", cache_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn MCP process")
}

fn parse_response(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("invalid JSON-RPC response: {e}\n{body}"))
}

#[test]
fn mcp_process_initialize_tools_list_snapshot_and_call() {
    let (_guard, _cache, cache_path) = support::isolated_cache();
    let mut child = spawn_mcp_process(&cache_path);
    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = std::io::BufReader::new(stdout);

    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "mcp-process-test", "version": "1" }
        }
    });
    write_mcp_frame(&mut stdin, &init.to_string());
    let init_resp = parse_response(&read_mcp_frame(&mut reader));
    assert_eq!(init_resp.get("id"), Some(&Value::from(1)));
    let result = init_resp.get("result").expect("initialize result");
    assert_eq!(
        result.pointer("/serverInfo/name").and_then(|v| v.as_str()),
        Some("codebase-memory-mcp")
    );

    let list = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    write_mcp_frame(&mut stdin, &list.to_string());
    let list_resp = parse_response(&read_mcp_frame(&mut reader));
    assert_eq!(list_resp.get("id"), Some(&Value::from(2)));
    let tools = list_resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools array");
    let runtime_names: BTreeSet<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    let expected = expected_tool_names_from_specs();
    assert_eq!(
        runtime_names, expected,
        "tools/list name set must match checked-in mcps specs"
    );
    assert!(
        !runtime_names.iter().any(|n| n.starts_with("rlm_")),
        "graph server must not advertise RLM tools"
    );

    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "list_projects",
            "arguments": {}
        }
    });
    write_mcp_frame(&mut stdin, &call.to_string());
    let call_resp = parse_response(&read_mcp_frame(&mut reader));
    assert_eq!(call_resp.get("id"), Some(&Value::from(3)));
    let content_text = call_resp
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .expect("tools/call text content");
    let payload: Value = serde_json::from_str(content_text).expect("parse tool result JSON");
    assert!(payload.get("projects").is_some());

    drop(stdin);
    let status = child.wait().expect("wait child");
    assert!(status.success() || status.code() == Some(0));
}
