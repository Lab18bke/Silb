use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{ClearBrowsingDataCompletedHandler, GetCookiesCompletedHandler};
use windows::core::{Interface, HSTRING};

use super::{com, run_main};
use crate::error::{Error, Result};
use crate::settings::{Theme, TrackingPrevention};
use crate::state::AppState;
use crate::util::now_ms;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct ClearKinds {
    pub history: bool,
    pub cookies: bool,
    pub cache: bool,
    pub downloads: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SiteData {
    pub domain: String,
    pub cookies: usize,
}

pub fn apply_profile_prefs(app: &AppHandle, core: &ICoreWebView2) {
    let s = app.state::<AppState>().settings();
    let Some(profile) = com::profile(core) else { return };
    let scheme = match s.theme {
        Theme::System => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_AUTO,
        Theme::Light => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
        Theme::Dark => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
    };
    let level = match s.tracking_prevention {
        TrackingPrevention::Off => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_NONE,
        TrackingPrevention::Basic => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BASIC,
        TrackingPrevention::Balanced => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BALANCED,
        TrackingPrevention::Strict => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_STRICT,
    };
    unsafe {
        let _ = profile.SetPreferredColorScheme(scheme);
        if let Ok(p3) = profile.cast::<ICoreWebView2Profile3>() {
            let _ = p3.SetPreferredTrackingPreventionLevel(level);
        }
    }
}

pub fn apply_profile_prefs_everywhere(app: &AppHandle) {
    for (_, core) in com::tab_cores() {
        apply_profile_prefs(app, &core);
    }
    for core in com::ui_cores() {
        apply_profile_prefs(app, &core);
    }
}

fn normal_core() -> Result<ICoreWebView2> {
    com::ui_cores().into_iter().next().ok_or(Error::invalid("No browser window is open."))
}

pub async fn clear(app: &AppHandle, kinds: ClearKinds, since: Option<i64>) -> Result<()> {
    let state = app.state::<AppState>();
    let to = now_ms();
    let from = since.unwrap_or(0);
    if kinds.history {
        state.db.history_delete_range(from, to)?;
        let _ = state.db.favicons_prune();
    }
    if kinds.downloads {
        let a = app.clone();
        run_main(app, move |_| super::downloads::clear(&a, false, from, to)).await??;
    }
    let mut data = COREWEBVIEW2_BROWSING_DATA_KINDS(0);
    if kinds.cookies {
        data |= COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES | COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_DOM_STORAGE;
    }
    if kinds.cache {
        data |= COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE;
    }
    if kinds.history {
        data |= COREWEBVIEW2_BROWSING_DATA_KINDS_BROWSING_HISTORY;
    }
    if data.0 == 0 {
        return Ok(());
    }
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    run_main(app, move |_| -> Result<()> {
        let core = normal_core()?;
        let profile = com::profile(&core).ok_or(Error::invalid("Browsing data isn't available."))?;
        let profile2: ICoreWebView2Profile2 = profile.cast()?;
        let mut tx = Some(tx);
        let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |hr| {
            if let Some(tx) = tx.take() {
                let _ = tx.send(hr.is_ok());
            }
            Ok(())
        }));
        unsafe {
            match since {
                Some(from) => profile2.ClearBrowsingDataInTimeRange(data, from as f64 / 1000.0, to as f64 / 1000.0 + 1.0, &handler)?,
                None => profile2.ClearBrowsingData(data, &handler)?,
            }
        }
        Ok(())
    })
    .await??;
    if rx.await.unwrap_or(false) {
        Ok(())
    } else {
        Err(Error::invalid("WebView2 couldn't clear some website data."))
    }
}

fn base_domain(domain: &str) -> String {
    let d = domain.trim_start_matches('.').to_ascii_lowercase();
    let parts: Vec<&str> = d.split('.').collect();
    if parts.len() <= 2 || d.parse::<std::net::Ipv4Addr>().is_ok() {
        return d;
    }
    let second = parts[parts.len() - 2];
    let compound = ["co", "com", "org", "net", "ac", "gov", "edu", "or", "ne", "go"].contains(&second) && parts[parts.len() - 1].len() == 2;
    let keep = if compound { 3 } else { 2 };
    parts[parts.len() - keep..].join(".")
}

async fn all_cookies(app: &AppHandle) -> Result<Vec<(String, ICookieHolder)>> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Vec<(String, ICookieHolder)>>();
    run_main(app, move |_| -> Result<()> {
        let core = normal_core()?;
        let manager = unsafe { core.cast::<ICoreWebView2_2>()?.CookieManager()? };
        let mut tx = Some(tx);
        let handler = GetCookiesCompletedHandler::create(Box::new(move |hr, list| {
            let mut out = Vec::new();
            if let (Ok(()), Some(list)) = (hr, list) {
                let mut count = 0u32;
                unsafe {
                    let _ = list.Count(&mut count);
                    for i in 0..count {
                        if let Ok(cookie) = list.GetValueAtIndex(i) {
                            let domain = com::string(|p| cookie.Domain(p));
                            out.push((domain, ICookieHolder(cookie)));
                        }
                    }
                }
            }
            if let Some(tx) = tx.take() {
                let _ = tx.send(out);
            }
            Ok(())
        }));
        unsafe { manager.GetCookies(&HSTRING::new(), &handler)? };
        Ok(())
    })
    .await??;
    rx.await.map_err(|_| Error::invalid("Couldn't read cookies."))
}

pub struct ICookieHolder(ICoreWebView2Cookie);
unsafe impl Send for ICookieHolder {}

pub async fn list(app: &AppHandle) -> Result<Vec<SiteData>> {
    let cookies = all_cookies(app).await?;
    let mut map: BTreeMap<String, usize> = BTreeMap::new();
    let mut holders = Vec::with_capacity(cookies.len());
    for (domain, holder) in cookies {
        *map.entry(base_domain(&domain)).or_default() += 1;
        holders.push(holder);
    }
    let _ = app.run_on_main_thread(move || drop(holders));
    Ok(map.into_iter().map(|(domain, cookies)| SiteData { domain, cookies }).collect())
}

pub async fn clear_site(app: &AppHandle, domain: &str) -> Result<()> {
    let domain = base_domain(domain);
    if domain.is_empty() || !domain.contains('.') && domain != "localhost" {
        return Err(Error::invalid("That isn't a valid site."));
    }
    let cookies = all_cookies(app).await?;
    let d = domain.clone();
    run_main(app, move |_| -> Result<()> {
        let core = normal_core()?;
        let manager = unsafe { core.cast::<ICoreWebView2_2>()?.CookieManager()? };
        for (cookie_domain, holder) in cookies {
            if base_domain(&cookie_domain) == d {
                unsafe {
                    let _ = manager.DeleteCookie(&holder.0);
                }
            }
        }
        for origin in [format!("https://{d}"), format!("https://www.{d}"), format!("http://{d}"), format!("http://www.{d}")] {
            let params = serde_json::json!({ "origin": origin, "storageTypes": "all" }).to_string();
            com::devtools_call(&core, "Storage.clearDataForOrigin", &params);
        }
        Ok(())
    })
    .await??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::base_domain;

    #[test]
    fn base_domains() {
        assert_eq!(base_domain(".accounts.google.com"), "google.com");
        assert_eq!(base_domain("www.bbc.co.uk"), "bbc.co.uk");
        assert_eq!(base_domain("example.com"), "example.com");
        assert_eq!(base_domain("127.0.0.1"), "127.0.0.1");
    }
}
