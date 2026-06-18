use std::fs;
use std::path::{Path, PathBuf};

use crate::path_validation::{has_null_bytes, is_path_within_allowed, normalize_path};

/// Safe directory access control for the filesystem MCP server.
///
/// Stores canonical (symlink‑resolved) paths and validates incoming paths
/// using **component‑based** comparison (not string prefix matching) to
/// prevent prefix‑sibling attacks.
#[derive(Debug, Clone)]
pub struct AllowedDirectories {
    dirs: Vec<PathBuf>,
}

impl AllowedDirectories {
    /// Create an empty set (no tools can operate).
    pub fn empty() -> Self {
        Self { dirs: Vec::new() }
    }

    /// Build from command‑line directory paths.
    ///
    /// Each entry is:
    /// 1. Checked for null bytes (skipped if present).
    /// 2. `~` is expanded to the user's home directory.
    /// 3. Canonicalised via `fs::canonicalize` (must exist and be accessible).
    /// 4. Verified to be a directory (not a file).
    ///
    /// Inaccessible entries are **skipped** with a warning on stderr.
    /// Returns `Err` when **every** entry is unusable.
    pub fn from_existing_dirs(dirs: &[impl AsRef<Path>]) -> Result<Self, String> {
        let mut validated: Vec<PathBuf> = Vec::new();

        for raw in dirs {
            let raw = raw.as_ref();

            // Reject null bytes
            if has_null_bytes(raw) {
                eprintln!("Warning: Skipping directory with null bytes: {:?}", raw);
                continue;
            }

            // Expand ~
            let expanded = expand_home(raw);

            // Canonicalise (resolves symlinks, verifies existence)
            match fs::canonicalize(&expanded) {
                Ok(canon) => {
                    if canon.is_dir() {
                        validated.push(canon);
                    } else {
                        eprintln!(
                            "Warning: Skipping non-directory path: {}",
                            expanded.display()
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Cannot access directory {}: {}",
                        expanded.display(),
                        e
                    );
                }
            }
        }

        if validated.is_empty() {
            return Err("None of the specified directories are accessible".to_string());
        }

        Ok(Self { dirs: validated })
    }

    /// Validate a path that **already exists** on disk.
    ///
    /// Returns the canonical (symlink‑resolved) path on success.
    /// The path is canonicalised and checked against every allowed
    /// directory via component comparison.
    pub fn validate_existing_path(&self, path: &Path) -> Result<PathBuf, String> {
        if self.dirs.is_empty() {
            return Err("No allowed directories configured".to_string());
        }
        if has_null_bytes(path) {
            return Err("Path contains null bytes".to_string());
        }

        let canonical = fs::canonicalize(path)
            .map_err(|e| format!("Cannot resolve path '{}': {}", path.display(), e))?;

        if is_path_within_allowed(&canonical, &self.dirs) {
            Ok(canonical)
        } else {
            Err(format!(
                "Path '{}' is not within any allowed directory",
                path.display()
            ))
        }
    }

    /// Validate a **candidate** path that may not exist yet.
    ///
    /// Returns the resolved (canonical ancestor + remaining components)
    /// path on success.
    ///
    /// Algorithm:
    /// 1. Normalise (resolve `.` / `..` without touching disk).
    /// 2. Walk up to find the deepest existing ancestor.
    /// 3. Canonicalise that ancestor (resolves symlinks).
    /// 4. Reconstruct the full path from that ancestor + remaining components.
    /// 5. Check the reconstructed path is within allowed directories.
    ///
    /// This reveals symlink‑based escapes: if the deepest existing ancestor
    /// is a symlink that resolves *outside* an allowed directory, the
    /// candidate is rejected.
    pub fn validate_candidate_path(&self, path: &Path) -> Result<PathBuf, String> {
        if self.dirs.is_empty() {
            return Err("No allowed directories configured".to_string());
        }
        if has_null_bytes(path) {
            return Err("Path contains null bytes".to_string());
        }

        let normalized = normalize_path(path);

        // Walk upward from the candidate to find the deepest existing ancestor.
        let mut current: &Path = &normalized;
        let mut missing_components: Vec<&Path> = Vec::new();

        while !current.exists() {
            match current.parent() {
                Some(parent) => {
                    if let Some(name) = current.file_name() {
                        missing_components.push(Path::new(name));
                    }
                    current = parent;
                }
                None => {
                    // Reached filesystem root and nothing exists — use root.
                    break;
                }
            }
        }

        // Canonicalise the deepest existing ancestor (resolves symlinks).
        let canonical_ancestor = fs::canonicalize(current)
            .map_err(|e| format!("Cannot resolve path '{}': {}", current.display(), e))?;

        // Reconstruct the full path from the canonical ancestor.
        let mut resolved = canonical_ancestor;
        for comp in missing_components.into_iter().rev() {
            resolved.push(comp);
        }

        if is_path_within_allowed(&resolved, &self.dirs) {
            Ok(resolved)
        } else {
            Err(format!(
                "Path '{}' resolves outside allowed directories",
                path.display()
            ))
        }
    }

    /// Convenience: call `validate_existing_path` or `validate_candidate_path`
    /// based on whether the path exists on disk.
    pub fn validate_path(&self, path: &Path) -> Result<PathBuf, String> {
        if path.exists() {
            self.validate_existing_path(path)
        } else {
            self.validate_candidate_path(path)
        }
    }

    /// Return the list of canonical allowed directory paths.
    pub fn list_allowed_directories(&self) -> &[PathBuf] {
        &self.dirs
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Expand a leading `~` to the user's home directory.
fn expand_home(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        home_dir().unwrap_or_else(|| path.to_path_buf())
    } else if let Some(tail) = s.strip_prefix("~/") {
        match home_dir() {
            Some(home) => home.join(tail),
            None => path.to_path_buf(),
        }
    } else {
        path.to_path_buf()
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .or_else(|_| {
                let drive = std::env::var("HOMEDRIVE").unwrap_or_default();
                let path = std::env::var("HOMEPATH").unwrap_or_default();
                if drive.is_empty() || path.is_empty() {
                    Err(std::env::VarError::NotPresent)
                } else {
                    Ok(format!("{drive}{path}"))
                }
            })
            .ok()
            .map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Temporary test sandbox — each instance gets a unique path.
    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let pid = std::process::id();
            let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("fs-server-test-{pid}-{seq}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn join(&self, rel: &str) -> PathBuf {
            self.path.join(rel)
        }

        fn mkdir(&self, rel: &str) -> PathBuf {
            let p = self.path.join(rel);
            fs::create_dir_all(&p).unwrap();
            p
        }

        fn touch(&self, rel: &str) -> PathBuf {
            let p = self.path.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut f = File::create(&p).unwrap();
            writeln!(f, "content").unwrap();
            p
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    // -----------------------------------------------------------------------
    // Symlink helper
    // -----------------------------------------------------------------------

    #[cfg(unix)]
    fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }

    fn symlink_supported() -> bool {
        let pid = std::process::id();
        let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = std::env::temp_dir().join(format!("symlink-check-{pid}-{seq}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let target = tmp.join("target");
        let link = tmp.join("link");
        let ok = File::create(&target).is_ok() && create_symlink(&target, &link).is_ok();
        let _ = fs::remove_dir_all(&tmp);
        ok
    }

    // -----------------------------------------------------------------------
    // from_existing_dirs
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_existing_dirs_valid_dirs() {
        let td = TestDir::new();
        let d1 = td.mkdir("allowed");
        let d2 = td.mkdir("other");

        let allowed = AllowedDirectories::from_existing_dirs(&[&d1, &d2]).unwrap();
        assert_eq!(allowed.dirs.len(), 2);
    }

    #[test]
    fn test_from_existing_dirs_skips_inaccessible_keeps_valid() {
        let td = TestDir::new();
        let valid = td.mkdir("valid");

        let allowed =
            AllowedDirectories::from_existing_dirs(&[td.join("nonexistent"), valid.clone()])
                .unwrap();
        assert_eq!(allowed.dirs.len(), 1);
        assert_eq!(allowed.dirs[0], fs::canonicalize(&valid).unwrap());
    }

    #[test]
    fn test_from_existing_dirs_errors_if_none_valid() {
        let td = TestDir::new();
        let result = AllowedDirectories::from_existing_dirs(&[
            td.join("nonexistent1"),
            td.join("nonexistent2"),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("None of the specified"));
    }

    #[test]
    fn test_from_existing_dirs_skips_file_path() {
        let td = TestDir::new();
        let file = td.touch("file.txt");
        let dir = td.mkdir("dir");

        let allowed = AllowedDirectories::from_existing_dirs(&[&file, &dir]).unwrap();
        // Only the directory should be kept
        assert_eq!(allowed.dirs.len(), 1);
    }

    // -----------------------------------------------------------------------
    // validate_existing_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_existing_path_allows_exact_root() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        assert!(allowed.validate_existing_path(&allowed_dir).is_ok());
    }

    #[test]
    fn test_validate_existing_path_allows_subpath() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let sub = td.touch("allowed/file.txt");
        let nested = td.touch("allowed/sub/dir/other.txt");

        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();
        assert!(allowed.validate_existing_path(&sub).is_ok());
        assert!(allowed.validate_existing_path(&nested).is_ok());
    }

    #[test]
    fn test_validate_existing_path_rejects_outside() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let outside = td.touch("outside/secret.txt");

        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();
        assert!(allowed.validate_existing_path(&outside).is_err());
    }

    #[test]
    fn test_validate_existing_path_rejects_prefix_sibling() {
        let td = TestDir::new();
        let project = td.mkdir("project");
        let project2 = td.mkdir("project2");

        let allowed = AllowedDirectories::from_existing_dirs(&[&project]).unwrap();
        assert!(
            allowed.validate_existing_path(&project2).is_err(),
            "should reject prefix sibling project2"
        );
    }

    #[test]
    fn test_validate_existing_path_rejects_null_bytes() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        let bad = Path::new("\0/etc/passwd");
        assert!(allowed.validate_existing_path(bad).is_err());
    }

    #[test]
    fn test_validate_existing_path_empty_allowed_fails() {
        let allowed = AllowedDirectories::empty();
        let tmp_buf = std::env::temp_dir();
        let tmp = Path::new(tmp_buf.to_str().unwrap());
        assert!(allowed.validate_existing_path(tmp).is_err());
    }

    // -----------------------------------------------------------------------
    // validate_candidate_path — non‑existent files
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_candidate_path_through_safe_parent() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        // File in a subdir that doesn't exist yet, but parent exists.
        let candidate = td.join("allowed/newdir/newfile.txt");
        assert!(
            allowed.validate_candidate_path(&candidate).is_ok(),
            "candidate through existing parent should be allowed"
        );

        // File in the allowed root directly.
        let candidate2 = td.join("allowed/another_file.rs");
        assert!(allowed.validate_candidate_path(&candidate2).is_ok());
    }

    #[test]
    fn test_validate_candidate_path_rejects_outside() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        // Non‑existent file outside allowed directory.
        let candidate = td.join("outside/newfile.txt");
        assert!(allowed.validate_candidate_path(&candidate).is_err());
    }

