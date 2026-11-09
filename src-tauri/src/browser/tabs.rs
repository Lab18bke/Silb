use std::time::Instant;

use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::core::Interface;

use super::{com, emit_tab, emit_tabs, emit_ui, layout, session_dirty, webview, with, InternalPos, Tab};
use crate::error::{Error, Result};
use crate::omnibox::{self, Target};
use crate::settings::NewTabPage;
use crate::state::AppState;

pub const ZOOM_LEVELS: [f64; 17] =
    [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0];

#[derive(Debug, Clone)]
pub struct TabSpec {
    pub target: Target,
    pub title: Option<String>,
    pub lazy: bool,
}

impl TabSpec {
    pub fn web(url: impl Into<String>) -> Self {
        TabSpec { target: Target::Web(url.into()), title: None, lazy: false }
    }

    pub fn internal(page: impl Into<String>) -> Self {
        TabSpec { target: Target::Internal(page.into()), title: None, lazy: false }
    }

    pub fn from_url(url: &str, title: Option<String>, lazy: bool) -> Self {
        let target = match omnibox::internal_page(url) {
            Some(page) => Target::Internal(page),
            None => Target::Web(url.to_string()),
        };
        TabSpec { target, title, lazy }
    }
}

pub enum Placement {
    End,
    After(u32),
    At(usize),
}

pub fn internal_title(page: &str) -> String {
    match page.split('/').next().unwrap_or("") {
        "history" => "History",
        "bookmarks" => "Bookmarks",
        "downloads" => "Downloads",
        "settings" => "Settings",
        "private" => "Private tab",
        _ => "New tab",
    }
    .into()
}

pub fn default_new_tab(app: &AppHandle, private: bool) -> TabSpec {
    let s = app.state::<AppState>().settings();
    match s.new_tab_page {
        NewTabPage::Blank => TabSpec::web("about:blank"),
        NewTabPage::Homepage if crate::util::is_web_url(&s.homepage) => TabSpec::web(s.homepage),
        _ => TabSpec::internal(if private { "private" } else { "newtab" }),
    }
}

pub fn open_tab(app: &AppHandle, win: &str, spec: TabSpec, activate: bool, placement: Placement) -> Option<u32> {
    let favicon = match &spec.target {
        Target::Web(u) if spec.lazy => app.state::<AppState>().db.favicon_for_url(u).ok().flatten(),
        _ => None,
    };
    let created = with(|b| {
        b.next_tab += 1;
        let id = b.next_tab;
        let w = b.win_mut(win)?;
        let mut tab = Tab::new(id);
        let mut load = None;
        match &spec.target {
            Target::Internal(page) => {
                tab.info.title = internal_title(page);
                tab.info.internal = Some(page.clone());
            }
            Target::Web(url) => {
                tab.info.url = url.clone();
                tab.info.title = spec.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| url.clone());
                tab.info.favicon = favicon;
                if spec.lazy && !activate {
                    tab.pending_url = Some(url.clone());
                } else {
                    tab.info.loading = true;
                    tab.pending_url = Some(url.clone());
                    load = Some(url.clone());
                }
            }
        }
        tab.refresh_security();
        tab.refresh_nav();
        let index = match placement {
            Placement::End => w.tabs.len(),
            Placement::At(i) => i.min(w.tabs.len()),
            Placement::After(id) => w.tabs.iter().position(|t| t.info.id == id).map(|i| i + 1).unwrap_or(w.tabs.len()),
        };
        w.tabs.insert(index, tab);
        if activate || w.active.is_none() {
            w.active = Some(id);
        }
        Some((id, load))
    })
    .flatten()?;
    let (id, load) = created;
    if let Some(url) = load {
        if let Err(e) = webview::create(app, win, id, &url) {
            fail_tab(app, id, e);
        }
    }
    finish(app, win);
    if activate {
        focus_active(app, win);
    }
    Some(id)
}

fn fail_tab(app: &AppHandle, id: u32, e: Error) {
    with(|b| {
        if let Some((w, i)) = b.tab_mut(id) {
            let t = &mut w.tabs[i];
            t.pending_url = None;
            t.info.loading = false;
            t.info.crashed = Some(e.to_string());
            emit_tab(app, &w.label, t);
        }
    });
}

fn finish(app: &AppHandle, win: &str) {
    with(|b| {
        if let Some(w) = b.win(win) {
            emit_tabs(app, w);
        }
    });
    layout(app, win);
    session_dirty(app);
}

pub fn focus_active(app: &AppHandle, win: &str) {
    let target = with(|b| {
        let w = b.win(win)?;
        let t = w.active_tab()?;
        Some(match (&t.info.internal, &t.webview) {
            (None, Some(wv)) if t.info.crashed.is_none() => Some(wv.clone()),
            _ => None,
        })
    })
    .flatten();
    match target {
        Some(Some(wv)) => {
            let _ = wv.set_focus();
        }
        Some(None) => {
            if let Some(ui) = with(|b| b.win(win).map(|w| w.ui.clone())).flatten() {
                let _ = ui.set_focus();
            }
            emit_ui(app, win, "focus-page", ());
        }
        None => {}
    }
}

