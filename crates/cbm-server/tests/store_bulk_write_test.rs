use codebase_memory_mcp::store::{Edge, Store, Symbol};

fn sample_symbol(qn: &str, name: &str) -> Symbol {
    Symbol {
        qualified_name: qn.into(),
        name: name.into(),
        label: "Function".into(),
        file_path: "lib.rs".into(),
        line_start: 1,
        line_end: 2,
        signature: None,
        properties_json: Some("{}".into()),
    }
}

#[test]
fn bulk_write_rollback_preserves_prior_graph() {
    let store = Store::open_memory().unwrap();
    store.upsert_project("/tmp/repo").unwrap();
    store
        .upsert_symbol(&sample_symbol("lib.rs::Function::keep@L1", "keep"))
        .unwrap();
    assert_eq!(store.count_symbols().unwrap(), 1);

    store.begin_bulk_write().unwrap();
    store.clear_project_data().unwrap();
    store
        .upsert_symbol(&sample_symbol("lib.rs::Function::new@L1", "new"))
        .unwrap();
    assert_eq!(store.count_symbols().unwrap(), 1);
    store.rollback_bulk_write().unwrap();

    assert_eq!(store.count_symbols().unwrap(), 1);
    let sym = store
        .find_symbol("lib.rs::Function::keep@L1")
        .unwrap()
        .expect("prior symbol");
    assert_eq!(sym.name, "keep");
}

#[test]
fn bulk_write_commit_persists_staged_graph() {
    let store = Store::open_memory().unwrap();
    store.upsert_project("/tmp/repo").unwrap();
    store
        .upsert_symbol(&sample_symbol("lib.rs::Function::old@L1", "old"))
        .unwrap();

    store.begin_bulk_write().unwrap();
    store.clear_project_data().unwrap();
    store
        .upsert_symbol(&sample_symbol("lib.rs::Function::fresh@L1", "fresh"))
        .unwrap();
    store
        .insert_edges_batch(&[Edge {
            src_qn: "lib.rs::Function::fresh@L1".into(),
            dst_qn: "lib.rs::Function::fresh@L1".into(),
            edge_type: "CALLS".into(),
            properties_json: Some("{}".into()),
        }])
        .unwrap();
    store.commit_bulk_write().unwrap();

    assert_eq!(store.count_symbols().unwrap(), 1);
    assert!(store
        .find_symbol("lib.rs::Function::fresh@L1")
        .unwrap()
        .is_some());
    assert!(store.count_edges_by_type("CALLS").unwrap() >= 1);
}

#[test]
fn nested_begin_bulk_write_is_rejected() {
    let store = Store::open_memory().unwrap();
    store.begin_bulk_write().unwrap();
    assert!(store.begin_bulk_write().is_err());
    store.rollback_bulk_write().unwrap();
}
