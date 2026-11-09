pub mod com;
pub mod context_menu;
pub mod downloads;
pub mod find;
pub mod permissions;
pub mod reader;
pub mod session;
pub mod shortcuts;
pub mod sitedata;
pub mod tabs;
pub mod webview;
pub mod window;

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::thread::ThreadId;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Rect, Webview, Window};

use crate::error::{Error, Result};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub zoom: f64,
    pub internal: Option<String>,
    pub crashed: Option<String>,
    pub reader_available: bool,
    pub reader_active: bool,
    pub security: String,
    pub audible: bool,
    pub muted: bool,
    pub blocked_popups: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InternalPos {
    Over,
    Behind,
}

pub struct Tab {
    pub info: TabInfo,
    pub webview: Option<Webview>,
    pub pending_url: Option<String>,
    pub internal_pos: InternalPos,
    pub internal_behind: Option<String>,
    pub web_can_back: bool,
    pub web_can_forward: bool,
    pub cert_errors: HashSet<String>,
    pub last_active: Instant,
    pub shown: bool,
    pub bounds: Option<[f64; 4]>,
    pub suspended: bool,
    pub browser_crashed: bool,
}

impl Tab {
    pub fn new(id: u32) -> Self {
        Tab {
            info: TabInfo { id, zoom: 1.0, security: "none".into(), ..Default::default() },
            webview: None,
            pending_url: None,
            internal_pos: InternalPos::Over,
            internal_behind: None,
            web_can_back: false,
            web_can_forward: false,
            cert_errors: HashSet::new(),
            last_active: Instant::now(),
            shown: false,
            bounds: None,
            suspended: false,
            browser_crashed: false,
        }
    }

    pub fn has_web(&self) -> bool {
        self.webview.is_some() || self.pending_url.is_some()
    }

    pub fn refresh_nav(&mut self) {
        let (back, fwd) = match (&self.info.internal, self.has_web()) {
            (Some(_), true) => (self.internal_pos == InternalPos::Over, self.internal_pos == InternalPos::Behind),
            (Some(_), false) => (false, false),
            (None, _) => (self.web_can_back || self.internal_behind.is_some(), self.web_can_forward),
        };
        self.info.can_go_back = back;
        self.info.can_go_forward = fwd;
    }

    pub fn refresh_security(&mut self) {
        let url = &self.info.url;
        self.info.security = if self.info.internal.is_some() {
            "internal"
        } else if url.starts_with("https://") {
            let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_string));
            if host.is_some_and(|h| self.cert_errors.contains(&h)) { "cert-error" } else { "secure" }
        } else if url.starts_with("http://") {
            "insecure"
        } else if url.starts_with("file:") {
            "file"
        } else {
            "none"
        }
        .into();
    }
}

pub struct ClosedTab {
    pub url: String,
    pub title: String,
    pub index: usize,
}

pub struct Win {
    pub label: String,
    pub window: Window,
    pub ui: Webview,
    pub overlay: Webview,
    pub overlay_hwnd: isize,
    pub private: bool,
    pub tabs: Vec<Tab>,
    pub active: Option<u32>,
    pub chrome_top: f64,
    pub html_fullscreen: bool,
    pub closed: Vec<ClosedTab>,
    pub content_visible: bool,
    pub force_close: bool,
    pub shown: bool,
    pub overlay_visible: bool,
    pub overlay_focused: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowSnapshot {
    pub label: String,
    pub private: bool,
    pub tabs: Vec<TabInfo>,
    pub active: Option<u32>,
    pub maximized: bool,
    pub fullscreen: bool,
    pub closed_count: usize,
}

impl Win {
    pub fn tab(&self, id: u32) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.info.id == id)
    }

    pub fn tab_mut(&mut self, id: u32) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.info.id == id)
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.tab(id))
    }

    pub fn snapshot(&self) -> WindowSnapshot {
        WindowSnapshot {
            label: self.label.clone(),
            private: self.private,
            tabs: self.tabs.iter().map(|t| t.info.clone()).collect(),
            active: self.active,
            maximized: self.window.is_maximized().unwrap_or(false),
            fullscreen: self.window.is_fullscreen().unwrap_or(false),
            closed_count: self.closed.len(),
        }
    }

    fn content_rect(&self) -> [f64; 4] {
        let scale = self.window.scale_factor().unwrap_or(1.0);
        let size = self.window.inner_size().map(|s| s.to_logical::<f64>(scale)).unwrap_or(LogicalSize::new(1.0, 1.0));
        if self.html_fullscreen {
            [0.0, 0.0, size.width, size.height]
        } else {
            let top = self.chrome_top.min(size.height);
            [0.0, top, size.width, (size.height - top).max(1.0)]
        }
    }

    fn layout_ops(&mut self) -> Vec<LayoutOp> {
        let rect = self.content_rect();
        let active = self.active;
        let visible = self.content_visible || self.html_fullscreen;
        let mut ops = Vec::new();
        for tab in &mut self.tabs {
            let Some(wv) = tab.webview.clone() else { continue };
            let show = Some(tab.info.id) == active && tab.info.internal.is_none() && tab.info.crashed.is_none() && visible;
            if show {
                if tab.bounds != Some(rect) || !tab.shown {
                    tab.bounds = Some(rect);
                    tab.shown = true;
                    tab.suspended = false;
                    ops.push(LayoutOp::Show(wv, rect));
                }
            } else if tab.shown {
                tab.shown = false;
                ops.push(LayoutOp::Hide(wv));
            }
        }
        ops
    }
}

