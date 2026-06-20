use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Per-file import bindings: local symbol name → resolved module path (repo-relative).
#[derive(Debug, Default, Clone)]
pub struct ImportMap {
    /// Imported bare name (e.g. `helper`) → target module path (`utils.py`).
    pub bindings: HashMap<String, String>,
    /// Local import alias → original symbol name (`h` → `helper` for `import helper as h`).
    pub symbol_aliases: HashMap<String, String>,
    /// Module paths imported wholesale (e.g. `utils` from `import utils`).
    pub modules: Vec<String>,
}

impl ImportMap {
    pub fn parse(file_path: &str, language: &str, content: &str) -> Self {
        Self::parse_with_root(file_path, language, content, None)
    }

    pub fn parse_with_root(
        file_path: &str,
        language: &str,
        content: &str,
        repo_root: Option<&Path>,
    ) -> Self {
        let mut map = ImportMap::default();
        let caller_dir = Path::new(file_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        match language {
            "python" => parse_python_imports(file_path, &caller_dir, content, &mut map),
            "rust" => parse_rust_imports(file_path, &caller_dir, content, &mut map),
            "javascript" | "typescript" | "tsx" | "jsx" => {
                parse_js_imports(file_path, &caller_dir, content, &mut map)
            }
            "go" => parse_go_imports(content, &mut map),
            "java" => parse_java_imports(content, &mut map),
            "php" => parse_php_imports(file_path, &caller_dir, content, repo_root, &mut map),
            _ => {}
        }
        map
    }

    pub fn is_reachable(&self, candidate_file: &str) -> bool {
        if self.bindings.is_empty() && self.modules.is_empty() {
            return false;
        }
        let norm = normalize_path(candidate_file);
        for module in &self.modules {
            if path_matches_module(&norm, module) {
                return true;
            }
        }
        for target in self.bindings.values() {
            if path_matches_module(&norm, target) {
                return true;
            }
        }
        false
    }

    pub fn target_files_for(&self, name: &str) -> Vec<String> {
        if let Some(target) = self.bindings.get(name) {
            return vec![target.clone()];
        }
        self.modules.clone()
    }
}

fn parse_python_imports(file_path: &str, caller_dir: &str, content: &str, map: &mut ImportMap) {
    let from_import = Regex::new(r"(?m)^\s*from\s+([\w.]+)\s+import\s+([^\n;#]+)").unwrap();
    for cap in from_import.captures_iter(content) {
        let module = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let names = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let target = resolve_python_module(file_path, caller_dir, module);
        for item in names.split(',') {
            let (local, imported) = parse_import_item(item.trim());
            if imported.is_empty() || imported == "*" {
                continue;
            }
            map.bindings.insert(local.to_string(), target.clone());
            if local != imported {
                map.symbol_aliases
                    .insert(local.to_string(), imported.to_string());
            }
        }
        map.modules.push(target);
    }

    let plain_import = Regex::new(r"(?m)^\s*import\s+([\w.]+)(?:\s+as\s+(\w+))?\s*$").unwrap();
    for cap in plain_import.captures_iter(content) {
        let module = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let alias = cap.get(2).map(|m| m.as_str());
        let target = resolve_python_module(file_path, caller_dir, module);
        let base = module.split('.').next_back().unwrap_or(module);
        let local = alias.unwrap_or(base);
        map.bindings.insert(local.to_string(), target.clone());
        if alias.is_some() {
            map.symbol_aliases
                .insert(local.to_string(), base.to_string());
        }
        map.modules.push(target);
    }
}

fn parse_import_item(item: &str) -> (&str, &str) {
    if let Some((imported, local)) = item.split_once(" as ") {
        let imported = imported.trim();
        let local = local.trim();
        if !imported.is_empty() && !local.is_empty() {
            return (local, imported);
        }
    }
    let imported = item.split_whitespace().next().unwrap_or("").trim();
    (imported, imported)
}

fn resolve_python_module(file_path: &str, caller_dir: &str, module: &str) -> String {
    let dotted = module.replace('.', "/");
    let candidates = [format!("{dotted}.py"), format!("{dotted}/__init__.py")];
    if let Some(hit) = candidates
        .iter()
        .find(|c| !c.starts_with('/') && !c.contains(".."))
    {
        return hit.clone();
    }
    let _ = (file_path, caller_dir);
    candidates[0].clone()
}

fn parse_rust_imports(_file_path: &str, _caller_dir: &str, content: &str, map: &mut ImportMap) {
    let use_re = Regex::new(r"(?m)^\s*use\s+([\w:]+)(?:::\{([^}]+)\})?").unwrap();
    for cap in use_re.captures_iter(content) {
        let path = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if let Some(items) = cap.get(2).map(|m| m.as_str()) {
            for item in items.split(',') {
                let name = item.trim();
                if !name.is_empty() {
                    map.bindings
                        .insert(name.to_string(), path_to_rs_module(path));
                }
            }
        } else {
            let simple = path.rsplit("::").next().unwrap_or(path);
            map.bindings
                .insert(simple.to_string(), path_to_rs_module(path));
        }
        map.modules.push(path_to_rs_module(path));
    }
}

fn path_to_rs_module(path: &str) -> String {
    let rel = path.replace("::", "/");
    if rel.ends_with(".rs") {
        rel
    } else {
        format!("{rel}.rs")
    }
}

fn parse_js_imports(file_path: &str, caller_dir: &str, content: &str, map: &mut ImportMap) {
    let named = Regex::new(r#"(?m)^\s*import\s+\{([^}]+)\}\s+from\s+['"]([^'"]+)['"]"#).unwrap();
    let default_import = Regex::new(r#"(?m)^\s*import\s+(\w+)\s+from\s+['"]([^'"]+)['"]"#).unwrap();
    let require_re =
        Regex::new(r#"(?m)(?:const|let|var)\s+(\w+)\s*=\s*require\(['"]([^'"]+)['"]\)"#).unwrap();

    for cap in named.captures_iter(content) {
        let names = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let from = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let target = resolve_js_module(file_path, caller_dir, from);
        for item in names.split(',') {
            let (local, imported) = parse_import_item(item.trim());
            if imported.is_empty() {
                continue;
            }
            map.bindings.insert(local.to_string(), target.clone());
            if local != imported {
                map.symbol_aliases
                    .insert(local.to_string(), imported.to_string());
            }
        }
        map.modules.push(target);
    }

    for cap in default_import.captures_iter(content) {
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let from = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let target = resolve_js_module(file_path, caller_dir, from);
        if !name.is_empty() {
            map.bindings.insert(name.to_string(), target.clone());
        }
        map.modules.push(target);
    }

    for cap in require_re.captures_iter(content) {
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let from = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let target = resolve_js_module(file_path, caller_dir, from);
        if !name.is_empty() {
            map.bindings.insert(name.to_string(), target);
        }
    }
}

fn resolve_js_module(file_path: &str, caller_dir: &str, from: &str) -> String {
    if from.starts_with('.') {
        let base = if caller_dir.is_empty() {
            PathBuf::from(Path::new(file_path).parent().unwrap_or(Path::new(".")))
        } else {
            PathBuf::from(caller_dir)
        };
        let joined = base.join(from);
        let mut normalized = normalize_path(&joined.to_string_lossy());
        while normalized.contains("/./") {
            normalized = normalized.replace("/./", "/");
        }
        if !normalized.ends_with(".js")
            && !normalized.ends_with(".ts")
            && !normalized.ends_with(".jsx")
            && !normalized.ends_with(".tsx")
        {
            normalized.push_str(".js");
        }
        return normalized;
    }
    format!("{from}.js")
}

fn parse_java_imports(content: &str, map: &mut ImportMap) {
    let import_re = Regex::new(r"(?m)^\s*import\s+(?:static\s+)?([\w.]+)\s*;").unwrap();
    for cap in import_re.captures_iter(content) {
        let path = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if path.is_empty() {
            continue;
        }
        if path.ends_with(".*") {
            let pkg = path.trim_end_matches(".*");
            map.modules.push(java_package_glob(pkg));
            continue;
        }
        let class_name = path.rsplit('.').next().unwrap_or(path);
        let target = java_import_to_file(path);
        map.bindings.insert(class_name.to_string(), target.clone());
        map.modules.push(target);
    }
}

fn java_import_to_file(import_path: &str) -> String {
    let parts: Vec<&str> = import_path.split('.').collect();
    if parts.len() < 2 {
        return format!("{import_path}.java");
    }
    let class_name = parts.last().copied().unwrap_or(import_path);
    let pkg = parts[..parts.len() - 1].join("/");
    format!("{pkg}/{class_name}.java")
}

fn java_package_glob(package: &str) -> String {
    format!("{}/", package.replace('.', "/"))
}

fn parse_php_imports(
    file_path: &str,
    caller_dir: &str,
    content: &str,
    repo_root: Option<&Path>,
    map: &mut ImportMap,
) {
    let psr4 = load_composer_psr4(file_path, repo_root);

    let use_re =
        Regex::new(r"(?m)^\s*use\s+(?:function|const\s+)?([\w\\]+)(?:\s+as\s+(\w+))?\s*;").unwrap();
    for cap in use_re.captures_iter(content) {
        let path = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if path.is_empty() || !path.contains('\\') {
            continue;
        }
        bind_php_use(path, cap.get(2).map(|m| m.as_str()), &psr4, map);
    }

    let grouped_use_re = Regex::new(r"(?m)^\s*use\s+([\w\\]+)\s*\\\s*\{([^}]+)\}\s*;").unwrap();
    for cap in grouped_use_re.captures_iter(content) {
        let prefix = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let items = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        for item in items.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (path, alias) = item
                .split_once(" as ")
                .map(|(p, a)| (format!("{prefix}\\{p}"), Some(a.trim())))
                .unwrap_or_else(|| (format!("{prefix}\\{item}"), None));
            bind_php_use(&path, alias, &psr4, map);
        }
    }

    let require_re =
        Regex::new(r#"(?i)(?:require|include)(?:_once)?\s*(?:\(\s*)?['"]([^'"]+)['"]"#).unwrap();
    for cap in require_re.captures_iter(content) {
        let target = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if target.is_empty() {
            continue;
        }
        let resolved = resolve_php_path(file_path, caller_dir, target);
        map.modules.push(resolved.clone());
        if let Some(stem) = Path::new(&resolved).file_stem().and_then(|s| s.to_str()) {
            map.bindings
                .entry(stem.to_string())
                .or_insert_with(|| resolved);
        }
    }
}

fn bind_php_use(
    use_path: &str,
    alias: Option<&str>,
    psr4: &[(String, String)],
    map: &mut ImportMap,
) {
    let class_name = alias.unwrap_or_else(|| use_path.rsplit('\\').next().unwrap_or(use_path));
    let target = php_use_to_file(use_path, psr4);
    map.bindings.insert(class_name.to_string(), target.clone());
    map.modules.push(target);
}

fn php_use_to_file(use_path: &str, psr4: &[(String, String)]) -> String {
    if let Some(path) = psr4_resolve(use_path, psr4) {
        return path;
    }
    let parts: Vec<&str> = use_path.split('\\').collect();
    if parts.len() < 2 {
        return format!("{use_path}.php");
    }
    let class_name = parts.last().copied().unwrap_or(use_path);
    let pkg = parts[..parts.len() - 1].join("/").to_ascii_lowercase();
    format!("{pkg}/{class_name}.php")
}

fn psr4_resolve(use_path: &str, psr4: &[(String, String)]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for (prefix, dir) in psr4 {
        if use_path.starts_with(prefix) {
            let suffix = &use_path[prefix.len()..];
            let rel = if suffix.is_empty() {
                String::new()
            } else {
                suffix.replace('\\', "/")
            };
            let class_name = use_path.rsplit('\\').next().unwrap_or(use_path);
            let path = if rel.is_empty() {
                format!("{dir}/{class_name}.php")
            } else {
                format!("{dir}/{rel}.php")
            };
            let normalized = normalize_path(&path);
            if best.as_ref().is_none_or(|(len, _)| prefix.len() > *len) {
                best = Some((prefix.len(), normalized));
            }
        }
    }
    best.map(|(_, p)| p)
}

fn resolve_php_path(file_path: &str, caller_dir: &str, target: &str) -> String {
    let normalized = target.replace('\\', "/");
    if normalized.starts_with('/') {
        let path = normalize_path(&normalized);
        return if path.ends_with(".php") {
            path
        } else {
            format!("{path}.php")
        };
    }

    let base = if caller_dir.is_empty() {
        PathBuf::from(Path::new(file_path).parent().unwrap_or(Path::new(".")))
    } else {
        PathBuf::from(caller_dir)
    };
    let rel = normalized.strip_prefix("./").unwrap_or(normalized.as_str());
    let joined = base.join(rel);
    let mut path = normalize_path(&joined.to_string_lossy());
    while path.contains("/./") {
        path = path.replace("/./", "/");
    }
    if !path.ends_with(".php") {
        path.push_str(".php");
    }
    path
}

fn load_composer_psr4(file_path: &str, repo_root: Option<&Path>) -> Vec<(String, String)> {
    let base = repo_root.unwrap_or(Path::new("."));
    let mut dir = base.join(Path::new(file_path).parent().unwrap_or(Path::new(".")));
    for _ in 0..8 {
        let composer = dir.join("composer.json");
        if composer.is_file() {
            if let Ok(raw) = std::fs::read_to_string(&composer) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    return parse_composer_psr4(&v, &dir, repo_root);
                }
            }
            break;
        }
        if !dir.pop() {
            break;
        }
    }
    Vec::new()
}

fn parse_composer_psr4(
    root: &serde_json::Value,
    composer_dir: &Path,
    repo_root: Option<&Path>,
) -> Vec<(String, String)> {
    let Some(psr4) = root
        .pointer("/autoload/psr-4")
        .or_else(|| root.pointer("/autoload-dev/psr-4"))
        .and_then(|v| v.as_object())
    else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for (prefix, dir_val) in psr4 {
        let Some(dir) = dir_val.as_str() else {
            continue;
        };
        let mut norm_prefix = prefix.replace('/', "\\");
        if !norm_prefix.ends_with('\\') {
            norm_prefix.push('\\');
        }
        let joined = composer_dir.join(dir);
        let base_path = repo_root
            .and_then(|root| joined.strip_prefix(root).ok())
            .unwrap_or(joined.as_path());
        let base = normalize_path(&base_path.to_string_lossy());
        entries.push((norm_prefix, base));
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    entries
}

fn parse_go_imports(content: &str, map: &mut ImportMap) {
    let import_block = Regex::new(r#"import\s+(?:\(([^)]*)\)|"([^"]+)")"#).unwrap();
    for cap in import_block.captures_iter(content) {
        let block = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str())
            .unwrap_or("");
        for line in block.lines() {
            let line = line.trim().trim_matches('"');
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let path = parts.last().copied().unwrap_or(line).trim_matches('"');
            let alias = if parts.len() > 1 {
                parts[0].to_string()
            } else {
                path.rsplit('/').next().unwrap_or(path).to_string()
            };
            map.bindings.insert(alias, format!("{path}.go"));
            map.modules.push(format!("{path}.go"));
        }
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn path_matches_module(candidate_file: &str, module_path: &str) -> bool {
    let c = normalize_path(candidate_file);
    let m = normalize_path(module_path);
    c == m
        || c.ends_with(&m)
        || m.ends_with(&c)
        || c.strip_suffix(".py")
            .is_some_and(|stem| m.starts_with(stem))
        || c.strip_suffix(".rs")
            .is_some_and(|stem| m.starts_with(stem))
        || c.strip_suffix(".js")
            .is_some_and(|stem| m.starts_with(stem))
        || c.strip_suffix(".ts")
            .is_some_and(|stem| m.starts_with(stem))
        || c.strip_suffix(".java")
            .is_some_and(|stem| m.starts_with(stem))
        || c.strip_suffix(".php")
            .is_some_and(|stem| m.starts_with(stem))
        || m.strip_suffix('/').is_some_and(|pkg| c.starts_with(pkg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_from_import_binds_helper_to_utils() {
        let src = "from utils import helper\n\ndef main():\n    pass\n";
        let map = ImportMap::parse("main.py", "python", src);
        assert_eq!(
            map.bindings.get("helper").map(String::as_str),
            Some("utils.py")
        );
    }

    #[test]
    fn python_from_import_alias_binds_local_name() {
        let src = "from utils import helper as h\n\ndef main():\n    pass\n";
        let map = ImportMap::parse("main.py", "python", src);
        assert_eq!(map.bindings.get("h").map(String::as_str), Some("utils.py"));
        assert_eq!(
            map.symbol_aliases.get("h").map(String::as_str),
            Some("helper")
        );
        assert_eq!(
            map.symbol_aliases.get("h").map(String::as_str),
            Some("helper")
        );
    }

    #[test]
    fn js_named_import_alias_binds_local_name() {
        let src = "import { helper as h } from './utils'\n";
        let map = ImportMap::parse("src/main.js", "javascript", src);
        assert_eq!(
            map.bindings.get("h").map(String::as_str),
            Some("src/utils.js")
        );
        assert_eq!(
            map.symbol_aliases.get("h").map(String::as_str),
            Some("helper")
        );
    }

    #[test]
    fn php_use_binds_class_to_namespace_path() {
        let src = "<?php\nuse Greeter\\Greeter;\n\nfunction main() {}\n";
        let map = ImportMap::parse("main.php", "php", src);
        assert_eq!(
            map.bindings.get("Greeter").map(String::as_str),
            Some("greeter/Greeter.php")
        );
    }

    #[test]
    fn php_require_once_adds_reachable_module() {
        let src = "<?php\nrequire_once 'config.php';\nclass App {}\n";
        let map = ImportMap::parse("main.php", "php", src);
        assert!(map.modules.iter().any(|m| m == "config.php"));
        assert!(map.is_reachable("config.php"));
    }

    #[test]
    fn php_require_resolves_relative_subdirectory() {
        let src = "<?php\nrequire 'lib/helper.php';\n";
        let map = ImportMap::parse("src/main.php", "php", src);
        assert!(map.modules.iter().any(|m| m == "src/lib/helper.php"));
    }

    #[test]
    fn php_grouped_use_binds_multiple_classes() {
        let src = "<?php\nuse App\\Models\\{User, Post};\n";
        let map = ImportMap::parse("main.php", "php", src);
        assert_eq!(
            map.bindings.get("User").map(String::as_str),
            Some("app/models/User.php")
        );
        assert_eq!(
            map.bindings.get("Post").map(String::as_str),
            Some("app/models/Post.php")
        );
    }

    #[test]
    fn php_psr4_maps_namespace_to_src_directory() {
        let psr4 = vec![(r"App\".to_string(), "src".to_string())];
        assert_eq!(
            php_use_to_file("App\\Service\\Helper", &psr4),
            "src/Service/Helper.php"
        );
    }

    #[test]
    fn php_psr4_with_repo_root_uses_repo_relative_path() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("composer.json"),
            r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#,
        )
        .unwrap();
        let src = "<?php\nuse App\\Service\\Helper;\n";
        let map = ImportMap::parse_with_root("main.php", "php", src, Some(dir.path()));
        assert_eq!(
            map.bindings.get("Helper").map(String::as_str),
            Some("src/Service/Helper.php")
        );
    }

    #[test]
    fn java_import_binds_class_to_package_path() {
        let src = "import greeter.Greeter;\n\nclass Main {\n  void main() {}\n}\n";
        let map = ImportMap::parse("Main.java", "java", src);
        assert_eq!(
            map.bindings.get("Greeter").map(String::as_str),
            Some("greeter/Greeter.java")
        );
    }

    #[test]
    fn js_relative_import_resolves_sibling() {
        let src = "import { helper } from './utils'\n";
        let map = ImportMap::parse("src/main.js", "javascript", src);
        assert_eq!(
            map.bindings.get("helper").map(String::as_str),
            Some("src/utils.js")
        );
    }
}
