use reqwest::Client;
use rocket::serde::json;

use crate::models::{Post, PostsApiResponse, TruncatedAccount, UserApiResponse};

pub static LIMIT: i32 = 320;
pub static BASE_URL: &str = "https://e621.net/";

pub async fn get_favorites(account: &TruncatedAccount, page: i32) -> Vec<Post> {
    let client = get_client();
    let post_response = client
        .get(format!(
            "{url}favorites.json?user_id={id}&limit={limit}&page={page}",
            url = BASE_URL,
            id = account.id,
            limit = LIMIT,
            page = page
        ))
        .basic_auth(account.name.clone(), Some(account.api_key.clone()))
        .send()
        .await
        .unwrap();

    json::from_str::<PostsApiResponse>(&post_response.text().await.unwrap())
        .unwrap()
        .posts
}

pub async fn get_account(account: &TruncatedAccount) -> UserApiResponse {
    let client = get_client();
    let user_response = client
        .get(format!(
            "{url}users/{id}.json",
            url = BASE_URL,
            id = account.id
        ))
        .basic_auth(account.name.clone(), Some(account.api_key.clone()))
        .send()
        .await
        .unwrap();

    let body = user_response.text().await.unwrap();

    json::from_str::<UserApiResponse>(&body).unwrap()
}

pub async fn get_posts(account: &TruncatedAccount, page: Option<i32>) -> Vec<Post> {
    let blacklisted_tags =  account.blacklisted_tags.clone().unwrap_or("".to_string());
    let blacklist = format!("-{}", blacklisted_tags.replace("\n", " -"));

    let client = get_client();
    let post_response = client
        .get(format!(
            "{url}posts.json?limit={limit}&page={page}&tags={blacklist}",
            url = BASE_URL,
            limit = LIMIT,
            page = match page {
                Some(p) => p,
                None => 0,
            },
            blacklist = blacklist
        ))
        .basic_auth(account.name.clone(), Some(account.api_key.clone()))
        .send()
        .await
        .unwrap();

    json::from_str::<PostsApiResponse>(&post_response.text().await.unwrap())
        .unwrap()
        .posts
}

pub fn get_client() -> Client {
    reqwest::Client::builder()
        .user_agent("account scraper (by zorolin)")
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))
        .unwrap()
}