    #[test]
    fn test_validate_candidate_path_empty_allowed_fails() {
        let allowed = AllowedDirectories::empty();
        let candidate = Path::new("/tmp/test.txt");
        assert!(allowed.validate_candidate_path(candidate).is_err());
    }

    // -----------------------------------------------------------------------
    // Symlink tests (conditionally run)
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_existing_path_rejects_symlink_outside() {
        if !symlink_supported() {
            eprintln!("   ⏭️  Skipping — symlinks not supported in this environment");
            return;
        }

        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let secret = td.touch("forbidden/secret.txt");

        // Create a symlink INSIDE the allowed dir pointing OUTSIDE.
        let bad_link = td.join("allowed/link_to_secret.txt");
        create_symlink(&secret, &bad_link).unwrap();

        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        // The symlink path itself looks like it's inside allowed_dir,
        // but canonicalising resolves it to the forbidden location.
        assert!(
            allowed.validate_existing_path(&bad_link).is_err(),
            "symlink to outside should be rejected"
        );
    }

    #[test]
    fn test_validate_existing_path_allows_symlink_inside() {
        if !symlink_supported() {
            eprintln!("   ⏭️  Skipping — symlinks not supported in this environment");
            return;
        }

        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let target = td.touch("allowed/target.txt");
        let good_link = td.join("allowed/good_link.txt");

        create_symlink(&target, &good_link).unwrap();

        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        assert!(
            allowed.validate_existing_path(&good_link).is_ok(),
            "symlink pointing inside allowed dir should be accepted"
        );
    }

