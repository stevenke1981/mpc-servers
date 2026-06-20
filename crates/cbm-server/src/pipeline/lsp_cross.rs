//! Cross-file LSP-style call resolution (reference `pass_lsp_cross.c` parity slice).
//!
//! In-process type-aware resolver — not an external language-server subprocess.
//! Supports Python, JavaScript/TypeScript, Go, and Java imported-type method dispatch.

use crate::pipeline::import_map::ImportMap;
use crate::pipeline::registry::{
    call_edge_properties_json, confidence_band, parent_class_from_props, CallResolution,
};
use crate::store::{Edge, SourceFile, Symbol};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor};

const LSP_CROSS_CONFIDENCE: f64 = 0.85;

const JS_LANGS: &[&str] = &["javascript", "jsx", "typescript", "tsx"];

#[allow(dead_code)]
pub fn resolve_cross_file_calls(symbols: &[Symbol], files: &[SourceFile]) -> Vec<Edge> {
    resolve_cross_file_calls_root(symbols, files, None)
}

pub fn resolve_cross_file_calls_root(
    symbols: &[Symbol],
    files: &[SourceFile],
    repo_root: Option<&Path>,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    let class_index = build_class_index(symbols);
    let methods_by_file = build_methods_by_file(symbols);

    for file in files {
        let file_syms: Vec<&Symbol> = symbols
            .iter()
            .filter(|s| s.file_path == file.path && is_callable_symbol(s))
            .collect();
        if file_syms.is_empty() {
            continue;
        }

        match file.language.as_str() {
            "python" => {
                let imports = ImportMap::parse_with_root(
                    &file.path,
                    &file.language,
                    &file.content,
                    repo_root,
                );
                let bindings = infer_python_type_bindings(&file.content, &imports);
                edges.extend(resolve_attribute_calls(
                    AttributeCallConfig {
                        language: tree_sitter_python::LANGUAGE.into(),
                        query_src: PYTHON_ATTR_QUERY,
                    },
                    &file.path,
                    &file.content,
                    &file_syms,
                    &imports,
                    &bindings,
                    &class_index,
                    &methods_by_file,
                ));
            }
            lang if JS_LANGS.contains(&lang) => {
                let imports = ImportMap::parse_with_root(
                    &file.path,
                    &file.language,
                    &file.content,
                    repo_root,
                );
                let bindings = infer_js_type_bindings(&file.content, &imports);
                let ts_lang = if matches!(lang, "typescript" | "tsx") {
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
                } else {
                    tree_sitter_javascript::LANGUAGE.into()
                };
                edges.extend(resolve_attribute_calls(
                    AttributeCallConfig {
                        language: ts_lang,
                        query_src: JS_ATTR_QUERY,
                    },
                    &file.path,
                    &file.content,
                    &file_syms,
                    &imports,
                    &bindings,
                    &class_index,
                    &methods_by_file,
                ));
            }
            "go" => {
                let imports = ImportMap::parse_with_root(
                    &file.path,
                    &file.language,
                    &file.content,
                    repo_root,
                );
                let bindings =
                    infer_go_type_bindings(&file.content, &imports, &class_index, &methods_by_file);
                edges.extend(resolve_attribute_calls(
                    AttributeCallConfig {
                        language: tree_sitter_go::LANGUAGE.into(),
                        query_src: GO_ATTR_QUERY,
                    },
                    &file.path,
                    &file.content,
                    &file_syms,
                    &imports,
                    &bindings,
                    &class_index,
                    &methods_by_file,
                ));
            }
            "java" => {
                let imports = ImportMap::parse_with_root(
                    &file.path,
                    &file.language,
                    &file.content,
                    repo_root,
                );
                let bindings = infer_java_type_bindings(&file.content, &imports);
                edges.extend(resolve_attribute_calls(
                    AttributeCallConfig {
                        language: tree_sitter_java::LANGUAGE.into(),
                        query_src: JAVA_ATTR_QUERY,
                    },
                    &file.path,
                    &file.content,
                    &file_syms,
                    &imports,
                    &bindings,
                    &class_index,
                    &methods_by_file,
                ));
            }
            "php" => {
                let imports = ImportMap::parse_with_root(
                    &file.path,
                    &file.language,
                    &file.content,
                    repo_root,
                );
                let bindings = infer_php_type_bindings(&file.content, &imports);
                edges.extend(resolve_attribute_calls(
                    AttributeCallConfig {
                        language: tree_sitter_php::LANGUAGE_PHP_ONLY.into(),
                        query_src: PHP_ATTR_QUERY,
                    },
                    &file.path,
                    &file.content,
                    &file_syms,
                    &imports,
                    &bindings,
                    &class_index,
                    &methods_by_file,
                ));
            }
            _ => {}
        }
    }
    edges
}

