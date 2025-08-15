use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::models::TagCount;

pub struct Priors {
    pub now: DateTime<Utc>,
    pub recency_tau_days: f32, // e.g., 14.0
    pub quality_a: f32,        // e.g., 0.01
    pub quality_b: f32,        // e.g., 0.001
    pub mix_sim: f32,          // e.g., 0.7
    pub mix_quality: f32,      // e.g., 0.2
    pub mix_recency: f32,      // e.g., 0.1
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

pub fn post_affinity(
    account_tag_counts: &[TagCount],
    post_tags: &[(String, String)],
    group_wts: &HashMap<&str, f32>,
    idf: Option<&HashMap<&str, f32>>,
    priors: Option<(&Priors, i64, i64, DateTime<Utc>)>,
) -> f32 {
    if account_tag_counts.is_empty() || post_tags.is_empty() {
        return 0.0;
    }

    let mut user: HashMap<(&str, &str), f32> = HashMap::with_capacity(account_tag_counts.len());
    for t in account_tag_counts {
        if t.count <= 0 {
            continue;
        }
        let g = t.group_type.as_str();
        let name = t.name.as_str();

        let gw = *group_wts.get(g).unwrap_or(&1.0);

        let iw = idf.and_then(|m| m.get(name)).copied().unwrap_or(1.0);

        let w = (t.count as f32).ln_1p() * gw * iw;
        if w > 0.0 {
            *user.entry((name, g)).or_insert(0.0) += w;
        }
    }
    if user.is_empty() {
        return 0.0;
    }

    let mut post: HashMap<(&str, &str), f32> = HashMap::with_capacity(post_tags.len());
    for (name_s, group_s) in post_tags {
        let name = name_s.as_str();
        let g = group_s.as_str();
        let gw = *group_wts.get(g).unwrap_or(&1.0);
        let iw = idf.and_then(|m| m.get(name)).copied().unwrap_or(1.0);
        post.entry((name, g)).or_insert(gw * iw);
    }
    if post.is_empty() {
        return 0.0;
    }

    let mut dot: f32 = 0.0;
    let mut u_norm_sq: f32 = 0.0;
    let mut p_norm_sq: f32 = 0.0;

    for (&_k, &uw) in &user {
        u_norm_sq += uw * uw;
    }
    for (&_k, &pw) in &post {
        p_norm_sq += pw * pw;
    }

    let (smaller, other) = if user.len() <= post.len() {
        (&user, &post)
    } else {
        (&post, &user)
    };
    for (k, &w) in smaller {
        if let Some(&w2) = other.get(k) {
            dot += w * w2;
        }
    }

    let sim = if u_norm_sq == 0.0 || p_norm_sq == 0.0 {
        0.0
    } else {
        (dot / (u_norm_sq.sqrt() * p_norm_sq.sqrt())).clamp(0.0, 1.0)
    };

    if let Some((p, score_total, fav_count, created_at)) = priors {
        let quality =
            sigmoid(p.quality_a * (score_total as f32) + p.quality_b * (fav_count as f32));
        let age_days = (p.now - created_at).num_seconds() as f32 / 86400.0;
        let recency = (-age_days / p.recency_tau_days.max(1e-3))
            .exp()
            .clamp(0.0, 1.0);

        (p.mix_sim * sim + p.mix_quality * quality + p.mix_recency * recency).clamp(0.0, 1.0)
    } else {
        sim
    }
}
