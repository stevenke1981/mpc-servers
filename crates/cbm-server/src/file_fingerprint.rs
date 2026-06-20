use crate::error::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileFingerprint {
    pub mtime_ns: i64,
    pub size_bytes: i64,
}

pub fn fingerprint(path: &Path) -> Result<FileFingerprint> {
    let meta = std::fs::metadata(path)?;
    let mtime_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    Ok(FileFingerprint {
        mtime_ns,
        size_bytes: meta.len() as i64,
    })
}
