use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{ContextMenuRequestedEventHandler, CustomItemSelectedEventHandler};
use windows::core::{Interface, HSTRING};
use windows::Win32::System::Com::IStream;

use super::tabs::{self, Placement, TabSpec};
use super::{com, defer};
use crate::state::AppState;

const REMOVED: [&str; 7] = ["openLinkInNewWindow", "share", "webCapture", "webSelect", "createQrCode", "loopback", "other"];

enum Action {
    LinkTab(String),
    LinkWindow(String),
    ImageTab(String),
    Search(String),
}

pub unsafe fn attach(
    app: &AppHandle,
    win: &str,
    tab: u32,
    private: bool,
    core: &ICoreWebView2,
) -> windows::core::Result<()> {
    let core11: ICoreWebView2_11 = core.cast()?;
    let (a, w) = (app.clone(), win.to_string());
    let mut token = 0i64;
    core11.add_ContextMenuRequested(
        &ContextMenuRequestedEventHandler::create(Box::new(move |sender, args| {
            let (Some(core), Some(args)) = (sender, args) else { return Ok(()) };
            build(&a, &w, tab, private, &core, &args)
        })),
        &mut token,
    )?;
    Ok(())
}

unsafe fn build(
    app: &AppHandle,
    win: &str,
    tab: u32,
    private: bool,
    core: &ICoreWebView2,
    args: &ICoreWebView2ContextMenuRequestedEventArgs,
) -> windows::core::Result<()> {
    let items = args.MenuItems()?;
    let target = args.ContextMenuTarget()?;
    let mut count = 0u32;
    items.Count(&mut count)?;
    for i in (0..count).rev() {
        let item = items.GetValueAtIndex(i)?;
        let name = com::string(|p| item.Name(p));
        if REMOVED.contains(&name.as_str()) {
            items.RemoveValueAtIndex(i)?;
        }
    }
    let env9: ICoreWebView2Environment9 = core.cast::<ICoreWebView2_2>()?.Environment()?.cast()?;
    let link = com::flag(|b| target.HasLinkUri(b)).then(|| com::string(|p| target.LinkUri(p))).filter(|u| crate::util::is_web_url(u));
    let mut kind = COREWEBVIEW2_CONTEXT_MENU_TARGET_KIND::default();
    target.Kind(&mut kind)?;
    let image = (kind == COREWEBVIEW2_CONTEXT_MENU_TARGET_KIND_IMAGE && com::flag(|b| target.HasSourceUri(b)))
        .then(|| com::string(|p| target.SourceUri(p)))
        .filter(|u| crate::util::is_web_url(u) || u.starts_with("data:image/"));
    let selection = com::flag(|b| target.HasSelection(b))
        .then(|| com::string(|p| target.SelectionText(p)))
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty() && s.len() < 400);

    let mut entries: Vec<Option<(String, Action)>> = Vec::new();
    if let Some(url) = &link {
        entries.push(Some(("Open link in new tab".into(), Action::LinkTab(url.clone()))));
        let label = if private { "Open link in new private window" } else { "Open link in new window" };
        entries.push(Some((label.into(), Action::LinkWindow(url.clone()))));
        entries.push(None);
    }
    if let Some(src) = &image {
        if !src.starts_with("data:") {
            entries.push(Some(("Open image in new tab".into(), Action::ImageTab(src.clone()))));
            entries.push(None);
        }
    }
    if let Some(text) = &selection {
        let (engine, _) = app.state::<AppState>().settings().engine();
        let short: String = text.chars().take(32).collect();
        let short = if short.len() < text.len() { format!("{short}…") } else { short };
        entries.push(Some((format!("Search {engine} for “{short}”"), Action::Search(text.clone()))));
        entries.push(None);
    }
    for (index, entry) in entries.into_iter().enumerate() {
        let item = match entry {
            None => env9.CreateContextMenuItem(&HSTRING::new(), None::<&IStream>, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_SEPARATOR)?,
            Some((label, action)) => {
                let item = env9.CreateContextMenuItem(&HSTRING::from(label), None::<&IStream>, COREWEBVIEW2_CONTEXT_MENU_ITEM_KIND_COMMAND)?;
                let (a, w) = (app.clone(), win.to_string());
                let mut action = Some(action);
                let mut token = 0i64;
                item.add_CustomItemSelected(
                    &CustomItemSelectedEventHandler::create(Box::new(move |_, _| {
                        if let Some(action) = action.take() {
                            let w = w.clone();
                            defer(&a, move |app| run(app, &w, tab, private, action));
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
                item
            }
        };
        items.InsertValueAtIndex(index as u32, &item)?;
    }
    Ok(())
}

fn run(app: &AppHandle, win: &str, tab: u32, private: bool, action: Action) {
    let background = !app.state::<AppState>().settings().switch_to_new_tabs;
    match action {
        Action::LinkTab(url) | Action::ImageTab(url) => {
            tabs::open_tab(app, win, TabSpec::web(url), !background, Placement::After(tab));
        }
        Action::LinkWindow(url) => {
            let _ = super::window::create_window(app, private, vec![TabSpec::web(url)], 0);
        }
        Action::Search(text) => {
            let template = app.state::<AppState>().settings().engine().1;
            let url = crate::omnibox::search_url(&template, &text);
            tabs::open_tab(app, win, TabSpec::web(url), true, Placement::After(tab));
        }
    }
}
