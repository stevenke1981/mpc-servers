use std::collections::HashSet;

pub fn is_exact_duplicate(similarity: f64, dedup_threshold: f64) -> bool {
    similarity >= dedup_threshold
}

pub fn is_near_duplicate(similarity: f64, dedup_threshold: f64, near_dedup_threshold: f64) -> bool {
    similarity < dedup_threshold && similarity >= near_dedup_threshold
}

pub fn entity_overlap(e1: &[String], e2: &[String]) -> f64 {
    if e1.is_empty() || e2.is_empty() {
        return 0.0;
    }
    let s1: HashSet<&String> = e1.iter().collect();
    let s2: HashSet<&String> = e2.iter().collect();
    let intersection = s1.intersection(&s2).count();

    // Overlap is the size of the intersection divided by the size of the smaller set
    intersection as f64 / s1.len().min(s2.len()) as f64
}
