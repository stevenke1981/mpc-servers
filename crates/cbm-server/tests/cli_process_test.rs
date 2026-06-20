//! Process-level CLI contract tests (Section 7.3).

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn cbm_mcp() -> Command {
    Command::cargo_bin("cbm").unwrap()
}

fn fixture_repo() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("lib.rs"),
        "pub fn answer() -> i32 { helper() }\nfn helper() -> i32 { 42 }\n",
    )
    .expect("write fixture");
    dir
}

fn parse_stdout_json(out: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(out).ok()?.trim();
    serde_json::from_str(text).ok()
}

#[test]
fn list_projects_json_quiet_stdout_is_parseable() {
    let output = cbm_mcp()
        .args(["cli", "list_projects", "--json", "--quiet"])
        .output()
        .expect("spawn cbm");
    assert!(output.status.success(), "exit failed: {:?}", output);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty under --quiet, got: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed = parse_stdout_json(&output.stdout).expect("stdout is not valid JSON");
    assert!(parsed.get("projects").is_some());
}

#[test]
fn index_repository_json_stdout_parseable_with_stderr_diagnostics() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("cache tempdir");
    let json = serde_json::json!({
        "repo_path": repo.path(),
        "project": "cli-process",
        "mode": "fast",
        "persistence": false
    })
    .to_string();
    let output = cbm_mcp()
        .args(["cli", "index_repository", "--json", &json])
        .env("CBRLM_CACHE_DIR", cache.path())
        .output()
        .expect("spawn cbm");
    assert!(output.status.success(), "exit failed: {:?}", output);
    let parsed = parse_stdout_json(&output.stdout).expect("stdout is not valid JSON");
    assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn json_without_quiet_keeps_stdout_parseable() {
    let repo = fixture_repo();
    let cache = tempfile::tempdir().expect("cache tempdir");
    let json = serde_json::json!({
        "repo_path": repo.path(),
        "project": "cli-process-2",
        "mode": "fast",
        "persistence": false
    })
    .to_string();
    let output = cbm_mcp()
        .args(["cli", "index_repository", "--json", &json])
        .env("CBRLM_CACHE_DIR", cache.path())
        .output()
        .expect("spawn cbm");
    assert!(output.status.success(), "exit failed: {:?}", output);
    assert!(parse_stdout_json(&output.stdout).is_some());
}
