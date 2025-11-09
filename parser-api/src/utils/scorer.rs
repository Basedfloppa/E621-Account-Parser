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
    pub idf_lambda: Option<f32>,
    pub idf_alpha:  Option<f32>,
    pub freq_alpha: f32,
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

    let lambda = priors.idf_lambda.unwrap_or(0.4);
    let alpha  = priors.idf_alpha.unwrap_or(0.5);

    let mut user: HashMap<String, f32> = HashMap::default();
    let mut u_norm_sq = 0.0f32;

    for t in account_tag_counts {
        if t.count <= 0 { continue; }
        let g = *group_wts_hash.get(t.group_type.as_str()).unwrap_or(&1.0);
        let tlc = t.name.to_lowercase();
        let idf_w = idf.idf_tempered(&tlc, lambda, alpha);
        let w = (t.count as f32).powf(priors.freq_alpha) * g * idf_w;
        if w > 0.0 {
            let key = format!("{}|{}", t.group_type, tlc);
            let e = user.entry(key).or_insert(0.0);
            *e += w;
        }
    }
    for &uw in user.values() { u_norm_sq += uw * uw; }

    let mut dot = 0.0f32;
    let mut p_norm_sq = 0.0f32;

    let mut acc = |tags: &Vec<String>, group: &'static str| {
        let g = gw(&group_wts_hash, group);
        for t in tags {
            if t.is_empty() { continue; }
            let tlc = t.to_lowercase();
            let idf_w = idf.idf_tempered(&tlc, lambda, alpha);
            let pw = g * idf_w;
            p_norm_sq += pw * pw;

            let key = {
                let mut s = String::with_capacity(group.len() + 1 + tlc.len());
                s.push_str(group);
                s.push('|');
                s.push_str(&tlc);
                s
            };
            if let Some(&uw) = user.get(&key) {
                dot += uw * pw;
            }
        }
    };

    acc(&origin_post.tags.artist,    "artist");
    acc(&origin_post.tags.character, "character");
    acc(&origin_post.tags.copyright, "copyright");
    acc(&origin_post.tags.general,   "general");
    acc(&origin_post.tags.lore,      "lore");
    acc(&origin_post.tags.meta,      "meta");
    acc(&origin_post.tags.species,   "species");

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