use rusqlite::{params, params_from_iter, OptionalExtension};
use serde::Serialize;

use super::Db;
use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_at: i64,
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopSite {
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub visits: i64,
}

pub(crate) fn like_pattern(q: &str) -> String {
    let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

const FAVICON_JOIN: &str = "LEFT JOIN favicons f ON f.host = h.host";

impl Db {
    pub fn history_add(&self, url: &str, title: &str, at: i64) -> Result<i64> {
        let conn = self.conn();
        let recent: Option<i64> = conn
            .query_row(
                "SELECT id FROM history WHERE url = ?1 AND visited_at > ?2 ORDER BY visited_at DESC LIMIT 1",
                params![url, at - 5_000],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = recent {
            conn.execute(
                "UPDATE history SET visited_at = ?2, title = CASE WHEN ?3 = '' THEN title ELSE ?3 END WHERE id = ?1",
                params![id, at, title],
            )?;
            return Ok(id);
        }
        let host = super::favicon_key(url).unwrap_or_default();
        conn.execute(
            "INSERT INTO history (url, host, title, visited_at) VALUES (?1, ?2, ?3, ?4)",
            params![url, host, title, at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn history_set_title(&self, url: &str, title: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE history SET title = ?2 WHERE id = (SELECT id FROM history WHERE url = ?1 ORDER BY visited_at DESC LIMIT 1)",
            params![url, title],
        )?;
        Ok(())
    }

    pub fn history_query(&self, search: &str, before: Option<i64>, limit: u32) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn();
        let before = before.unwrap_or(i64::MAX);
        let search = search.trim();
        let sql = format!(
            "SELECT h.id, h.url, h.title, h.visited_at, f.data FROM history h {FAVICON_JOIN}
             WHERE h.visited_at < ?1 AND (?2 = '' OR h.url LIKE ?3 ESCAPE '\\' OR h.title LIKE ?3 ESCAPE '\\')
             ORDER BY h.visited_at DESC LIMIT ?4"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![before, search, like_pattern(search), limit.min(500)], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?,
                url: r.get(1)?,
                title: r.get(2)?,
                visited_at: r.get(3)?,
                favicon: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn history_delete(&self, ids: &[i64]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let marks = vec!["?"; ids.len()].join(",");
        Ok(self
            .conn()
            .execute(&format!("DELETE FROM history WHERE id IN ({marks})"), params_from_iter(ids))?)
    }

    pub fn history_delete_range(&self, from: i64, to: i64) -> Result<usize> {
        Ok(self
            .conn()
            .execute("DELETE FROM history WHERE visited_at >= ?1 AND visited_at <= ?2", params![from, to])?)
    }

    pub fn top_sites(&self, limit: u32, since: i64) -> Result<Vec<TopSite>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT h.url, (SELECT title FROM history t WHERE t.url = h.url ORDER BY visited_at DESC LIMIT 1),
                    f.data, COUNT(*) AS c
             FROM history h {FAVICON_JOIN}
             WHERE h.visited_at > ?1 AND h.url NOT LIKE '%/search?%' AND h.url NOT LIKE '%?q=%'
             GROUP BY h.url ORDER BY c DESC, MAX(h.visited_at) DESC LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![since, limit], |r| {
            Ok(TopSite {
                url: r.get(0)?,
                title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                favicon: r.get(2)?,
                visits: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn history_suggest(&self, q: &str, limit: u32) -> Result<Vec<(String, String, i64)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT url, (SELECT title FROM history t WHERE t.url = h.url ORDER BY visited_at DESC LIMIT 1), COUNT(*) AS c
             FROM history h WHERE url LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\'
             GROUP BY url ORDER BY (url LIKE ?2 ESCAPE '\\' OR url LIKE ?3 ESCAPE '\\') DESC, c DESC, MAX(visited_at) DESC LIMIT ?4",
        )?;
        let prefix_https = format!("https://{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let prefix_www = format!("https://www.{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let rows = stmt.query_map(params![like_pattern(q), prefix_https, prefix_www, limit], |r| {
            Ok((r.get(0)?, r.get::<_, Option<String>>(1)?.unwrap_or_default(), r.get(2)?))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_roundtrip() {
        let db = Db::in_memory().unwrap();
        db.history_add("https://example.com/a", "A", 1_000).unwrap();
        db.history_add("https://example.com/a", "", 2_000).unwrap();
        db.history_add("https://example.com/b", "B 50%", 10_000).unwrap();
        db.history_add("https://example.com/a", "A2", 20_000).unwrap();
        let all = db.history_query("", None, 100).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].title, "A2");
        assert_eq!(db.history_query("50%", None, 100).unwrap().len(), 1);
        assert_eq!(db.history_query("", Some(15_000), 100).unwrap().len(), 2);
        let top = db.top_sites(5, 0).unwrap();
        assert_eq!(top[0].url, "https://example.com/a");
        assert_eq!(top[0].visits, 2);
        db.history_delete_range(0, 12_000).unwrap();
        assert_eq!(db.history_query("", None, 100).unwrap().len(), 1);
        db.history_delete(&[all[0].id]).unwrap();
        assert!(db.history_query("", None, 100).unwrap().is_empty());
    }

    #[test]
    fn favicon_join_matches_host() {
        let db = Db::in_memory().unwrap();
        db.favicon_set("example.com", "data:image/png;base64,AA").unwrap();
        db.history_add("https://example.com/x/y", "X", 1).unwrap();
        db.history_add("https://www.example.com", "W", 1).unwrap();
        let rows = db.history_query("", None, 10).unwrap();
        assert!(rows.iter().any(|r| r.url == "https://example.com/x/y" && r.favicon.is_some()));
    }
}
