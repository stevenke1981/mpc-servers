use memory_core::models::{HybridWeights, SearchQuery};

#[test]
fn rejects_zero_top_k() {
    let query = SearchQuery {
        top_k: 0,
        ..Default::default()
    };
    assert!(query.validate().is_err());
}

#[test]
fn rejects_invalid_hybrid_weights() {
    let query = SearchQuery {
        query: "test".to_string(),
        weights: Some(HybridWeights {
            semantic: 0.5,
            bm25: 0.2,
            temporal: 0.1,
        }),
        ..Default::default()
    };
    assert!(query.validate().is_err());
}
