#[macro_use]
extern crate rocket;

use log::info;
use reqwest;
use rocket::serde::json::Json;
use rusqlite::Result;

use cors::*;
use database::*;

use crate::models::{FavoritesApiResponse, TruncatedAccount, TruncatedPost, UserApiResponse};
use crate::rocket::serde::json;

mod cors;
mod database;
mod models;

static LIMIT: i32 = 320;
static BASE_URL: &str = "https://e621.net/";

#[post("/process/<account_id>")]
async fn process_posts(account_id: i32) -> Result<String, std::io::Error> {
    let client = reqwest::Client::builder()
        .user_agent("account scraper (by zorolin)")
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))
        .unwrap();

    let account = get_account_id(account_id).unwrap();

    let user_response = client
        .get(format!(
            "{url}users/{id}.json",
            url = BASE_URL,
            id = account_id
        ))
        .basic_auth(account.name.clone(), Some(account.api_key.clone()))
        .send()
        .await
        .unwrap();
    let user = json::from_str::<UserApiResponse>(&user_response.text().await.unwrap())?;
    let favcount = match user {
        UserApiResponse::FullCurrentUser(u) => u.favorite_count,
        UserApiResponse::FullUser(u) => u.favorite_count,
    };
    let pages = (favcount / LIMIT) + (if favcount % LIMIT > 0 { 1 } else { 0 });

    for i in 1..pages + 1 {
        info!(
            "Getting => {url}posts.json?limit={limit}&page={page}&tags='fav:{username}'",
            url = BASE_URL,
            limit = LIMIT,
            page = i,
            username = account.name
        );

        let post_response = client
            .get(format!(
                "{url}favorites.json?user_id={id}&limit={limit}&page={page}",
                url = BASE_URL,
                id = account.id,
                limit = LIMIT,
                page = i
            ))
            .basic_auth(account.name.clone(), Some(account.api_key.clone()))
            .send()
            .await
            .unwrap();

        let posts =
            json::from_str::<FavoritesApiResponse>(&post_response.text().await.unwrap())?.posts;
        let parsed: Vec<TruncatedPost> = posts.iter().map(TruncatedPost::from).collect();

        info!("{} post found", parsed.len());

        save_posts(&parsed, account.id)
            .map_err(|e| format!("Failed to save posts: {}", e))
            .unwrap();

        for post in &parsed {
            save_post_tags(post)
                .map_err(|e| format!("Failed to save tags for post {}: {}", post.id, e))
                .unwrap();
        }
    }

    Ok(json::to_string(&"okay :3").unwrap())
}

#[get("/account/<account_id>/tag_counts")]
fn get_account_tag_counts(account_id: i32) -> Result<Json<Vec<TagCount>>, String> {
    match get_tag_counts(account_id) {
        Ok(counts) => Ok(Json(counts.to_vec())),
        Err(e) => {
            let error_msg = format!("Failed to get tag counts: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[get("/user/name/<name>")]
fn get_account_name(name: &str) -> Result<Json<TruncatedAccount>, String> {
    match get_account_by_name(name.to_string()) {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            let error_msg = format!("Failed to get account: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[get("/user/id/<id>")]
fn get_account_id(id: i32) -> Result<Json<TruncatedAccount>, String> {
    match get_account_by_id(id) {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            let error_msg = format!("Failed to get account: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[post("/account", data="<account>")]
fn create_account(account: Json<TruncatedAccount>) -> Result<(), String> {
    match save_account(account.id, &account.name, &account.api_key) {
        Ok(account) => { Ok(account) },
        Err(e) => {
            let error_msg = format!("Failed to get account: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }        
    }
}

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![
            process_posts, 
            get_account_tag_counts,
            get_account_id,
            get_account_name,
            create_account
            ])
        .attach(CORS)
        .attach(DbInit)
}
