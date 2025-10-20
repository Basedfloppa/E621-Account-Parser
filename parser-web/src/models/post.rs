use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Post {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub file: Option<FileInfo>,
    pub preview: Option<Preview>,
    pub sample: Option<Sample>,
    pub score: Score,
    pub tags: Tags,
    pub locked_tags: Option<Vec<String>>,
    pub change_seq: f64,
    pub flags: Flags,
    pub rating: Rating,
    pub fav_count: i64,
    pub sources: Vec<String>,
    pub pools: Vec<i64>,
    pub relationships: Relationships,
    pub approver_id: Option<i64>,
    pub uploader_id: i64,
    pub description: Option<String>,
    pub comment_count: i64,
    pub is_favorited: bool,
    pub has_notes: bool,
    pub duration: Option<f64>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct FileInfo {
    pub width: i64,
    pub height: i64,
    pub ext: Option<String>,
    pub size: i64,
    pub md5: Option<String>,
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Preview {
    pub width: i64,
    pub height: i64,
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Sample {
    pub has: Option<bool>,
    pub height: Option<i64>,
    pub width: Option<i64>,
    pub url: Option<String>,
    pub alternates: Option<Alternates>,
    pub variants: Option<Variants>,
    pub samples: Option<Samples>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct PostSampleAlternate {
    pub fps: f32,
    pub codec: Option<String>,
    pub size: i64,
    pub width: i64,
    pub height: i64,
    pub url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Alternates {
    pub has: Option<bool>,
    pub original: Option<PostSampleAlternate>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Variants {
    pub webm: PostSampleAlternate,
    pub mp4: PostSampleAlternate,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Samples {
    #[serde(rename = "480p")]
    pub p480: PostSampleAlternate,
    #[serde(rename = "720p")]
    pub p720: PostSampleAlternate,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Score {
    pub up: i64,
    pub down: i64,
    pub total: i64,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Tags {
    pub general: Vec<String>,
    pub artist: Vec<String>,
    pub copyright: Vec<String>,
    pub character: Vec<String>,
    pub species: Vec<String>,
    pub invalid: Vec<String>,
    pub meta: Vec<String>,
    pub lore: Vec<String>,
    pub contributor: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Flags {
    pub pending: bool,
    pub flagged: bool,
    pub note_locked: bool,
    pub status_locked: bool,
    pub rating_locked: bool,
    pub deleted: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    S,
    Q,
    E,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Relationships {
    pub parent_id: Option<i64>,
    pub has_children: bool,
    pub has_active_children: bool,
    pub children: Vec<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScoredPost {
    pub post: Post,
    pub score: f32,
}
