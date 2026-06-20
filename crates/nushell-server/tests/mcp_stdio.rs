use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
};

use serde_json::{Value, json};

fn fake_nu(dir: &Path) -> PathBuf {
    let path = if cfg!(windows) {
        dir.join("fake-nu.cmd")
    } else {
        dir.join("fake-nu")
    };

    if cfg!(windows) {
        fs::write(
            &path,
            r#"@echo off
if "%~1"=="--version" (
  echo 0.100.0
  exit /b 0
)
echo unsupported fake nu call 1>&2
exit /b 2
"#,
        )
        .unwrap();
    } else {
        fs::write(
            &path,
            r#"#!/usr/bin/env sh
if [ "$1" = "--version" ]; then
  echo "0.100.0"
  exit 0
fi
echo "unsupported fake nu call" >&2
exit 2
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }
    }

    path
}

fn read_response(stdout: &mut BufReader<std::process::ChildStdout>) -> Value {
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(!line.trim().is_empty(), "server closed stdout");
    serde_json::from_str(&line)
        .unwrap_or_else(|error| panic!("invalid JSON line {line:?}: {error}"))
}

fn send(stdin: &mut ChildStdin, message: Value) {
    writeln!(stdin, "{message}").unwrap();
    stdin.flush().unwrap();
}

fn spawn_server() -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    let exe = env!("CARGO_BIN_EXE_nushell-mcp");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    (child, stdin, stdout)
}

#[test]
fn stdio_initialize_lists_tools_and_calls_version() {
    let temp = tempfile::tempdir().unwrap();
    let nu_path = fake_nu(temp.path()).to_string_lossy().to_string();
    let (mut child, mut stdin, mut stdout) = spawn_server();

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "nushell-mcp-test",
                    "version": "0.1.0"
                }
            }
        }),
    );
    let initialized = read_response(&mut stdout);
    assert_eq!(initialized["id"], 1);
    assert_eq!(initialized["result"]["serverInfo"]["name"], "nushell-mcp");

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    );

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    );
    let listed = read_response(&mut stdout);
    assert_eq!(listed["id"], 2);
    assert_no_boolean_schema_nodes(&listed["result"]["tools"]);
    let mut names = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        vec![
            "git_branch",
            "git_commit",
            "git_diff",
            "git_log",
            "git_precommit_review",
            "git_stash",
            "git_status",
            "git_tree",
            "nu_eval",
            "nu_find",
            "nu_grep",
            "nu_ls",
            "nu_read",
            "nu_script",
            "nu_version"
        ]
    );

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "nu_version",
                "arguments": {
                    "nu_path": nu_path
                }
            }
        }),
    );
    let called = read_response(&mut stdout);
    assert_eq!(called["id"], 3);
    let text = called["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("0.100.0"), "{text}");

    drop(stdin);
    child.kill().ok();
    child.wait().ok();
}

fn assert_no_boolean_schema_nodes(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(
                    key.as_str(),
                    "inputSchema" | "properties" | "items" | "$defs"
                ) {
                    assert!(
                        !matches!(child, Value::Bool(_)),
                        "boolean schema node at {key}: {child}"
                    );
                }
                assert_no_boolean_schema_nodes(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_boolean_schema_nodes(item);
            }
        }
        _ => {}
    }
}
