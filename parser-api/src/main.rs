#[macro_use]
extern crate rocket;

use log::info;
use rocket::serde::json::Json;
use rusqlite::Result;

use cors::*;
use db::*;
use models::*;

use std::io::{Error, ErrorKind};
use rocket::fs::{FileServer, relative};

use crate::rocket::serde::json;

mod api;
mod cors;
mod db;
mod models;
mod utils;

#[post("/process/<account_id>")]
async fn process_posts(account_id: i32) -> Result<String, std::io::Error> {
    let account = db::get_account_by_id(account_id).unwrap();
    let user = api::get_account(&account).await;
    let favcount = match user {
        UserApiResponse::FullCurrentUser(u) => u.favorite_count,
        UserApiResponse::FullUser(u) => u.favorite_count,
    };
    let pages = (favcount / api::LIMIT) + (if favcount % api::LIMIT > 0 { 1 } else { 0 });
    for i in 1..pages + 1 {
        let posts = api::get_favorites(&account, i).await;
        let parsed: Vec<TruncatedPost> = posts.iter().map(TruncatedPost::from).collect();

        info!("{} post found", parsed.len());

        db::save_posts(&parsed, account.id)
            .map_err(|e| format!("Failed to save posts: {}", e))
            .unwrap();

        for post in &parsed {
            db::save_post_tags(post)
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

#[post("/account", data = "<account>")]
fn create_account(account: Json<TruncatedAccount>) -> Result<(), String> {
    match save_account(account.id, &account.name, &account.api_key) {
        Ok(account) => Ok(account),
        Err(e) => {
            let error_msg = format!("Failed to get account: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[get("/recomendations/<account_id>?<page>")]
async fn get_recomendations(account_id: i32, page: Option<i32>) -> Result<Json<Vec<Post>>, std::io::Error> {
    let tags = match get_tag_counts(account_id) {
        Ok(counts) => counts.to_vec(),
        Err(e) => {
            let error_msg = format!("Failed to get tag counts: {}", e);
            eprintln!("{}", error_msg);
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to get tag counts: {}", e),
            ));
        }
    };

    let account = db::get_account_by_id(account_id).unwrap();
    let posts = api::get_posts(&account, page).await;

    let mut result: Vec<(Post, f32)> = Vec::new();

    for mut post in posts {
        let mut post_tags: Vec<(String, String)> = Vec::new();
        let tmp_post = post.clone();
        post_tags.append(
            &mut post
                .tags
                .artist
                .into_iter()
                .map(|t| (t, "artist".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .character
                .into_iter()
                .map(|t| (t, "character".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .contributor
                .into_iter()
                .map(|t| (t, "contributor".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .copyright
                .into_iter()
                .map(|t| (t, "copyright".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .general
                .into_iter()
                .map(|t| (t, "general".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .invalid
                .into_iter()
                .map(|t| (t, "invalid".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .lore
                .into_iter()
                .map(|t| (t, "lore".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .meta
                .into_iter()
                .map(|t| (t, "meta".to_string()))
                .collect(),
        );
        post_tags.append(
            &mut post
                .tags
                .species
                .into_iter()
                .map(|t| (t, "species".to_string()))
                .collect(),
        );

        let score = utils::post_affinity(&tags, &post_tags);

        result.push((tmp_post, score));
    }

    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(Json(result.into_iter().map(|p| p.0).collect()))
}

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                process_posts,
                get_account_tag_counts,
                get_account_id,
                get_account_name,
                create_account,
                get_recomendations,
            ],
        )
        .mount("/", FileServer::from(relative!("src/static")))
        .attach(CORS)
        .attach(DbInit)
}
