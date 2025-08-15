use log::{debug, error, info, trace, warn};
use reqwest::{Client, Response, StatusCode};
use rocket::serde::json;
use std::time::Duration;
use tokio::time::sleep;
use urlencoding::encode;

use crate::{
    db::{record_alias_probe, record_implication_probe},
    models::{
        Post, PostsApiResponse, TagAlias, TagAliasesApiResponse, TagImplication,
        TagImplicationsApiResponse, TruncatedAccount, UserApiResponse,
    },
};

pub const LIMIT: i32 = 320;
const BASE_URL: &str = "https://e621.net/";
const RPS_DELAY_MS: u64 = 250;
const MAX_RETRIES: usize = 3;

fn build_url(path: &str, params: &[(&str, String)]) -> String {
    let url = if params.is_empty() {
        format!("{BASE_URL}{path}")
    } else {
        let qs = params
            .iter()
            .map(|(k, v)| format!("{k}={}", encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("{BASE_URL}{path}?{qs}")
    };
    trace!("build_url: path={path} -> {url}");
    url
}

fn get_client() -> Client {
    info!("Building HTTP client");
    reqwest::Client::builder()
        .user_agent("account scraper (by zorolin)")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| {
            error!("Failed to build client: {e}");
            format!("Failed to build client: {e}")
        })
        .unwrap()
}

async fn send_with_retry(builder: reqwest::RequestBuilder) -> Result<Response, String> {
    let mut delay = Duration::from_millis(300);

    for attempt in 0..=MAX_RETRIES {
        if let Some(b) = builder.try_clone() {
            match b.build() {
                Ok(req) => debug!(
                    "HTTP attempt {}/{}: {} {} (rps_delay={}ms)",
                    attempt + 1,
                    MAX_RETRIES + 1,
                    req.method(),
                    req.url(),
                    RPS_DELAY_MS
                ),
                Err(e) => warn!("Could not build request for logging: {e}"),
            }
        } else {
            warn!(
                "Unable to clone request for logging on attempt {}",
                attempt + 1
            );
        }

        sleep(Duration::from_millis(RPS_DELAY_MS)).await;

        match builder
            .try_clone()
            .ok_or_else(|| {
                let m = "unable to clone request".to_string();
                error!("{m}");
                m
            })?
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                trace!("HTTP status received: {status}");

                if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                    && attempt < MAX_RETRIES
                {
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("n/a");
                    warn!(
                        "Request got {} (retry-after: {}). Backing off for {:?} (attempt {}/{})",
                        status,
                        retry_after,
                        delay,
                        attempt + 1,
                        MAX_RETRIES + 1
                    );
                    sleep(delay).await;
                    delay = delay.saturating_mul(2);
                    continue;
                }

                if status.is_success() {
                    info!("Request succeeded with {status}");
                } else {
                    warn!("Request completed with non-retryable status {status}");
                }
                return Ok(resp);
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    warn!(
                        "Request error on attempt {}/{}: {}. Retrying in {:?}",
                        attempt + 1,
                        MAX_RETRIES + 1,
                        e,
                        delay
                    );
                    sleep(delay).await;
                    delay = delay.saturating_mul(2);
                    continue;
                }
                error!("Request failed after {} attempts: {}", MAX_RETRIES + 1, e);
                return Err(format!("request failed after retries: {e}"));
            }
        }
    }

    error!("send_with_retry exhausted attempts but reached unreachable branch");
    Err("unreachable".into())
}

pub async fn fetch_tag_aliases_for(name: &str) -> Result<Vec<TagAlias>, String> {
    info!("Fetching tag aliases for antecedent_name='{name}'");
    let client = get_client();
    let url = build_url(
        "tag_aliases.json",
        &[
            ("search[antecedent_name]", name.to_string()),
            ("search[status]", "active".to_string()),
            ("limit", LIMIT.to_string()),
        ],
    );
    debug!("GET {url}");
    let resp = send_with_retry(client.get(url)).await?;

    let body = resp.text().await.map_err(|e| {
        error!("aliases body read error: {e}");
        format!("aliases body: {e}")
    })?;

    let parsed: TagAliasesApiResponse = json::from_str(&body).map_err(|e| {
        error!("aliases parse error: {e}");
        format!("aliases parse: {e} body={body}")
    })?;

    let all = parsed.into_vec();
    let total = all.len();
    let active: Vec<TagAlias> = all.into_iter().filter(|a| a.status == "active").collect();
    record_alias_probe(name, active.len())?;
    info!(
        "Tag aliases fetched: total={}, active={}",
        total,
        active.len()
    );
    Ok(active)
}

