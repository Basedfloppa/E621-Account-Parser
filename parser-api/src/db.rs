use crate::models::{Post, TagCount, TruncatedAccount};
use chrono::Utc;
use rocket::{
    Build, Rocket,
    fairing::{Fairing, Info, Kind},
};
use rusqlite::{Connection, Result, params};
use std::{collections::HashSet, fs};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

pub struct DbInit;

#[rocket::async_trait]
impl Fairing for DbInit {
    fn info(&self) -> Info {
        Info {
            name: "SQLite DB Initializer",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        match ensure_sqlite() {
            Ok(_) => {
                println!("SQLite DB Initialized");
                Ok(rocket)
            }
            Err(e) => {
                println!("Database initialization failed: {e}");
                Err(rocket)
            }
        }
    }
}

fn open_db() -> Result<Connection, String> {
    if fs::exists("database.db").is_err() {
        if let Err(e) = fs::File::create("database.db") {
            eprintln!("{e}")
        }
    }

    let connection =
        Connection::open("database.db").map_err(|e| format!("Failed to get connection: {e}"))?;

    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA busy_timeout=5000;
            ",
        )
        .map_err(|e| format!("Failed to assert pragma: {e}"))?;

    Ok(connection)
}

pub fn ensure_sqlite() -> Result<(), String> {
    if fs::exists("database.db").is_err() {
        fs::File::create("database.db").map_err(|e| format!("Failed to create file: {e}"))?;
    }

    let mut conn = open_db().map_err(|e| e.to_string())?;

    embedded::migrations::runner()
        .run(&mut conn)
        .map_err(|e| format!("Failed to run migrations: {e}"))?;

    Ok(())
}

