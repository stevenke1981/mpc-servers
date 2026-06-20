use crate::pipeline::registry::{
    call_edge_properties_json, parent_class_from_props, CallResolution, CallTargetKind,
    FileCallResolver,
};
use crate::store::{Edge, Symbol};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

/// AST-aware CALLS using tree-sitter; returns edges with high confidence when parsing succeeds.
pub fn resolve_calls_ast(
    language: &str,
    symbols: &[Symbol],
    content: &str,
    resolver: &mut FileCallResolver<'_>,
) -> Vec<Edge> {
    let Some(lang) = language_to_tree_sitter(language) else {
        return Vec::new();
    };
    let Some(query_src) = call_query_for(language) else {
        return Vec::new();
    };

    let mut parser = Parser::new();
    if parser.set_language(&lang).is_err() {
        return Vec::new();
    }
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let Ok(query) = Query::new(&lang, query_src) else {
        return Vec::new();
    };

    let mut cursor = QueryCursor::new();
    let mut edges = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let functions: Vec<&Symbol> = symbols
        .iter()
        .filter(|s| s.label == "Function" || s.label == "Method")
        .collect();

    for sym in functions {
        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        while let Some(m) = matches.next() {
            let mut callee = String::new();
            let mut line = 0usize;
            let mut kind = CallTargetKind::FreeFunction;
            for cap in m.captures {
                let name = query.capture_names()[cap.index as usize];
                match name {
                    "callee" | "scoped" => {
                        callee = cap
                            .node
                            .utf8_text(content.as_bytes())
                            .unwrap_or("")
                            .to_string();
                        line = cap.node.start_position().row;
                        kind = CallTargetKind::FreeFunction;
                    }
                    "method" => {
                        callee = cap
                            .node
                            .utf8_text(content.as_bytes())
                            .unwrap_or("")
                            .to_string();
                        line = cap.node.start_position().row;
                        kind = CallTargetKind::Method;
                    }
                    _ => {}
                }
            }
            if callee.is_empty() || is_noise_callee(language, &callee) || callee == sym.name {
                continue;
            }
            let call_line = (line + 1) as i64;
            if call_line < sym.line_start || call_line > sym.line_end {
                continue;
            }
            let parent_class = parent_class_from_props(&sym.properties_json);
            let res = match (kind, parent_class.as_deref()) {
                (CallTargetKind::Method, parent) => {
                    resolver.resolve_kind_scoped(&callee, kind, parent)
                }
                (CallTargetKind::FreeFunction, Some(parent)) => resolver
                    .resolve_kind_scoped(&callee, CallTargetKind::Method, Some(parent))
                    .or_else(|| resolver.resolve_kind(&callee, kind)),
                _ => resolver.resolve_kind(&callee, kind),
            };
            if let Some(res) = res {
                push_edge(&mut edges, &mut seen, sym, &callee, &res, "ast");
            }
        }
    }
    edges
}

fn language_to_tree_sitter(language: &str) -> Option<Language> {
    Some(match language {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" | "tsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "cpp" => tree_sitter_cpp::LANGUAGE.into(),
        "csharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        _ => return None,
    })
}

fn call_query_for(language: &str) -> Option<&'static str> {
    Some(match language {
        "rust" => {
            r#"
(call_expression
  function: (identifier) @callee)
(call_expression
  function: (field_expression
    field: (field_identifier) @method))
(call_expression
  function: (scoped_identifier
    name: (identifier) @scoped))
"#
        }
        "python" => {
            r#"
(call
  function: (identifier) @callee)
(call
  function: (attribute
    attribute: (identifier) @method))
"#
        }
        "javascript" | "jsx" | "typescript" | "tsx" => {
            r#"
(call_expression
  function: (identifier) @callee)
(call_expression
  function: (member_expression
    property: (property_identifier) @method))
"#
        }
        "go" => {
            r#"
(call_expression
  function: (identifier) @callee)
(call_expression
  function: (selector_expression
    field: (field_identifier) @method))
"#
        }
        "java" => {
            r#"
(method_invocation
  name: (identifier) @callee)
(method_invocation
  object: (_)
  name: (identifier) @method)
"#
        }
        "c" | "cpp" => {
            r#"
(call_expression
  function: (identifier) @callee)
(call_expression
  function: (field_expression
    argument: (_)
    field: (field_identifier) @method))
"#
        }
        "csharp" => {
            r#"
(invocation_expression
  function: (identifier) @callee)
(invocation_expression
  function: (member_access_expression
    name: (identifier) @method))
"#
        }
        _ => return None,
    })
}

