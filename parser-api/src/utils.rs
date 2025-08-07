use std::collections::{HashMap, HashSet};

use crate::models::TagCount;

pub fn post_affinity(account_tag_counts: &[TagCount], post_tags: &[(String, String)]) -> f32 {
    if account_tag_counts.is_empty() || post_tags.is_empty() {
        return 0.0;
    }

    // Build the account vector: weights over (name, group)
    let mut acc: HashMap<(&str, &str), f32> = HashMap::with_capacity(account_tag_counts.len());
    for entry in account_tag_counts {
        if entry.count > 0 {
            let name = entry.name.as_str();
            let group = entry.group_type.as_str();
            // If duplicates exist, accumulate; otherwise just insert
            *acc.entry((name, group)).or_insert(0.0) += entry.count as f32;
        }
    }
    if acc.is_empty() {
        return 0.0;
    }

    // Build a set of unique (name, group) pairs present in the post
    let post_set: HashSet<(&str, &str)> = post_tags
        .iter()
        .map(|(name, group)| (name.as_str(), group.as_str()))
        .collect();
    if post_set.is_empty() {
        return 0.0;
    }

    // Dot product between account weights and post's binary vector
    let mut dot: f32 = 0.0;
    for key in &post_set {
        if let Some(&w) = acc.get(key) {
            dot += w;
        }
    }

    // Norms
    let acc_norm_sq: f32 = acc.values().map(|w| w * w).sum();
    let acc_norm = acc_norm_sq.sqrt();
    let post_norm = (post_set.len() as f32).sqrt(); // binary vector: 1 per unique (name, group)

    if acc_norm == 0.0 || post_norm == 0.0 {
        0.0
    } else {
        (dot / (acc_norm * post_norm)).clamp(0.0, 1.0)
    }
}
