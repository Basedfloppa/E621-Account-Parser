use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::Deserialize;
use crate::models::{Post, TagCount};

#[derive(Debug, Clone, Deserialize)]
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

#[inline]
fn gw<'a>(group_wts: &HashMap<&'a str, f32>, group: &'a str) -> f32 {
    *group_wts.get(group).unwrap_or(&1.0)
}

pub fn post_affinity(
    account_tag_counts: &[TagCount],
    origin_post: &Post,
    group_wts: &HashMap<String, f32>,
    priors: &Priors,
) -> f32 {
    let group_wts_hash: HashMap<&str, f32> = group_wts.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    let mut post: HashMap<(&str, &str), f32> = HashMap::default();

    for t in &origin_post.tags.artist     { post.entry((t.as_str(), "artist"))    .or_insert(gw(&group_wts_hash, "artist")); }
    for t in &origin_post.tags.character  { post.entry((t.as_str(), "character")) .or_insert(gw(&group_wts_hash, "character")); }
    for t in &origin_post.tags.copyright  { post.entry((t.as_str(), "copyright")) .or_insert(gw(&group_wts_hash, "copyright")); }
    for t in &origin_post.tags.general    { post.entry((t.as_str(), "general"))   .or_insert(gw(&group_wts_hash, "general")); }
    for t in &origin_post.tags.lore       { post.entry((t.as_str(), "lore"))      .or_insert(gw(&group_wts_hash, "lore")); }
    for t in &origin_post.tags.meta       { post.entry((t.as_str(), "meta"))      .or_insert(gw(&group_wts_hash, "meta")); }
    for t in &origin_post.tags.species    { post.entry((t.as_str(), "species"))   .or_insert(gw(&group_wts_hash, "species")); }

    let mut user: HashMap<(&str, &str), f32> = HashMap::default();
    for t in account_tag_counts {
        if t.count <= 0 { continue; }
        let w = (t.count as f32).ln_1p() * *group_wts_hash.get(t.group_type.as_str()).unwrap_or(&1.0);
        if w > 0.0 {
            *user.entry((t.name.as_str(), t.group_type.as_str())).or_insert(0.0) += w;
        }
    }

    let mut dot = 0.0f32;
    let mut u_norm_sq = 0.0f32;
    let mut p_norm_sq = 0.0f32;

    for &uw in user.values()  { u_norm_sq += uw * uw; }
    for &pw in post.values()  { p_norm_sq += pw * pw; }

    if user.len() <= post.len() {
        for (k, &uw) in &user {
            if let Some(&pw) = post.get(k) { dot += uw * pw; }
        }
    } else {
        for (k, &pw) in &post {
            if let Some(&uw) = user.get(k) { dot += uw * pw; }
        }
    }

    let sim = if u_norm_sq == 0.0 || p_norm_sq == 0.0 {
        0.0
    } else {
        dot / (u_norm_sq.sqrt() * p_norm_sq.sqrt())
    };

    let quality = sigmoid(
        priors.quality_a * origin_post.score.total as f32 +
            priors.quality_b * origin_post.fav_count as f32
    );

    let age_days = (priors.now - origin_post.created_at).num_seconds() as f32 / 86_400.0;
    let recency = (-age_days / priors.recency_tau_days.max(1e-3)).exp().clamp(0.0, 1.0);

    let sum = priors.mix_sim + priors.mix_quality + priors.mix_recency;
    let (ms, mq, mr) = if sum > 0.0 {
        (priors.mix_sim / sum, priors.mix_quality / sum, priors.mix_recency / sum)
    } else { (0.0, 0.0, 0.0) };

    (ms * sim + mq * quality + mr * recency).clamp(0.0, 1.0)
}