const PYTHON_ATTR_QUERY: &str = r#"
(call
  function: (attribute
    object: (_) @recv
    attribute: (identifier) @method))
"#;

const JS_ATTR_QUERY: &str = r#"
(call_expression
  function: (member_expression
    object: (_) @recv
    property: (property_identifier) @method))
"#;

const GO_ATTR_QUERY: &str = r#"
(call_expression
  function: (selector_expression
    operand: (_) @recv
    field: (field_identifier) @method))
"#;

const JAVA_ATTR_QUERY: &str = r#"
(method_invocation
  object: (_) @recv
  name: (identifier) @method)
"#;

const PHP_ATTR_QUERY: &str = r#"
(member_call_expression
  object: (_) @recv
  name: (name) @method)
"#;

struct AttributeCallConfig {
    language: Language,
    query_src: &'static str,
}

fn build_class_index(symbols: &[Symbol]) -> HashMap<String, Vec<ClassEntry>> {
    let mut index: HashMap<String, Vec<ClassEntry>> = HashMap::new();
    for sym in symbols {
        if sym.label != "Class" {
            continue;
        }
        index.entry(sym.name.clone()).or_default().push(ClassEntry {
            file: sym.file_path.clone(),
            line: sym.line_start,
        });
    }
    index
}

fn is_callable_symbol(sym: &Symbol) -> bool {
    sym.label == "Function" || sym.label == "Method"
}

fn is_method_entry(sym: &Symbol) -> bool {
    sym.label == "Method" || parent_class_from_props(&sym.properties_json).is_some()
}

fn build_methods_by_file(symbols: &[Symbol]) -> HashMap<String, Vec<MethodEntry>> {
    let mut by_file: HashMap<String, Vec<MethodEntry>> = HashMap::new();
    for sym in symbols {
        if !is_method_entry(sym) {
            continue;
        }
        by_file
            .entry(sym.file_path.clone())
            .or_default()
            .push(MethodEntry {
                name: sym.name.clone(),
                qn: sym.qualified_name.clone(),
                line: sym.line_start,
                parent_class: parent_class_from_props(&sym.properties_json),
            });
    }
    for methods in by_file.values_mut() {
        methods.sort_by_key(|m| m.line);
    }
    by_file
}

#[derive(Debug, Clone)]
struct ClassEntry {
    file: String,
    line: i64,
}

#[derive(Debug, Clone)]
struct MethodEntry {
    name: String,
    qn: String,
    line: i64,
    parent_class: Option<String>,
}

fn infer_python_type_bindings(content: &str, imports: &ImportMap) -> HashMap<String, String> {
    let mut bindings = import_name_bindings(imports);
    let assign_re = regex::Regex::new(r"(?m)^\s*(\w+)\s*=\s*(\w+)\s*\(").expect("assign regex");
    for cap in assign_re.captures_iter(content) {
        let var = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let class_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        if !var.is_empty() && !class_name.is_empty() {
            bindings.insert(var.to_string(), class_name.to_string());
        }
    }
    bindings
}

fn infer_js_type_bindings(content: &str, imports: &ImportMap) -> HashMap<String, String> {
    let mut bindings = import_name_bindings(imports);
    let new_re = regex::Regex::new(r"(?m)(?:const|let|var)\s+(\w+)\s*=\s*new\s+(\w+)\s*\(")
        .expect("js new regex");
    for cap in new_re.captures_iter(content) {
        let var = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let class_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        if !var.is_empty() && !class_name.is_empty() {
            bindings.insert(var.to_string(), class_name.to_string());
        }
    }
    let ctor_re = regex::Regex::new(r"(?m)(?:const|let|var)\s+(\w+)\s*=\s*(\w+)\s*\(")
        .expect("js ctor regex");
    for cap in ctor_re.captures_iter(content) {
        let var = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let class_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        if !var.is_empty() && !class_name.is_empty() {
            bindings
                .entry(var.to_string())
                .or_insert(class_name.to_string());
        }
    }
    bindings
}

fn infer_php_type_bindings(content: &str, imports: &ImportMap) -> HashMap<String, String> {
    let mut bindings = import_name_bindings(imports);
    let assign_re =
        regex::Regex::new(r"(?m)\$(\w+)\s*=\s*new\s+(\w+)\s*\(").expect("php assign regex");
    for cap in assign_re.captures_iter(content) {
        let var = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let class_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        if !var.is_empty() && !class_name.is_empty() {
            bindings.insert(var.to_string(), class_name.to_string());
        }
    }
    bindings
}

