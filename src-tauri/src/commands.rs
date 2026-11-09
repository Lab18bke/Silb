use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, Webview};

use crate::browser::permissions::{PermissionPrompt, SITE_KINDS};
use crate::browser::sitedata::{ClearKinds, SiteData};
use crate::browser::tabs::{self, Placement, TabSpec};
use crate::browser::window::{self as bwin, OverlayRect};
use crate::browser::{self, com, downloads, emit_chrome, emit_ui, find, permissions, reader, run_main, run_with, sitedata, window_of_label, WindowSnapshot};
use crate::db::{Bookmark, DownloadRecord, HistoryEntry, SiteSetting, TopSite};
use crate::error::{Error, Result};
use crate::omnibox::{self, Target};
use crate::settings::{Settings, BUILTIN_ENGINES};
use crate::state::AppState;

fn win_of(webview: &Webview) -> String {
    window_of_label(webview.label()).to_string()
}

fn is_private(webview: &Webview) -> bool {
    webview.label().starts_with('p')
}

fn check_url(url: &str) -> Result<()> {
    let ok = crate::util::is_web_url(url)
        || url.starts_with("file:///")
        || url.strip_prefix("view-source:").is_some_and(crate::util::is_web_url)
        || omnibox::internal_page(url).is_some();
    if ok && url.len() < 8192 {
        Ok(())
    } else {
        Err(Error::invalid("Only web addresses can be used here."))
    }
}

async fn blocking<T: Send + 'static>(app: &AppHandle, f: impl FnOnce(&AppState) -> Result<T> + Send + 'static) -> Result<T> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| Error::invalid(e.to_string()))?
}

#[tauri::command]
pub async fn ui_ready(app: AppHandle, webview: Webview) -> Result<WindowSnapshot> {
    let win = win_of(&webview);
    run_main(&app, move |a| {
        bwin::show_window(a, &win);
        bwin::snapshot(&win).ok_or(Error::NotFound("Window"))
    })
    .await?
}

#[tauri::command]
pub async fn window_snapshot(app: AppHandle, webview: Webview) -> Result<WindowSnapshot> {
    let win = win_of(&webview);
    run_main(&app, move |_| bwin::snapshot(&win).ok_or(Error::NotFound("Window"))).await?
}

#[tauri::command]
pub async fn window_control(app: AppHandle, webview: Webview, action: String) -> Result<()> {
    let win = win_of(&webview);
    let window = webview.window();
    match action.as_str() {
        "minimize" => window.minimize()?,
        "toggle-maximize" => {
            if window.is_maximized()? {
                window.unmaximize()?
            } else {
                window.maximize()?
            }
        }
        "close" => run_main(&app, move |a| bwin::close_window(a, &win, false)).await?,
        "close-force" => run_main(&app, move |a| bwin::close_window(a, &win, true)).await?,
        "fullscreen" => {
            run_main(&app, move |a| bwin::toggle_fullscreen(a, &win)).await??;
        }
        _ => return Err(Error::invalid("Unknown window action.")),
    }
    Ok(())
}

#[tauri::command]
pub async fn window_new(app: AppHandle, private: bool, url: Option<String>) -> Result<()> {
    if let Some(u) = &url {
        check_url(u)?;
    }
    run_main(&app, move |a| {
        let specs = url.map(|u| vec![TabSpec::from_url(&u, None, false)]).unwrap_or_default();
        bwin::create_window(a, private, specs, 0).map(|_| ())
    })
    .await?
}

#[tauri::command]
pub async fn set_chrome_top(app: AppHandle, webview: Webview, top: f64) -> Result<()> {
    if !top.is_finite() {
        return Err(Error::invalid("Invalid size."));
    }
    let win = win_of(&webview);
    run_main(&app, move |a| bwin::set_chrome_top(a, &win, top)).await
}

#[tauri::command]
pub async fn set_content_visible(app: AppHandle, webview: Webview, visible: bool) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| bwin::set_content_visible(a, &win, visible)).await
}

#[tauri::command]
pub async fn focus_content(app: AppHandle, webview: Webview) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| tabs::focus_active(a, &win)).await
}

