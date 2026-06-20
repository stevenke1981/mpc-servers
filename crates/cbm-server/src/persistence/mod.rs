//! Compressed graph artifact export/import (`.codebase-memory/graph.db.zst`).

use crate::error::{Error, Result};
use crate::git;
use crate::project::{project_db_path, project_name_from_path};
use crate::store::Store;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ARTIFACT_DIR: &str = ".codebase-memory";
pub const ARTIFACT_FILE: &str = "graph.db.zst";
pub const ARTIFACT_META: &str = "artifact.json";
/// Legacy metadata filename written by earlier Rust builds.
pub const LEGACY_MANIFEST_FILE: &str = "manifest.json";
pub const ARTIFACT_SCHEMA_VERSION: i32 = 1;

const ZSTD_LEVEL: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactMetadata {
    schema_version: i32,
    #[serde(default)]
    commit: String,
    indexed_at: String,
    project: String,
    nodes: i64,
    edges: i64,
    original_size: u64,
    compressed_size: u64,
    compression_level: i32,
}

#[derive(Debug, Deserialize)]
struct LegacyManifest {
    #[serde(default)]
    bytes_raw: Option<u64>,
    #[serde(default)]
    bytes_compressed: Option<u64>,
    #[serde(default)]
    schema_version: Option<i32>,
}

pub fn env_enabled() -> bool {
    matches!(
        std::env::var("CBRLM_PERSISTENCE")
            .or_else(|_| std::env::var("CBM_PERSISTENCE"))
            .as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

pub fn artifact_dir(repo_path: &Path) -> PathBuf {
    repo_path.join(ARTIFACT_DIR)
}

pub fn artifact_path(repo_path: &Path) -> PathBuf {
    artifact_dir(repo_path).join(ARTIFACT_FILE)
}

pub fn metadata_path(repo_path: &Path) -> PathBuf {
    artifact_dir(repo_path).join(ARTIFACT_META)
}

pub fn legacy_manifest_path(repo_path: &Path) -> PathBuf {
    artifact_dir(repo_path).join(LEGACY_MANIFEST_FILE)
}

pub fn artifact_exists(repo_path: &Path) -> bool {
    if !artifact_path(repo_path).is_file() {
        return false;
    }
    read_metadata(repo_path)
        .map(|m| {
            m.schema_version >= 0
                && m.schema_version <= ARTIFACT_SCHEMA_VERSION
                && m.original_size > 0
        })
        .unwrap_or(false)
}

pub fn export_artifact(repo_path: &Path, project: &str, store: &Store) -> Result<PathBuf> {
    store.checkpoint_truncate()?;
    let db_path = project_db_path(project);
    if !db_path.is_file() {
        return Err(Error::Other(format!(
            "database not found: {}",
            db_path.display()
        )));
    }

    let raw = fs::read(&db_path)?;
    let compressed = zstd::encode_all(&raw[..], ZSTD_LEVEL)?;

    fs::create_dir_all(artifact_dir(repo_path))?;

    let dest = artifact_path(repo_path);
    write_atomic(&dest, &compressed)?;

    let nodes = store.count_symbols().unwrap_or(0);
    let edges = store.count_edges().unwrap_or(0);
    let commit = git::head_sha(repo_path).ok().flatten().unwrap_or_default();
    let metadata = ArtifactMetadata {
        schema_version: ARTIFACT_SCHEMA_VERSION,
        commit,
        indexed_at: iso_timestamp(),
        project: project.to_string(),
        nodes,
        edges,
        original_size: raw.len() as u64,
        compressed_size: compressed.len() as u64,
        compression_level: ZSTD_LEVEL,
    };
    write_atomic(
        &metadata_path(repo_path),
        (serde_json::to_string_pretty(&metadata)? + "\n").as_bytes(),
    )?;

    Ok(dest)
}

pub fn import_artifact(repo_path: &Path, project: Option<&str>) -> Result<bool> {
    let src = artifact_path(repo_path);
    if !src.is_file() {
        return Ok(false);
    }

    let metadata = read_metadata(repo_path)
        .map_err(|e| Error::Other(format!("artifact metadata invalid: {e}")))?;
    if metadata.schema_version < 0 || metadata.schema_version > ARTIFACT_SCHEMA_VERSION {
        return Err(Error::Other(format!(
            "artifact schema_version {} incompatible with {}",
            metadata.schema_version, ARTIFACT_SCHEMA_VERSION
        )));
    }
    if metadata.original_size == 0 {
        return Err(Error::Other("artifact missing original_size".into()));
    }

    let project = match project {
        Some(p) => crate::project::normalize_project_name(p),
        None => project_name_from_path(repo_path),
    };

    let compressed = fs::read(&src)?;
    let raw = zstd::decode_all(&compressed[..])
        .map_err(|e| Error::Other(format!("zstd decode failed: {e}")))?;
    if raw.len() as u64 != metadata.original_size {
        return Err(Error::Other(format!(
            "artifact size mismatch: expected {} bytes, got {}",
            metadata.original_size,
            raw.len()
        )));
    }

    let db_path = project_db_path(&project);
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = db_path.with_extension("import_tmp");
    write_atomic(&tmp_path, &raw)?;

    if !sqlite_integrity_ok(&tmp_path)? {
        let _ = fs::remove_file(&tmp_path);
        return Err(Error::Other("artifact integrity_check failed".into()));
    }

    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    fs::rename(&tmp_path, &db_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        Error::Io(e)
    })?;

    let store = Store::open(&project)?;
    store.upsert_project(repo_path.to_string_lossy().as_ref())?;
    Ok(true)
}

pub fn try_restore(repo_path: &Path, project: &str) -> Result<bool> {
    if project_db_path(project).is_file() {
        return Ok(false);
    }
    if !artifact_exists(repo_path) {
        return Ok(false);
    }
    import_artifact(repo_path, Some(project))
}

fn read_metadata(repo_path: &Path) -> Result<ArtifactMetadata> {
    let meta_path = metadata_path(repo_path);
    if meta_path.is_file() {
        let raw = fs::read_to_string(&meta_path)?;
        let meta: ArtifactMetadata = serde_json::from_str(&raw)?;
        return Ok(meta);
    }

    let legacy_path = legacy_manifest_path(repo_path);
    if !legacy_path.is_file() {
        return Err(Error::Other("artifact metadata missing".into()));
    }
    let raw = fs::read_to_string(&legacy_path)?;
    let legacy: LegacyManifest = serde_json::from_str(&raw)?;
    let schema_version = legacy.schema_version.unwrap_or(ARTIFACT_SCHEMA_VERSION);
    let original_size = legacy
        .bytes_raw
        .ok_or_else(|| Error::Other("legacy manifest missing bytes_raw".into()))?;
    let compressed_size = legacy.bytes_compressed.unwrap_or(0);
    Ok(ArtifactMetadata {
        schema_version,
        commit: String::new(),
        indexed_at: String::new(),
        project: String::new(),
        nodes: 0,
        edges: 0,
        original_size,
        compressed_size,
        compression_level: ZSTD_LEVEL,
    })
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::Io(e)
    })
}

