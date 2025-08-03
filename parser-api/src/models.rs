use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserApiResponse {
    FullUser(FullUser),
    FullCurrentUser(FullCurrentUser),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullUser {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub level: i32,
    pub base_upload_limit: i32,
    pub post_upload_count: i32,
    pub post_update_count: i32,
    pub note_update_count: i32,
    pub is_banned: bool,
    pub can_approve_posts: bool,
    pub can_upload_free: bool,
    pub level_string: String,
    pub avatar_id: Option<i32>,
    pub wiki_page_version_count: i32,
    pub artist_version_count: i32,
    pub pool_version_count: i32,
    pub forum_post_count: i32,
    pub comment_count: i32,
    pub flag_count: i32,
    pub favorite_count: i32,
    pub positive_feedback_count: i32,
    pub neutral_feedback_count: i32,
    pub negative_feedback_count: i32,
    pub upload_limit: i32,
    pub profile_about: String,
    pub profile_artinfo: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullCurrentUser {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub level: i32,
    pub base_upload_limit: i32,
    pub post_upload_count: i32,
    pub post_update_count: i32,
    pub note_update_count: i32,
    pub is_banned: bool,
    pub can_approve_posts: bool,
    pub can_upload_free: bool,
    pub level_string: String,
    pub avatar_id: Option<i32>,
    pub blacklist_users: bool,
    pub description_collapsed_initially: bool,
    pub hide_comments: bool,
    pub show_hidden_comments: bool,
    pub show_post_statistics: bool,
    pub receive_email_notifications: bool,
    pub enable_keyboard_navigation: bool,
    pub enable_privacy_mode: bool,
    pub style_usernames: bool,
    pub enable_auto_complete: bool,
    pub disable_cropped_thumbnails: bool,
    pub enable_safe_mode: bool,
    pub disable_responsive_mode: bool,
    pub no_flagging: bool,
    pub disable_user_dmails: bool,
    pub enable_compact_uploader: bool,
    pub replacements_beta: bool,
    pub updated_at: DateTime<Utc>,
    pub email: String,
    pub last_logged_in_at: String,
    pub last_forum_read_at: String,
    pub recent_tags: String,
    pub comment_threshold: i32,
    pub default_image_size: String, // Enum option possible
    pub favorite_tags: String,
    pub blacklisted_tags: String,
    pub time_zone: String,
    pub per_page: i32,
    pub custom_style: String,
    pub favorite_count: i32,
    pub api_regen_multiplier: i32,
    pub api_burst_limit: i32,
    pub remaining_api_limit: i32,
    pub statement_timeout: i32,
    pub favorite_limit: i32,
    pub tag_query_limit: i32,
    pub has_mail: bool,
    pub forum_notification_dot: bool,
    pub wiki_page_version_count: i32,
    pub artist_version_count: i32,
    pub pool_version_count: i32,
    pub forum_post_count: i32,
    pub comment_count: i32,
    pub flag_count: i32,
    pub positive_feedback_count: i32,
    pub neutral_feedback_count: i32,
    pub negative_feedback_count: i32,
    pub upload_limit: i32,
    pub profile_about: String,
    pub profile_artinfo: String,
}

#[derive(Deserialize)]
pub struct FavoritesApiResponse {
    pub posts: Vec<Post>,
}

#[derive(Deserialize)]
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

#[derive(Deserialize, Serialize)]
pub struct TruncatedPost {
    pub id: i64,
    pub tags: Tags,
}

impl From<&Post> for TruncatedPost {
    fn from(post: &Post) -> Self {
        TruncatedPost {
            id: post.id,
            tags: post.tags.clone(),
        }
    }
}

#[derive(Deserialize)]
pub struct FileInfo {
    pub width: i64,
    pub height: i64,
    pub ext: Option<String>,
    pub size: i64,
    pub md5: Option<String>,
    pub url: Option<String>,
}

#[derive(Deserialize)]
pub struct Preview {
    pub width: i64,
    pub height: i64,
    pub url: Option<String>,
}

#[derive(Deserialize)]
pub struct Sample {
    pub has: Option<bool>,
    pub height: Option<i64>,
    pub width: Option<i64>,
    pub url: Option<String>,
    pub alternates: Option<Alternates>,
    pub variants: Option<Variants>,
    pub samples: Option<Samples>,
}

#[derive(Deserialize)]
pub struct PostSampleAlternate {
    pub fps: f32,
    pub codec: Option<String>,
    pub size: i64,
    pub width: i64,
    pub height: i64,
    pub url: Option<String>,
}

#[derive(Deserialize)]
pub struct Alternates {
    pub has: Option<bool>,
    pub original: Option<PostSampleAlternate>,
}

#[derive(Deserialize)]
pub struct Variants {
    pub webm: PostSampleAlternate,
    pub mp4: PostSampleAlternate,
}

#[derive(Deserialize)]
pub struct Samples {
    #[serde(rename = "480p")]
    pub p480: PostSampleAlternate,
    #[serde(rename = "720p")]
    pub p720: PostSampleAlternate,
}

#[derive(Deserialize)]
pub struct Score {
    pub up: i64,
    pub down: i64,
    pub total: i64,
}

#[derive(Deserialize, Clone, Serialize)]
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

#[derive(Deserialize)]
pub struct Flags {
    pub pending: bool,
    pub flagged: bool,
    pub note_locked: bool,
    pub status_locked: bool,
    pub rating_locked: bool,
    pub deleted: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    S,
    Q,
    E,
}

#[derive(Deserialize)]
pub struct Relationships {
    pub parent_id: Option<i64>,
    pub has_children: bool,
    pub has_active_children: bool,
    pub children: Vec<i64>,
}