#[tauri::command]
pub async fn tab_new(app: AppHandle, webview: Webview, url: Option<String>, active: Option<bool>, after: Option<u32>) -> Result<Option<u32>> {
    let win = win_of(&webview);
    let private = is_private(&webview);
    if let Some(u) = &url {
        check_url(u)?;
    }
    run_main(&app, move |a| {
        let spec = match url {
            Some(u) => TabSpec::from_url(&u, None, false),
            None => tabs::default_new_tab(a, private),
        };
        let placement = after.map(Placement::After).unwrap_or(Placement::End);
        tabs::open_tab(a, &win, spec, active.unwrap_or(true), placement)
    })
    .await
}

macro_rules! tab_op {
    ($name:ident, |$a:ident, $w:ident, $id:ident $(, $arg:ident : $ty:ty)*| $body:expr) => {
        #[tauri::command]
        pub async fn $name(app: AppHandle, webview: Webview, id: u32 $(, $arg: $ty)*) -> Result<()> {
            let $w = win_of(&webview);
            run_main(&app, move |$a| { let $id = id; $body }).await?
        }
    };
}

tab_op!(tab_close, |a, w, id| tabs::close(a, &w, id));
tab_op!(tab_close_others, |a, w, id| tabs::close_others(a, &w, id));
tab_op!(tab_activate, |a, w, id| tabs::activate(a, &w, id));
tab_op!(tab_move, |a, w, id, index: usize| tabs::move_tab(a, &w, id, index));
tab_op!(tab_back, |a, w, id| tabs::back(a, &w, id));
tab_op!(tab_forward, |a, w, id| tabs::forward(a, &w, id));
tab_op!(tab_reload, |a, w, id, hard: bool| tabs::reload(a, &w, id, hard));
tab_op!(tab_stop, |_a, _w, id| tabs::stop(id));
tab_op!(tab_mute, |a, _w, id| tabs::toggle_mute(a, id));
tab_op!(tab_reader, |a, w, id| reader::toggle(a, &w, id));
tab_op!(tab_devtools, |_a, w, id| tabs::devtools(&w, id));
tab_op!(tab_print, |_a, _w, id| tabs::print(id));

#[tauri::command]
pub async fn tab_navigate(app: AppHandle, webview: Webview, id: u32, input: String) -> Result<()> {
    if input.len() > 16_384 {
        return Err(Error::invalid("That address is too long."));
    }
    let win = win_of(&webview);
    run_main(&app, move |a| tabs::navigate(a, &win, id, &input)).await?
}

#[tauri::command]
pub async fn tab_zoom(app: AppHandle, id: u32, action: String) -> Result<f64> {
    run_main(&app, move |a| tabs::zoom(a, id, &action)).await?
}

#[tauri::command]
pub async fn tab_reopen(app: AppHandle, webview: Webview) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| tabs::reopen_closed(a, &win)).await?
}

#[tauri::command]
pub async fn tab_duplicate(app: AppHandle, webview: Webview, id: u32) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| {
        let url = browser::with(|b| {
            let t = b.win(&win)?.tab(id)?;
            Some(match &t.info.internal {
                Some(p) => format!("silb://{p}"),
                None => t.info.url.clone(),
            })
        })
        .flatten()
        .ok_or(Error::NotFound("Tab"))?;
        tabs::open_tab(a, &win, TabSpec::from_url(&url, None, false), true, Placement::After(id));
        Ok(())
    })
    .await?
}

#[tauri::command]
pub async fn tab_open_popup(app: AppHandle, webview: Webview, id: u32, url: Option<String>, allow_site: bool) -> Result<()> {
    if let Some(u) = &url {
        check_url(u)?;
    }
    let win = win_of(&webview);
    let private = is_private(&webview);
    let opener = run_with(&app, move |b, a| {
        let (w, i) = b.tab_mut(id)?;
        let t = &mut w.tabs[i];
        let opener = t.info.url.clone();
        match &url {
            Some(u) => t.info.blocked_popups.retain(|p| p != u),
            None => t.info.blocked_popups.clear(),
        }
        browser::emit_tab(a, &w.label, t);
        Some((opener, url))
    })
    .await?
    .ok_or(Error::NotFound("Tab"))?;
    let (opener, url) = opener;
    if allow_site {
        if let Some(origin) = crate::util::origin_of(&opener) {
            app.state::<AppState>().set_site_value(private, &origin, "popups", Some("allow"))?;
        }
    }
    if let Some(u) = url {
        run_main(&app, move |a| {
            tabs::open_tab(a, &win, TabSpec::web(u), true, Placement::After(id));
        })
        .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn find_start(app: AppHandle, webview: Webview, term: String, case_sensitive: bool) -> Result<()> {
    let win = win_of(&webview);
    let term: String = term.chars().take(512).collect();
    run_main(&app, move |a| find::start(a, &win, &term, case_sensitive)).await?
}

#[tauri::command]
pub async fn find_step(app: AppHandle, webview: Webview, forward: bool) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |_| find::step(&win, forward)).await?
}