pub fn activate(app: &AppHandle, win: &str, id: u32) -> Result<()> {
    let load = with(|b| {
        let w = b.win_mut(win)?;
        let t = w.tab_mut(id)?;
        t.last_active = Instant::now();
        let load = (t.webview.is_none() && t.info.internal.is_none()).then(|| t.pending_url.clone()).flatten();
        if load.is_some() {
            t.info.loading = true;
        }
        w.active = Some(id);
        Some(load)
    })
    .flatten()
    .ok_or(Error::NotFound("Tab"))?;
    super::find::stop_all(win);
    if let Some(url) = load {
        if let Err(e) = webview::create(app, win, id, &url) {
            fail_tab(app, id, e);
        }
    }
    finish(app, win);
    focus_active(app, win);
    Ok(())
}

pub fn close(app: &AppHandle, win: &str, id: u32) -> Result<()> {
    let result = with(|b| {
        let w = b.win_mut(win)?;
        let index = w.tabs.iter().position(|t| t.info.id == id)?;
        let tab = w.tabs.remove(index);
        let url = match &tab.info.internal {
            Some(page) if !tab.has_web() || tab.internal_pos == InternalPos::Behind => format!("silb://{page}"),
            _ => tab.info.url.clone(),
        };
        if !url.is_empty() && url != "silb://newtab" && url != "silb://private" {
            w.closed.push(super::ClosedTab { url, title: tab.info.title.clone(), index });
            if w.closed.len() > 25 {
                w.closed.remove(0);
            }
        }
        if w.active == Some(id) {
            let next = w.tabs.get(index).or_else(|| w.tabs.get(index.wrapping_sub(1))).or_else(|| w.tabs.last());
            w.active = next.map(|t| t.info.id);
        }
        Some((tab.webview, w.tabs.is_empty(), w.active))
    })
    .flatten()
    .ok_or(Error::NotFound("Tab"))?;
    let (webview, empty, active) = result;
    super::permissions::cancel_for_tab(app, id);
    com::remove_tab(id);
    if let Some(wv) = webview {
        super::downloads::release_webview(wv);
    }
    if empty {
        super::window::close_window(app, win, true);
        return Ok(());
    }
    if let Some(next) = active {
        activate(app, win, next)?;
    } else {
        finish(app, win);
    }
    Ok(())
}

pub fn close_others(app: &AppHandle, win: &str, keep: u32) -> Result<()> {
    let ids: Vec<u32> = with(|b| b.win(win).map(|w| w.tabs.iter().map(|t| t.info.id).filter(|i| *i != keep).collect()))
        .flatten()
        .unwrap_or_default();
    for id in ids {
        close(app, win, id)?;
    }
    activate(app, win, keep)
}

pub fn reopen_closed(app: &AppHandle, win: &str) -> Result<()> {
    let closed = with(|b| b.win_mut(win).and_then(|w| w.closed.pop())).flatten();
    let Some(c) = closed else { return Ok(()) };
    open_tab(app, win, TabSpec::from_url(&c.url, Some(c.title), false), true, Placement::At(c.index));
    Ok(())
}

pub fn move_tab(app: &AppHandle, win: &str, id: u32, index: usize) -> Result<()> {
    with(|b| {
        let w = b.win_mut(win)?;
        let from = w.tabs.iter().position(|t| t.info.id == id)?;
        let tab = w.tabs.remove(from);
        let to = index.min(w.tabs.len());
        w.tabs.insert(to, tab);
        Some(())
    })
    .flatten()
    .ok_or(Error::NotFound("Tab"))?;
    finish(app, win);
    Ok(())
}

pub fn navigate(app: &AppHandle, win: &str, id: u32, input: &str) -> Result<()> {
    let template = app.state::<AppState>().settings().engine().1;
    let target = omnibox::resolve(input, &template).ok_or_else(|| Error::invalid("Type an address or search."))?;
    navigate_to(app, win, id, target)
}

