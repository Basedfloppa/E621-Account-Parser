-- tables
CREATE TABLE tags (
                      id INTEGER PRIMARY KEY AUTOINCREMENT,
                      name TEXT NOT NULL,
                      group_type TEXT NOT NULL CHECK (group_type IN (
                                                                     'artist','character','copyright',
                                                                     'general','lore','meta','species'
                          )),
                      UNIQUE(name, group_type)
) STRICT;

CREATE TABLE posts (
                       id INTEGER PRIMARY KEY,
                       created_at TEXT NOT NULL,
                       score_total INTEGER NOT NULL,
                       fav_count INTEGER NOT NULL,
                       rating TEXT NOT NULL CHECK (rating IN ('s','q','e')),
                       last_seen_at TEXT NOT NULL
) STRICT;

CREATE TABLE accounts (
                          id INTEGER PRIMARY KEY,
                          name TEXT NOT NULL,
                          blacklisted_tags TEXT,
                          UNIQUE(id, name)
) STRICT;

CREATE TABLE tags_posts (
                            tag_id INTEGER NOT NULL,
                            post_id INTEGER NOT NULL,
                            PRIMARY KEY(tag_id, post_id),
                            FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE,
                            FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE accounts_post (
                               post_id INTEGER NOT NULL,
                               account_id INTEGER NOT NULL,
                               PRIMARY KEY(post_id, account_id),
                               FOREIGN KEY(post_id) REFERENCES posts(id) ON DELETE CASCADE,
                               FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE account_tag_counts (
                                    account_id INTEGER NOT NULL,
                                    tag_name   TEXT NOT NULL,
                                    group_type TEXT NOT NULL,
                                    count      INTEGER NOT NULL,
                                    PRIMARY KEY(account_id, tag_name, group_type),
                                    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE tag_aliases (
                             antecedent_name TEXT PRIMARY KEY,
                             consequent_name TEXT NOT NULL,
                             status TEXT NOT NULL CHECK (status IN ('active','deleted','processing','queued','retired','error','pending')),
                             created_at TEXT,
                             updated_at TEXT
) STRICT;

CREATE TABLE tag_implications (
                                  antecedent_name TEXT NOT NULL,
                                  consequent_name TEXT NOT NULL,
                                  status TEXT NOT NULL CHECK (status IN ('active','deleted','processing','queued','retired','error','pending')),
                                  created_at TEXT,
                                  updated_at TEXT,
                                  PRIMARY KEY(antecedent_name, consequent_name)
) STRICT;

CREATE TABLE tag_relation_probe (
                                    tag TEXT PRIMARY KEY,
                                    aliases_last_checked TIMESTAMP,
                                    aliases_count INTEGER NOT NULL DEFAULT 0,
                                    implications_last_checked TIMESTAMP,
                                    implications_count INTEGER NOT NULL DEFAULT 0
);

-- indexes
CREATE INDEX idx_ap_acc_post            ON accounts_post(account_id, post_id);
CREATE INDEX idx_atc_acc_group          ON account_tag_counts(account_id, group_type);
CREATE INDEX idx_tag_aliases_consequent ON tag_aliases(consequent_name);
CREATE INDEX idx_tag_imps_ante          ON tag_implications(antecedent_name);
CREATE INDEX idx_tags_name_group        ON tags(name, group_type);
CREATE INDEX idx_tp_tag                 ON tags_posts(tag_id);
CREATE INDEX idx_tp_post                ON tags_posts(post_id);
CREATE INDEX idx_ap_account             ON accounts_post(account_id);
CREATE INDEX idx_ap_post                ON accounts_post(post_id);
