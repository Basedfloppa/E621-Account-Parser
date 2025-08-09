#[macro_use]
extern crate rocket;

use chrono::Utc;
use log::info;
use rocket::serde::json::Json;
use rusqlite::Result;

use cors::*;
use db::*;
use models::*;

use std::{
    collections::HashMap,
    io::ErrorKind,
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

    let priors = utils::Priors {
        now: Utc::now(),
        recency_tau_days: 14.0,
        quality_a: 0.01,
        quality_b: 0.001,
        mix_sim: 0.7,
        mix_quality: 0.2,
        mix_recency: 0.1,
    };

    let tags = get_tag_counts(account_id)
        .map_err(|e| {
            std::io::Error::new(ErrorKind::Other, format!("Failed to get tag counts: {}", e))
        })?
        .to_vec();

    let account = db::get_account_by_id(account_id).unwrap();
    let posts = api::get_posts(&account, page).await;

    let idf: Option<HashMap<&str, f32>> = None;

    let mut scored: Vec<(Post, f32)> = Vec::with_capacity(posts.len());
    for post in posts {
        let mut post_tags: Vec<(String, String)> = Vec::new();
        let tmp_post = post.clone();

        // flatten tags by group (keep names as-is; consider normalization in #3)
        post_tags.extend(post.tags.artist.into_iter().map(|t| (t, "artist".into())));
        post_tags.extend(
            post.tags
                .character
                .into_iter()
                .map(|t| (t, "character".into())),
        );
        post_tags.extend(
            post.tags
                .contributor
                .into_iter()
                .map(|t| (t, "contributor".into())),
        );
        post_tags.extend(
            post.tags
                .copyright
                .into_iter()
                .map(|t| (t, "copyright".into())),
        );
        post_tags.extend(post.tags.general.into_iter().map(|t| (t, "general".into())));
        post_tags.extend(post.tags.invalid.into_iter().map(|t| (t, "invalid".into())));
        post_tags.extend(post.tags.lore.into_iter().map(|t| (t, "lore".into())));
        post_tags.extend(post.tags.meta.into_iter().map(|t| (t, "meta".into())));
        post_tags.extend(post.tags.species.into_iter().map(|t| (t, "species".into())));

        let score_total: i64 = tmp_post.score.total;
        let fav_count: i64 = tmp_post.fav_count;
        let created_at = tmp_post.created_at;

        let s = utils::post_affinity(
            &tags,
            &post_tags,
            &group_weights,
            idf.as_ref(),
            Some((&priors, score_total, fav_count, created_at)),
        );
        scored.push((tmp_post, s));
    }

    Ok(Json(scored))
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
