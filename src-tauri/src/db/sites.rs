use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::Db;
use crate::error::Result;
use crate::util::now_ms;

pub fn favicon_key(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(|h| h.to_ascii_lowercase())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteSetting {
    pub origin: String,
    pub kind: String,
    pub value: String,
}

impl Db {
    pub fn favicon_set(&self, host: &str, data: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO favicons (host, data, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(host) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
            params![host, data, now_ms()],
        )?;
        Ok(())
    }

    pub fn favicon_for_url(&self, url: &str) -> Result<Option<String>> {
        let Some(host) = favicon_key(url) else { return Ok(None) };
        Ok(self
            .conn()
            .query_row("SELECT data FROM favicons WHERE host = ?1", [host], |r| r.get(0))
            .optional()?)
    }

    pub fn favicons_prune(&self) -> Result<()> {
        let conn = self.conn();
        let mut keep: std::collections::HashSet<String> = conn
            .prepare("SELECT DISTINCT host FROM history")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<_, _>>()?;
        let bookmarked: Vec<String> = conn
            .prepare("SELECT DISTINCT host FROM bookmarks WHERE host IS NOT NULL")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<_, _>>()?;
        keep.extend(bookmarked);
        let hosts: Vec<String> = conn
            .prepare("SELECT host FROM favicons")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<_, _>>()?;
        for host in hosts.iter().filter(|h| !keep.contains(*h)) {
            conn.execute("DELETE FROM favicons WHERE host = ?1", [host])?;
        }
        Ok(())
    }

    pub fn site_settings_all(&self) -> Result<Vec<SiteSetting>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT origin, kind, value FROM site_settings ORDER BY origin, kind")?;
        let rows = stmt.query_map([], |r| {
            Ok(SiteSetting { origin: r.get(0)?, kind: r.get(1)?, value: r.get(2)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn site_setting_set(&self, origin: &str, kind: &str, value: Option<&str>) -> Result<()> {
        let conn = self.conn();
        match value {
            Some(v) => conn.execute(
                "INSERT INTO site_settings (origin, kind, value) VALUES (?1, ?2, ?3)
                 ON CONFLICT(origin, kind) DO UPDATE SET value = excluded.value",
                params![origin, kind, v],
            )?,
            None => conn.execute(
                "DELETE FROM site_settings WHERE origin = ?1 AND kind = ?2",
                params![origin, kind],
            )?,
        };
        Ok(())
    }

    pub fn zoom_all(&self) -> Result<HashMap<String, f64>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT host, factor FROM site_zoom")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn zoom_set(&self, host: &str, factor: f64) -> Result<()> {
        let conn = self.conn();
        if (factor - 1.0).abs() < 0.001 {
            conn.execute("DELETE FROM site_zoom WHERE host = ?1", [host])?;
        } else {
            conn.execute(
                "INSERT INTO site_zoom (host, factor) VALUES (?1, ?2)
                 ON CONFLICT(host) DO UPDATE SET factor = excluded.factor",
                params![host, factor],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_settings_roundtrip() {
        let db = Db::in_memory().unwrap();
        db.site_setting_set("https://a.com", "camera", Some("allow")).unwrap();
        db.site_setting_set("https://a.com", "camera", Some("block")).unwrap();
        db.site_setting_set("https://b.com", "popups", Some("allow")).unwrap();
        let all = db.site_settings_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].value, "block");
        db.site_setting_set("https://a.com", "camera", None).unwrap();
        assert_eq!(db.site_settings_all().unwrap().len(), 1);
    }

    #[test]
    fn zoom_resets_delete_row() {
        let db = Db::in_memory().unwrap();
        db.zoom_set("a.com", 1.25).unwrap();
        assert_eq!(db.zoom_all().unwrap().get("a.com"), Some(&1.25));
        db.zoom_set("a.com", 1.0).unwrap();
        assert!(db.zoom_all().unwrap().is_empty());
    }
}
