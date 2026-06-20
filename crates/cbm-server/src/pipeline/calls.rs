use crate::pipeline::import_map::ImportMap;
use crate::pipeline::registry::{
    call_edge_properties_json, CallTargetKind, FileCallResolver, SymbolRegistry,
};
use crate::store::{Edge, Symbol};
use std::path::Path;

pub use crate::pipeline::registry::SymbolRegistry as Registry;

/// Build a project-wide symbol registry (replaces legacy `HashMap` helper).
pub fn build_symbol_registry(symbols: &[Symbol]) -> SymbolRegistry {
    SymbolRegistry::from_symbols(symbols)
}

/// Legacy alias — returns the registry (older tests used a plain `HashMap`).
pub fn build_name_registry(symbols: &[Symbol]) -> SymbolRegistry {
    build_symbol_registry(symbols)
}

/// Resolve CALLS edges using registry + per-file import map.
pub fn resolve_calls_with_registry(
    symbols: &[Symbol],
    content: &str,
    language: &str,
    registry: &SymbolRegistry,
    caller_file: &str,
) -> Vec<Edge> {
    resolve_calls_with_registry_root(symbols, content, language, registry, caller_file, None)
}

pub fn resolve_calls_with_registry_root(
    symbols: &[Symbol],
    content: &str,
    language: &str,
    registry: &SymbolRegistry,
    caller_file: &str,
    repo_root: Option<&Path>,
) -> Vec<Edge> {
    let imports = ImportMap::parse_with_root(caller_file, language, content, repo_root);
    let mut resolver = FileCallResolver::new(registry, caller_file, imports);

    if matches!(
        language,
        "rust"
            | "python"
            | "javascript"
            | "jsx"
            | "typescript"
            | "tsx"
            | "go"
            | "java"
            | "c"
            | "cpp"
            | "csharp"
    ) {
        let ast_edges =
            super::calls_ast::resolve_calls_ast(language, symbols, content, &mut resolver);
        if !ast_edges.is_empty() {
            return ast_edges;
        }
    }
    resolve_calls_regex(symbols, content, language, &mut resolver)
}

/// Resolve CALLS edges from symbol definitions (single-file registry).
pub fn resolve_calls(symbols: &[Symbol], content: &str, language: &str) -> Vec<Edge> {
    let file = symbols
        .first()
        .map(|s| s.file_path.as_str())
        .unwrap_or("unknown");
    let registry = build_symbol_registry(symbols);
    resolve_calls_with_registry(symbols, content, language, &registry, file)
}

fn resolve_calls_regex(
    symbols: &[Symbol],
    content: &str,
    _language: &str,
    resolver: &mut FileCallResolver<'_>,
) -> Vec<Edge> {
    let call_patterns: &[(&regex::Regex, CallTargetKind)] = &[
        (
            &regex::Regex::new(r"\b(\w+)\s*\(").unwrap(),
            CallTargetKind::FreeFunction,
        ),
        (
            &regex::Regex::new(r"\.(\w+)\s*\(").unwrap(),
            CallTargetKind::Method,
        ),
    ];

    let mut edges = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for sym in symbols {
        if sym.label != "Function" && sym.label != "Method" {
            continue;
        }
        let start = sym.line_start.saturating_sub(1) as usize;
        let end = sym.line_end.min(lines.len() as i64) as usize;
        if start >= end {
            continue;
        }
        let body = lines[start..end].join("\n");
        let mut seen = std::collections::HashSet::new();
        for (re, kind) in call_patterns {
            for cap in re.captures_iter(&body) {
                if let Some(name_match) = cap.get(1) {
                    let callee_name = name_match.as_str();
                    if callee_name == sym.name || is_regex_noise(callee_name) {
                        continue;
                    }
                    let parent_class =
                        crate::pipeline::registry::parent_class_from_props(&sym.properties_json);
                    let res = match (*kind, parent_class.as_deref()) {
                        (CallTargetKind::Method, parent) => {
                            resolver.resolve_kind_scoped(callee_name, *kind, parent)
                        }
                        (CallTargetKind::FreeFunction, Some(parent)) => resolver
                            .resolve_kind_scoped(callee_name, CallTargetKind::Method, Some(parent))
                            .or_else(|| resolver.resolve_kind(callee_name, *kind)),
                        _ => resolver.resolve_kind(callee_name, *kind),
                    };
                    if let Some(res) = res {
                        if res.qn == sym.qualified_name {
                            continue;
                        }
                        let key = (sym.qualified_name.clone(), res.qn.clone());
                        if seen.insert(key.clone()) {
                            edges.push(Edge {
                                src_qn: key.0,
                                dst_qn: key.1,
                                edge_type: "CALLS".into(),
                                properties_json: Some(call_edge_properties_json(
                                    callee_name,
                                    &res,
                                    "regex",
                                )),
                            });
                        }
                    }
                }
            }
        }
    }
    edges
}

