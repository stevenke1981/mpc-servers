use std::path::{Component, Path, PathBuf};

// ---------------------------------------------------------------------------
// Core path comparison — compare by components, not string prefix
// ---------------------------------------------------------------------------

/// Check if a path contains null bytes (forbidden in all paths).
pub(crate) fn has_null_bytes(path: &Path) -> bool {
    path.to_string_lossy().contains('\0')
}

/// Normalize a path by resolving `.` and `..` components without touching the
/// filesystem.  Preserves the absolute/relative nature of the input.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::with_capacity(path.as_os_str().len());
    for component in path.components() {
        match component {
            Component::CurDir => { /* skip */ }
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

/// Component‑based containment check.
///
/// Returns `true` when `path` is exactly `allowed` or a sub‑path of it.
/// On Windows, components are compared case-insensitively to match the usual
/// filesystem semantics; on other platforms, comparison remains exact.
///
/// This approach **prevents** prefix sibling attacks — e.g. `/tmp/project`
/// is NOT a prefix of `/tmp/project2` because the `Normal("project")`
/// component differs from `Normal("project2")`.
pub(crate) fn is_subpath(allowed: &Path, path: &Path) -> bool {
    let allowed_comps: Vec<_> = allowed.components().collect();
    let path_comps: Vec<_> = path.components().collect();

    if path_comps.len() < allowed_comps.len() {
        return false;
    }

    path_comps[..allowed_comps.len()]
        .iter()
        .zip(&allowed_comps)
        .all(|(path_comp, allowed_comp)| component_eq(path_comp, allowed_comp))
}

fn component_eq(left: &Component<'_>, right: &Component<'_>) -> bool {
    #[cfg(windows)]
    {
        left.as_os_str().to_string_lossy().to_lowercase()
            == right.as_os_str().to_string_lossy().to_lowercase()
    }

    #[cfg(not(windows))]
    {
        left == right
    }
}

/// Core validation: check if a (normalised / canonical) path is within any
/// of the allowed directories using component‑based comparison.
pub(crate) fn is_path_within_allowed(path: &Path, allowed_dirs: &[PathBuf]) -> bool {
    // Reject null bytes
    if has_null_bytes(path) {
        return false;
    }

    // Reject relative paths — all validation works on absolute paths
    if !path.has_root() {
        return false;
    }

    allowed_dirs.iter().any(|allowed| is_subpath(allowed, path))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // has_null_bytes
    // -----------------------------------------------------------------------

    #[test]
    fn test_has_null_bytes_rejects_nulls() {
        assert!(has_null_bytes(Path::new("/path/with\x00null")));
        assert!(has_null_bytes(Path::new("/path\x00/more")));
        assert!(!has_null_bytes(Path::new("/normal/path")));
    }

    // -----------------------------------------------------------------------
    // normalize_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_path_removes_dot() {
        let result = normalize_path(Path::new("/home/user/./project"));
        assert_eq!(result, Path::new("/home/user/project"));
    }

    #[test]
    fn test_normalize_path_resolves_parent_dir() {
        let result = normalize_path(Path::new("/home/user/project/../other"));
        assert_eq!(result, Path::new("/home/user/other"));
    }

    #[test]
    fn test_normalize_path_removes_traversal_escape() {
        let result = normalize_path(Path::new("/home/user/project/../../../etc/passwd"));
        assert_eq!(result, Path::new("/etc/passwd"));
    }

    #[test]
    fn test_normalize_path_keeps_mixed_dots_as_filename() {
        // "..test" is a regular filename (starts with two dots but also has
        // other chars), so it's a Normal component, not ParentDir.
        assert_eq!(
            normalize_path(Path::new("/project/..test")),
            Path::new("/project/..test")
        );
        assert_eq!(
            normalize_path(Path::new("/project/test..")),
            Path::new("/project/test..")
        );
    }

    #[test]
    fn test_normalize_path_triple_dot_is_filename() {
        // "..." is a valid filename, not ParentDir
        let result = normalize_path(Path::new("/project/..."));
        assert_eq!(result, Path::new("/project/..."));
    }

    #[test]
    fn test_normalize_path_preserves_absolute() {
        let result = normalize_path(Path::new("/"));
        assert_eq!(result, Path::new("/"));
    }

    // -----------------------------------------------------------------------
    // is_subpath — component containment
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_subpath_exact_match() {
        assert!(is_subpath(
            Path::new("/home/user/project"),
            Path::new("/home/user/project")
        ));
    }

    #[test]
    fn test_is_subpath_subdirectory() {
        let allowed = Path::new("/home/user/project");
        assert!(is_subpath(allowed, Path::new("/home/user/project/src")));
        assert!(is_subpath(
            allowed,
            Path::new("/home/user/project/src/main.rs")
        ));
        assert!(is_subpath(allowed, Path::new("/home/user/project/a/b/c/d")));
    }

    #[test]
    fn test_is_subpath_rejects_prefix_sibling_attack() {
        let allowed = Path::new("/home/user/project");
        assert!(!is_subpath(allowed, Path::new("/home/user/project2")));
        assert!(!is_subpath(allowed, Path::new("/home/user/project_backup")));
        assert!(!is_subpath(allowed, Path::new("/home/user/project-old")));
        assert!(!is_subpath(allowed, Path::new("/home/user/project.bak")));
        assert!(!is_subpath(allowed, Path::new("/home/user/projectile")));
    }

    #[test]
    fn test_is_subpath_rejects_outside() {
        let allowed = Path::new("/home/user/project");
        assert!(!is_subpath(allowed, Path::new("/home/user/other")));
        assert!(!is_subpath(allowed, Path::new("/etc/passwd")));
        assert!(!is_subpath(allowed, Path::new("/")));
    }

    #[test]
    fn test_is_subpath_rejects_parent() {
        let allowed = Path::new("/home/user/project");
        assert!(!is_subpath(allowed, Path::new("/home/user")));
        assert!(!is_subpath(allowed, Path::new("/home")));
    }

    #[test]
    fn test_is_subpath_multiple_dirs() {
        let a = Path::new("/home/user/project1");
        let b = Path::new("/home/user/project2");

        assert!(is_subpath(a, Path::new("/home/user/project1/src")));
        assert!(is_subpath(b, Path::new("/home/user/project2/src")));
        // sibling of either should be rejected
        assert!(!is_subpath(a, Path::new("/home/user/project3")));
        assert!(!is_subpath(b, Path::new("/home/user/project1")));
    }

    #[test]
    fn test_is_subpath_special_chars_in_name() {
        let allowed = Path::new("/home/user/my-project (v2)");
        assert!(is_subpath(allowed, Path::new("/home/user/my-project (v2)")));
        assert!(is_subpath(
            allowed,
            Path::new("/home/user/my-project (v2)/src")
        ));
        assert!(!is_subpath(
            allowed,
            Path::new("/home/user/my-project (v2)_backup")
        ));
    }

    #[test]
    fn test_is_subpath_nested_allowed_dirs() {
        // When multiple allowed dirs overlap, the most permissive one applies
        let allowed = [
            Path::new("/home"),
            Path::new("/home/user"),
            Path::new("/home/user/project"),
        ];
        for a in &allowed {
            assert!(is_subpath(a, Path::new("/home/user/project/anything")));
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_is_subpath_windows_case_insensitive_components() {
        let allowed = Path::new(r"C:\Users\Project");

        assert!(is_subpath(allowed, Path::new(r"c:\users\project")));
        assert!(is_subpath(allowed, Path::new(r"c:\users\project\src")));
        assert!(!is_subpath(allowed, Path::new(r"c:\users\project2")));
    }
}
