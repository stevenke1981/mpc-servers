use memory_core::storage::VectorStore;
use tempfile::tempdir;

#[test]
fn remove_is_immediate_and_persistent() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("vectors.usearch");
    let path = path.to_string_lossy().into_owned();

    {
        let store = VectorStore::new(&path, 3).unwrap();
        store.add(7, &[1.0, 0.0, 0.0]).unwrap();
        assert_eq!(store.search(&[1.0, 0.0, 0.0], 5).unwrap().len(), 1);

        store.remove(7).unwrap();
        assert!(store.search(&[1.0, 0.0, 0.0], 5).unwrap().is_empty());
        assert_eq!(store.size(), 0);
    }

    let reopened = VectorStore::new(&path, 3).unwrap();
    assert!(reopened.search(&[1.0, 0.0, 0.0], 5).unwrap().is_empty());
    assert_eq!(reopened.size(), 0);
}
