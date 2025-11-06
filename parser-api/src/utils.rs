use chrono::{DateTime, Utc};
use std::collections::HashMap;
use rocket::serde::Deserialize;
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

pub fn post_affinity(
    account_tag_counts: &[TagCount],
    origin_post: &Post,
    group_wts: &HashMap<String, f32>,
    priors: &Priors,
) -> f32 {
    let mut post_tags: Vec<(String, String)> = Vec::new();

    post_tags.extend(origin_post.clone().tags.artist.into_iter().map(|t| (t, "artist".into())));
    post_tags.extend(origin_post.clone().tags.character.into_iter().map(|t| (t, "character".into())));
    post_tags.extend(origin_post.clone().tags.copyright.into_iter().map(|t| (t, "copyright".into())));
    post_tags.extend(origin_post.clone().tags.general.into_iter().map(|t| (t, "general".into())));
    post_tags.extend(origin_post.clone().tags.lore.into_iter().map(|t| (t, "lore".into())));
    post_tags.extend(origin_post.clone().tags.meta.into_iter().map(|t| (t, "meta".into())));
    post_tags.extend(origin_post.clone().tags.species.into_iter().map(|t| (t, "species".into())));

    if account_tag_counts.is_empty() || post_tags.is_empty() {
        return 0.0;
    }

    let mut user: HashMap<(String, String), f32> = HashMap::with_capacity(account_tag_counts.len());
    for t in account_tag_counts {
        if t.count <= 0 {
            continue;
        }

        let gw = *group_wts.get(&t.group_type).unwrap_or(&1.0);

        let w = (t.count as f32).ln_1p() * gw;
        if w > 0.0 {
            *user.entry((t.name.clone(), t.group_type.clone())).or_insert(0.0) += w;
        }
    }
    if user.is_empty() {
        return 0.0;
    }

    let mut post: HashMap<(String, String), f32> = HashMap::with_capacity(post_tags.len());
    for (name, group) in post_tags {
        let gw = *group_wts.get(group.as_str()).unwrap_or(&1.0);
        post.entry((name, group)).or_insert(gw);
    }
    if post.is_empty() {
        return 0.0;
    }

    let mut dot: f32 = 0.0;
    let mut u_norm_sq: f32 = 0.0;
    let mut p_norm_sq: f32 = 0.0;

    for (_k, uw) in &user {
        u_norm_sq += uw * uw;
    }
    for (_k, pw) in &post {
        p_norm_sq += pw * pw;
    }

    let (smaller, other) = if user.len() <= post.len() {
        (user, post)
    } else {
        (post, user)
    };
    for (k, w) in smaller {
        if let Some(&w2) = other.get(&k) {
            dot += w * w2;
        }
    }

    let sim = if u_norm_sq == 0.0 || p_norm_sq == 0.0 {
        0.0
    } else {
        (dot / (u_norm_sq.sqrt() * p_norm_sq.sqrt())).clamp(0.0, 1.0)
    };

    let quality =
        sigmoid(priors.quality_a * (origin_post.score.total as f32) + priors.quality_b * (origin_post.fav_count as f32));
    let age_days = (priors.now - origin_post.created_at).num_seconds() as f32 / 86400.0;
    let recency = (-age_days / priors.recency_tau_days.max(1e-3))
        .exp()
        .clamp(0.0, 1.0);

    (priors.mix_sim * sim + priors.mix_quality * quality + priors.mix_recency * recency).clamp(0.0, 1.0)
}
