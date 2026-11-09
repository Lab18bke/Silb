mod bookmarks;
mod downloads;
mod history;
mod sites;

pub use bookmarks::*;
pub use downloads::*;
pub use history::*;
pub use sites::*;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;

pub struct Db {
    conn: Mutex<Connection>,
}

const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE history (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL,
    host TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL DEFAULT '',
    visited_at INTEGER NOT NULL
);
CREATE INDEX history_visited ON history(visited_at DESC);
CREATE INDEX history_url ON history(url);

CREATE TABLE favicons (
    host TEXT PRIMARY KEY,
    data TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE bookmarks (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER REFERENCES bookmarks(id) ON DELETE CASCADE,
    is_folder INTEGER NOT NULL,
    title TEXT NOT NULL,
    url TEXT,
    host TEXT,
    position INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX bookmarks_parent ON bookmarks(parent_id, position);
CREATE INDEX bookmarks_url ON bookmarks(url);
INSERT INTO bookmarks (id, parent_id, is_folder, title, url, position, created_at)
    VALUES (1, NULL, 1, 'Bookmarks bar', NULL, 0, 0), (2, NULL, 1, 'Other bookmarks', NULL, 1, 0);

CREATE TABLE downloads (
    id INTEGER PRIMARY KEY,
    url TEXT NOT NULL,
    path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime TEXT NOT NULL DEFAULT '',
    total_bytes INTEGER,
    received_bytes INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL,
    error TEXT,
    started_at INTEGER NOT NULL,
    finished_at INTEGER
);

CREATE TABLE site_settings (
    origin TEXT NOT NULL,
    kind TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (origin, kind)
);

CREATE TABLE site_zoom (
    host TEXT PRIMARY KEY,
    factor REAL NOT NULL
);

CREATE TABLE kv (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#];

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;",
        )?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
            tx.commit()?;
        }
        conn.execute(
            "UPDATE downloads SET state = 'interrupted', error = 'Silb was closed during the download' WHERE state IN ('in_progress', 'paused')",
            [],
        )?;
        Ok(Db { conn: Mutex::new(conn) })
    }

    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn kv_get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn()
            .query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}