    #[test]
    fn test_validate_candidate_path_rejects_symlink_parent_outside() {
        if !symlink_supported() {
            eprintln!("   ⏭️  Skipping — symlinks not supported in this environment");
            return;
        }

        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let forbidden_dir = td.mkdir("forbidden");

        // Create a symlink: allowed/sublink → forbidden_dir
        let sublink = td.join("allowed/sublink");
        create_symlink(&forbidden_dir, &sublink).unwrap();

        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        // A candidate path through the symlink'd parent should be rejected
        // because the parent resolves outside the allowed directory.
        let candidate = td.join("allowed/sublink/evil.txt");
        assert!(
            allowed.validate_candidate_path(&candidate).is_err(),
            "candidate through symlink parent outside root should be rejected"
        );
    }

    // -----------------------------------------------------------------------
    // Windows case‑insensitive comparison
    // -----------------------------------------------------------------------

    #[cfg(windows)]
    #[test]
    fn test_windows_case_insensitive_paths() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("Project");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        // Access via different case should succeed.
        let mixed_case = td.join("project/file.txt");
        let f = File::create(&mixed_case).unwrap();
        drop(f);

        assert!(
            allowed.validate_existing_path(&mixed_case).is_ok(),
            "Windows paths should match case-insensitively"
        );

