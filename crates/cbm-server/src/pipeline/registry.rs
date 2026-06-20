use crate::pipeline::import_map::ImportMap;
use crate::store::Symbol;
use std::collections::HashMap;

const MAX_CANDIDATES: usize = 256;
const CONF_IMPORT_BINDING: f64 = 0.95;
const CONF_SAME_FILE: f64 = 0.90;
const CONF_UNIQUE_NAME: f64 = 0.75;
const CONF_IMPORT_FILTERED: f64 = 0.55;
const CONF_IMPORT_FILTERED_PENALTY: f64 = 0.30;

/// Whether a call site targets a free function or an instance/class method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallTargetKind {
    FreeFunction,
    Method,
    Any,
}

/// Resolution result aligned with reference `cbm_resolution_t`.
#[derive(Debug, Clone, PartialEq)]
pub struct CallResolution {
    pub qn: String,
    pub strategy: String,
    pub confidence: f64,
    pub band: String,
    pub candidates: usize,
}

/// Project-wide symbol registry for call resolution (reference `cbm_registry_t`).
#[derive(Debug, Default)]
pub struct SymbolRegistry {
    exact: HashMap<String, String>,
    by_name: HashMap<String, Vec<String>>,
    parent_class: HashMap<String, String>,
}

impl SymbolRegistry {
    pub fn from_symbols(symbols: &[Symbol]) -> Self {
        let mut reg = Self::default();
        for sym in symbols {
            reg.add(sym);
        }
        reg
    }

    pub fn add(&mut self, sym: &Symbol) {
        if self.exact.contains_key(&sym.qualified_name) {
            return;
        }
        self.exact
            .insert(sym.qualified_name.clone(), sym.label.clone());
        if let Some(parent) = parent_class_from_props(&sym.properties_json) {
            self.parent_class.insert(sym.qualified_name.clone(), parent);
        }
        let simple = simple_name(&sym.qualified_name);
        self.by_name
            .entry(simple)
            .or_default()
            .push(sym.qualified_name.clone());
    }

    fn candidate_parent_class(&self, qn: &str) -> Option<&str> {
        self.parent_class.get(qn).map(String::as_str)
    }

