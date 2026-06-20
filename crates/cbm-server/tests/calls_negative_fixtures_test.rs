//! Negative CALLS fixtures — ambiguous imports, aliases, overload-like methods, framework noise.

mod support;

use codebase_memory_mcp::discover::IndexMode;
use codebase_memory_mcp::pipeline::Pipeline;
use codebase_memory_mcp::store::Store;
use support::isolated_cache;
use tempfile::TempDir;

fn calls_edges(store: &Store) -> Vec<codebase_memory_mcp::store::Edge> {
    store
        .list_edges_limited(500)
        .unwrap()
        .into_iter()
        .filter(|e| e.edge_type == "CALLS")
        .collect()
}

#[test]
fn python_import_alias_resolves_aliased_helper() {
    let (_guard, _cache, _) = isolated_cache();
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("main.py"),
        "from utils import helper as h\n\ndef main():\n    h()\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("utils.py"), "def helper():\n    pass\n").unwrap();
    std::fs::write(dir.path().join("decoy.py"), "def helper():\n    pass\n").unwrap();

    let pipeline = Pipeline::new(IndexMode::Full);
    let index = pipeline.run(dir.path(), Some("py-alias-calls")).unwrap();
    let store = Store::open(&index.project).unwrap();

    let main_calls: Vec<_> = calls_edges(&store)
        .into_iter()
        .filter(|e| e.src_qn.contains("main.py::Function::main@"))
        .collect();
    assert_eq!(
        main_calls.len(),
        1,
        "expected aliased import CALLS: {main_calls:?}"
    );
    assert!(
        main_calls[0].dst_qn.starts_with("utils.py::"),
        "alias h should resolve to utils.helper, got {}",
        main_calls[0].dst_qn
    );

    let _ = codebase_memory_mcp::store::delete_project_db(&index.project);
}

#[test]
fn javascript_import_alias_resolves_aliased_helper() {
    let (_guard, _cache, _) = isolated_cache();
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("main.js"),
        "import { helper as h } from './utils'\nfunction main() { h(); }\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("utils.js"), "export function helper() {}\n").unwrap();
    std::fs::write(dir.path().join("decoy.js"), "export function helper() {}\n").unwrap();

    let pipeline = Pipeline::new(IndexMode::Full);
    let index = pipeline.run(dir.path(), Some("js-alias-calls")).unwrap();
    let store = Store::open(&index.project).unwrap();

    let main_calls: Vec<_> = calls_edges(&store)
        .into_iter()
        .filter(|e| e.src_qn.contains("main.js::Function::main@"))
        .collect();
    assert_eq!(
        main_calls.len(),
        1,
        "expected aliased import CALLS: {main_calls:?}"
    );
    assert!(
        main_calls[0].dst_qn.starts_with("utils.js::"),
        "alias h should resolve to utils.helper, got {}",
        main_calls[0].dst_qn
    );

    let _ = codebase_memory_mcp::store::delete_project_db(&index.project);
}

#[test]
fn java_overloaded_like_methods_resolve_within_same_class() {
    let (_guard, _cache, _) = isolated_cache();
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("App.java"),
        "class A {\n  void run() { helper(); }\n  void helper() {}\n}\nclass B {\n  void helper() {}\n}\n",
    )
    .unwrap();

    let pipeline = Pipeline::new(IndexMode::Full);
    let index = pipeline
        .run(dir.path(), Some("java-overload-like"))
        .unwrap();
    let store = Store::open(&index.project).unwrap();

    let run_calls: Vec<_> = calls_edges(&store)
        .into_iter()
        .filter(|e| e.src_qn.contains("App.java::") && e.src_qn.contains("::run@"))
        .collect();
    assert_eq!(
        run_calls.len(),
        1,
        "expected scoped class CALLS: {run_calls:?}"
    );
    assert!(
        run_calls[0].dst_qn.contains("::helper@") && run_calls[0].dst_qn.contains("App.java::"),
        "A.run should call A.helper, got {}",
        run_calls[0].dst_qn
    );

    let _ = codebase_memory_mcp::store::delete_project_db(&index.project);
}

#[test]
fn javascript_console_log_does_not_link_decoy_log_function() {
    let (_guard, _cache, _) = isolated_cache();
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("main.js"),
        "function main() { console.log('hi'); }\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("decoy.js"), "function log() {}\n").unwrap();

    let pipeline = Pipeline::new(IndexMode::Full);
    let index = pipeline
        .run(dir.path(), Some("js-framework-noise"))
        .unwrap();
    let store = Store::open(&index.project).unwrap();

    let main_calls: Vec<_> = calls_edges(&store)
        .into_iter()
        .filter(|e| e.src_qn.contains("main.js::Function::main@"))
        .collect();
    assert!(
        main_calls.is_empty(),
        "console.log should not create CALLS to decoy log(): {main_calls:?}"
    );

    let _ = codebase_memory_mcp::store::delete_project_db(&index.project);
}
