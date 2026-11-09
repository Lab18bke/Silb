use std::cell::RefCell;
use std::collections::HashSet;

use serde::Serialize;
use tauri::AppHandle;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{FindActiveMatchIndexChangedEventHandler, FindMatchCountChangedEventHandler, FindStartCompletedHandler};
use windows::core::{Interface, HSTRING};

use super::{com, emit_ui, with};
use crate::error::{Error, Result};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FindResult {
    tab_id: u32,
    active: i32,
    count: i32,
}

thread_local! {
    static RUNNING: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

fn find_of(core: &ICoreWebView2) -> windows::core::Result<ICoreWebView2Find> {
    unsafe { core.cast::<ICoreWebView2_28>()?.Find() }
}

fn report(app: &AppHandle, win: &str, tab: u32, find: &ICoreWebView2Find) {
    let (mut active, mut count) = (-1i32, 0i32);
    unsafe {
        let _ = find.ActiveMatchIndex(&mut active);
        let _ = find.MatchCount(&mut count);
    }
    emit_ui(app, win, "find-result", FindResult { tab_id: tab, active: active + 1, count });
}

pub unsafe fn attach(app: &AppHandle, win: &str, tab: u32, core: &ICoreWebView2) -> windows::core::Result<()> {
    let Ok(find) = find_of(core) else { return Ok(()) };
    let mut token = 0i64;
    let (a, w) = (app.clone(), win.to_string());
    find.add_MatchCountChanged(
        &FindMatchCountChangedEventHandler::create(Box::new(move |sender, _| {
            if let Some(f) = sender {
                report(&a, &w, tab, &f);
            }
            Ok(())
        })),
        &mut token,
    )?;
    let (a, w) = (app.clone(), win.to_string());
    find.add_ActiveMatchIndexChanged(
        &FindActiveMatchIndexChangedEventHandler::create(Box::new(move |sender, _| {
            if let Some(f) = sender {
                report(&a, &w, tab, &f);
            }
            Ok(())
        })),
        &mut token,
    )?;
    Ok(())
}

fn active_tab(win: &str) -> Result<u32> {
    with(|b| b.win(win).and_then(|w| w.active)).flatten().ok_or(Error::NotFound("Tab"))
}

pub fn start(app: &AppHandle, win: &str, term: &str, case_sensitive: bool) -> Result<()> {
    let tab = active_tab(win)?;
    let c = com::tab(tab).ok_or(Error::NotFound("Tab"))?;
    let find = find_of(&c.core).map_err(|_| Error::invalid("Find isn't supported by this WebView2 version."))?;
    if term.is_empty() {
        unsafe { find.Stop()? };
        RUNNING.with(|r| r.borrow_mut().remove(&tab));
        emit_ui(app, win, "find-result", FindResult { tab_id: tab, active: 0, count: 0 });
        return Ok(());
    }
    unsafe {
        let env15: ICoreWebView2Environment15 = c.core.cast::<ICoreWebView2_2>()?.Environment()?.cast()?;
        let options = env15.CreateFindOptions()?;
        options.SetFindTerm(&HSTRING::from(term))?;
        options.SetIsCaseSensitive(case_sensitive)?;
        options.SetShouldHighlightAllMatches(true)?;
        options.SetSuppressDefaultFindDialog(true)?;
        let (a, w, f) = (app.clone(), win.to_string(), find.clone());
        find.Start(
            &options,
            &FindStartCompletedHandler::create(Box::new(move |_| {
                report(&a, &w, tab, &f);
                Ok(())
            })),
        )?;
    }
    RUNNING.with(|r| r.borrow_mut().insert(tab));
    Ok(())
}

pub fn step(win: &str, forward: bool) -> Result<()> {
    let tab = active_tab(win)?;
    if !RUNNING.with(|r| r.borrow().contains(&tab)) {
        return Ok(());
    }
    let c = com::tab(tab).ok_or(Error::NotFound("Tab"))?;
    let find = find_of(&c.core)?;
    unsafe {
        if forward {
            find.FindNext()?;
        } else {
            find.FindPrevious()?;
        }
    }
    Ok(())
}

pub fn stop_all(win: &str) {
    let tabs: Vec<u32> = with(|b| b.win(win).map(|w| w.tabs.iter().map(|t| t.info.id).collect()))
        .flatten()
        .unwrap_or_default();
    for tab in tabs {
        if RUNNING.with(|r| r.borrow_mut().remove(&tab)) {
            if let Some(c) = com::tab(tab) {
                if let Ok(find) = find_of(&c.core) {
                    unsafe {
                        let _ = find.Stop();
                    }
                }
            }
        }
    }
}
