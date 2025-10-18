use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct TagCount {
    pub name: String,
    pub group_type: String,
    pub count: i64,
}