fn sqlite_integrity_ok(path: &Path) -> Result<bool> {
    let conn = Connection::open(path)?;
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    Ok(result == "ok")
}

fn iso_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::IndexMode;
    use crate::pipeline::Pipeline;
    use crate::test_lock;
    use tempfile::TempDir;

    fn fixture_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "pub fn persist_me() {}\n").unwrap();
        dir
    }

    #[test]
    fn roundtrip_artifact() {
        let _guard = test_lock::acquire();
        let cache = TempDir::new().unwrap();
        std::env::set_var("CBRLM_CACHE_DIR", cache.path());

        let dir = fixture_repo();
        let project = "cbm+artifact-roundtrip";
        let result = Pipeline::new(IndexMode::Full)
            .set_export_artifact(true)
            .run(dir.path(), Some("artifact-roundtrip"))
            .unwrap();
        assert!(result.success);
        assert!(result.artifact_path.is_some());

        let artifact = artifact_path(dir.path());
        assert!(artifact.is_file());
        assert!(metadata_path(dir.path()).is_file());
        assert!(artifact_exists(dir.path()));

        {
            let store = Store::open(project).unwrap();
            store.delete_project().unwrap();
        }
        crate::store::delete_project_db(project).unwrap();
        assert!(import_artifact(dir.path(), Some("artifact-roundtrip")).unwrap());

        let store = Store::open(project).unwrap();
        let count = store.count_symbols().unwrap();
        assert!(count >= 1);
        assert!(store
            .search(&crate::store::SearchFilter {
                query: Some("persist_me".into()),
                ..Default::default()
            })
            .unwrap()
            .symbols
            .iter()
            .any(|s| s.name == "persist_me"));
    }

    #[test]
    fn try_restore_skips_when_cache_exists() {
        let _guard = test_lock::acquire();
        let cache = TempDir::new().unwrap();
        std::env::set_var("CBRLM_CACHE_DIR", cache.path());
        let dir = fixture_repo();
        let project = "cbm+restore-skip";
        Pipeline::new(IndexMode::Full)
            .set_export_artifact(true)
            .run(dir.path(), Some("restore-skip"))
            .unwrap();
        assert!(!try_restore(dir.path(), project).unwrap());
    }

    #[test]
    fn rejects_incompatible_schema_version() {
        let _guard = test_lock::acquire();
        let cache = TempDir::new().unwrap();
        std::env::set_var("CBRLM_CACHE_DIR", cache.path());
        let dir = fixture_repo();
        Pipeline::new(IndexMode::Full)
            .set_export_artifact(true)
            .run(dir.path(), Some("schema-mismatch"))
            .unwrap();

        let meta_path = metadata_path(dir.path());
        let mut meta: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
        meta["schema_version"] = serde_json::json!(99);
        fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();

        assert!(!artifact_exists(dir.path()));
        let err = import_artifact(dir.path(), Some("schema-mismatch")).unwrap_err();
        assert!(err.to_string().contains("schema_version"));
    }

    #[test]
    fn rejects_corrupt_compressed_payload() {
        let _guard = test_lock::acquire();
        let cache = TempDir::new().unwrap();
        std::env::set_var("CBRLM_CACHE_DIR", cache.path());
        let dir = fixture_repo();
        Pipeline::new(IndexMode::Full)
            .set_export_artifact(true)
            .run(dir.path(), Some("corrupt-zst"))
            .unwrap();

        fs::write(artifact_path(dir.path()), b"not-valid-zstd").unwrap();
        let err = import_artifact(dir.path(), Some("corrupt-zst")).unwrap_err();
        assert!(err.to_string().contains("zstd") || err.to_string().contains("size mismatch"));
    }
}