fn infer_java_type_bindings(content: &str, imports: &ImportMap) -> HashMap<String, String> {
    let mut bindings = import_name_bindings(imports);
    let assign_re =
        regex::Regex::new(r"(?m)(\w+)\s+(\w+)\s*=\s*new\s+(\w+)\s*\(").expect("java assign regex");
    for cap in assign_re.captures_iter(content) {
        let var = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let class_name = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        if !var.is_empty() && !class_name.is_empty() {
            bindings.insert(var.to_string(), class_name.to_string());
        }
    }
    bindings
}

fn infer_go_type_bindings(
    content: &str,
    imports: &ImportMap,
    class_index: &HashMap<String, Vec<ClassEntry>>,
    methods_by_file: &HashMap<String, Vec<MethodEntry>>,
) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    let new_re = regex::Regex::new(r"(\w+)\s*:=\s*(\w+)\.New\w*\(").expect("go new regex");
    for cap in new_re.captures_iter(content) {
        let var = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let pkg = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        if var.is_empty() || pkg.is_empty() {
            continue;
        }
        let Some(target) = imports.bindings.get(pkg) else {
            continue;
        };
        if let Some(class_name) = guess_go_struct_for_file(target, class_index, methods_by_file) {
            bindings.insert(var.to_string(), class_name);
        }
    }
    bindings
}

fn guess_go_struct_for_file(
    target: &str,
    class_index: &HashMap<String, Vec<ClassEntry>>,
    methods_by_file: &HashMap<String, Vec<MethodEntry>>,
) -> Option<String> {
    let norm = target.replace('\\', "/");
    for (class_name, entries) in class_index {
        if entries.iter().any(|e| path_matches(&e.file, &norm))
            && methods_by_file.get(&entries[0].file).is_some()
        {
            return Some(class_name.clone());
        }
    }
    None
}