    pub fn candidates(&self, callee_name: &str) -> &[String] {
        static EMPTY: Vec<String> = Vec::new();
        let lookup = simple_name(callee_name);
        self.by_name
            .get(&lookup)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    pub fn symbol_label(&self, qn: &str) -> Option<&str> {
        self.exact.get(qn).map(String::as_str)
    }

    fn filter_by_kind(&self, candidates: &[String], kind: CallTargetKind) -> Vec<String> {
        match kind {
            CallTargetKind::Any => candidates.to_vec(),
            CallTargetKind::FreeFunction => candidates
                .iter()
                .filter(|qn| {
                    self.symbol_label(qn) == Some("Function")
                        && self.candidate_parent_class(qn).is_none()
                })
                .cloned()
                .collect(),
            CallTargetKind::Method => candidates
                .iter()
                .filter(|qn| {
                    self.symbol_label(qn) == Some("Method")
                        || self.candidate_parent_class(qn).is_some()
                })
                .cloned()
                .collect(),
        }
    }

    pub fn resolve(
        &self,
        callee_name: &str,
        caller_file: &str,
        imports: &ImportMap,
    ) -> Option<CallResolution> {
        self.resolve_kind(callee_name, caller_file, imports, CallTargetKind::Any)
    }

    pub fn resolve_kind(
        &self,
        callee_name: &str,
        caller_file: &str,
        imports: &ImportMap,
        kind: CallTargetKind,
    ) -> Option<CallResolution> {
        self.resolve_kind_scoped(callee_name, caller_file, imports, kind, None)
    }

    pub fn resolve_kind_scoped(
        &self,
        callee_name: &str,
        caller_file: &str,
        imports: &ImportMap,
        kind: CallTargetKind,
        parent_class: Option<&str>,
    ) -> Option<CallResolution> {
        if callee_name.is_empty() {
            return None;
        }

        let lookup_name = imports
            .symbol_aliases
            .get(callee_name)
            .map(String::as_str)
            .unwrap_or(callee_name);
        let mut candidates = self.filter_by_kind(self.candidates(lookup_name), kind);
        if let Some(parent) = parent_class {
            let scoped: Vec<String> = candidates
                .iter()
                .filter(|qn| self.candidate_parent_class(qn).is_some_and(|p| p == parent))
                .cloned()
                .collect();
            if !scoped.is_empty() {
                candidates = scoped;
            }
        }
        if candidates.is_empty() {
            return None;
        }
        if candidates.len() > MAX_CANDIDATES {
            return None;
        }

        // Strategy 1: direct import binding → symbol in target module.
        if let Some(target) = imports.bindings.get(callee_name) {
            let scoped: Vec<String> = candidates
                .iter()
                .filter(|qn| qn_belongs_to_module(qn, target))
                .cloned()
                .collect();
            if scoped.len() == 1 {
                return Some(resolution(
                    &scoped[0],
                    "import_binding",
                    CONF_IMPORT_BINDING,
                    scoped.len(),
                ));
            }
        }

        // Strategy 2: same-file match.
        let same_file: Vec<String> = candidates
            .iter()
            .filter(|qn| qn.starts_with(&format!("{caller_file}::")))
            .cloned()
            .collect();
        if same_file.len() == 1 {
            return Some(resolution(
                &same_file[0],
                "same_file",
                CONF_SAME_FILE,
                same_file.len(),
            ));
        }
        if same_file.len() > 1 {
            return None;
        }

        // Strategy 3: globally unique name.
        if candidates.len() == 1 {
            let conf = if imports.is_reachable(qn_file(&candidates[0]))
                || (imports.bindings.is_empty() && imports.modules.is_empty())
            {
                CONF_UNIQUE_NAME
            } else {
                CONF_UNIQUE_NAME * 0.5
            };
            return Some(resolution(
                &candidates[0],
                "unique_name",
                conf,
                candidates.len(),
            ));
        }

        if candidates.is_empty() {
            return None;
        }

        // Strategy 4: import-filtered suffix match among multiple candidates.
        if !imports.bindings.is_empty() || !imports.modules.is_empty() {
            let reachable: Vec<String> = candidates
                .iter()
                .filter(|qn| imports.is_reachable(qn_file(qn)))
                .cloned()
                .collect();
            if reachable.len() == 1 {
                return Some(resolution(
                    &reachable[0],
                    "import_filtered",
                    CONF_IMPORT_FILTERED,
                    reachable.len(),
                ));
            }
            if reachable.len() > 1 {
                if let Some(best) = best_candidate(&reachable, caller_file) {
                    return Some(resolution(
                        best,
                        "import_filtered",
                        CONF_IMPORT_FILTERED_PENALTY,
                        reachable.len(),
                    ));
                }
                return None;
            }
            return None;
        }

        None
    }
}

pub fn parent_class_from_props(properties_json: &Option<String>) -> Option<String> {
    let props = properties_json.as_ref()?;
    let key = "\"parent_class\":\"";
    let start = props.find(key)? + key.len();
    let rest = &props[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

pub fn confidence_band(score: f64) -> &'static str {
    if score >= 0.7 {
        "high"
    } else if score >= 0.45 {
        "medium"
    } else if score >= 0.25 {
        "speculative"
    } else {
        "low"
    }
}

fn resolution(qn: &str, strategy: &str, confidence: f64, candidates: usize) -> CallResolution {
    CallResolution {
        qn: qn.to_string(),
        strategy: strategy.to_string(),
        confidence,
        band: confidence_band(confidence).to_string(),
        candidates,
    }
}

/// Standard CALLS edge metadata aligned with reference `pass_calls.c` properties.
pub fn call_edge_properties_json(callee: &str, res: &CallResolution, method: &str) -> String {
    serde_json::json!({
        "callee": callee,
        "confidence": res.confidence,
        "strategy": res.strategy,
        "candidates": res.candidates,
        "method": method,
        "band": res.band,
        "score": res.confidence,
    })
    .to_string()
}

fn simple_name(qn: &str) -> String {
    qn.rsplit("::")
        .next()
        .and_then(|s| s.split('@').next())
        .unwrap_or(qn)
        .to_string()
}

fn qn_file(qn: &str) -> &str {
    qn.split("::").next().unwrap_or(qn)
}

fn qn_belongs_to_module(qn: &str, module_path: &str) -> bool {
    let file = qn_file(qn);
    let norm_file = file.replace('\\', "/");
    let norm_mod = module_path.replace('\\', "/");
    norm_file == norm_mod
        || norm_file.ends_with(&norm_mod)
        || norm_mod.ends_with(&norm_file)
        || norm_file
            .strip_suffix(".py")
            .is_some_and(|s| norm_mod.starts_with(s))
        || norm_mod
            .strip_suffix(".py")
            .is_some_and(|s| norm_file.starts_with(s))
}

fn best_candidate<'a>(candidates: &'a [String], caller_file: &str) -> Option<&'a str> {
    candidates
        .iter()
        .max_by_key(|qn| candidate_score(qn, caller_file))
        .map(String::as_str)
}

