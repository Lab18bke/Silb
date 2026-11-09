use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::tabs::TabSpec;
use super::{with, InternalPos};
use crate::settings::Startup;
use crate::state::AppState;

#[derive(Serialize, Deserialize, Default)]
pub struct Session {
    pub windows: Vec<SessionWindow>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionWindow {
    pub tabs: Vec<SessionTab>,
    pub active: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SessionTab {
    pub url: String,
    pub title: String,
}

fn snapshot() -> Option<Session> {
    with(|b| Session {
        windows: b
            .windows
            .iter()
            .filter(|w| !w.private)
            .map(|w| SessionWindow {
                tabs: w
                    .tabs
                    .iter()
                    .map(|t| {
                        let url = match &t.info.internal {
                            Some(page) if !t.has_web() || t.internal_pos == InternalPos::Behind => format!("silb://{page}"),
                            _ if t.info.url.is_empty() => "silb://newtab".to_string(),
                            _ => t.info.url.clone(),
                        };
                        SessionTab { url, title: t.info.title.clone() }
                    })
                    .collect(),
                active: w.tabs.iter().position(|t| Some(t.info.id) == w.active).unwrap_or(0),
            })
            .filter(|w| !w.tabs.is_empty())
            .collect(),
    })
}

pub fn save_now(app: &AppHandle) {
    if let Some(session) = snapshot() {
        if let Ok(json) = serde_json::to_string(&session) {
            let state = app.state::<AppState>();
            let _ = state.db.kv_set("session", &json);
            state.session_dirty.store(false, Ordering::Relaxed);
        }
    }
}

pub fn spawn_saver(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticks = 0u64;
        loop {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            ticks += 1;
            let dirty = app.state::<AppState>().session_dirty.swap(false, Ordering::Relaxed);
            let suspend = app.state::<AppState>().settings().suspend_background_tabs && ticks % 40 == 0;
            if !dirty && !suspend {
                continue;
            }
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                if dirty {
                    let has_normal = with(|b| b.windows.iter().any(|w| !w.private)).unwrap_or(false);
                    if has_normal {
                        save_now(&a);
                    }
                }
                if suspend {
                    super::tabs::suspend_idle(15 * 60);
                }
            });
        }
    });
}

pub fn restore(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let session: Session = state
        .db
        .kv_get("session")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let created = match settings.startup {
        Startup::Restore if !session.windows.is_empty() => session
            .windows
            .into_iter()
            .map(|w| {
                let specs = w.tabs.into_iter().map(|t| TabSpec::from_url(&t.url, Some(t.title), true)).collect();
                super::window::create_window(app, false, specs, w.active).is_ok()
            })
            .fold(false, |acc, ok| acc || ok),
        Startup::Homepage if crate::util::is_web_url(&settings.homepage) => {
            super::window::create_window(app, false, vec![TabSpec::web(settings.homepage.clone())], 0).is_ok()
        }
        _ => false,
    };
    if !created {
        open_first_window(app, Vec::new());
    }
}

pub fn open_first_window(app: &AppHandle, specs: Vec<TabSpec>) {
    let mut last = None;
    for attempt in 0..12 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(500));
        }
        match super::window::create_window(app, false, specs.clone(), 0) {
            Ok(_) => return,
            Err(e) => last = Some(e),
        }
    }
    let detail = last.map(|e| e.to_string()).unwrap_or_default();
    fatal(&format!(
        "Silb couldn't open a window.\n\n{detail}\n\nIf another copy of Silb was just closed, wait a few seconds and try again."
    ));
}

pub fn fatal(message: &str) -> ! {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Silb Browser")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    std::process::exit(1)
}