fn import_name_bindings(imports: &ImportMap) -> HashMap<String, String> {
    imports
        .bindings
        .keys()
        .map(|local| (local.clone(), local.clone()))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn resolve_attribute_calls(
    config: AttributeCallConfig,
    file_path: &str,
    content: &str,
    functions: &[&Symbol],
    imports: &ImportMap,
    bindings: &HashMap<String, String>,
    class_index: &HashMap<String, Vec<ClassEntry>>,
    methods_by_file: &HashMap<String, Vec<MethodEntry>>,
) -> Vec<Edge> {
    let mut parser = Parser::new();
    if parser.set_language(&config.language).is_err() {
        return Vec::new();
    }
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let Ok(query) = Query::new(&config.language, config.query_src) else {
        return Vec::new();
    };

    let mut cursor = QueryCursor::new();
    let mut edges = Vec::new();
    let mut seen = HashSet::new();

    for caller in functions {
        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        while let Some(m) = matches.next() {
            let mut recv_node = None;
            let mut method_name = String::new();
            let mut call_line = 0i64;
            for cap in m.captures {
                let name = query.capture_names()[cap.index as usize];
                if name == "recv" {
                    recv_node = Some(cap.node);
                } else if name == "method" {
                    method_name = cap
                        .node
                        .utf8_text(content.as_bytes())
                        .unwrap_or("")
                        .to_string();
                    call_line = (cap.node.start_position().row + 1) as i64;
                }
            }
            if method_name.is_empty()
                || call_line < caller.line_start
                || call_line > caller.line_end
            {
                continue;
            }
            let Some(recv) = recv_node else {
                continue;
            };
            let Some(class_name) = infer_receiver_class(recv, content, bindings, imports) else {
                continue;
            };
            let Some(res) = resolve_class_method(
                &class_name,
                &method_name,
                file_path,
                imports,
                class_index,
                methods_by_file,
            ) else {
                continue;
            };
            push_lsp_edge(&mut edges, &mut seen, caller, &method_name, &res);
        }
    }
    edges
}

fn infer_receiver_class(
    recv: tree_sitter::Node,
    content: &str,
    bindings: &HashMap<String, String>,
    imports: &ImportMap,
) -> Option<String> {
    match recv.kind() {
        "identifier" => {
            let name = recv.utf8_text(content.as_bytes()).ok()?;
            if let Some(class) = bindings.get(name) {
                return Some(class.clone());
            }
            if imports.bindings.contains_key(name) {
                return Some(name.to_string());
            }
            None
        }
        "call" | "call_expression" => infer_call_constructor(recv, content),
        "new_expression" => {
            let ctor = recv.child_by_field_name("constructor")?;
            if ctor.kind() == "identifier" {
                return ctor.utf8_text(content.as_bytes()).ok().map(str::to_string);
            }
            None
        }
        "object_creation_expression" => {
            if let Some(type_node) = recv.child_by_field_name("type") {
                if let Some(class) = java_type_name(type_node, content) {
                    return Some(class);
                }
            }
            php_object_class(recv, content)
        }
        "variable_name" => {
            let text = recv.utf8_text(content.as_bytes()).ok()?;
            let name = text.strip_prefix('$').unwrap_or(text);
            if let Some(class) = bindings.get(name) {
                return Some(class.clone());
            }
            if imports.bindings.contains_key(name) {
                return Some(name.to_string());
            }
            None
        }
        "parenthesized_expression" => {
            let mut cursor = recv.walk();
            for child in recv.children(&mut cursor) {
                if let Some(class) = infer_receiver_class(child, content, bindings, imports) {
                    return Some(class);
                }
            }
            None
        }
        _ => None,
    }
}

fn php_object_class(recv: tree_sitter::Node, content: &str) -> Option<String> {
    if let Some(class_node) = recv.child_by_field_name("class") {
        if let Some(class) = php_type_name(class_node, content) {
            return Some(class);
        }
    }
    let mut cursor = recv.walk();
    for child in recv.children(&mut cursor) {
        if child.kind() == "name" {
            return php_type_name(child, content);
        }
    }
    None
}

fn php_type_name(node: tree_sitter::Node, content: &str) -> Option<String> {
    let text = node.utf8_text(content.as_bytes()).ok()?;
    Some(text.rsplit('\\').next().unwrap_or(text).to_string())
}

fn java_type_name(node: tree_sitter::Node, content: &str) -> Option<String> {
    match node.kind() {
        "type_identifier" | "identifier" => {
            node.utf8_text(content.as_bytes()).ok().map(str::to_string)
        }
        "scoped_type_identifier" => {
            let text = node.utf8_text(content.as_bytes()).ok()?;
            text.rsplit('.').next().map(str::to_string)
        }
        _ => None,
    }
}

fn infer_call_constructor(recv: tree_sitter::Node, content: &str) -> Option<String> {
    let func = recv.child_by_field_name("function")?;
    match func.kind() {
        "identifier" => func.utf8_text(content.as_bytes()).ok().map(str::to_string),
        "selector_expression" => {
            let field = func.child_by_field_name("field")?;
            let op = func.child_by_field_name("operand")?;
            if field.utf8_text(content.as_bytes()).ok()? != "New"
                && !field.utf8_text(content.as_bytes()).ok()?.starts_with("New")
            {
                return None;
            }
            if op.kind() == "identifier" {
                return op.utf8_text(content.as_bytes()).ok().map(str::to_string);
            }
            None
        }
        _ => None,
    }
}

fn resolve_class_method(
    class_name: &str,
    method_name: &str,
    caller_file: &str,
    imports: &ImportMap,
    class_index: &HashMap<String, Vec<ClassEntry>>,
    methods_by_file: &HashMap<String, Vec<MethodEntry>>,
) -> Option<CallResolution> {
    let candidates = class_index.get(class_name)?;
    let scoped: Vec<&ClassEntry> = if let Some(target) = imports.bindings.get(class_name) {
        candidates
            .iter()
            .filter(|c| path_matches(&c.file, target))
            .collect()
    } else {
        candidates
            .iter()
            .filter(|c| imports.is_reachable(&c.file) || c.file == caller_file)
            .collect()
    };
    if scoped.len() != 1 {
        return None;
    }
    let class_entry = scoped[0];
    let methods = methods_by_file.get(&class_entry.file)?;
    let class_methods: Vec<&MethodEntry> = methods
        .iter()
        .filter(|m| {
            m.name == method_name
                && m.line > class_entry.line
                && m.parent_class.as_ref().is_none_or(|p| p == class_name)
        })
        .collect();
    if class_methods.len() != 1 {
        return None;
    }
    Some(CallResolution {
        qn: class_methods[0].qn.clone(),
        strategy: "lsp_cross".into(),
        confidence: LSP_CROSS_CONFIDENCE,
        band: confidence_band(LSP_CROSS_CONFIDENCE).to_string(),
        candidates: class_methods.len(),
    })
}

fn path_matches(file: &str, module: &str) -> bool {
    let norm_file = file.replace('\\', "/");
    let norm_mod = module.replace('\\', "/");
    norm_file == norm_mod
        || norm_file.ends_with(&norm_mod)
        || norm_mod.ends_with(&norm_file)
        || norm_file
            .strip_suffix(".py")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".js")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".ts")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".jsx")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".tsx")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".go")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".java")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_file
            .strip_suffix(".php")
            .is_some_and(|s| norm_mod.starts_with(s))
}

