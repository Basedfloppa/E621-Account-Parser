use std::collections::HashMap;
use chrono::{DateTime, Utc};

const DF_FLOOR: f32 = 2.0;
const IDF_MAX:  f32 = 6.0;

#[derive(Debug, Clone)]
pub struct IdfIndex {
    idf: HashMap<String, f32>,
    pub n_posts: i64,
    pub computed_at: DateTime<Utc>,
}

impl IdfIndex {
    pub fn from_df(df: &HashMap<String, i64>, n_posts: i64, now: DateTime<Utc>) -> Self {
        let mut idf = HashMap::with_capacity(df.len());
        let n = n_posts.max(1) as f32;

        for (tag, &df_raw) in df {
            let dfv = (df_raw.max(0)) as f32;
            let dfp = dfv + DF_FLOOR;
            let val = (1.0 + ((n - dfp + 0.5) / (dfp + 0.5)).max(0.0)).ln()
                .min(IDF_MAX)
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
    pub fn idf(&self, tag: &str) -> f32 {
        *self.idf.get(&tag.to_lowercase()).unwrap_or(&1.0)
    }

    pub fn as_map(&self) -> &HashMap<String, f32> { &self.idf }
}