#[tauri::command]
pub async fn find_stop(app: AppHandle, webview: Webview) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |_| find::stop_all(&win)).await
}

#[tauri::command]
pub async fn overlay_show(app: AppHandle, webview: Webview, rect: OverlayRect, payload: Value, focus: bool) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| bwin::show_overlay(a, &win, rect, payload, focus)).await?
}

#[tauri::command]
pub async fn overlay_hide(app: AppHandle, webview: Webview) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| bwin::hide_overlay(a, &win)).await
}

#[tauri::command]
pub async fn overlay_event(app: AppHandle, webview: Webview, name: String, data: Value) -> Result<()> {
    let win = win_of(&webview);
    emit_ui(&app, &win, "overlay-event", serde_json::json!({ "name": name, "data": data }));
    Ok(())
}

#[tauri::command]
pub async fn overlay_resize(app: AppHandle, webview: Webview, height: f64) -> Result<()> {
    if !height.is_finite() {
        return Err(Error::invalid("Invalid size."));
    }
    let win = win_of(&webview);
    run_main(&app, move |_| bwin::resize_overlay(&win, height)).await?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    kind: &'static str,
    title: String,
    url: String,
    favicon: Option<String>,
}

#[tauri::command]
pub async fn suggest(app: AppHandle, query: String) -> Result<Vec<Suggestion>> {
    let q = query.trim().to_string();
    if q.is_empty() || q.len() > 2048 {
        return Ok(Vec::new());
    }
    blocking(&app, move |state| {
        let (engine, template) = state.settings().engine();
        let mut out = Vec::new();
        let typed = omnibox::resolve(&q, &template);
        let search = omnibox::search_url(&template, &q);
        match &typed {
            Some(Target::Web(url)) if *url != search => {
                out.push(Suggestion { kind: "url", title: url.clone(), url: url.clone(), favicon: None });
            }
            Some(Target::Internal(page)) => {
                out.push(Suggestion { kind: "url", title: format!("silb://{page}"), url: format!("silb://{page}"), favicon: None });
            }
            _ => {}
        }
        let mut seen: Vec<String> = out.iter().map(|s| s.url.clone()).collect();
        for b in state.db.bookmarks_search(&q, 3)? {
            if let Some(url) = b.url {
                if !seen.contains(&url) {
                    seen.push(url.clone());
                    out.push(Suggestion { kind: "bookmark", title: b.title, url, favicon: b.favicon });
                }
            }
        }
        for (url, title, _) in state.db.history_suggest(&q, 8)? {
            if out.len() >= 7 {
                break;
            }
            if !seen.contains(&url) {
                seen.push(url.clone());
                let favicon = state.db.favicon_for_url(&url)?;
                out.push(Suggestion { kind: "history", title, url, favicon });
            }
        }
        let at = if matches!(typed, Some(Target::Web(ref u)) if *u == search) { 0 } else { out.len().min(1) };
        out.insert(at, Suggestion { kind: "search", title: format!("{q} — Search with {engine}"), url: search, favicon: None });
        Ok(out)
    })
    .await
}

#[tauri::command]
pub async fn history_query(app: AppHandle, search: String, before: Option<i64>, limit: Option<u32>) -> Result<Vec<HistoryEntry>> {
    blocking(&app, move |s| s.db.history_query(&search, before, limit.unwrap_or(100))).await
}

#[tauri::command]
pub async fn history_delete(app: AppHandle, ids: Vec<i64>) -> Result<()> {
    blocking(&app, move |s| s.db.history_delete(&ids).map(|_| ())).await
}

#[tauri::command]
pub async fn top_sites(app: AppHandle, limit: Option<u32>) -> Result<Vec<TopSite>> {
    let since = crate::util::now_ms() - 90 * 24 * 3600 * 1000;
    blocking(&app, move |s| s.db.top_sites(limit.unwrap_or(8).min(24), since)).await
}

