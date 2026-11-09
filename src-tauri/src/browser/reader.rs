use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;

use super::{com, emit_ui, with};
use crate::error::{Error, Result};
use crate::state::AppState;

const READABILITY: &str = include_str!("../../scripts/vendor/Readability.js");
const READERABLE: &str = include_str!("../../scripts/vendor/Readability-readerable.js");
const READER: &str = include_str!("../../scripts/reader.js");

pub fn detect(app: &AppHandle, tab: u32, core: &ICoreWebView2) {
    let script = format!(
        "(function(){{\n{READERABLE}\ntry {{ return isProbablyReaderable(document, {{ minContentLength: 140, minScore: 20 }}); }} catch (e) {{ return false; }}\n}})()"
    );
    let a = app.clone();
    com::exec_script(core, &script, move |result| {
        let available = result.as_deref() == Some("true");
        super::webview::update(&a, tab, move |t| t.info.reader_available = available);
    });
}

fn options(app: &AppHandle) -> Value {
    let s = app.state::<AppState>().settings();
    json!({
        "fontSize": s.reader_font_size,
        "width": s.reader_width,
        "theme": s.reader_theme,
        "font": s.reader_font,
    })
}

pub fn toggle(app: &AppHandle, win: &str, tab: u32) -> Result<()> {
    let active = with(|b| b.win(win)?.tab(tab).map(|t| t.info.reader_active))
        .flatten()
        .ok_or(Error::NotFound("Tab"))?;
    let c = com::tab(tab).ok_or(Error::NotFound("Tab"))?;
    if active {
        com::exec_script(&c.core, "window.__silbReaderClose && window.__silbReaderClose()", |_| {});
        super::webview::update(app, tab, |t| t.info.reader_active = false);
        return Ok(());
    }
    let script = format!("(function(){{\n{READABILITY}\n{READER}\nreturn __silbReaderOpen({});\n}})()", options(app));
    let (a, w) = (app.clone(), win.to_string());
    com::exec_script(&c.core, &script, move |result| {
        let ok = result
            .and_then(|r| serde_json::from_str::<Value>(&r).ok())
            .and_then(|v| v.get("ok").and_then(Value::as_bool))
            .unwrap_or(false);
        if ok {
            super::webview::update(&a, tab, |t| t.info.reader_active = true);
        } else {
            emit_ui(&a, &w, "toast", "This page can't be shown in Reading mode.");
        }
    });
    Ok(())
}

pub fn refresh_open(app: &AppHandle) {
    let open: Vec<u32> = with(|b| {
        b.windows
            .iter()
            .flat_map(|w| w.tabs.iter().filter(|t| t.info.reader_active).map(|t| t.info.id))
            .collect()
    })
    .unwrap_or_default();
    let script = format!("window.__silbReaderApply && window.__silbReaderApply({})", options(app));
    for id in open {
        if let Some(c) = com::tab(id) {
            com::exec_script(&c.core, &script, |_| {});
        }
    }
}

pub fn on_message(app: &AppHandle, tab: u32, json: &str) {
    let Ok(value) = serde_json::from_str::<Value>(json) else { return };
    let Some(msg) = value.get("silbReader") else { return };
    if msg.get("closed").and_then(Value::as_bool) == Some(true) {
        super::webview::update(app, tab, |t| t.info.reader_active = false);
        return;
    }
    if let Some(p) = msg.get("prefs") {
        let mut s = app.state::<AppState>().settings();
        if let Some(v) = p.get("fontSize").and_then(Value::as_u64) {
            s.reader_font_size = v as u32;
        }
        if let Some(v) = p.get("width").and_then(Value::as_u64) {
            s.reader_width = v as u32;
        }
        if let Some(v) = p.get("theme").and_then(Value::as_str) {
            s.reader_theme = v.chars().take(16).collect();
        }
        if let Some(v) = p.get("font").and_then(Value::as_str) {
            s.reader_font = v.chars().take(16).collect();
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::commands::store_settings(&app, s).await;
        });
    }
}