pub async fn fetch_tag_implications_for(name: &str) -> Result<Vec<TagImplication>, String> {
    info!("Fetching tag implications for antecedent_name='{name}'");
    let client = get_client();
    let url = build_url(
        "tag_implications.json",
        &[
            ("search[antecedent_name]", name.to_string()),
            ("search[status]", "active".to_string()),
            ("limit", LIMIT.to_string()),
        ],
    );
    debug!("GET {url}");
    let resp = send_with_retry(client.get(url)).await?;

    let body = resp.text().await.map_err(|e| {
        error!("imps body read error: {e}");
        format!("imps body: {e}")
    })?;

    let parsed: TagImplicationsApiResponse = json::from_str(&body).map_err(|e| {
        error!("imps parse error: {e}");
        format!("imps parse: {e} body={body}")
    })?;

    let all = parsed.into_vec();
    let total = all.len();
    let active: Vec<TagImplication> = all.into_iter().filter(|i| i.status == "active").collect();
    record_implication_probe(name, active.len())?;
    info!(
        "Tag implications fetched: total={}, active={}",
        total,
        active.len()
    );
    Ok(active)
}

pub async fn get_favorites(account: &TruncatedAccount, page: i32) -> Vec<Post> {
    info!(
        "Fetching favorites: user_id={} user_name='{}' page={}",
        account.id, account.name, page
    );
    let client = get_client();
    let url = build_url(
        "favorites.json",
        &[
            ("user_id", account.id.to_string()),
            ("limit", LIMIT.to_string()),
            ("page", page.to_string()),
        ],
    );
    debug!("GET (auth) {url}");
    let resp = send_with_retry(
        client
            .get(url)
            .basic_auth(account.name.clone(), Some(account.api_key.clone())),
    )
    .await
    .expect("favorites request failed");

    let body = resp.text().await.expect("favorites body read failed");
    let posts = json::from_str::<PostsApiResponse>(&body)
        .expect("favorites parse failed")
        .posts;

    info!("Fetched {} favorite posts", posts.len());
    posts
}

pub async fn get_account(account: &TruncatedAccount) -> UserApiResponse {
    info!(
        "Fetching account: id={} name='{}'",
        account.id, account.name
    );
    let client = get_client();
    let url = format!("{BASE_URL}users/{}.json", account.id);
    debug!("GET (auth) {url}");
    let resp = send_with_retry(
        client
            .get(url)
            .basic_auth(account.name.clone(), Some(account.api_key.clone())),
    )
    .await
    .expect("account request failed");
    let body = resp.text().await.expect("account body read failed");
    let parsed = json::from_str::<UserApiResponse>(&body).expect("account parse failed");
    info!("Fetched account successfully for id={}", account.id);
    parsed
}

pub async fn get_posts(account: &TruncatedAccount, page: Option<i32>) -> Vec<Post> {
    let blacklisted_tags = account.blacklisted_tags.clone().unwrap_or_default();
    let blacklist = if blacklisted_tags.trim().is_empty() {
        String::new()
    } else {
        format!("-{}", blacklisted_tags.replace('\n', " -"))
    };
    debug!(
        "Preparing posts fetch: page={} blacklist_len={}",
        page.unwrap_or(0),
        blacklist.split_whitespace().count()
    );

    let client = get_client();
    let url = build_url(
        "posts.json",
        &[
            ("limit", LIMIT.to_string()),
            ("page", page.unwrap_or(0).to_string()),
            ("tags", blacklist),
        ],
    );
    debug!("GET (auth) {url}");
    let resp = send_with_retry(
        client
            .get(url)
            .basic_auth(account.name.clone(), Some(account.api_key.clone())),
    )
    .await
    .expect("posts request failed");

    let body = resp.text().await.expect("posts body read failed");
    let posts = json::from_str::<PostsApiResponse>(&body)
        .expect("posts parse failed")
        .posts;

    info!("Fetched {} posts", posts.len());
    posts
}
