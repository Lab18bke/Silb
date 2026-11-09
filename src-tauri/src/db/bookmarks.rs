use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;

use super::history::like_pattern;
use super::Db;
use crate::error::{Error, Result};
use crate::util::now_ms;

pub const BOOKMARKS_BAR: i64 = 1;
pub const OTHER_BOOKMARKS: i64 = 2;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub is_folder: bool,
    pub title: String,
    pub url: Option<String>,
    pub position: i64,
    pub created_at: i64,
    pub favicon: Option<String>,
}

const SELECT: &str = "SELECT b.id, b.parent_id, b.is_folder, b.title, b.url, b.position, b.created_at, f.data FROM bookmarks b LEFT JOIN favicons f ON f.host = b.host";

fn row(r: &Row) -> rusqlite::Result<Bookmark> {
    Ok(Bookmark {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        is_folder: r.get::<_, i64>(2)? != 0,
        title: r.get(3)?,
        url: r.get(4)?,
        position: r.get(5)?,
        created_at: r.get(6)?,
        favicon: r.get(7)?,
    })
}

fn is_root(id: i64) -> bool {
    id == BOOKMARKS_BAR || id == OTHER_BOOKMARKS
}

impl Db {
    pub fn bookmarks_all(&self) -> Result<Vec<Bookmark>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY b.parent_id, b.position"))?;
        let rows = stmt.query_map([], row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn bookmark_get(&self, id: i64) -> Result<Bookmark> {
        self.conn()
            .query_row(&format!("{SELECT} WHERE b.id = ?1"), [id], row)
            .optional()?
            .ok_or(Error::NotFound("Bookmark"))
    }

    pub fn bookmarks_by_url(&self, url: &str) -> Result<Vec<Bookmark>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!("{SELECT} WHERE b.url = ?1 ORDER BY b.created_at"))?;
        let rows = stmt.query_map([url], row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn bookmarks_search(&self, q: &str, limit: u32) -> Result<Vec<Bookmark>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "{SELECT} WHERE b.is_folder = 0 AND (b.title LIKE ?1 ESCAPE '\\' OR b.url LIKE ?1 ESCAPE '\\') ORDER BY b.created_at DESC LIMIT ?2"
        ))?;
        let rows = stmt.query_map(params![like_pattern(q.trim()), limit], row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    fn ensure_folder(&self, id: i64) -> Result<()> {
        let is_folder: Option<i64> = self
            .conn()
            .query_row("SELECT is_folder FROM bookmarks WHERE id = ?1", [id], |r| r.get(0))
            .optional()?;
        match is_folder {
            Some(1) => Ok(()),
            Some(_) => Err(Error::invalid("The destination is not a folder.")),
            None => Err(Error::NotFound("Folder")),
        }
    }

    pub fn bookmark_create(&self, parent: i64, title: &str, url: Option<&str>) -> Result<Bookmark> {
        self.ensure_folder(parent)?;
        let id = {
            let conn = self.conn();
            let pos: i64 = conn.query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM bookmarks WHERE parent_id = ?1",
                [parent],
                |r| r.get(0),
            )?;
            conn.execute(
                "INSERT INTO bookmarks (parent_id, is_folder, title, url, host, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![parent, url.is_none() as i64, title, url, url.and_then(super::favicon_key), pos, now_ms()],
            )?;
            conn.last_insert_rowid()
        };
        self.bookmark_get(id)
    }

    pub fn bookmark_update(&self, id: i64, title: &str, url: Option<&str>) -> Result<Bookmark> {
        let current = self.bookmark_get(id)?;
        if is_root(id) {
            return Err(Error::invalid("Built-in folders can't be renamed."));
        }
        if current.is_folder && url.is_some() {
            return Err(Error::invalid("Folders don't have an address."));
        }
        let url = if current.is_folder { None } else { url.or(current.url.as_deref()) };
        self.conn()
            .execute(
                "UPDATE bookmarks SET title = ?2, url = ?3, host = ?4 WHERE id = ?1",
                params![id, title, url, url.and_then(super::favicon_key)],
            )?;
        self.bookmark_get(id)
    }

    pub fn bookmark_move(&self, id: i64, parent: i64, index: i64) -> Result<()> {
        if is_root(id) {
            return Err(Error::invalid("Built-in folders can't be moved."));
        }
        self.ensure_folder(parent)?;
        let mut conn = self.conn();
        let mut cursor = Some(parent);
        while let Some(c) = cursor {
            if c == id {
                return Err(Error::invalid("A folder can't be moved into itself."));
            }
            cursor = conn.query_row("SELECT parent_id FROM bookmarks WHERE id = ?1", [c], |r| r.get(0))?;
        }
        let tx = conn.transaction()?;
        let old_parent: Option<i64> = tx.query_row("SELECT parent_id FROM bookmarks WHERE id = ?1", [id], |r| r.get(0))?;
        let mut siblings: Vec<i64> = tx
            .prepare("SELECT id FROM bookmarks WHERE parent_id = ?1 AND id != ?2 ORDER BY position")?
            .query_map(params![parent, id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let at = index.clamp(0, siblings.len() as i64) as usize;
        siblings.insert(at, id);
        for (pos, sid) in siblings.iter().enumerate() {
            tx.execute(
                "UPDATE bookmarks SET parent_id = ?2, position = ?3 WHERE id = ?1",
                params![sid, parent, pos as i64],
            )?;
        }
        if let Some(old) = old_parent.filter(|o| *o != parent) {
            let rest: Vec<i64> = tx
                .prepare("SELECT id FROM bookmarks WHERE parent_id = ?1 ORDER BY position")?
                .query_map([old], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            for (pos, sid) in rest.iter().enumerate() {
                tx.execute("UPDATE bookmarks SET position = ?2 WHERE id = ?1", params![sid, pos as i64])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn bookmark_delete(&self, id: i64) -> Result<()> {
        if is_root(id) {
            return Err(Error::invalid("Built-in folders can't be deleted."));
        }
        self.conn().execute("DELETE FROM bookmarks WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_move_delete() {
        let db = Db::in_memory().unwrap();
        let folder = db.bookmark_create(BOOKMARKS_BAR, "Reading", None).unwrap();
        assert!(folder.is_folder);
        let a = db.bookmark_create(BOOKMARKS_BAR, "A", Some("https://a.com/")).unwrap();
        let b = db.bookmark_create(BOOKMARKS_BAR, "B", Some("https://b.com/")).unwrap();
        db.bookmark_move(b.id, BOOKMARKS_BAR, 0).unwrap();
        let bar: Vec<_> = db.bookmarks_all().unwrap().into_iter().filter(|x| x.parent_id == Some(BOOKMARKS_BAR)).collect();
        assert_eq!(bar.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(), ["B", "Reading", "A"]);
        db.bookmark_move(a.id, folder.id, 0).unwrap();
        assert_eq!(db.bookmark_get(a.id).unwrap().parent_id, Some(folder.id));
        assert!(db.bookmark_move(folder.id, folder.id, 0).is_err());
        assert!(db.bookmark_move(a.id, b.id, 0).is_err());
        assert_eq!(db.bookmarks_by_url("https://a.com/").unwrap().len(), 1);
        assert_eq!(db.bookmarks_search("b.com", 10).unwrap().len(), 1);
        db.bookmark_update(a.id, "A renamed", Some("https://a2.com/")).unwrap();
        assert_eq!(db.bookmark_get(a.id).unwrap().url.as_deref(), Some("https://a2.com/"));
        db.bookmark_delete(folder.id).unwrap();
        assert!(db.bookmark_get(a.id).is_err());
        assert!(db.bookmark_delete(BOOKMARKS_BAR).is_err());
    }
}
