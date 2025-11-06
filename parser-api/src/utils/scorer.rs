use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::Deserialize;
use crate::models::{Post, TagCount};
use crate::utils::idf::IdfIndex;

#[derive(Debug, Clone, Deserialize)]
pub struct Priors {
    pub now: DateTime<Utc>,
    pub recency_tau_days: f32,
    pub quality_a: f32,
    pub quality_b: f32,
    pub mix_sim: f32,
    pub mix_quality: f32,
    pub mix_recency: f32,
}

#[inline]
fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }

#[inline]
fn gw<'a>(group_wts: &HashMap<&'a str, f32>, group: &'a str) -> f32 {
    *group_wts.get(group).unwrap_or(&1.0)
}

pub fn post_affinity(
    account_tag_counts: &[TagCount],
    origin_post: &Post,
    group_wts: &HashMap<String, f32>,
    priors: &Priors,
    idf: &IdfIndex,
) -> f32 {
    let group_wts_hash: HashMap<&str, f32> =
        group_wts.iter().map(|(k, v)| (k.as_str(), *v)).collect();

    let mut post: HashMap<(String, &'static str), f32> = HashMap::default();

    let mut add_post_tags = |tags: &Vec<String>, group: &'static str| {
        let g = gw(&group_wts_hash, group);
        for t in tags {
            if t.is_empty() { continue; }
            let tlc = t.to_lowercase();
            let w = g * idf.idf(&tlc);
            post.entry((tlc, group)).or_insert(w);
        }
    };

    add_post_tags(&origin_post.tags.artist,    "artist");
    add_post_tags(&origin_post.tags.character, "character");
    add_post_tags(&origin_post.tags.copyright, "copyright");
    add_post_tags(&origin_post.tags.general,   "general");
    add_post_tags(&origin_post.tags.lore,      "lore");
    add_post_tags(&origin_post.tags.meta,      "meta");
    add_post_tags(&origin_post.tags.species,   "species");

    // User vector: weight = ln1p(count) * group_weight * idf(tag)
    let mut user: HashMap<(String, String), f32> = HashMap::default();
    for t in account_tag_counts {
        if t.count <= 0 { continue; }
        let g = *group_wts_hash.get(t.group_type.as_str()).unwrap_or(&1.0);
        let tlc = t.name.to_lowercase();
        let w = (t.count as f32).ln_1p() * g * idf.idf(&tlc);
        if w > 0.0 {
            *user.entry((tlc, t.group_type.clone())).or_insert(0.0) += w;
        }
    }

    let mut dot = 0.0f32;
    let mut u_norm_sq = 0.0f32;
    let mut p_norm_sq = 0.0f32;

    for &uw in user.values() { u_norm_sq += uw * uw; }
    for &pw in post.values() { p_norm_sq += pw * pw; }

    if u_norm_sq > 0.0 && p_norm_sq > 0.0 {
        if user.len() <= post.len() {
            for ((tag, group), &uw) in &user {
                if let Some(&pw) = post.get(&(tag.clone(), group.as_str())) {
                    dot += uw * pw;
                }
            }
        } else {
            for ((tag, group), &pw) in &post {
                if let Some(&uw) = user.get(&(tag.clone(), group.to_string())) {
                    dot += uw * pw;
                }
            }
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