fn is_regex_noise(name: &str) -> bool {
    matches!(
        name,
        "if" | "for"
            | "while"
            | "match"
            | "return"
            | "let"
            | "const"
            | "var"
            | "new"
            | "self"
            | "super"
            | "print"
            | "println"
            | "format"
            | "log"
            | "console"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_id::qualified_name;

    fn qn(file: &str, label: &str, name: &str, line: i64) -> String {
        qualified_name(file, label, name, line)
    }

    #[test]
    fn resolves_internal_calls() {
        let symbols = vec![
            Symbol {
                qualified_name: qn("a.rs", "Function", "main", 1),
                name: "main".into(),
                label: "Function".into(),
                file_path: "a.rs".into(),
                line_start: 1,
                line_end: 5,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("a.rs", "Function", "helper", 7),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "a.rs".into(),
                line_start: 7,
                line_end: 9,
                signature: None,
                properties_json: None,
            },
        ];
        let src = "fn main() {\n    helper();\n}\n\nfn helper() {}\n";
        let edges = resolve_calls(&symbols, src, "rust");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].src_qn, qn("a.rs", "Function", "main", 1));
        assert_eq!(edges[0].dst_qn, qn("a.rs", "Function", "helper", 7));
    }

    #[test]
    fn skips_ambiguous_cross_file_calls() {
        let symbols = vec![
            Symbol {
                qualified_name: qn("a.rs", "Function", "main", 1),
                name: "main".into(),
                label: "Function".into(),
                file_path: "a.rs".into(),
                line_start: 1,
                line_end: 3,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("b.rs", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "b.rs".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("c.rs", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "c.rs".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
        ];
        let src = "fn main() { helper(); }\n";
        let registry = build_symbol_registry(&symbols);
        let edges = resolve_calls_with_registry(&symbols[..1], src, "rust", &registry, "a.rs");
        assert!(
            edges.is_empty(),
            "ambiguous cross-file callee should not link"
        );
    }

    #[test]
    fn resolves_cross_file_via_python_import_alias() {
        let symbols = vec![
            Symbol {
                qualified_name: qn("main.py", "Function", "main", 3),
                name: "main".into(),
                label: "Function".into(),
                file_path: "main.py".into(),
                line_start: 3,
                line_end: 5,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("utils.py", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "utils.py".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("decoy.py", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "decoy.py".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
        ];
        let src = "from utils import helper as h\n\ndef main():\n    h()\n";
        let registry = build_symbol_registry(&symbols);
        let edges = resolve_calls_with_registry(&symbols[..1], src, "python", &registry, "main.py");
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("utils.py::"));
    }

    #[test]
    fn resolves_cross_file_via_python_import() {
        let symbols = vec![
            Symbol {
                qualified_name: qn("main.py", "Function", "main", 4),
                name: "main".into(),
                label: "Function".into(),
                file_path: "main.py".into(),
                line_start: 4,
                line_end: 6,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("utils.py", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "utils.py".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qn("decoy.py", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "decoy.py".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
        ];
        let src = "from utils import helper\n\ndef main():\n    helper()\n";
        let registry = build_symbol_registry(&symbols);
        let edges = resolve_calls_with_registry(&symbols[..1], src, "python", &registry, "main.py");
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("utils.py::"));
        assert!(edges[0]
            .properties_json
            .as_ref()
            .is_some_and(|p| p.contains("import_binding") || p.contains("ast")));
    }
}
