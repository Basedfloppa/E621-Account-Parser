#[macro_use]
extern crate rocket;

use log::info;
use rocket::serde::json::Json;
use rusqlite::Result;

use cors::*;
use db::*;
use models::*;

use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};

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
async fn create_account(account: Json<TruncatedAccount>) -> Result<(), String> {
    let user = api::get_account(&account).await;
    let blacklisted_tags = match user {
        UserApiResponse::FullCurrentUser(u) => u.blacklisted_tags,
        UserApiResponse::FullUser(u) => "".to_string(),
    };

    match save_account(
        account.id,
        &account.name,
        &account.api_key,
        &blacklisted_tags,
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = format!("Failed to get account: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[get("/recommendations/<account_id>?<page>")]
async fn get_recomendations(
    account_id: i32,
    page: Option<i32>,
) -> Result<Json<Vec<(Post, f32)>>, std::io::Error> {
    let group_weights = HashMap::from([
        ("artist", 2.0),
        ("character", 1.5),
        ("copyright", 1.3),
        ("species", 1.2),
        ("general", 1.0),
        ("meta", 0.4),
        ("invalid", 0.2),
        ("lore", 0.6),
        ("contributor", 0.8),
    ]);

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

    for post in posts {
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

        let score = utils::post_affinity(&tags, &post_tags, &group_weights, None, None);

        result.push((tmp_post, score));
    }

    Ok(Json(result))
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
        .attach(CORS)
        .attach(DbInit)
}
