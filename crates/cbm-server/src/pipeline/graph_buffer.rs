//! In-memory graph staging during indexing (reference `graph_buffer.c`).
//!
//! Pipeline passes accumulate nodes and edges here, then flush to SQLite in one shot.

use crate::error::Result;
use crate::store::{Edge, SourceFile, Store, Symbol};
use std::collections::{HashMap, HashSet};

const STRUCTURE_LABELS: &[&str] = &["Project", "Folder", "File", "Module"];

#[derive(Debug, Default)]
pub struct GraphBuffer {
    pub project: String,
    pub root_path: String,
    files: HashMap<String, SourceFile>,
    symbols: HashMap<String, Symbol>,
    by_name: HashMap<String, Vec<String>>,
    edges: Vec<Edge>,
    edge_keys: HashSet<(String, String, String)>,
}

impl GraphBuffer {
    pub fn new(project: impl Into<String>, root_path: impl Into<String>) -> Self {
        Self {
            project: project.into(),
            root_path: root_path.into(),
            ..Default::default()
        }
    }

    pub fn upsert_file(&mut self, file: SourceFile) {
        self.files.insert(file.path.clone(), file);
    }

    pub fn upsert_symbol(&mut self, sym: Symbol) {
        let qn = sym.qualified_name.clone();
        let name = sym.name.clone();
        let label = sym.label.clone();
        match self.symbols.entry(qn.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(sym);
                return;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(sym);
            }
        }
        self.by_name.entry(name).or_default().push(qn.clone());
        let _ = label;
    }

    pub fn upsert_symbols_batch(&mut self, symbols: &[Symbol]) {
        for sym in symbols {
            self.upsert_symbol(sym.clone());
        }
    }

    pub fn insert_edge(&mut self, edge: Edge) -> bool {
        let key = (
            edge.src_qn.clone(),
            edge.dst_qn.clone(),
            edge.edge_type.clone(),
        );
        if !self.edge_keys.insert(key) {
            if let Some(existing) = self.edges.iter_mut().find(|e| {
                e.src_qn == edge.src_qn && e.dst_qn == edge.dst_qn && e.edge_type == edge.edge_type
            }) {
                if edge.properties_json.is_some() {
                    existing.properties_json = edge.properties_json;
                }
            }
            return false;
        }
        self.edges.push(edge);
        true
    }

    pub fn insert_edges_batch(&mut self, edges: &[Edge]) {
        for edge in edges {
            self.insert_edge(edge.clone());
        }
    }

    pub fn delete_edges_by_type(&mut self, edge_type: &str) {
        self.edges.retain(|e| {
            let keep = e.edge_type != edge_type;
            if !keep {
                self.edge_keys
                    .remove(&(e.src_qn.clone(), e.dst_qn.clone(), e.edge_type.clone()));
            }
            keep
        });
    }

    pub fn delete_symbols_by_labels(&mut self, labels: &[&str]) {
        let label_set: HashSet<&str> = labels.iter().copied().collect();
        let removed: Vec<String> = self
            .symbols
            .iter()
            .filter_map(|(qn, sym)| label_set.contains(sym.label.as_str()).then_some(qn.clone()))
            .collect();
        for qn in removed {
            if let Some(sym) = self.symbols.remove(&qn) {
                if let Some(bucket) = self.by_name.get_mut(&sym.name) {
                    bucket.retain(|x| x != &qn);
                }
            }
        }
    }

    pub fn list_files(&self) -> Vec<SourceFile> {
        let mut out: Vec<_> = self.files.values().cloned().collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    pub fn list_symbols(&self) -> Vec<Symbol> {
        let mut out: Vec<_> = self.symbols.values().cloned().collect();
        out.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
        out
    }

    pub fn list_edges(&self) -> Vec<Edge> {
        self.edges.clone()
    }

    pub fn code_symbols(&self) -> Vec<Symbol> {
        self.list_symbols()
            .into_iter()
            .filter(|s| !STRUCTURE_LABELS.contains(&s.label.as_str()))
            .collect()
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Write staged graph into SQLite (reference `cbm_gbuf_flush_to_store`).
    pub fn flush_to_store(&self, store: &Store) -> Result<()> {
        for file in self.list_files() {
            store.upsert_file(&file)?;
        }
        store.upsert_symbols_batch(&self.list_symbols())?;
        store.insert_edges_batch(&self.edges)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_id::qualified_name;
    use crate::test_lock;

    fn sym(file: &str, name: &str, line: i64) -> Symbol {
        Symbol {
            qualified_name: qualified_name(file, "Function", name, line),
            name: name.into(),
            label: "Function".into(),
            file_path: file.into(),
            line_start: line,
            line_end: line + 1,
            signature: None,
            properties_json: None,
        }
    }

    #[test]
    fn deduplicates_edges_by_src_dst_type() {
        let mut buf = GraphBuffer::new("p", "/repo");
        let e1 = Edge {
            src_qn: "a::Function::main@L1".into(),
            dst_qn: "a::Function::helper@L2".into(),
            edge_type: "CALLS".into(),
            properties_json: Some(r#"{"v":1}"#.into()),
        };
        let e2 = Edge {
            src_qn: "a::Function::main@L1".into(),
            dst_qn: "a::Function::helper@L2".into(),
            edge_type: "CALLS".into(),
            properties_json: Some(r#"{"v":2}"#.into()),
        };
        assert!(buf.insert_edge(e1));
        assert!(!buf.insert_edge(e2));
        assert_eq!(buf.edge_count(), 1);
        assert!(buf.edges[0]
            .properties_json
            .as_ref()
            .is_some_and(|p| p.contains("\"v\":2")));
    }

    #[test]
    fn flush_roundtrips_through_store() {
        let _guard = test_lock::acquire();
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("CBRLM_CACHE_DIR", dir.path());

        let mut buf = GraphBuffer::new("buf-proj", "/repo");
        buf.upsert_file(SourceFile {
            path: "main.rs".into(),
            content: "fn main() {}".into(),
            language: "rust".into(),
            line_count: 1,
            mtime_ns: None,
            size_bytes: None,
        });
        buf.upsert_symbol(sym("main.rs", "main", 1));
        buf.insert_edge(Edge {
            src_qn: "main.rs::File::main.rs".into(),
            dst_qn: qualified_name("main.rs", "Function", "main", 1),
            edge_type: "CONTAINS".into(),
            properties_json: None,
        });

        let store = Store::open("buf-proj").unwrap();
        store.clear_project_data().unwrap();
        store.upsert_project("/repo").unwrap();
        buf.flush_to_store(&store).unwrap();

        assert_eq!(store.count_symbols().unwrap(), 1);
        assert_eq!(store.list_edges().unwrap().len(), 1);
        assert_eq!(store.count_files().unwrap(), 1);

        let _ = crate::store::delete_project_db("buf-proj");
    }
}
