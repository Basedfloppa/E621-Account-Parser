use rocket::{
    Build, Rocket,
    fairing::{Fairing, Info, Kind},
};
use rusqlite::{Connection, Result, params};

use crate::models::{TruncatedAccount, TruncatedPost, TagCount};

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
            Ok(_) => Ok(rocket),
            Err(e) => {
                eprintln!("Database initialization failed: {}", e);
                Err(rocket)
            }
        }
    }
}

fn open_db() -> Result<Connection> {
    let connection = Connection::open("database.db")?;
    connection.execute("PRAGMA foreign_keys = ON;", [])?;
    Ok(connection)
}

fn ensure_sqlite() -> Result<()> {
    open_db()?.execute_batch(
        "
    CREATE TABLE IF NOT EXISTS tags (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        group_type TEXT NOT NULL,
        UNIQUE(name, group_type)
    );
    CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY);
    CREATE TABLE IF NOT EXISTS accounts (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        api_key TEXT NOT NULL,
        blacklisted_tags TEXT NOT NULL,
        UNIQUE(id, name)
    );
    CREATE TABLE IF NOT EXISTS tags_posts (
        tag_id INTEGER NOT NULL,
        post_id INTEGER NOT NULL,
        PRIMARY KEY(tag_id, post_id),
        FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE,
        FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE
    );
    CREATE TABLE IF NOT EXISTS accounts_post (
        post_id INTEGER NOT NULL,
        account_id INTEGER NOT NULL,
        PRIMARY KEY(post_id, account_id),
        FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE,
        FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
    );
    ",
    )?;

    Ok(())
}

pub fn save_account(account_id: i32, name: &str, api_key: &str, blacklisted_tags: &str) -> Result<()> {
    open_db()?.execute(
        "INSERT OR REPLACE INTO accounts (id, name, api_key, blacklisted_tags) VALUES (?1, ?2, ?3, ?4)",
        params![account_id, name, api_key, blacklisted_tags],
    )?;
    Ok(())
}

pub fn save_posts(posts: &[TruncatedPost], account_id: i32) -> Result<()> {
    let mut connection = open_db()?;
    let tx = connection.transaction()?;

    {
        let mut insert_post = tx.prepare_cached("INSERT OR IGNORE INTO posts (id) VALUES (?1);")?;
        let mut insert_account = tx.prepare_cached(
            "INSERT OR IGNORE INTO accounts_post (account_id, post_id) VALUES (?1, ?2);",
        )?;

        for post in posts {
            insert_post.execute(params![post.id])?;
            insert_account.execute(params![account_id, post.id])?;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn save_post_tags(post: &TruncatedPost) -> Result<()> {
    let mut connection = open_db()?;
    let tx = connection.transaction()?;

    {
        let mut insert_tag_stmt =
            tx.prepare_cached("INSERT OR IGNORE INTO tags (name, group_type) VALUES (?1, ?2)")?;
        let mut select_tag_id_stmt =
            tx.prepare_cached("SELECT id FROM tags WHERE name = ?1 AND group_type = ?2")?;
        let mut insert_link_stmt = tx
            .prepare_cached("INSERT OR IGNORE INTO tags_posts (tag_id, post_id) VALUES (?1, ?2)")?;

        for (group_type, tags) in [
            ("artist", &post.tags.artist),
            ("character", &post.tags.character),
            ("contributor", &post.tags.contributor),
            ("copyright", &post.tags.copyright),
            ("general", &post.tags.general),
            ("invalid", &post.tags.invalid),
            ("lore", &post.tags.lore),
            ("meta", &post.tags.meta),
            ("species", &post.tags.species),
        ] {
            for tag in tags {
                insert_tag_stmt.execute(params![tag, group_type])?;

                let tag_id = select_tag_id_stmt
                    .query_row(params![tag, group_type], |row| row.get::<usize, i64>(0))
                    .unwrap_or_else(|_| {
                        panic!("Tag must exist after insert: {}:{}", group_type, tag)
                    });

                insert_link_stmt.execute(params![tag_id, post.id])?;
            }
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn get_tag_counts(account_id: i32) -> rusqlite::Result<Vec<TagCount>> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT t.name, t.group_type, COUNT(*) as count
        FROM tags t
        INNER JOIN tags_posts tp ON t.id = tp.tag_id
        INNER JOIN accounts_post ap ON tp.post_id = ap.post_id
        WHERE ap.account_id = ?
        GROUP BY t.name, t.group_type
        ORDER BY count DESC
        "#,
    )?;

    let counts = stmt
        .query_map([account_id], |row| {
            Ok(TagCount {
                name: row.get(0)?,
                group_type: row.get(1)?,
                count: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(counts)
}

pub fn get_account_by_name(name: String) -> rusqlite::Result<TruncatedAccount> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT a.id, a.name, a.api_key, a.blacklisted_tags
        FROM accounts a
        WHERE a.name = ?
        "#,
    )?;
    let accounts = stmt
        .query_map([name], |row| {
            Ok(TruncatedAccount {
                id: row.get(0)?,
                name: row.get(1)?,
                api_key: row.get(2)?,
                blacklisted_tags: row.get(3)?
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(accounts[0].clone())
}

pub fn get_account_by_id(id: i32) -> rusqlite::Result<TruncatedAccount> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT a.id, a.name, a.api_key, a.blacklisted_tags
        FROM accounts a
        WHERE a.id = ?
        "#,
    )?;
    let accounts = stmt
        .query_map([id], |row| {
            Ok(TruncatedAccount {
                id: row.get(0)?,
                name: row.get(1)?,
                api_key: row.get(2)?,
                blacklisted_tags: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(accounts[0].clone())
}
