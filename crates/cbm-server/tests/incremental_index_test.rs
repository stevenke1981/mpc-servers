//! Incremental indexing: git HEAD/dirty detection, fingerprint drift, deletions.

mod support;

use codebase_memory_mcp::discover::IndexMode;
use codebase_memory_mcp::pipeline::Pipeline;
use codebase_memory_mcp::store::{SearchFilter, Store};
use std::path::Path;
use std::process::Command;

fn init_git_repo(path: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "t@t.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "test"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn git_commit_all(path: &Path, message: &str) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output()
        .unwrap();
}

#[test]
fn run_smart_reindexes_on_head_change_with_clean_worktree() {
    let (_guard, _cache, _) = support::isolated_cache();
    let repo = tempfile::TempDir::new().unwrap();
    init_git_repo(repo.path());
    std::fs::write(repo.path().join("lib.rs"), "pub fn v1() {}\n").unwrap();
    git_commit_all(repo.path(), "v1");

    let pipeline = Pipeline::new(IndexMode::Full);
    let full = pipeline.run(repo.path(), Some("head-incr")).unwrap();
    assert!(full.symbols_extracted >= 1);

    std::fs::write(repo.path().join("lib.rs"), "pub fn v2() {}\n").unwrap();
    git_commit_all(repo.path(), "v2");

    let smart = pipeline
        .run_smart(repo.path(), Some("head-incr"), true)
        .unwrap();
    assert!(smart.incremental);
    assert_eq!(smart.files_indexed, 1);

    let store = Store::open(&full.project).unwrap();
    let hits = store
        .search(&SearchFilter {
            query: Some("v2".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(hits.symbols.iter().any(|s| s.name == "v2"));
    assert!(!hits.symbols.iter().any(|s| s.name == "v1"));
}

#[test]
fn incremental_removes_deleted_file_symbols() {
    let (_guard, _cache, _) = support::isolated_cache();
    let repo = tempfile::TempDir::new().unwrap();
    init_git_repo(repo.path());
    std::fs::write(repo.path().join("keep.rs"), "pub fn keep() {}\n").unwrap();
    std::fs::write(repo.path().join("gone.rs"), "pub fn gone() {}\n").unwrap();
    git_commit_all(repo.path(), "init");

    let pipeline = Pipeline::new(IndexMode::Full);
    let full = pipeline.run(repo.path(), Some("del-incr")).unwrap();
    assert!(full.symbols_extracted >= 2);

    std::fs::remove_file(repo.path().join("gone.rs")).unwrap();
    let incr = pipeline
        .run_incremental(repo.path(), &full.project, &["gone.rs".into()])
        .unwrap();
    assert!(incr.incremental);

    let store = Store::open(&full.project).unwrap();
    let gone = store
        .search(&SearchFilter {
            query: Some("gone".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(gone.symbols.is_empty());
}

#[test]
fn fingerprint_drift_detected_without_git_dirty() {
    let (_guard, _cache, _) = support::isolated_cache();
    let repo = tempfile::TempDir::new().unwrap();
    std::fs::write(repo.path().join("plain.rs"), "pub fn alpha() {}\n").unwrap();

    let pipeline = Pipeline::new(IndexMode::Full);
    let full = pipeline.run(repo.path(), Some("hash-incr")).unwrap();

    std::fs::write(repo.path().join("plain.rs"), "pub fn beta() {}\n").unwrap();

    let store = Store::open(&full.project).unwrap();
    let drift = store.files_with_fingerprint_drift(repo.path()).unwrap();
    assert!(drift.iter().any(|p| p == "plain.rs"));

    let smart = pipeline
        .run_smart(repo.path(), Some("hash-incr"), true)
        .unwrap();
    assert!(smart.incremental);
    assert_eq!(smart.files_indexed, 1);

    let hits = store
        .search(&SearchFilter {
            query: Some("beta".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(hits.symbols.iter().any(|s| s.name == "beta"));
}