pub fn navigate_to(app: &AppHandle, win: &str, id: u32, target: Target) -> Result<()> {
    enum Next {
        None,
        Create(String),
        Navigate(String),
    }
    let next = with(|b| {
        let w = b.win_mut(win)?;
        let t = w.tab_mut(id)?;
        let next = match &target {
            Target::Internal(page) => {
                t.info.internal = Some(page.clone());
                t.info.title = internal_title(page);
                t.internal_pos = InternalPos::Over;
                t.info.loading = false;
                Next::None
            }
            Target::Web(url) => {
                if t.webview.is_none() {
                    if let Some(page) = t.info.internal.take() {
                        t.internal_behind = Some(page);
                    }
                    t.pending_url = Some(url.clone());
                    Next::Create(url.clone())
                } else {
                    t.info.internal = None;
                    Next::Navigate(url.clone())
                }
            }
        };
        if let Next::Create(url) | Next::Navigate(url) = &next {
            t.info.url = url.clone();
            t.info.loading = true;
            t.info.crashed = None;
        }
        t.refresh_security();
        t.refresh_nav();
        Some(next)
    })
    .flatten()
    .ok_or(Error::NotFound("Tab"))?;
    match next {
        Next::Create(url) => {
            if let Err(e) = webview::create(app, win, id, &url) {
                fail_tab(app, id, e);
            }
        }
        Next::Navigate(url) => webview::navigate_core(id, &url)?,
        Next::None => {}
    }
    finish(app, win);
    focus_active(app, win);
    Ok(())
}

pub fn back(app: &AppHandle, win: &str, id: u32) -> Result<()> {
    let core_back = with(|b| {
        let t = b.win_mut(win)?.tab_mut(id)?;
        if t.info.internal.is_some() && t.has_web() && t.internal_pos == InternalPos::Over {
            t.info.internal = None;
            Some(false)
        } else if t.info.internal.is_none() && t.web_can_back {
            Some(true)
        } else if t.info.internal.is_none() && t.internal_behind.is_some() {
            t.info.internal = t.internal_behind.clone();
            t.internal_pos = InternalPos::Behind;
            Some(false)
        } else {
            None
        }
        .map(|core| {
            t.refresh_security();
            t.refresh_nav();
            core
        })
    })
    .flatten();
    if core_back == Some(true) {
        if let Some(c) = com::tab(id) {
            unsafe { c.core.GoBack()? };
        }
    }
    finish(app, win);
    Ok(())
}

pub fn forward(app: &AppHandle, win: &str, id: u32) -> Result<()> {
    let core_fwd = with(|b| {
        let t = b.win_mut(win)?.tab_mut(id)?;
        if t.info.internal.is_some() && t.has_web() && t.internal_pos == InternalPos::Behind {
            t.info.internal = None;
            t.refresh_security();
            t.refresh_nav();
            Some(false)
        } else {
            Some(t.info.internal.is_none() && t.web_can_forward)
        }
    })
    .flatten();
    if core_fwd == Some(true) {
        if let Some(c) = com::tab(id) {
            unsafe { c.core.GoForward()? };
        }
    }
    let needs_load = with(|b| {
        let t = b.win(win)?.tab(id)?;
        (t.webview.is_none() && t.info.internal.is_none()).then(|| t.pending_url.clone()).flatten()
    })
    .flatten();
    if let Some(url) = needs_load {
        webview::create(app, win, id, &url)?;
    }
    finish(app, win);
    Ok(())
}