fn is_noise_callee(language: &str, name: &str) -> bool {
    match language {
        "rust" => matches!(
            name,
            "if" | "for"
                | "while"
                | "match"
                | "return"
                | "let"
                | "loop"
                | "move"
                | "async"
                | "await"
        ),
        "python" => matches!(name, "if" | "for" | "while" | "return" | "print" | "len"),
        "javascript" | "jsx" | "typescript" | "tsx" => {
            matches!(
                name,
                "if" | "for" | "while" | "return" | "console" | "require" | "log"
            )
        }
        "csharp" => matches!(
            name,
            "if" | "for" | "while" | "return" | "new" | "typeof" | "nameof"
        ),
        _ => false,
    }
}

fn push_edge(
    edges: &mut Vec<Edge>,
    seen: &mut std::collections::HashSet<(String, String)>,
    caller: &Symbol,
    callee: &str,
    res: &CallResolution,
    method: &str,
) {
    if res.qn == caller.qualified_name {
        return;
    }
    let key = (caller.qualified_name.clone(), res.qn.clone());
    if seen.insert(key) {
        edges.push(Edge {
            src_qn: caller.qualified_name.clone(),
            dst_qn: res.qn.clone(),
            edge_type: "CALLS".into(),
            properties_json: Some(call_edge_properties_json(callee, res, method)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::calls::build_symbol_registry;
    use crate::pipeline::extract::extract_symbols;
    use crate::pipeline::import_map::ImportMap;
    use crate::pipeline::registry::FileCallResolver;

    #[test]
    fn python_import_alias_call_resolves() {
        use crate::pipeline::calls::build_symbol_registry;
        use crate::pipeline::import_map::ImportMap;
        use crate::symbol_id::qualified_name;

        let src = "from utils import helper as h\n\ndef main():\n    h()\n";
        let symbols = vec![
            Symbol {
                qualified_name: qualified_name("main.py", "Function", "main", 3),
                name: "main".into(),
                label: "Function".into(),
                file_path: "main.py".into(),
                line_start: 3,
                line_end: 5,
                signature: None,
                properties_json: None,
            },
            Symbol {
                qualified_name: qualified_name("utils.py", "Function", "helper", 1),
                name: "helper".into(),
                label: "Function".into(),
                file_path: "utils.py".into(),
                line_start: 1,
                line_end: 2,
                signature: None,
                properties_json: None,
            },
        ];
        let registry = build_symbol_registry(&symbols);
        let imports = ImportMap::parse("main.py", "python", src);
        assert!(imports.bindings.contains_key("h"));
        let mut resolver = FileCallResolver::new(&registry, "main.py", imports);
        let edges = resolve_calls_ast("python", &symbols[..1], src, &mut resolver);
        assert!(
            !edges.is_empty(),
            "expected python alias AST edge; resolver={:?}",
            resolver.resolve("h")
        );
    }

    #[test]
    fn csharp_local_method_call_resolves() {
        let src = "class App {\n    static void Main() {\n        Helper();\n    }\n    static void Helper() {}\n}\n";
        let symbols = extract_symbols("App.cs", "csharp", src).unwrap();
        assert!(
            symbols.iter().any(|s| s.name == "Main"),
            "symbols: {:?}",
            symbols
        );
        let registry = build_symbol_registry(&symbols);
        let imports = ImportMap::parse("App.cs", "csharp", src);
        let mut resolver = FileCallResolver::new(&registry, "App.cs", imports);
        let edges = resolve_calls_ast("csharp", &symbols, src, &mut resolver);
        assert!(!edges.is_empty(), "expected AST edges; symbols={symbols:?}");
    }
}
