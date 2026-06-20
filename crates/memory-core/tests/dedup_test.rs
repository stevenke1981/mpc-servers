use memory_core::consolidation::dedup::{is_exact_duplicate, is_near_duplicate};

#[test]
fn threshold_boundaries_are_inclusive() {
    assert!(is_exact_duplicate(0.92, 0.92));
    assert!(is_near_duplicate(0.75, 0.92, 0.75));
    assert!(!is_near_duplicate(0.92, 0.92, 0.75));
}