pub fn reload(app: &AppHandle, win: &str, id: u32, hard: bool) -> Result<()> {
    let state = with(|b| {
        let t = b.win_mut(win)?.tab_mut(id)?;
        let s = (t.info.internal.is_some(), t.webview.clone(), t.browser_crashed, t.info.url.clone(), t.pending_url.clone());
        t.info.crashed = None;
        t.browser_crashed = false;
        Some(s)
    })
    .flatten()
    .ok_or(Error::NotFound("Tab"))?;
    let (internal, webview, browser_crashed, url, pending) = state;
    if internal {
        emit_ui(app, win, "reload-page", id);
        return Ok(());
    }
    match webview {
        Some(wv) if browser_crashed => {
            com::remove_tab(id);
            let _ = wv.close();
            with(|b| {
                if let Some((w, i)) = b.tab_mut(id) {
                    w.tabs[i].webview = None;
                }
            });
            webview::create(app, win, id, &url)?;
        }
        Some(_) => {
            if let Some(c) = com::tab(id) {
                if hard {
                    com::devtools_call(&c.core, "Page.reload", r#"{"ignoreCache":true}"#);
                } else {
                    unsafe { c.core.Reload()? };
                }
            }
        }
        None => {
            if let Some(url) = pending.or_else(|| crate::util::is_web_url(&url).then_some(url)) {
                webview::create(app, win, id, &url)?;
            }
        }
    }
    finish(app, win);
    Ok(())
}

pub fn after_download(app: &AppHandle, id: u32) {
    let source = com::tab(id).map(|c| com::string(|p| unsafe { c.core.Source(p) })).unwrap_or_default();
    let committed = !source.is_empty() && source != "about:blank";
    enum Next {
        Updated,
        Internal(String, Option<tauri::Webview>),
        Close(String),
    }
    let next = with(|b| {
        let (w, i) = b.tab_mut(id)?;
        let label = w.label.clone();
        let only = w.tabs.len() == 1;
        let private = w.private;
        let t = &mut w.tabs[i];
        t.info.loading = false;
        if committed || t.web_can_back {
            if committed {
                t.info.url = source.clone();
            }
            t.refresh_security();
            emit_tab(app, &label, t);
            return Some(Next::Updated);
        }
        let page = t.internal_behind.take().or_else(|| only.then(|| if private { "private" } else { "newtab" }.to_string()));
        match page {
            Some(page) => {
                t.info.title = internal_title(&page);
                t.info.internal = Some(page);
                t.info.url.clear();
                t.info.favicon = None;
                t.pending_url = None;
                let wv = t.webview.take();
                t.refresh_security();
                t.refresh_nav();
                Some(Next::Internal(label, wv))
            }
            None => Some(Next::Close(label)),
        }
    })
    .flatten();
    match next {
        Some(Next::Updated) => session_dirty(app),
        Some(Next::Internal(label, wv)) => {
            com::remove_tab(id);
            if let Some(wv) = wv {
                super::downloads::release_webview(wv);
            }
            finish(app, &label);
        }
        Some(Next::Close(label)) => {
            let _ = close(app, &label, id);
        }
        None => {}
    }
}

pub fn stop(id: u32) -> Result<()> {
    if let Some(c) = com::tab(id) {
        unsafe { c.core.Stop()? };
    }
    Ok(())
}

pub fn zoom(app: &AppHandle, id: u32, action: &str) -> Result<f64> {
    let c = com::tab(id).ok_or(Error::NotFound("Tab"))?;
    let mut current = 1.0;
    unsafe { c.controller.ZoomFactor(&mut current)? };
    let next = match action {
        "in" => ZOOM_LEVELS.iter().copied().find(|z| *z > current + 0.001).unwrap_or(5.0),
        "out" => ZOOM_LEVELS.iter().rev().copied().find(|z| *z < current - 0.001).unwrap_or(0.25),
        "reset" => 1.0,
        _ => return Err(Error::invalid("Unknown zoom action.")),
    };
    unsafe { c.controller.SetZoomFactor(next)? };
    record_zoom(app, id, next);
    Ok(next)
}

pub fn record_zoom(app: &AppHandle, id: u32, factor: f64) {
    with(|b| {
        let (w, i) = b.tab_mut(id)?;
        let private = w.private;
        let t = &mut w.tabs[i];
        if (t.info.zoom - factor).abs() < 0.001 {
            return Some(());
        }
        t.info.zoom = factor;
        if let Some(host) = crate::util::host_of(&t.info.url) {
            app.state::<AppState>().set_zoom(private, &host, factor);
        }
        emit_tab(app, &w.label, &w.tabs[i]);
        Some(())
    });
}

pub fn toggle_mute(app: &AppHandle, id: u32) -> Result<()> {
    let c = com::tab(id).ok_or(Error::NotFound("Tab"))?;
    let core8: ICoreWebView2_8 = c.core.cast()?;
    let muted = !com::flag(|b| unsafe { core8.IsMuted(b) });
    unsafe { core8.SetIsMuted(muted)? };
    with(|b| {
        if let Some((w, i)) = b.tab_mut(id) {
            w.tabs[i].info.muted = muted;
            emit_tab(app, &w.label, &w.tabs[i]);
        }
    });
    Ok(())
}

pub fn print(id: u32) -> Result<()> {
    let c = com::tab(id).ok_or(Error::NotFound("Tab"))?;
    let core16: ICoreWebView2_16 = c.core.cast()?;
    unsafe { core16.ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_BROWSER)? };
    Ok(())
}

pub fn devtools(win: &str, id: u32) -> Result<()> {
    let wv = with(|b| b.win(win)?.tab(id)?.webview.clone()).flatten().ok_or(Error::NotFound("Tab"))?;
    wv.open_devtools();
    Ok(())
}

pub fn suspend_idle(after_secs: u64) {
    let idle: Vec<u32> = with(|b| {
        b.windows
            .iter_mut()
            .flat_map(|w| {
                let active = w.active;
                w.tabs.iter_mut().filter_map(move |t| {
                    let eligible = Some(t.info.id) != active
                        && t.webview.is_some()
                        && !t.shown
                        && !t.suspended
                        && !t.info.audible
                        && t.last_active.elapsed().as_secs() >= after_secs;
                    eligible.then(|| {
                        t.suspended = true;
                        t.info.id
                    })
                })
            })
            .collect()
    })
    .unwrap_or_default();
    for id in idle {
        if let Some(c) = com::tab(id) {
            if let Ok(core3) = c.core.cast::<ICoreWebView2_3>() {
                let handler = webview2_com::TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                unsafe {
                    let _ = core3.TrySuspend(&handler);
                }
            }
        }
    }
}