pub fn set_account(account_id: i32, name: &str, mut blacklisted_tags: &str) -> Result<(), String> {
    if blacklisted_tags.is_empty() {
        blacklisted_tags = "
gore
scat
watersports
young -rating:s
loli
shota";
    }

    eprint!("{blacklisted_tags:?}");

    open_db()?
        .execute(
            "
            INSERT INTO accounts (id, name, blacklisted_tags) 
            VALUES (?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            blacklisted_tags = excluded.blacklisted_tags",
            params![account_id, name, blacklisted_tags],
        )
        .map_err(|e| format!("Failed to execute transaction: {e}"))?;

    Ok(())
}

pub fn get_account_by_name(name: String) -> Result<TruncatedAccount, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            r#"
        SELECT a.id, a.name, a.blacklisted_tags
        FROM accounts a
        WHERE a.name = ?
        "#,
        )
        .map_err(|e| format!("Failed to construct query: {e}"))?;

    let accounts = stmt
        .query_map([name], |row| {
            Ok(TruncatedAccount {
                id: row.get(0)?,
                name: row.get(1)?,
                blacklist: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to get accounts: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to enumerate accounts: {e}"))?;

    if let Some(account) = accounts.first() {
        Ok(account.clone())
    } else {
        Err("No account found".to_string())
    }
}

pub fn get_account_by_id(id: i32) -> Result<TruncatedAccount, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            r#"
        SELECT a.id, a.name, a.blacklisted_tags
        FROM accounts a
        WHERE a.id = ?
        "#,
        )
        .map_err(|e| format!("Failed to construct query: {e}"))?;

    let accounts = stmt
        .query_map([id], |row| {
            Ok(TruncatedAccount {
                id: row.get(0)?,
                name: row.get(1)?,
                blacklist: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to get accounts: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to enumerate accounts: {e}"))?;

    if let Some(account) = accounts.first() {
        Ok(account.clone())
    } else {
        Err("No account found".to_string())
    }
}

pub fn drop_account_posts(account_id: i32) -> Result<(), String> {
    let mut connection = open_db()?;

    let tx = connection
        .transaction()
        .map_err(|e| format!("Failed to get transaction: {e}"))?;

    {
        let mut clear_account_post = tx
            .prepare_cached("DELETE FROM accounts_post WHERE account_id = ?1")
            .map_err(|e| format!("Failed to prepare transaction: {e}"))?;
        clear_account_post
            .execute(params![account_id])
            .map_err(|e| format!("Failed to execute transaction: {e}"))?;
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    Ok(())
}

pub fn save_posts(posts: &[Post], account_id: i32) -> Result<(), String> {
    let mut connection = open_db()?;

    let tx = connection
        .transaction()
        .map_err(|e| format!("Failed to get transaction: {e}"))?;

    {
        let mut insert_post = tx
            .prepare_cached(
                "
            INSERT INTO posts (id, created_at, score_total, fav_count, rating, last_seen_at) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
            score_total = excluded.score_total,
            fav_count   = excluded.fav_count,
            rating      = excluded.rating,
            last_seen_at= excluded.last_seen_at;",
            )
            .map_err(|e| format!("Failed to prepare transaction: {e}"))?;
        let mut insert_account = tx
            .prepare_cached(
                "INSERT OR IGNORE INTO accounts_post (account_id, post_id) VALUES (?1, ?2);",
            )
            .map_err(|e| format!("Failed to prepare transaction: {e}"))?;

        for post in posts {
            insert_post
                .execute(params![
                    post.id,
                    post.created_at.to_string(),
                    post.score.total,
                    post.fav_count,
                    post.rating.to_string(),
                    Utc::now().to_string()
                ])
                .map_err(|e| format!("Failed to execute transaction: {e}"))?;

            insert_account
                .execute(params![account_id, post.id])
                .map_err(|e| format!("Failed to execute transaction: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    Ok(())
}

pub fn set_tag_counts(account_id: i32) -> Result<(), String> {
    let mut counts: Vec<TagCount> = Vec::new();
    let mut connection = open_db()?;

    {
        let mut stmt = connection
            .prepare(
                r#"
        SELECT t.name, t.group_type, COUNT(*) as count
        FROM tags t
        INNER JOIN tags_posts tp ON t.id = tp.tag_id
        INNER JOIN accounts_post ap ON tp.post_id = ap.post_id
        WHERE ap.account_id = ?
        GROUP BY t.name, t.group_type
        ORDER BY count DESC
        "#,
            )
            .map_err(|e| format!("Failed to construct query: {e}"))?;

        counts = stmt
            .query_map([account_id], |row| {
                Ok(TagCount {
                    name: row.get(0)?,
                    group_type: row.get(1)?,
                    count: row.get(2)?,
                })
            })
            .map_err(|e| format!("Failed to get accounts: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to enumerate accounts: {e}"))?;
    }

    let tx = connection
        .transaction()
        .map_err(|e| format!("Failed to get transaction: {e}"))?;

    {
        tx.execute(
            "DELETE FROM account_tag_counts WHERE account_id = ?1",
            params![account_id],
        )
        .map_err(|e| format!("Failed to delete account_tag_counts: {e}"))?;

        let mut insert_calc = tx
            .prepare_cached(
                "
        INSERT INTO account_tag_counts (account_id, tag_name, group_type, count) 
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(account_id, tag_name, group_type) DO UPDATE SET
        count = excluded.count;
        ",
            )
            .map_err(|e| format!("Failed to prepare transaction: {e}"))?;

        for entry in counts {
            insert_calc
                .execute(params![
                    account_id,
                    entry.name,
                    entry.group_type,
                    entry.count
                ])
                .map_err(|e| format!("Failed to execute transaction: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    Ok(())
}

pub fn get_tag_counts(account_id: i32) -> Result<Vec<TagCount>, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare("SELECT * FROM account_tag_counts WHERE account_id = ?")
        .map_err(|e| format!("Failed to construct query: {e}"))?;

    let counts = stmt
        .query_map([account_id], |row| {
            Ok(TagCount {
                name: row.get(1)?,
                group_type: row.get(2)?,
                count: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to get accounts: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to enumerate accounts: {e}"))?;

    Ok(counts)
}

pub fn save_posts_tags_batch(posts: &[Post], blacklist: &HashSet<String>) -> Result<(), String> {
    if posts.is_empty() {
        return Ok(());
    }

    let mut connection = open_db()?;
    let tx = connection.transaction().map_err(|e| format!("tx: {e}"))?;

    {
        let mut insert_tag = tx
            .prepare_cached("INSERT OR IGNORE INTO tags (name, group_type) VALUES (?1, ?2)")
            .map_err(|e| format!("prep ins tag: {e}"))?;
        let mut select_id = tx
            .prepare_cached("SELECT id FROM tags WHERE name = ?1 AND group_type = ?2")
            .map_err(|e| format!("prep sel id: {e}"))?;
        let mut link = tx
            .prepare_cached("INSERT OR IGNORE INTO tags_posts(tag_id, post_id) VALUES (?1, ?2)")
            .map_err(|e| format!("prep link: {e}"))?;

        for post in posts {
            for (group, tags) in [
                ("artist", &post.tags.artist),
                ("character", &post.tags.character),
                ("copyright", &post.tags.copyright),
                ("general", &post.tags.general),
                ("lore", &post.tags.lore),
                ("meta", &post.tags.meta),
                ("species", &post.tags.species),
            ] {
                let pid = post.id;
                for tag in tags {
                    if tag.is_empty() || blacklist.contains(tag) {
                        continue;
                    }

                    insert_tag
                        .execute(params![&tag, group])
                        .map_err(|e| format!("ins tag: {e}"))?;

                    let tag_id: i64 = select_id
                        .query_row(params![&tag, group], |r| r.get(0))
                        .map_err(|e| format!("get id {tag}:{group}: {e}"))?;

                    link.execute(params![tag_id, pid])
                        .map_err(|e| format!("link tag_id={tag_id} post_id={pid}: {e}"))?;
                }
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("commit save_posts_tags_batch: {e}"))?;
    Ok(())
}
