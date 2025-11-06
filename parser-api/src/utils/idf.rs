use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::models::cfg;

#[derive(Debug, Clone)]
pub struct IdfIndex {
    idf: HashMap<String, f32>,
    pub n_posts: i64,
    pub computed_at: DateTime<Utc>,
}

impl IdfIndex {
    pub fn from_df(df: &HashMap<String, i64>, n_posts: i64, now: DateTime<Utc>) -> Self {
        let cfg = cfg();
        let mut idf = HashMap::with_capacity(df.len());
        let n = n_posts.max(1) as f32;

        for (tag, &df_raw) in df {
            let dfv = df_raw.max(0) as f32;
            let dfp = dfv + cfg.df_floor;
            let val = (1.0 + ((n - dfp + 0.5) / (dfp + 0.5)).max(0.0)).ln()
                .min(cfg.idf_max)
                .max(0.0);
            idf.insert(tag.to_lowercase(), val);
        }

        Self { idf, n_posts, computed_at: now }
    }

    pub fn from_db(
        get_df: impl Fn() -> rusqlite::Result<HashMap<String, i64>>,
        get_post_count: impl Fn() -> i64,
        now: DateTime<Utc>,
    ) -> rusqlite::Result<Self> {
        let df = get_df()?;
        let n_posts = get_post_count();
        Ok(Self::from_df(&df, n_posts, now))
    }

    #[inline]
    pub fn idf_raw(&self, tag: &str) -> f32 {
        *self.idf.get(&tag.to_lowercase()).unwrap_or(&1.0)
    }

    #[inline]
    pub fn idf_tempered(&self, tag: &str, lambda: f32, alpha: f32) -> f32 {
        let raw = self.idf_raw(tag);
        let blended = 1.0 + lambda.clamp(0.0, 1.0) * (raw - 1.0);
        blended.powf(alpha.clamp(0.0, 1.0))
    }

    pub fn as_map(&self) -> &HashMap<String, f32> { &self.idf }
}
