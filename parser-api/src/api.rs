use reqwest::{Client, Response, StatusCode};
use rocket::serde::json;
use std::time::Duration;
use tokio::time::sleep;

use crate::models::{
    Post, PostsApiResponse, TagAlias, TagAliasesApiResponse, TagImplication,
    TagImplicationsApiResponse, TruncatedAccount, UserApiResponse,
};

pub const LIMIT: i32 = 320;
const BASE_URL: &str = "https://e621.net/";
const RPS_DELAY_MS: u64 = 250;
const MAX_RETRIES: usize = 3;

async fn send_with_retry(builder: reqwest::RequestBuilder) -> Result<Response, String> {
    let mut delay = Duration::from_millis(300);
    for attempt in 0..=MAX_RETRIES {
        sleep(Duration::from_millis(RPS_DELAY_MS)).await;

        match builder
            .try_clone()
            .ok_or("unable to clone request".to_string())?
            .send()
            .await
        {
            Ok(resp) => {
                if (resp.status() == StatusCode::TOO_MANY_REQUESTS
                    || resp.status().is_server_error())
                    && attempt < MAX_RETRIES
                {
                    sleep(delay).await;
                    delay = delay.saturating_mul(2);
                    continue;
                }
                return Ok(resp);
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    sleep(delay).await;
                    delay = delay.saturating_mul(2);
                    continue;
                }
                return Err(format!("request failed after retries: {e}"));
            }
        }
    }
    Err("unreachable".into())
}

pub fn get_client() -> Client {
    reqwest::Client::builder()
        .user_agent("account scraper (by zorolin)")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build client: {e}"))
        .unwrap()
}

pub async fn fetch_tag_aliases_for(name: &str) -> Result<Vec<TagAlias>, String> {
    let client = get_client();
    let url = format!(
        "{url}tag_aliases.json?search[antecedent_name]={name}&search[status]=active&limit=320",
        url = BASE_URL,
        name = urlencoding::encode(name)
    );
    let resp = send_with_retry(client.get(url)).await?;
    let body = resp
        .text()
        .await
        .map_err(|e| format!("aliases body: {e}"))?;
    let parsed: TagAliasesApiResponse =
        json::from_str(&body).map_err(|e| format!("aliases parse: {e} body={body}"))?;
    Ok(parsed
        .into_vec()
        .into_iter()
        .filter(|a| a.status == "active")
        .collect())
}

pub async fn fetch_tag_implications_for(name: &str) -> Result<Vec<TagImplication>, String> {
    let client = get_client();
    let url = format!(
        "{url}tag_implications.json?search[antecedent_name]={name}&search[status]=active&limit=320",
        url = BASE_URL,
        name = urlencoding::encode(name)
    );
    let resp = send_with_retry(client.get(url)).await?;
    let body = resp.text().await.map_err(|e| format!("imps body: {e}"))?;
    let parsed: TagImplicationsApiResponse =
        json::from_str(&body).map_err(|e| format!("imps parse: {e} body={body}"))?;
    Ok(parsed
        .into_vec()
        .into_iter()
        .filter(|i| i.status == "active")
        .collect())
}

pub async fn get_favorites(account: &TruncatedAccount, page: i32) -> Vec<Post> {
    let client = get_client();
    let url = format!(
        "{url}favorites.json?user_id={id}&limit={limit}&page={page}",
        url = BASE_URL,
        id = account.id,
        limit = LIMIT,
        page = page
    );
    let resp = send_with_retry(
        client
            .get(url)
            .basic_auth(account.name.clone(), Some(account.api_key.clone())),
    )
    .await
    .expect("favorites request failed");
    json::from_str::<PostsApiResponse>(&resp.text().await.expect("favorites body read failed"))
        .expect("favorites parse failed")
        .posts
}

pub async fn get_account(account: &TruncatedAccount) -> UserApiResponse {
    let client = get_client();
    let url = format!("{url}users/{id}.json", url = BASE_URL, id = account.id);
    let resp = send_with_retry(
        client
            .get(url)
            .basic_auth(account.name.clone(), Some(account.api_key.clone())),
    )
    .await
    .expect("account request failed");
    let body = resp.text().await.expect("account body read failed");
    json::from_str::<UserApiResponse>(&body).expect("account parse failed")
}

pub async fn get_posts(account: &TruncatedAccount, page: Option<i32>) -> Vec<Post> {
    let blacklisted_tags = account.blacklisted_tags.clone().unwrap_or_default();
    let blacklist = if blacklisted_tags.trim().is_empty() {
        String::new()
    } else {
        format!("-{}", blacklisted_tags.replace('\n', " -"))
    };

    let client = get_client();
    let url = format!(
        "{url}posts.json?limit={limit}&page={page}&tags={blacklist}",
        url = BASE_URL,
        limit = LIMIT,
        page = page.unwrap_or(0),
        blacklist = blacklist
    );
    let resp = send_with_retry(
        client
            .get(url)
            .basic_auth(account.name.clone(), Some(account.api_key.clone())),
    )
    .await
    .expect("posts request failed");
    json::from_str::<PostsApiResponse>(&resp.text().await.expect("posts body read failed"))
        .expect("posts parse failed")
        .posts
}