fn push_lsp_edge(
    edges: &mut Vec<Edge>,
    seen: &mut HashSet<(String, String)>,
    caller: &Symbol,
    callee: &str,
    res: &CallResolution,
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
            properties_json: Some(call_edge_properties_json(callee, res, "lsp_cross")),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_id::qualified_name;

    fn sym(file: &str, label: &str, name: &str, line: i64) -> Symbol {
        Symbol {
            qualified_name: qualified_name(file, label, name, line),
            name: name.into(),
            label: label.into(),
            file_path: file.into(),
            line_start: line,
            line_end: line + 5,
            signature: None,
            properties_json: None,
        }
    }

    #[test]
    fn resolves_imported_python_class_method() {
        let symbols = vec![
            sym("main.py", "Function", "main", 3),
            sym("greeter.py", "Class", "Greeter", 1),
            sym("greeter.py", "Method", "greet", 2),
        ];
        let files = vec![SourceFile {
            path: "main.py".into(),
            language: "python".into(),
            content: "from greeter import Greeter\n\ndef main():\n    Greeter().greet()\n".into(),
            line_count: 4,
            mtime_ns: None,
            size_bytes: None,
        }];
        let edges = resolve_cross_file_calls(&symbols, &files);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("greeter.py::"));
        assert!(edges[0].dst_qn.contains("greet"));
    }

    #[test]
    fn resolves_imported_js_class_method() {
        let symbols = vec![
            sym("main.js", "Function", "main", 3),
            sym("greeter.js", "Class", "Greeter", 1),
            sym("greeter.js", "Method", "greet", 2),
        ];
        let files = vec![SourceFile {
            path: "main.js".into(),
            language: "javascript".into(),
            content: "import { Greeter } from './greeter';\n\nfunction main() {\n  new Greeter().greet();\n}\n"
                .into(),
            line_count: 5,
            mtime_ns: None,
            size_bytes: None,
        }];
        let edges = resolve_cross_file_calls(&symbols, &files);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("greeter.js::"));
        assert!(edges[0].dst_qn.contains("greet"));
    }

    #[test]
    fn resolves_imported_java_class_method() {
        let symbols = vec![
            sym("Main.java", "Function", "main", 4),
            sym("greeter/Greeter.java", "Class", "Greeter", 1),
            sym("greeter/Greeter.java", "Method", "greet", 2),
        ];
        let files = vec![SourceFile {
            path: "Main.java".into(),
            language: "java".into(),
            content: "import greeter.Greeter;\n\nclass Main {\n  void main() {\n    new Greeter().greet();\n  }\n}\n"
                .into(),
            line_count: 7,
            mtime_ns: None,
            size_bytes: None,
        }];
        let edges = resolve_cross_file_calls(&symbols, &files);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("greeter/Greeter.java::"));
        assert!(edges[0].dst_qn.contains("greet"));
    }

    #[test]
    fn resolves_imported_php_class_method() {
        let symbols = vec![
            sym("main.php", "Function", "main", 4),
            sym("greeter/Greeter.php", "Class", "Greeter", 3),
            sym("greeter/Greeter.php", "Method", "greet", 4),
        ];
        let files = vec![SourceFile {
            path: "main.php".into(),
            language: "php".into(),
            content: "<?php\nuse Greeter\\Greeter;\n\nfunction main() {\n    (new Greeter())->greet();\n}\n"
                .into(),
            line_count: 6,
            mtime_ns: None,
            size_bytes: None,
        }];
        let edges = resolve_cross_file_calls(&symbols, &files);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("greeter/Greeter.php::"));
        assert!(edges[0].dst_qn.contains("greet"));
    }

    #[test]
    fn resolves_imported_go_struct_method() {
        let symbols = vec![
            sym("main.go", "Function", "main", 5),
            sym("greeter.go", "Class", "Greeter", 3),
            sym("greeter.go", "Method", "Greet", 4),
        ];
        let files = vec![SourceFile {
            path: "main.go".into(),
            language: "go".into(),
            content: "package main\n\nimport \"greeter\"\n\nfunc main() {\n  g := greeter.NewGreeter()\n  g.Greet()\n}\n"
                .into(),
            line_count: 8,
            mtime_ns: None,
            size_bytes: None,
        }];
        let edges = resolve_cross_file_calls(&symbols, &files);
        assert_eq!(edges.len(), 1);
        assert!(edges[0].dst_qn.starts_with("greeter.go::"));
        assert!(edges[0].dst_qn.contains("Greet"));
    }
}
