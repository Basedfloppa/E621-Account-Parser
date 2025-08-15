use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rocket::{
    Build, Rocket,
    fairing::{Fairing, Info, Kind},
};
use rusqlite::{Connection, Result, params};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use crate::models::{Post, TagAlias, TagCount, TagImplication, TruncatedAccount};

pub struct RelationMaps {
    pub alias: HashMap<String, String>,
    pub implied: HashMap<String, Vec<String>>,
}

pub struct DbInit;

const IN_CHUNK: usize = 800;
const EMPTY_RECHECK_TTL: ChronoDuration = ChronoDuration::days(30);

#[derive(Debug, Clone)]
pub struct TagRelationProbe {
    pub tag: String,
    pub aliases_last_checked: Option<DateTime<Utc>>,
    pub aliases_count: i64,
    pub implications_last_checked: Option<DateTime<Utc>>,
    pub implications_count: i64,
}

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
                eprintln!("Database initialization failed: {e}");
                Err(rocket)
            }
        }
    }
}

fn chunk<'a, T: 'a + Clone>(v: &'a [T], size: usize) -> impl Iterator<Item = Vec<T>> + 'a {
    v.chunks(size).map(|c| c.to_vec())
}

fn parse_rfc3339_opt(s: Option<String>) -> Option<DateTime<Utc>> {
    s.and_then(|v| chrono::DateTime::parse_from_rfc3339(&v).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn open_db() -> Result<Connection, String> {
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

fn ensure_sqlite() -> Result<(), String> {
    let connection = open_db()?;

    connection
        .execute_batch(
            "
    CREATE TABLE IF NOT EXISTS tags (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        group_type TEXT NOT NULL CHECK (group_type IN (
            'artist','character','contributor','copyright',
            'general','invalid','lore','meta','species'
        )),
        UNIQUE(name, group_type)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS posts (
        id INTEGER PRIMARY KEY,                
        created_at TEXT NOT NULL,            
        score_total INTEGER NOT NULL,
        fav_count INTEGER NOT NULL,
        rating TEXT NOT NULL CHECK (rating IN ('s','q','e')),
        last_seen_at TEXT NOT NULL              
    ) STRICT;

    CREATE TABLE IF NOT EXISTS accounts (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        api_key TEXT NOT NULL,
        blacklisted_tags TEXT,
        UNIQUE(id, name)
    ) STRICT;

    CREATE TABLE IF NOT EXISTS tags_posts (
        tag_id INTEGER NOT NULL,
        post_id INTEGER NOT NULL,
        PRIMARY KEY(tag_id, post_id),
        FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE,
        FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE
    ) STRICT;
    
    CREATE TABLE IF NOT EXISTS accounts_post (
        post_id INTEGER NOT NULL,
        account_id INTEGER NOT NULL,
        PRIMARY KEY(post_id, account_id),
        FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE,
        FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
    ) STRICT;

    CREATE TABLE IF NOT EXISTS account_tag_counts (
        account_id INTEGER NOT NULL,
        tag_name   TEXT NOT NULL,
        group_type TEXT NOT NULL,
        count      INTEGER NOT NULL,
        PRIMARY KEY(account_id, tag_name, group_type),
        FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
    ) STRICT;

    CREATE TABLE IF NOT EXISTS tag_aliases (
        antecedent_name TEXT PRIMARY KEY,
        consequent_name TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('active','deleted','processing','queued','retired','error','pending')),
        created_at TEXT,
        updated_at TEXT
    ) STRICT;

    CREATE TABLE IF NOT EXISTS tag_implications (
        antecedent_name TEXT NOT NULL,
        consequent_name TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('active','deleted','processing','queued','retired','error','pending')),
        created_at TEXT,
        updated_at TEXT,
        PRIMARY KEY(antecedent_name, consequent_name)
    ) STRICT;
    
    CREATE TABLE IF NOT EXISTS tag_relation_probe (
        tag TEXT PRIMARY KEY,
        aliases_last_checked TIMESTAMP,
        aliases_count INTEGER NOT NULL DEFAULT 0,
        implications_last_checked TIMESTAMP,
        implications_count INTEGER NOT NULL DEFAULT 0
    );
    ",
        )
        .map_err(|e| format!("Failed to execute batch: {e}"))?;

    connection
        .execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_ap_acc_post ON accounts_post(account_id, post_id);
            CREATE INDEX IF NOT EXISTS idx_atc_acc_group ON account_tag_counts(account_id, group_type);
            CREATE INDEX IF NOT EXISTS idx_tag_aliases_consequent ON tag_aliases(consequent_name);
            CREATE INDEX IF NOT EXISTS idx_tag_imps_ante ON tag_implications(antecedent_name);
            CREATE INDEX IF NOT EXISTS idx_tags_name_group ON tags(name, group_type);
            CREATE INDEX IF NOT EXISTS idx_tp_tag    ON tags_posts(tag_id);
            CREATE INDEX IF NOT EXISTS idx_tp_post   ON tags_posts(post_id);
            CREATE INDEX IF NOT EXISTS idx_ap_account ON accounts_post(account_id);
            CREATE INDEX IF NOT EXISTS idx_ap_post    ON accounts_post(post_id);
        ",
        )
        .map_err(|e| format!("Failed to execute batch: {e}"))?;

    Ok(())
}

pub fn set_account(
    account_id: i32,
    name: &str,
    api_key: &str,
    blacklisted_tags: &str,
) -> Result<(), String> {
    open_db()?
        .execute(
            "INSERT OR REPLACE INTO accounts (id, name, api_key, blacklisted_tags) VALUES (?1, ?2, ?3, ?4)",
            params![account_id, name, api_key, blacklisted_tags],
        )
        .map_err(|e| format!("Failed to execute transaction: {e}"))?;

    Ok(())
}

pub fn get_account_by_name(name: String) -> rusqlite::Result<TruncatedAccount, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            r#"
        SELECT a.id, a.name, a.api_key, a.blacklisted_tags
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
                api_key: row.get(2)?,
                blacklisted_tags: row.get(3)?,
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

pub fn get_account_by_id(id: i32) -> rusqlite::Result<TruncatedAccount, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            r#"
        SELECT a.id, a.name, a.api_key, a.blacklisted_tags
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
                api_key: row.get(2)?,
                blacklisted_tags: row.get(3)?,
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

pub fn get_tag_counts(account_id: i32) -> rusqlite::Result<Vec<TagCount>, String> {
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

pub fn set_tag_aliases(aliases: &[TagAlias]) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn.transaction().map_err(|e| format!("tx: {e}"))?;

    {
        let mut st = tx
            .prepare_cached(
                r#"
        INSERT INTO tag_aliases(antecedent_name, consequent_name, status, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(antecedent_name) DO UPDATE SET
          consequent_name=excluded.consequent_name,
          status=excluded.status,
          created_at=excluded.created_at,
          updated_at=excluded.updated_at
        "#,
            )
            .map_err(|e| format!("prep alias upsert: {e}"))?;

        for a in aliases {
            let created: Option<String> = a.created_at.map(|t| t.to_rfc3339());
            let updated: Option<String> = a.updated_at.map(|t| t.to_rfc3339());
            st.execute(params![
                a.antecedent_name,
                a.consequent_name,
                a.status,
                created,
                updated
            ])
            .map_err(|e| format!("alias upsert: {e}"))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("commit alias upsert: {e}"))?;
    Ok(())
}

pub fn set_tag_implications(imps: &[TagImplication]) -> Result<(), String> {
    let mut conn = open_db()?;
    let tx = conn.transaction().map_err(|e| format!("tx: {e}"))?;

    {
        let mut st = tx.prepare_cached(
        r#"
        INSERT INTO tag_implications(antecedent_name, consequent_name, status, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(antecedent_name, consequent_name) DO UPDATE SET
          status=excluded.status,
          created_at=excluded.created_at,
          updated_at=excluded.updated_at
        "#,
    ).map_err(|e| format!("prep imp upsert: {e}"))?;

        for i in imps {
            let created: Option<String> = i.created_at.map(|t| t.to_rfc3339());
            let updated: Option<String> = i.updated_at.map(|t| t.to_rfc3339());

            st.execute(params![
                i.antecedent_name,
                i.consequent_name,
                i.status,
                &created,
                &updated
            ])
            .map_err(|e| format!("imp upsert: {e}"))?;
        }
    }

    tx.commit().map_err(|e| format!("commit imp upsert: {e}"))?;
    Ok(())
}

pub fn get_tag_aliases() -> Result<Vec<TagAlias>, String> {
    let connection = open_db()?;
    let mut stmt = connection
        .prepare("SELECT * FROM tag_aliases")
        .map_err(|e| format!("Failed to construct query: {e}"))?;

    let aliases = stmt
        .query_map([], |row| {
            Ok(TagAlias {
                id: row.get(0)?,
                antecedent_name: row.get(1)?,
                consequent_name: row.get(2)?,
                status: row.get(3)?,
                created_at: Some(
                    DateTime::<Utc>::from_str(&row.get::<usize, String>(4)?.to_string())
                        .unwrap_or_default(),
                ),
                updated_at: Some(
                    DateTime::<Utc>::from_str(&row.get::<usize, String>(4)?.to_string())
                        .unwrap_or_default(),
                ),
            })
        })
        .map_err(|e| format!("Failed to get accounts: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to enumerate accounts: {e}"))?;

    Ok(aliases)
}

pub fn get_tag_implications() -> Result<Vec<TagImplication>, String> {
    let connection = open_db()?;
    let mut stmt = connection
        .prepare("SELECT * FROM tag_implications")
        .map_err(|e| format!("Failed to construct query: {e}"))?;

    let implications = stmt
        .query_map([], |row| {
            Ok(TagImplication {
                id: row.get(0)?,
                antecedent_name: row.get(1)?,
                consequent_name: row.get(2)?,
                status: row.get(3)?,
                created_at: Some(
                    DateTime::<Utc>::from_str(&row.get::<usize, String>(4)?.to_string())
                        .unwrap_or_default(),
                ),
                updated_at: Some(
                    DateTime::<Utc>::from_str(&row.get::<usize, String>(4)?.to_string())
                        .unwrap_or_default(),
                ),
            })
        })
        .map_err(|e| format!("Failed to get accounts: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to enumerate accounts: {e}"))?;

    Ok(implications)
}

pub fn find_missing_relations(
    tags: &HashSet<String>,
) -> Result<(Vec<String>, Vec<String>), String> {
    if tags.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let conn = open_db()?;

    let mut have_alias = HashSet::with_capacity(tags.len());
    let mut have_imp = HashSet::with_capacity(tags.len());

    let all: Vec<String> = tags.iter().cloned().collect();

    for part in chunk(&all, IN_CHUNK) {
        let placeholders = std::iter::repeat_n("?", part.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut st = conn
            .prepare(&format!(
                "SELECT antecedent_name FROM tag_aliases
             WHERE status='active' AND antecedent_name IN ({placeholders})"
            ))
            .map_err(|e| format!("prep alias IN: {e}"))?;
        let rows = st
            .query_map(rusqlite::params_from_iter(part.iter()), |r| {
                r.get::<usize, String>(0)
            })
            .map_err(|e| format!("alias IN query: {e}"))?;
        for r in rows {
            have_alias.insert(r.map_err(|e| format!("alias row: {e}"))?);
        }
    }

    for part in chunk(&all, IN_CHUNK) {
        let placeholders = std::iter::repeat_n("?", part.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut st = conn
            .prepare(&format!(
                "SELECT antecedent_name FROM tag_implications
             WHERE status='active' AND antecedent_name IN ({placeholders})"
            ))
            .map_err(|e| format!("prep imp IN: {e}"))?;
        let rows = st
            .query_map(rusqlite::params_from_iter(part.iter()), |r| {
                r.get::<usize, String>(0)
            })
            .map_err(|e| format!("imp IN query: {e}"))?;
        for r in rows {
            have_imp.insert(r.map_err(|e| format!("imp row: {e}"))?);
        }
    }

    let probe_map = get_probe_map(tags)?;
    let now = Utc::now();

    let mut miss_alias = Vec::new();
    let mut miss_imp = Vec::new();

    for t in tags {
        if !have_alias.contains(t) {
            let skip_alias = probe_map
                .get(t)
                .and_then(|p| p.aliases_last_checked)
                .map_or(false, |last| {
                    probe_map.get(t).unwrap().aliases_count == 0
                        && now.signed_duration_since(last) <= EMPTY_RECHECK_TTL
                });
            if !skip_alias {
                miss_alias.push(t.clone());
            }
        }

        if !have_imp.contains(t) {
            let skip_imp = probe_map
                .get(t)
                .and_then(|p| p.implications_last_checked)
                .map_or(false, |last| {
                    probe_map.get(t).unwrap().implications_count == 0
                        && now.signed_duration_since(last) <= EMPTY_RECHECK_TTL
                });
            if !skip_imp {
                miss_imp.push(t.clone());
            }
        }
    }

    Ok((miss_alias, miss_imp))
}

pub fn load_relation_maps_for(tags: &HashSet<String>) -> Result<RelationMaps, String> {
    let mut alias = HashMap::new();
    let mut implied: HashMap<String, Vec<String>> = HashMap::new();
    if tags.is_empty() {
        return Ok(RelationMaps { alias, implied });
    }

    let conn = open_db()?;
    let all: Vec<String> = tags.iter().cloned().collect();

    // aliases
    for part in chunk(&all, IN_CHUNK) {
        let placeholders = std::iter::repeat("?")
            .take(part.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut st = conn
            .prepare(&format!(
                "SELECT antecedent_name, consequent_name FROM tag_aliases
             WHERE status='active' AND antecedent_name IN ({})",
                placeholders
            ))
            .map_err(|e| format!("prep alias map: {e}"))?;
        let rows = st
            .query_map(rusqlite::params_from_iter(part.iter()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| format!("alias map query: {e}"))?;
        for r in rows {
            let (a, c) = r.map_err(|e| format!("alias map row: {e}"))?;
            alias.insert(a, c);
        }
    }

    // implications
    for part in chunk(&all, IN_CHUNK) {
        let placeholders = std::iter::repeat("?")
            .take(part.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut st = conn
            .prepare(&format!(
                "SELECT antecedent_name, consequent_name FROM tag_implications
             WHERE status='active' AND antecedent_name IN ({})",
                placeholders
            ))
            .map_err(|e| format!("prep imp map: {e}"))?;
        let rows = st
            .query_map(rusqlite::params_from_iter(part.iter()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| format!("imp map query: {e}"))?;
        for r in rows {
            let (a, c) = r.map_err(|e| format!("imp map row: {e}"))?;
            implied.entry(a).or_default().push(c);
        }
    }

    Ok(RelationMaps { alias, implied })
}

pub fn save_posts_tags_batch_with_maps(posts: &[Post], maps: &RelationMaps) -> Result<(), String> {
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
            .prepare_cached("INSERT OR IGNORE INTO tags_posts (tag_id, post_id) VALUES (?1, ?2)")
            .map_err(|e| format!("prep link: {e}"))?;

        let mut touch = |name: &str, group: &str, post_id: i64| -> Result<(), String> {
            insert_tag
                .execute(params![name, group])
                .map_err(|e| format!("ins tag: {e}"))?;
            let id: i64 = select_id
                .query_row(params![name, group], |r| r.get(0))
                .map_err(|e| format!("get id {name}:{group}: {e}"))?;
            link.execute(params![id, post_id])
                .map_err(|e| format!("link: {e}"))?;
            Ok(())
        };

        for post in posts {
            for (group, tags) in [
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
                for raw in tags {
                    let mut canonical = raw.to_lowercase().trim().to_string();
                    if let Some(c) = maps.alias.get(&canonical) {
                        canonical = c.clone();
                    }
                    touch(&canonical, group, post.id)?;
                    if let Some(list) = maps.implied.get(&canonical) {
                        for imp in list {
                            let imp_canonical =
                                maps.alias.get(imp).cloned().unwrap_or_else(|| imp.clone());
                            touch(&imp_canonical, group, post.id)?;
                        }
                    }
                }
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("commit save_posts_tags_batch_with_maps: {e}"))?;
    Ok(())
}

pub fn record_alias_probe(tag: &str, count: usize) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        r#"
        INSERT INTO tag_relation_probe (tag, aliases_last_checked, aliases_count)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(tag) DO UPDATE SET
          aliases_last_checked = excluded.aliases_last_checked,
          aliases_count        = excluded.aliases_count
        "#,
        params![tag, Utc::now().to_rfc3339(), count as i64],
    )
    .map_err(|e| format!("record_alias_probe: {e}"))?;
    Ok(())
}

pub fn record_implication_probe(tag: &str, count: usize) -> Result<(), String> {
    let conn = open_db()?;
    conn.execute(
        r#"
        INSERT INTO tag_relation_probe (tag, implications_last_checked, implications_count)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(tag) DO UPDATE SET
          implications_last_checked = excluded.implications_last_checked,
          implications_count        = excluded.implications_count
        "#,
        params![tag, Utc::now().to_rfc3339(), count as i64],
    )
    .map_err(|e| format!("record_implication_probe: {e}"))?;
    Ok(())
}

pub fn get_probe_map(tags: &HashSet<String>) -> Result<HashMap<String, TagRelationProbe>, String> {
    let mut out = HashMap::new();
    if tags.is_empty() {
        return Ok(out);
    }
    let conn = open_db()?;
    let all: Vec<String> = tags.iter().cloned().collect();

    for part in chunk(&all, IN_CHUNK) {
        let placeholders = std::iter::repeat_n("?", part.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut st = conn
            .prepare(&format!(
                r#"
            SELECT tag, aliases_last_checked, aliases_count,
                   implications_last_checked, implications_count
            FROM tag_relation_probe
            WHERE tag IN ({placeholders})
            "#
            ))
            .map_err(|e| format!("prep probe IN: {e}"))?;

        let rows = st
            .query_map(rusqlite::params_from_iter(part.iter()), |r| {
                Ok(TagRelationProbe {
                    tag: r.get::<_, String>(0)?,
                    aliases_last_checked: parse_rfc3339_opt(r.get::<_, Option<String>>(1)?),
                    aliases_count: r.get::<_, i64>(2)?,
                    implications_last_checked: parse_rfc3339_opt(r.get::<_, Option<String>>(3)?),
                    implications_count: r.get::<_, i64>(4)?,
                })
            })
            .map_err(|e| format!("probe IN query: {e}"))?;

        for row in rows {
            let p = row.map_err(|e| format!("probe row: {e}"))?;
            out.insert(p.tag.clone(), p);
        }
    }

    Ok(out)
}