enum LayoutOp {
    Show(Webview, [f64; 4]),
    Hide(Webview),
}

#[derive(Default)]
pub struct Browser {
    pub windows: Vec<Win>,
    pub next_tab: u32,
    pub next_win: u32,
    pub next_popup: u32,
}

impl Browser {
    pub fn win(&self, label: &str) -> Option<&Win> {
        self.windows.iter().find(|w| w.label == label)
    }

    pub fn win_mut(&mut self, label: &str) -> Option<&mut Win> {
        self.windows.iter_mut().find(|w| w.label == label)
    }

    pub fn tab_mut(&mut self, id: u32) -> Option<(&mut Win, usize)> {
        let w = self.windows.iter_mut().find(|w| w.tabs.iter().any(|t| t.info.id == id))?;
        let i = w.tabs.iter().position(|t| t.info.id == id)?;
        Some((w, i))
    }

    pub fn window_of_tab(&self, id: u32) -> Option<&Win> {
        self.windows.iter().find(|w| w.tabs.iter().any(|t| t.info.id == id))
    }

    pub fn has_private_windows(&self) -> bool {
        self.windows.iter().any(|w| w.private)
    }
}

thread_local! {
    static BROWSER: RefCell<Browser> = RefCell::new(Browser::default());
}

static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();

pub fn mark_main_thread() {
    let _ = MAIN_THREAD.set(std::thread::current().id());
}

pub fn on_main_thread() -> bool {
    MAIN_THREAD.get() == Some(&std::thread::current().id())
}

pub fn with<R>(f: impl FnOnce(&mut Browser) -> R) -> Option<R> {
    if !on_main_thread() {
        return None;
    }
    BROWSER.with(|b| b.try_borrow_mut().ok().map(|mut g| f(&mut g)))
}

pub fn defer(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let inner = app.clone();
        let _ = app.run_on_main_thread(move || f(&inner));
    });
}

pub fn mutate(app: &AppHandle, f: impl FnOnce(&mut Browser, &AppHandle) + Send + 'static) {
    if on_main_thread() {
        let done = BROWSER.with(|b| match b.try_borrow_mut() {
            Ok(mut g) => {
                f(&mut g, app);
                None
            }
            Err(_) => Some(f),
        });
        if let Some(f) = done {
            defer(app, move |a| mutate(a, f));
        }
    } else {
        defer(app, move |a| mutate(a, f));
    }
}

pub async fn run_main<R: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> R + Send + 'static,
) -> Result<R> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let inner = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(&inner));
    })?;
    rx.await.map_err(|_| Error::invalid("The browser is shutting down."))
}

pub async fn run_with<R: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&mut Browser, &AppHandle) -> R + Send + 'static,
) -> Result<R> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    mutate(app, move |b, a| {
        let _ = tx.send(f(b, a));
    });
    rx.await.map_err(|_| Error::invalid("The browser is shutting down."))
}

pub fn ui_label(win: &str) -> String {
    format!("{win}-ui")
}

pub fn overlay_label(win: &str) -> String {
    format!("{win}-ov")
}

pub fn window_of_label(label: &str) -> &str {
    label.split('-').next().unwrap_or(label)
}

pub fn emit_ui<S: Serialize + Clone>(app: &AppHandle, win: &str, event: &str, payload: S) {
    let _ = app.emit_to(EventTarget::Webview { label: ui_label(win) }, event, payload);
}

pub fn emit_overlay<S: Serialize + Clone>(app: &AppHandle, win: &str, event: &str, payload: S) {
    let _ = app.emit_to(EventTarget::Webview { label: overlay_label(win) }, event, payload);
}

pub fn emit_chrome<S: Serialize + Clone>(app: &AppHandle, event: &str, payload: S, private: Option<bool>) {
    let _ = app.emit_filter(event, payload, |target| match target {
        EventTarget::Webview { label } => {
            let trusted = label.ends_with("-ui") || label.ends_with("-ov");
            let scope = match private {
                Some(true) => label.starts_with('p'),
                Some(false) => label.starts_with('w'),
                None => true,
            };
            trusted && scope
        }
        _ => false,
    });
}

pub fn emit_tabs(app: &AppHandle, win: &Win) {
    emit_ui(app, &win.label, "tabs-changed", win.snapshot());
}

pub fn emit_tab(app: &AppHandle, win: &str, tab: &Tab) {
    emit_ui(app, win, "tab-updated", tab.info.clone());
}

pub fn layout(app: &AppHandle, label: &str) {
    if !on_main_thread() {
        let label = label.to_string();
        defer(app, move |a| layout(a, &label));
        return;
    }
    match with(|b| b.win_mut(label).map(Win::layout_ops)) {
        Some(Some(ops)) => {
            for op in ops {
                match op {
                    LayoutOp::Show(wv, [x, y, w, h]) => {
                        let _ = wv.set_bounds(Rect {
                            position: LogicalPosition::new(x, y).into(),
                            size: LogicalSize::new(w, h).into(),
                        });
                        let _ = wv.show();
                    }
                    LayoutOp::Hide(wv) => {
                        let _ = wv.hide();
                    }
                }
            }
        }
        Some(None) => {}
        None => {
            let label = label.to_string();
            defer(app, move |a| layout(a, &label));
        }
    }
}

pub fn session_dirty(app: &AppHandle) {
    app.state::<AppState>().session_dirty.store(true, std::sync::atomic::Ordering::Relaxed);
}
