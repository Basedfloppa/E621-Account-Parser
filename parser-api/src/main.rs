#[macro_use]
extern crate rocket;

use anyhow::Context;
use arc_swap::ArcSwap;
use chrono::Utc;
use log::info;
use moka::sync::Cache;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use rocket::{State, serde::json::Json};
use rusqlite::Result;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, LazyLock},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{Semaphore, mpsc},
    task::JoinSet,
};

use crate::{
    cors::Cors,
    db::{
        DbInit, find_missing_relations, get_account_by_id, get_account_by_name, get_tag_counts,
        set_account, set_tag_aliases, set_tag_counts, set_tag_implications,
    },
    models::{Post, TagCount, TruncatedAccount, UserApiResponse},
    rocket::serde::json,
    utils::Priors,
};

mod api;
mod cors;
mod db;
mod models;
mod utils;

const QUEUE_CAP: usize = 10_000;
const BATCH_SIZE: usize = 500;
const DEDUP_TTL_SECS: u64 = 60 * 30;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub db_url: String,
    pub port: u16,
}

static CONFIG: LazyLock<ArcSwap<Config>> = LazyLock::new(|| {
    let p = default_path().expect("config path");
    let cfg = load_config(&p).expect("initial config");
    ArcSwap::from_pointee(cfg)
});

#[derive(Clone)]
struct AppState {
    tx: mpsc::Sender<Vec<String>>,
}

struct ConfigWatcher {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn load_config(p: &Path) -> anyhow::Result<Config> {
    let s = fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
    toml::from_str(&s).context("parsing config.toml")
}

fn default_path() -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from("config.toml"))
}

