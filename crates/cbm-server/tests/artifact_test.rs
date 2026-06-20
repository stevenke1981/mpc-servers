//! Artifact export/import parity: round-trip, skip-restore, validation gates.

mod support;

use codebase_memory_mcp::discover::IndexMode;
use codebase_memory_mcp::persistence::{
    artifact_exists, artifact_path, import_artifact, metadata_path, try_restore,
    ARTIFACT_SCHEMA_VERSION,
};
use codebase_memory_mcp::pipeline::Pipeline;
use codebase_memory_mcp::store::{SearchFilter, Store};
use std::fs;

#[test]
fn export_writes_reference_style_metadata() {
    let (_guard, _cache, _) = support::isolated_cache();
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn artifact_meta() {}\n").unwrap();

    let result = Pipeline::new(IndexMode::Full)
        .set_export_artifact(true)
        .run(dir.path(), Some("meta-test"))
        .unwrap();
    assert!(result.artifact_path.is_some());
    assert!(artifact_exists(dir.path()));

    let meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(metadata_path(dir.path())).unwrap()).unwrap();
    assert_eq!(
        meta.get("schema_version").and_then(|v| v.as_i64()),
        Some(ARTIFACT_SCHEMA_VERSION as i64)
    );
    assert!(
        meta.get("original_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        meta.get("compressed_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(meta.get("nodes").and_then(|v| v.as_i64()).unwrap_or(0) >= 1);
}

#[test]
fn try_restore_imports_when_cache_missing() {
    let (_guard, _cache, _) = support::isolated_cache();
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn restore_me() {}\n").unwrap();

    let project = "cbm+restore-missing";
    Pipeline::new(IndexMode::Full)
        .set_export_artifact(true)
        .run(dir.path(), Some("restore-missing"))
        .unwrap();

    {
        let store = Store::open(project).unwrap();
        store.delete_project().unwrap();
    }
    codebase_memory_mcp::store::delete_project_db(project).unwrap();

    assert!(try_restore(dir.path(), project).unwrap());

    let store = Store::open(project).unwrap();
    let hits = store
        .search(&SearchFilter {
            query: Some("restore_me".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(hits.symbols.iter().any(|s| s.name == "restore_me"));
}

#[test]
fn corrupt_artifact_does_not_replace_existing_cache() {
    let (_guard, _cache, _) = support::isolated_cache();
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn keep_cache() {}\n").unwrap();

    let project = "cbm+corrupt-keep";
    Pipeline::new(IndexMode::Full)
        .set_export_artifact(true)
        .run(dir.path(), Some("corrupt-keep"))
        .unwrap();

    let before = Store::open(project).unwrap().count_symbols().unwrap();
    fs::write(artifact_path(dir.path()), b"broken").unwrap();

    assert!(import_artifact(dir.path(), Some("corrupt-keep")).is_err());
    let after = Store::open(project).unwrap().count_symbols().unwrap();
    assert_eq!(before, after);
    assert!(after >= 1);
}
