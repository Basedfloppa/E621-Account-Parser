#[macro_use]
extern crate rocket;

use chrono::Utc;
use rocket::{futures::lock::Mutex, serde::json::Json};
use rocket::{get, http::Method, routes};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use rusqlite::Result;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;

use crate::models::{cfg, default_path, reload_from, start_config_watcher};
use crate::{
    db::{
        DbInit, get_account_by_id, get_account_by_name, get_tag_counts, set_account, set_tag_counts,
    },
    models::{Post, TagCount, TruncatedAccount, UserApiResponse},
    rocket::serde::json,
    utils::Priors,
};

mod api;
mod db;
mod models;
mod utils;

const QUEUE_CAP: usize = 10_000;

#[post("/process/<account_id>")]
async fn process_posts(account_id: i32) -> Result<String, String> {
    let blacklist: HashSet<String> = cfg()
        .tag_blacklist
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let account = get_account_by_id(account_id).map_err(|e| e.to_string())?;
    let user = api::get_account(&account).await;
    let favcount = match user {
        UserApiResponse::FullCurrentUser(u) => u.favorite_count,
        UserApiResponse::FullUser(u) => u.favorite_count,
    };
    let pages = (favcount / api::LIMIT) + (if favcount % api::LIMIT > 0 { 1 } else { 0 });

    db::drop_account_posts(account_id).map_err(|e| format!("Failed to drop account posts: {e}"))?;

    for i in 1..=pages {
        let raw_posts = api::get_favorites(&account, i).await;
        let posts: Vec<Post> = raw_posts
            .into_iter()
            .map(|p| strip_blacklisted_tags(p, &blacklist))
            .collect();
        info!("{} post(s) found on page {}", posts.len(), i);

        db::save_posts(&posts, account.id).map_err(|e| format!("Failed to save posts: {e}"))?;

        db::save_posts_tags_batch(&posts, &blacklist)
            .map_err(|e| format!("Failed to save tags for page {i}: {e}"))?;
    }

    set_tag_counts(account_id).map_err(|e| format!("Failed to set account tag counts: {e}"))?;
    Ok(json::to_string(&"okay :3").unwrap())
}

fn strip_blacklisted_tags(mut p: Post, blacklist: &HashSet<String>) -> Post {
    let filter = |v: &mut Vec<String>| {
        v.retain(|t| !blacklist.contains(&t.to_lowercase().trim().to_string()));
    };
    filter(&mut p.tags.artist);
    filter(&mut p.tags.character);
    filter(&mut p.tags.copyright);
    filter(&mut p.tags.general);
    filter(&mut p.tags.lore);
    filter(&mut p.tags.meta);
    filter(&mut p.tags.species);
    p
}

#[get("/account/<account_id>/tag_counts")]
async fn get_account_tag_counts(account_id: i32) -> Result<Json<Vec<TagCount>>, String> {
    match get_tag_counts(account_id) {
        Ok(counts) => Ok(Json(counts.to_vec())),
        Err(e) => {
            let error_msg = format!("Failed to get tag counts: {e}");
            eprintln!("{error_msg}");
            Err(error_msg)
        }
    }
}

#[get("/user/name/<name>")]
async fn get_account_name(name: &str) -> Result<Json<TruncatedAccount>, String> {
    match get_account_by_name(name.to_string()) {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            let error_msg = format!("Failed to get account: {e}");
            eprintln!("{error_msg}");
            Err(error_msg)
        }
    }
}

#[get("/user/id/<id>")]
async fn get_account_id(id: i32) -> Result<Json<TruncatedAccount>, String> {
    match get_account_by_id(id) {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            let error_msg = format!("Failed to get account: {e}");
            eprintln!("{error_msg}");
            Err(error_msg)
        }
    }
}

#[post("/account", data = "<account>")]
async fn create_account(account: Json<TruncatedAccount>) -> Result<(), String> {
    match set_account(account.id, &account.name, &account.blacklist) {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = format!("Failed to get account: {e}");
            eprintln!("{error_msg}");
            Err(error_msg)
        }
    }
}

#[get("/recommendations/<account_id>?<page>&<affinity_threshold>")]
async fn get_recommendations(
    account_id: i32,
    page: Option<i32>,
    affinity_threshold: Option<f32>,
) -> Result<Json<Vec<(Post, f32)>>, std::io::Error> {
    let group_weights = HashMap::from([
        ("artist", 2.0),
        ("character", 1.5),
        ("copyright", 1.3),
        ("species", 1.2),
        ("general", 1.0),
        ("meta", 0.4),
        ("lore", 0.6),
    ]);
    let priors = Priors {
        now: Utc::now(),
        recency_tau_days: 14.0,
        quality_a: 0.01,
        quality_b: 0.001,
        mix_sim: 0.7,
        mix_quality: 0.2,
        mix_recency: 0.1,
    };

    let tags: Vec<TagCount> = get_tag_counts(account_id)
        .map_err(|e| std::io::Error::other(format!("Failed to get tag counts: {e}")))?
        .to_vec();

    let account = get_account_by_id(account_id)
        .map_err(|e| std::io::Error::other(format!("Failed to get account: {e}")))?;
    let posts: Vec<Post> = api::get_posts(&account, page).await;

    let idf: Option<HashMap<&str, f32>> = None;

    let mut scored: Vec<(Post, f32)> = Vec::with_capacity(posts.len());
    for post in posts {
        let mut post_tags: Vec<(String, String)> = Vec::new();
        let tmp_post = post.clone();

        post_tags.extend(post.tags.artist.into_iter().map(|t| (t, "artist".into())));
        post_tags.extend(
            post.tags
                .character
                .into_iter()
                .map(|t| (t, "character".into())),
        );
        post_tags.extend(
            post.tags
                .copyright
                .into_iter()
                .map(|t| (t, "copyright".into())),
        );
        post_tags.extend(post.tags.general.into_iter().map(|t| (t, "general".into())));
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

    if let Some(threshold) = affinity_threshold {
        scored.retain(|(_, s)| *s >= threshold);
    }

    Ok(Json(scored))
}

#[launch]
async fn rocket() -> _ {
    let path = default_path().unwrap();
    let _ = reload_from(&path);
    let watcher = start_config_watcher(path).unwrap();

    let (_tx, _rx) = mpsc::channel::<Vec<String>>(QUEUE_CAP);

    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::some_exact(&cfg().frontend_domains),
        allowed_methods: [Method::Get, Method::Post]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        max_age: Some(86400),
        ..Default::default()
    }
    .to_cors()
    .expect("CORS configuration");

    rocket::build()
        .manage(Mutex::new(watcher))
        .mount(
            "/",
            routes![
                process_posts,
                get_account_tag_counts,
                get_account_id,
                get_account_name,
                create_account,
                get_recommendations,
            ],
        )
        .attach(cors)
        .attach(DbInit)
}