fn start_config_watcher(path: PathBuf) -> anyhow::Result<ConfigWatcher> {
    let parent = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();

    let handle = thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .expect("create watcher");

        watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .expect("watch parent");

        let mut last_mtime: Option<SystemTime> = file_mtime(&path).ok();

        while !stop_flag.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Ok(event)) => {
                    if event
                        .paths
                        .iter()
                        .any(|p| p == &path || p.file_name() == path.file_name())
                    {
                        thread::sleep(Duration::from_millis(120));

                        if let Ok(mtime) = file_mtime(&path) {
                            if last_mtime.map_or(true, |old| old < mtime) {
                                match reload_from(&path) {
                                    Ok(_) => {
                                        last_mtime = Some(mtime);
                                        eprintln!("[config] reloaded {}", path.display());
                                    }
                                    Err(e) => {
                                        eprintln!("[config] reload failed: {e:#}");
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("[config] watch error: {e}"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(ConfigWatcher {
        stop,
        handle: Some(handle),
    })
}

fn file_mtime(p: &Path) -> std::io::Result<SystemTime> {
    fs::metadata(p)?.modified()
}

pub fn cfg() -> Arc<Config> {
    CONFIG.load_full()
}

pub fn reload_from(p: &Path) -> anyhow::Result<()> {
    let new = load_config(p)?;
    CONFIG.store(Arc::new(new));
    Ok(())
}

async fn relations_worker(mut rx: mpsc::Receiver<Vec<String>>, dedup: Cache<String, ()>) {
    while let Some(batch) = rx.recv().await {
        let unique: Vec<String> = batch
            .into_iter()
            .filter(|t| {
                if dedup.contains_key(t) {
                    false
                } else {
                    dedup.insert(t.clone(), ());
                    true
                }
            })
            .collect();

        if unique.is_empty() {
            continue;
        }

        for chunk in unique.chunks(BATCH_SIZE) {
            if let Err(e) = refresh_relations_chunk(chunk).await {
                eprintln!("warn: background refresh chunk failed: {e}");
            }
        }
    }
}

async fn refresh_relations_chunk(tags: &[String]) -> Result<(), String> {
    use std::collections::HashSet;
    let set: HashSet<String> = tags.iter().cloned().collect();
    refresh_relations_for_tags(&set).await
}

async fn refresh_relations_for_tags(tags: &HashSet<String>) -> Result<(), String> {
    let (miss_alias, miss_imp) = find_missing_relations(tags)?;

    let con_limit = 10usize;

    {
        let sem = Arc::new(Semaphore::new(con_limit));
        let mut jobs = JoinSet::new();

        for t in miss_alias {
            let sem = Arc::clone(&sem);
            jobs.spawn(async move {
                let _permit = sem.acquire_owned().await.expect("semaphore");
                let res = crate::api::fetch_tag_aliases_for(&t).await;
                (t, res)
            });
        }

        while let Some(res) = jobs.join_next().await {
            let (tag, result) = res.map_err(|e| format!("alias task join: {e}"))?;
            match result {
                Ok(list) => set_tag_aliases(&list)?,
                Err(err) => eprintln!("warn: alias fetch failed for {tag}: {err}"),
            }
        }
    }

    {
        let sem = Arc::new(Semaphore::new(con_limit));
        let mut jobs = JoinSet::new();

        for t in miss_imp {
            let sem = Arc::clone(&sem);
            jobs.spawn(async move {
                let _permit = sem.acquire_owned().await.expect("semaphore");
                let res = crate::api::fetch_tag_implications_for(&t).await;
                (t, res)
            });
        }

        while let Some(res) = jobs.join_next().await {
            let (tag, result) = res.map_err(|e| format!("imp task join: {e}"))?;
            match result {
                Ok(list) => set_tag_implications(&list)?,
                Err(err) => eprintln!("warn: implication fetch failed for {tag}: {err}"),
            }
        }
    }

    Ok(())
}

#[post("/process/<account_id>")]
async fn process_posts(account_id: i32, state: &State<AppState>) -> Result<String, String> {
    let account = db::get_account_by_id(account_id).map_err(|e| e.to_string())?;
    let user = api::get_account(&account).await;
    let favcount = match user {
        UserApiResponse::FullCurrentUser(u) => u.favorite_count,
        UserApiResponse::FullUser(u) => u.favorite_count,
    };
    let pages = (favcount / api::LIMIT) + (if favcount % api::LIMIT > 0 { 1 } else { 0 });

    db::drop_account_posts(account_id).map_err(|e| format!("Failed to drop account posts: {e}"))?;

    let mut seen_tags: HashSet<String> = HashSet::new();

    for i in 1..=pages {
        let posts = api::get_favorites(&account, i).await;
        info!("{} post(s) found on page {}", posts.len(), i);

        db::save_posts(&posts, account.id).map_err(|e| format!("Failed to save posts: {e}"))?;

        let page_tagset: HashSet<String> = posts
            .iter()
            .flat_map(|p| {
                p.tags
                    .artist
                    .iter()
                    .chain(p.tags.character.iter())
                    .chain(p.tags.contributor.iter())
                    .chain(p.tags.copyright.iter())
                    .chain(p.tags.general.iter())
                    .chain(p.tags.invalid.iter())
                    .chain(p.tags.lore.iter())
                    .chain(p.tags.meta.iter())
                    .chain(p.tags.species.iter())
                    .map(|t| t.to_lowercase().trim().to_string())
            })
            .collect();

        let to_refresh: Vec<String> = page_tagset.difference(&seen_tags).cloned().collect();

        if !to_refresh.is_empty() {
            match state.tx.try_send(to_refresh.clone()) {
                Ok(_) => info!("queued {} tags for background refresh", to_refresh.len()),
                Err(err) => {
                    eprintln!("warn: tag refresh queue is full ({err}); skipping enqueue");
                }
            }
            seen_tags.extend(page_tagset.into_iter());
        }

        let maps = db::load_relation_maps_for(&seen_tags).map_err(|e| format!("load maps: {e}"))?;
        db::save_posts_tags_batch_with_maps(&posts, &maps)
            .map_err(|e| format!("Failed to save tags for page {i}: {e}"))?;
    }

    set_tag_counts(account_id).map_err(|e| format!("Failed to set account tag counts: {e}"))?;
    Ok(json::to_string(&"okay :3").unwrap())
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
    let user = api::get_account(&account).await;
    let blacklisted_tags = match user {
        UserApiResponse::FullCurrentUser(u) => u.blacklisted_tags,
        UserApiResponse::FullUser(_) => "".to_string(),
    };

    match set_account(
        account.id,
        &account.name,
        &account.api_key,
        &blacklisted_tags,
    ) {
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
        ("invalid", 0.2),
        ("lore", 0.6),
        ("contributor", 0.8),
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

    let account = db::get_account_by_id(account_id)
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

    if let Some(threshold) = affinity_threshold {
        scored.retain(|(_, s)| *s >= threshold);
    }

    Ok(Json(scored))
}

#[launch]
async fn rocket() -> _ {
    let path = default_path().unwrap();
    let _ = reload_from(&path);
    let _watcher = start_config_watcher(path).unwrap();

    let (tx, rx) = mpsc::channel::<Vec<String>>(QUEUE_CAP);

    let dedup = Cache::builder()
        .time_to_live(Duration::from_secs(DEDUP_TTL_SECS))
        .max_capacity(500_000)
        .build();

    tokio::spawn(relations_worker(rx, dedup));

    rocket::build()
        .manage(AppState { tx })
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
        .attach(Cors)
        .attach(DbInit)
}