        // Prefix sibling with different case should still fail.
        let sibling = td.join("project2");
        assert!(
            allowed.validate_existing_path(&sibling).is_err(),
            "prefix sibling should be rejected even with case variation"
        );
    }

    // -----------------------------------------------------------------------
    // list_allowed_directories
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_allowed_directories() {
        let td = TestDir::new();
        let d1 = td.mkdir("a");
        let d2 = td.mkdir("b");
        let allowed = AllowedDirectories::from_existing_dirs(&[&d1, &d2]).unwrap();

        let listed = allowed.list_allowed_directories();
        assert_eq!(listed.len(), 2);
        assert!(listed.contains(&fs::canonicalize(&d1).unwrap()));
        assert!(listed.contains(&fs::canonicalize(&d2).unwrap()));
    }

    #[test]
    fn test_list_allowed_directories_empty() {
        let allowed = AllowedDirectories::empty();
        assert!(allowed.list_allowed_directories().is_empty());
    }

    // -----------------------------------------------------------------------
    // Traversal via normalize_path integration
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_candidate_path_rejects_traversal_escape() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        let candidate = td.join("allowed/../../../etc/passwd");
        assert!(
            allowed.validate_candidate_path(&candidate).is_err(),
            "traversal escape should be rejected"
        );
    }

    #[test]
    fn test_validate_existing_path_rejects_traversal_escaped_path() {
        let td = TestDir::new();
        let allowed_dir = td.mkdir("allowed");
        let allowed = AllowedDirectories::from_existing_dirs(&[&allowed_dir]).unwrap();

        // Create a file via real path, then try to access via traversal.
        let _secret = td.touch("forbidden/secret.txt");

        // The traversal path normalizes to outside the allowed dir.
        let traversal = td.join("allowed/../forbidden/secret.txt");
        // normalize_path + canonicalize should show it's outside.
        assert!(
            allowed.validate_existing_path(&traversal).is_err(),
            "traversal via existing path should be rejected"
        );
    }
}
