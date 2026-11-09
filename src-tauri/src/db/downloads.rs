use rusqlite::{params, Row};
use serde::Serialize;

use super::Db;
use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    pub id: i64,
    pub url: String,
    pub path: String,
    pub file_name: String,
    pub mime: String,
    pub total_bytes: Option<i64>,
    pub received_bytes: i64,
    pub state: String,
    pub error: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub exists: bool,
    pub private: bool,
    pub can_resume: bool,
}

fn row(r: &Row) -> rusqlite::Result<DownloadRecord> {
    let path: String = r.get(2)?;
    let state: String = r.get(7)?;
    Ok(DownloadRecord {
        id: r.get(0)?,
        url: r.get(1)?,
        exists: state != "completed" || std::path::Path::new(&path).exists(),
        path,
        file_name: r.get(3)?,
        mime: r.get(4)?,
        total_bytes: r.get(5)?,
        received_bytes: r.get(6)?,
        state,
        error: r.get(8)?,
        started_at: r.get(9)?,
        finished_at: r.get(10)?,
        private: false,
        can_resume: false,
    })
}

const COLUMNS: &str =
    "id, url, path, file_name, mime, total_bytes, received_bytes, state, error, started_at, finished_at";

impl Db {
    pub fn download_insert(&self, d: &DownloadRecord) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO downloads (url, path, file_name, mime, total_bytes, received_bytes, state, error, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![d.url, d.path, d.file_name, d.mime, d.total_bytes, d.received_bytes, d.state, d.error, d.started_at, d.finished_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn download_update(&self, d: &DownloadRecord) -> Result<()> {
        self.conn().execute(
            "UPDATE downloads SET url = ?2, path = ?3, file_name = ?4, mime = ?5, total_bytes = ?6, received_bytes = ?7,
             state = ?8, error = ?9, started_at = ?10, finished_at = ?11 WHERE id = ?1",
            params![d.id, d.url, d.path, d.file_name, d.mime, d.total_bytes, d.received_bytes, d.state, d.error, d.started_at, d.finished_at],
        )?;
        Ok(())
    }

    pub fn downloads_list(&self) -> Result<Vec<DownloadRecord>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM downloads ORDER BY started_at DESC LIMIT 1000"))?;
        let rows = stmt.query_map([], row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn download_get(&self, id: i64) -> Result<DownloadRecord> {
        Ok(self
            .conn()
            .query_row(&format!("SELECT {COLUMNS} FROM downloads WHERE id = ?1"), [id], row)?)
    }

    pub fn download_delete(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM downloads WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn downloads_clear_range(&self, from: i64, to: i64) -> Result<usize> {
        Ok(self.conn().execute(
            "DELETE FROM downloads WHERE state NOT IN ('in_progress', 'paused') AND started_at >= ?1 AND started_at <= ?2",
            params![from, to],
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_lifecycle() {
        let db = Db::in_memory().unwrap();
        let mut d = DownloadRecord {
            id: 0,
            url: "https://x.com/f.zip".into(),
            path: "C:\\nope\\f.zip".into(),
            file_name: "f.zip".into(),
            mime: "application/zip".into(),
            total_bytes: Some(10),
            received_bytes: 0,
            state: "in_progress".into(),
            error: None,
            started_at: 100,
            finished_at: None,
            exists: true,
            private: false,
            can_resume: false,
        };
        d.id = db.download_insert(&d).unwrap();
        assert_eq!(db.downloads_clear_range(0, i64::MAX).unwrap(), 0);
        d.state = "completed".into();
        d.received_bytes = 10;
        db.download_update(&d).unwrap();
        let got = db.download_get(d.id).unwrap();
        assert_eq!(got.received_bytes, 10);
        assert!(!got.exists);
        assert_eq!(db.downloads_clear_range(0, i64::MAX).unwrap(), 1);
        assert!(db.downloads_list().unwrap().is_empty());
    }
}