fn candidate_score(qn: &str, caller_file: &str) -> i32 {
    let mut score = 0i32;
    let file = qn_file(qn);
    if !file.contains("test") && !file.contains("mock") && !file.contains("spec") {
        score += 1000;
    }
    let caller_stem = caller_file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(caller_file)
        .split('.')
        .next()
        .unwrap_or(caller_file);
    if file.contains(caller_stem) {
        score += 10;
    }
    score
}

/// Per-file resolver with memoization (reference per-file resolve cache).
#[derive(Debug)]
pub struct FileCallResolver<'a> {
    registry: &'a SymbolRegistry,
    caller_file: String,
    imports: ImportMap,
    cache: HashMap<String, Option<CallResolution>>,
}

impl<'a> FileCallResolver<'a> {
    pub fn new(registry: &'a SymbolRegistry, caller_file: &str, imports: ImportMap) -> Self {
        Self {
            registry,
            caller_file: caller_file.to_string(),
            imports,
            cache: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, callee_name: &str) -> Option<CallResolution> {
        self.resolve_kind(callee_name, CallTargetKind::Any)
    }

    pub fn resolve_kind(
        &mut self,
        callee_name: &str,
        kind: CallTargetKind,
    ) -> Option<CallResolution> {
        self.resolve_kind_scoped(callee_name, kind, None)
    }

    pub fn resolve_kind_scoped(
        &mut self,
        callee_name: &str,
        kind: CallTargetKind,
        parent_class: Option<&str>,
    ) -> Option<CallResolution> {
        let cache_key = format!("{callee_name}:{kind:?}:{parent_class:?}");
        if let Some(hit) = self.cache.get(&cache_key) {
            return hit.clone();
        }
        let res = self.registry.resolve_kind_scoped(
            callee_name,
            &self.caller_file,
            &self.imports,
            kind,
            parent_class,
        );
        self.cache.insert(cache_key, res.clone());
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_id::qualified_name;

    fn sym(file: &str, name: &str, line: i64) -> Symbol {
        Symbol {
            qualified_name: qualified_name(file, "Function", name, line),
            name: name.into(),
            label: "Function".into(),
            file_path: file.into(),
            line_start: line,
            line_end: line + 2,
            signature: None,
            properties_json: None,
        }
    }

    #[test]
    fn resolves_same_file_helper() {
        let reg =
            SymbolRegistry::from_symbols(&[sym("main.py", "main", 4), sym("main.py", "helper", 1)]);
        let imports = ImportMap::default();
        let res = reg.resolve("helper", "main.py", &imports).unwrap();
        assert_eq!(res.strategy, "same_file");
        assert!(res.qn.contains("helper"));
    }

    #[test]
    fn resolves_import_bound_cross_file_helper() {
        let reg = SymbolRegistry::from_symbols(&[
            sym("main.py", "main", 4),
            sym("utils.py", "helper", 1),
            sym("decoy.py", "helper", 1),
        ]);
        let mut imports = ImportMap::default();
        imports.bindings.insert("helper".into(), "utils.py".into());
        let res = reg.resolve("helper", "main.py", &imports).unwrap();
        assert_eq!(res.strategy, "import_binding");
        assert!(res.qn.starts_with("utils.py::"));
    }

    fn method_sym(file: &str, name: &str, line: i64) -> Symbol {
        Symbol {
            qualified_name: qualified_name(file, "Method", name, line),
            name: name.into(),
            label: "Method".into(),
            file_path: file.into(),
            line_start: line,
            line_end: line + 2,
            signature: None,
            properties_json: None,
        }
    }

    #[test]
    fn free_function_call_prefers_function_over_method() {
        let reg = SymbolRegistry::from_symbols(&[
            sym("app.js", "main", 10),
            sym("app.js", "run", 1),
            method_sym("app.js", "run", 5),
        ]);
        let imports = ImportMap::default();
        let res = reg
            .resolve_kind("run", "app.js", &imports, CallTargetKind::FreeFunction)
            .unwrap();
        assert_eq!(res.strategy, "same_file");
        assert!(reg.symbol_label(&res.qn) == Some("Function"));
    }

    #[test]
    fn method_call_does_not_resolve_to_free_function() {
        let reg = SymbolRegistry::from_symbols(&[
            sym("app.js", "main", 10),
            sym("app.js", "run", 1),
            method_sym("app.js", "run", 5),
        ]);
        let imports = ImportMap::default();
        let res = reg.resolve_kind("run", "app.js", &imports, CallTargetKind::Method);
        assert!(res.is_some());
        assert!(reg.symbol_label(&res.as_ref().unwrap().qn) == Some("Method"));
    }

    #[test]
    fn skips_ambiguous_cross_file_without_import() {
        let reg = SymbolRegistry::from_symbols(&[
            sym("main.rs", "main", 1),
            sym("a.rs", "helper", 1),
            sym("b.rs", "helper", 1),
        ]);
        let imports = ImportMap::default();
        assert!(reg.resolve("helper", "main.rs", &imports).is_none());
    }

    #[test]
    fn resolves_import_alias_to_bound_module_symbol() {
        let reg = SymbolRegistry::from_symbols(&[
            sym("main.py", "main", 4),
            sym("utils.py", "helper", 1),
            sym("decoy.py", "helper", 1),
        ]);
        let mut imports = ImportMap::default();
        imports.bindings.insert("h".into(), "utils.py".into());
        imports.symbol_aliases.insert("h".into(), "helper".into());
        let res = reg.resolve("h", "main.py", &imports).unwrap();
        assert_eq!(res.strategy, "import_binding");
        assert!(res.qn.starts_with("utils.py::"));
    }

    #[test]
    fn call_edge_properties_include_reference_fields() {
        let res = resolution("main.py::Function::helper@1", "same_file", 0.90, 1);
        let props = call_edge_properties_json("helper", &res, "ast");
        let v: serde_json::Value = serde_json::from_str(&props).unwrap();
        assert_eq!(v["callee"], "helper");
        assert_eq!(v["confidence"], 0.90);
        assert_eq!(v["strategy"], "same_file");
        assert_eq!(v["candidates"], 1);
        assert_eq!(v["method"], "ast");
        assert_eq!(v["band"], "high");
        assert_eq!(v["score"], 0.90);
    }
}