fn bookmarks_changed(app: &AppHandle) {
    emit_chrome(app, "bookmarks-changed", (), None);
}

#[tauri::command]
pub async fn bookmarks_all(app: AppHandle) -> Result<Vec<Bookmark>> {
    blocking(&app, |s| s.db.bookmarks_all()).await
}

fn clean_title(title: &str) -> String {
    title.trim().chars().take(512).collect()
}

#[tauri::command]
pub async fn bookmark_create(app: AppHandle, parent_id: i64, title: String, url: Option<String>) -> Result<Bookmark> {
    if let Some(u) = &url {
        check_url(u)?;
    }
    let title = clean_title(&title);
    if url.is_none() && title.is_empty() {
        return Err(Error::invalid("Give the folder a name."));
    }
    let b = blocking(&app, move |s| s.db.bookmark_create(parent_id, &title, url.as_deref())).await?;
    bookmarks_changed(&app);
    Ok(b)
}

#[tauri::command]
pub async fn bookmark_update(app: AppHandle, id: i64, title: String, url: Option<String>) -> Result<Bookmark> {
    if let Some(u) = &url {
        check_url(u)?;
    }
    let title = clean_title(&title);
    let b = blocking(&app, move |s| s.db.bookmark_update(id, &title, url.as_deref())).await?;
    bookmarks_changed(&app);
    Ok(b)
}

