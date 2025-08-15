use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TagAlias {
    pub id: i64,
    pub antecedent_name: String,
    pub consequent_name: String,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TagAliasesApiResponse {
    Wrapped { tag_aliases: Vec<TagAlias> },
    Direct(Vec<TagAlias>),
}

impl TagAliasesApiResponse {
    pub fn into_vec(self) -> Vec<TagAlias> {
        match self {
            TagAliasesApiResponse::Wrapped { tag_aliases } => tag_aliases,
            TagAliasesApiResponse::Direct(v) => v,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TagImplication {
    pub id: i64,
    pub antecedent_name: String,
    pub consequent_name: String,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TagImplicationsApiResponse {
    Wrapped {
        tag_implications: Vec<TagImplication>,
    },
    Direct(Vec<TagImplication>),
}

impl TagImplicationsApiResponse {
    pub fn into_vec(self) -> Vec<TagImplication> {
        match self {
            TagImplicationsApiResponse::Wrapped { tag_implications } => tag_implications,
            TagImplicationsApiResponse::Direct(v) => v,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct TagCount {
    pub name: String,
    pub group_type: String,
    pub count: i64,
}
