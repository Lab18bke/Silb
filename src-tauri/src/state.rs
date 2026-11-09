use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};

use crate::db::Db;
use crate::error::Result;
use crate::settings::Settings;

pub type SiteKey = (String, String);

pub struct AppState {
    pub db: Db,
    pub settings: RwLock<Settings>,
    pub sites: RwLock<HashMap<SiteKey, String>>,
    pub private_sites: Mutex<HashMap<SiteKey, String>>,
    pub zoom: RwLock<HashMap<String, f64>>,
    pub private_zoom: Mutex<HashMap<String, f64>>,
    pub session_dirty: AtomicBool,
    pub language_at_start: String,
    pub default_download_dir: PathBuf,
}

impl AppState {
    pub fn load(db: Db, default_download_dir: PathBuf) -> Result<Self> {
        let settings: Settings = db
            .kv_get("settings")?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let sites = db
            .site_settings_all()?
            .into_iter()
            .map(|s| ((s.origin, s.kind), s.value))
            .collect();
        let zoom = db.zoom_all()?;
        Ok(AppState {
            language_at_start: settings.website_language.clone(),
            settings: RwLock::new(settings),
            sites: RwLock::new(sites),
            private_sites: Mutex::new(HashMap::new()),
            zoom: RwLock::new(zoom),
            private_zoom: Mutex::new(HashMap::new()),
            session_dirty: AtomicBool::new(false),
            default_download_dir,
            db,
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn download_dir(&self) -> PathBuf {
        let s = self.settings();
        let custom = PathBuf::from(&s.download_dir);
        if !s.download_dir.is_empty() && custom.is_dir() {
            custom
        } else {
            self.default_download_dir.clone()
        }
    }

    pub fn site_value(&self, private: bool, origin: &str, kind: &str) -> Option<String> {
        let key = (origin.to_string(), kind.to_string());
        if private {
            if let Some(v) = self.private_sites.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
                return Some(v.clone());
            }
            let stored = self.sites.read().unwrap_or_else(|e| e.into_inner()).get(&key).cloned();
            return stored.filter(|v| v == "block");
        }
        self.sites.read().unwrap_or_else(|e| e.into_inner()).get(&key).cloned()
    }

    pub fn set_site_value(&self, private: bool, origin: &str, kind: &str, value: Option<&str>) -> Result<()> {
        let key = (origin.to_string(), kind.to_string());
        if private {
            let mut map = self.private_sites.lock().unwrap_or_else(|e| e.into_inner());
            match value {
                Some(v) => map.insert(key, v.to_string()),
                None => map.remove(&key),
            };
            return Ok(());
        }
        self.db.site_setting_set(origin, kind, value)?;
        let mut map = self.sites.write().unwrap_or_else(|e| e.into_inner());
        match value {
            Some(v) => map.insert(key, v.to_string()),
            None => map.remove(&key),
        };
        Ok(())
    }

    pub fn zoom_for(&self, private: bool, host: &str) -> f64 {
        if private {
            if let Some(z) = self.private_zoom.lock().unwrap_or_else(|e| e.into_inner()).get(host) {
                return *z;
            }
        }
        self.zoom.read().unwrap_or_else(|e| e.into_inner()).get(host).copied().unwrap_or(1.0)
    }

    pub fn set_zoom(&self, private: bool, host: &str, factor: f64) {
        if private {
            self.private_zoom.lock().unwrap_or_else(|e| e.into_inner()).insert(host.to_string(), factor);
            return;
        }
        let mut map = self.zoom.write().unwrap_or_else(|e| e.into_inner());
        let unchanged = map.get(host).copied().unwrap_or(1.0) == factor;
        if (factor - 1.0).abs() < 0.001 {
            map.remove(host);
        } else {
            map.insert(host.to_string(), factor);
        }
        drop(map);
        if !unchanged {
            let _ = self.db.zoom_set(host, factor);
        }
    }

    pub fn end_private_session(&self) {
        self.private_sites.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.private_zoom.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}