#[tauri::command]
pub async fn bookmark_move(app: AppHandle, id: i64, parent_id: i64, index: i64) -> Result<()> {
    blocking(&app, move |s| s.db.bookmark_move(id, parent_id, index)).await?;
    bookmarks_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn bookmark_delete(app: AppHandle, id: i64) -> Result<()> {
    blocking(&app, move |s| s.db.bookmark_delete(id)).await?;
    bookmarks_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn bookmarks_for_url(app: AppHandle, url: String) -> Result<Vec<Bookmark>> {
    blocking(&app, move |s| s.db.bookmarks_by_url(&url)).await
}

#[tauri::command]
pub async fn bookmarks_search(app: AppHandle, query: String) -> Result<Vec<Bookmark>> {
    blocking(&app, move |s| s.db.bookmarks_search(&query, 200)).await
}

#[tauri::command]
pub async fn downloads_list(app: AppHandle, webview: Webview) -> Result<Vec<DownloadRecord>> {
    let private = is_private(&webview);
    run_main(&app, move |a| downloads::list(a, private)).await?
}

#[tauri::command]
pub async fn download_cancel(app: AppHandle, id: i64) -> Result<()> {
    run_main(&app, move |_| downloads::cancel(id)).await?
}

#[tauri::command]
pub async fn download_retry(app: AppHandle, webview: Webview, id: i64) -> Result<()> {
    let win = win_of(&webview);
    run_main(&app, move |a| downloads::retry(a, &win, id)).await?
}

#[tauri::command]
pub async fn download_open(app: AppHandle, id: i64) -> Result<()> {
    run_main(&app, move |a| downloads::open(a, id)).await?
}

#[tauri::command]
pub async fn download_reveal(app: AppHandle, id: i64) -> Result<()> {
    run_main(&app, move |a| downloads::reveal(a, id)).await?
}

#[tauri::command]
pub async fn download_remove(app: AppHandle, id: i64) -> Result<()> {
    run_main(&app, move |a| downloads::remove(a, id)).await?
}

#[tauri::command]
pub async fn downloads_clear(app: AppHandle, webview: Webview) -> Result<()> {
    let private = is_private(&webview);
    run_main(&app, move |a| downloads::clear(a, private, 0, i64::MAX)).await?
}

#[tauri::command]
pub async fn settings_get(app: AppHandle) -> Result<Settings> {
    Ok(app.state::<AppState>().settings())
}

pub async fn store_settings(app: &AppHandle, settings: Settings) -> Result<Settings> {
    let settings = settings.validated()?;
    let json = serde_json::to_string(&settings)?;
    let s2 = settings.clone();
    blocking(app, move |s| {
        s.db.kv_set("settings", &json)?;
        *s.settings.write().unwrap_or_else(|e| e.into_inner()) = s2;
        Ok(())
    })
    .await?;
    emit_chrome(app, "settings-changed", settings.clone(), None);
    run_main(app, |a| {
        sitedata::apply_profile_prefs_everywhere(a);
        reader::refresh_open(a);
        let windows: Vec<tauri::Window> = browser::with(|b| b.windows.iter().map(|w| w.window.clone()).collect()).unwrap_or_default();
        for window in windows {
            let _ = window.set_theme(bwin::window_theme(a));
        }
    })
    .await?;
    Ok(settings)
}

#[tauri::command]
pub async fn settings_set(app: AppHandle, settings: Settings) -> Result<Settings> {
    store_settings(&app, settings).await
}

#[tauri::command]
pub async fn pick_download_dir(app: AppHandle, webview: Webview) -> Result<Option<String>> {
    let current = app.state::<AppState>().download_dir();
    let window = webview.window();
    let picked = rfd::AsyncFileDialog::new()
        .set_title("Choose a download folder")
        .set_directory(current)
        .set_parent(&window)
        .pick_folder()
        .await;
    Ok(picked.map(|h| h.path().to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn clear_browsing_data(app: AppHandle, kinds: ClearKinds, since: Option<i64>) -> Result<()> {
    sitedata::clear(&app, kinds, since).await
}

#[tauri::command]
pub async fn site_data_list(app: AppHandle) -> Result<Vec<SiteData>> {
    sitedata::list(&app).await
}

#[tauri::command]
pub async fn site_data_clear(app: AppHandle, domain: String) -> Result<()> {
    sitedata::clear_site(&app, &domain).await
}

#[tauri::command]
pub async fn site_settings_list(app: AppHandle) -> Result<Vec<SiteSetting>> {
    blocking(&app, |s| s.db.site_settings_all()).await
}

#[tauri::command]
pub async fn site_setting_set(app: AppHandle, webview: Webview, origin: String, kind: String, value: Option<String>) -> Result<()> {
    if !SITE_KINDS.contains(&kind.as_str()) {
        return Err(Error::invalid("Unknown permission."));
    }
    if value.as_deref().is_some_and(|v| v != "allow" && v != "block") {
        return Err(Error::invalid("Unknown permission value."));
    }
    let origin = crate::util::origin_of(&origin).ok_or_else(|| Error::invalid("That isn't a valid site address."))?;
    let private = is_private(&webview);
    app.state::<AppState>().set_site_value(private, &origin, &kind, value.as_deref())?;
    emit_chrome(&app, "site-settings-changed", (), None);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteInfo {
    origin: Option<String>,
    host: String,
    security: String,
    permissions: Vec<(String, Option<String>)>,
}

#[tauri::command]
pub async fn site_info(app: AppHandle, webview: Webview, id: u32) -> Result<SiteInfo> {
    let win = win_of(&webview);
    let private = is_private(&webview);
    let (url, security) = run_main(&app, move |_| {
        browser::with(|b| b.win(&win)?.tab(id).map(|t| (t.info.url.clone(), t.info.security.clone()))).flatten()
    })
    .await?
    .ok_or(Error::NotFound("Tab"))?;
    let origin = crate::util::origin_of(&url);
    let host = url::Url::parse(&url).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default();
    let state = app.state::<AppState>();
    let permissions = SITE_KINDS
        .iter()
        .map(|k| (k.to_string(), origin.as_ref().and_then(|o| state.site_value(private, o, k))))
        .collect();
    Ok(SiteInfo { origin, host, security, permissions })
}

#[tauri::command]
pub async fn permission_resolve(app: AppHandle, id: u32, allow: bool, remember: bool) -> Result<()> {
    run_main(&app, move |a| permissions::resolve(a, id, allow, remember)).await?
}

#[tauri::command]
pub async fn permissions_pending(app: AppHandle, webview: Webview) -> Result<Vec<PermissionPrompt>> {
    let win = win_of(&webview);
    run_main(&app, move |_| permissions::pending_for_window(&win)).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    tauri: String,
    webview2: String,
    arch: String,
    data_dir: String,
    download_dir: String,
    engines: Vec<(String, String, String)>,
}

#[tauri::command]
pub async fn app_info(app: AppHandle) -> Result<AppInfo> {
    let webview2 = run_main(&app, |_| com::browser_version()).await?;
    let data_dir = app.path().app_local_data_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        tauri: tauri::VERSION.into(),
        webview2,
        arch: std::env::consts::ARCH.into(),
        data_dir,
        download_dir: app.state::<AppState>().download_dir().to_string_lossy().into_owned(),
        engines: BUILTIN_ENGINES.iter().map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())).collect(),
    })
}
