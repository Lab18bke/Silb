use std::cell::RefCell;
use std::collections::HashMap;

use base64::Engine;
use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewBuilder, WebviewUrl, WebviewWindowBuilder, Wry};
use url::Url;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::*;
use windows::core::Interface;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MBUTTON};

use super::com::{self, ViewCom};
use super::{defer, emit_tab, layout, mutate, session_dirty, with};
use crate::db::favicon_key;
use crate::error::{Error, Result};
use crate::state::AppState;
use crate::util::{host_of, is_web_url, origin_of};

thread_local! {
    static NEW_WINDOW_HINTS: RefCell<HashMap<String, (bool, bool)>> = RefCell::new(HashMap::new());
}

pub fn is_blocked_navigation(uri: &str) -> bool {
    let Ok(u) = Url::parse(uri) else { return false };
    match u.scheme() {
        "tauri" | "ipc" | "asset" | "silb" => true,
        "http" | "https" => {
            let host = u.host_str().unwrap_or("").to_ascii_lowercase();
            let internal = ["tauri.localhost", "ipc.localhost", "asset.localhost"];
            internal.iter().any(|h| host == *h || host.ends_with(&format!(".{h}")))
                || (cfg!(debug_assertions) && host == "localhost" && u.port() == Some(1420))
        }
        _ => false,
    }
}

pub fn create(app: &AppHandle, win_label: &str, tab_id: u32, url: &str) -> Result<()> {
    let (window, private) = with(|b| b.win(win_label).map(|w| (w.window.clone(), w.private)))
        .flatten()
        .ok_or(Error::NotFound("Window"))?;
    let parsed: Url = url.parse().map_err(|_| Error::invalid("That address isn't valid."))?;
    if is_blocked_navigation(parsed.as_str()) {
        return Err(Error::invalid("That address can't be opened in a tab."));
    }
    let label = format!("{win_label}-t{tab_id}");
    let (a, w, l) = (app.clone(), win_label.to_string(), label.clone());
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(parsed))
        .with_environment(com::env())
        .incognito(private)
        .disable_drag_drop_handler()
        .zoom_hotkeys_enabled(true)
        .devtools(true)
        .on_new_window(move |url, features| on_new_window(&a, &w, tab_id, &l, private, url, features));
    let wv = window.add_child(builder, LogicalPosition::new(0.0, 0.0), LogicalSize::new(1.0, 1.0))?;
    let _ = wv.hide();
    let (a, w) = (app.clone(), win_label.to_string());
    wv.with_webview(move |pw| {
        if let Err(e) = unsafe { attach(&a, &w, tab_id, private, pw.controller()) } {
            eprintln!("silb: failed to attach tab handlers: {e}");
        }
    })?;
    let orphan = with(|b| match b.tab_mut(tab_id) {
        Some((win, i)) => {
            let t = &mut win.tabs[i];
            t.webview = Some(wv.clone());
            t.pending_url = None;
            t.shown = false;
            t.bounds = None;
            t.info.loading = true;
            false
        }
        None => true,
    })
    .unwrap_or(true);
    if orphan {
        com::remove_tab(tab_id);
        let _ = wv.close();
    }
    layout(app, win_label);
    Ok(())
}

fn on_new_window(
    app: &AppHandle,
    win_label: &str,
    tab_id: u32,
    wv_label: &str,
    private: bool,
    url: Url,
    features: NewWindowFeatures,
) -> NewWindowResponse<Wry> {
    let (user, background) = NEW_WINDOW_HINTS
        .with(|h| h.borrow_mut().remove(wv_label))
        .unwrap_or((false, false));
    let target = url.to_string();
    if is_blocked_navigation(&target) {
        return NewWindowResponse::Deny;
    }
    let opener = with(|b| b.tab_mut(tab_id).map(|(w, i)| w.tabs[i].info.url.clone()))
        .flatten()
        .unwrap_or_default();
    let origin = origin_of(&opener).unwrap_or_default();
    if !user && !super::permissions::popups_allowed(app, private, &origin) {
        mutate(app, move |b, app| {
            if let Some((w, i)) = b.tab_mut(tab_id) {
                let t = &mut w.tabs[i];
                if !t.info.blocked_popups.contains(&target) {
                    t.info.blocked_popups.push(target);
                    if t.info.blocked_popups.len() > 20 {
                        t.info.blocked_popups.remove(0);
                    }
                }
                emit_tab(app, &w.label, t);
            }
        });
        return NewWindowResponse::Deny;
    }
    if features.size().is_some() || features.position().is_some() {
        match create_popup(app, win_label, private, url, features) {
            Ok(window) => return NewWindowResponse::Create { window },
            Err(e) => eprintln!("silb: popup failed: {e}"),
        }
    }
    let settings = app.state::<AppState>().settings();
    let foreground = if background { settings.switch_to_new_tabs } else { true };
    let win = win_label.to_string();
    defer(app, move |app| {
        if settings.open_links_in_new_tab || background {
            super::tabs::open_tab(
                app,
                &win,
                super::tabs::TabSpec::web(target),
                foreground,
                super::tabs::Placement::After(tab_id),
            );
        } else {
            let _ = super::window::create_window(app, private, vec![super::tabs::TabSpec::web(target)], 0);
        }
    });
    NewWindowResponse::Deny
}

fn create_popup(
    app: &AppHandle,
    opener_win: &str,
    private: bool,
    url: Url,
    features: NewWindowFeatures,
) -> Result<tauri::WebviewWindow> {
    let n = with(|b| {
        b.next_popup += 1;
        b.next_popup
    })
    .unwrap_or_else(|| crate::util::now_ms() as u32);
    let label = format!("popup{n}");
    let (a, w) = (app.clone(), opener_win.to_string());
    let hidden = super::window::hidden_mode();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(if private { "Silb — Private" } else { "Silb" })
        .inner_size(560.0, 680.0)
        .window_features(features)
        .skip_taskbar(hidden)
        .focused(!hidden)
        .incognito(private)
        .disable_drag_drop_handler()
        .zoom_hotkeys_enabled(true)
        .devtools(true)
        .on_navigation(|u| !is_blocked_navigation(u.as_str()))
        .on_document_title_changed(|wv, title| {
            let _ = wv.set_title(&title);
        })
        .on_new_window(move |url, _| {
            let target = url.to_string();
            if !is_blocked_navigation(&target) {
                let w = w.clone();
                defer(&a, move |app| {
                    super::tabs::open_tab(app, &w, super::tabs::TabSpec::web(target), true, super::tabs::Placement::End);
                });
            }
            NewWindowResponse::Deny
        })
        .build()?;
    if hidden {
        let _ = window.set_position(tauri::LogicalPosition::new(-4000.0, -3000.0));
    }
    let (a, w) = (app.clone(), opener_win.to_string());
    window.with_webview(move |pw| unsafe {
        let Ok(core) = pw.controller().CoreWebView2() else { return };
        super::sitedata::apply_profile_prefs(&a, &core);
        let _ = super::permissions::attach(&a, &w, None, private, &core);
        let _ = super::downloads::attach(&a, &w, None, private, &core);
    })?;
    Ok(window)
}

pub fn update(app: &AppHandle, tab_id: u32, f: impl FnOnce(&mut super::Tab) + Send + 'static) {
    mutate(app, move |b, app| {
        if let Some((w, i)) = b.tab_mut(tab_id) {
            f(&mut w.tabs[i]);
            emit_tab(app, &w.label, &w.tabs[i]);
        }
    });
}

fn record_visit(app: &AppHandle, url: String, title: String) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = app.state::<AppState>().db.history_add(&url, &title, crate::util::now_ms());
    });
}

fn apply_site_zoom(app: &AppHandle, tab_id: u32, private: bool, url: &str) {
    let Some(host) = host_of(url) else { return };
    let factor = app.state::<AppState>().zoom_for(private, &host);
    if let Some(c) = com::tab(tab_id) {
        let mut current = 1.0;
        unsafe {
            let _ = c.controller.ZoomFactor(&mut current);
            if (current - factor).abs() > 0.001 {
                let _ = c.controller.SetZoomFactor(factor);
            }
        }
        super::tabs::record_zoom(app, tab_id, factor);
    }
}

unsafe fn attach(
    app: &AppHandle,
    win_label: &str,
    tab_id: u32,
    private: bool,
    controller: ICoreWebView2Controller,
) -> windows::core::Result<()> {
    let core = controller.CoreWebView2()?;
    com::set_tab(tab_id, ViewCom { controller: controller.clone(), core: core.clone() });
    let settings = core.Settings()?;
    settings.SetIsStatusBarEnabled(true)?;
    if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
        let _ = s4.SetIsPasswordAutosaveEnabled(false);
    }
    super::sitedata::apply_profile_prefs(app, &core);
    let mut token = 0i64;
    let label = format!("{win_label}-t{tab_id}");

    let a = app.clone();
    core.add_NavigationStarting(
        &NavigationStartingEventHandler::create(Box::new(move |sender, args| {
            let (Some(core), Some(args)) = (sender, args) else { return Ok(()) };
            let uri = com::string(|p| args.Uri(p));
            if is_blocked_navigation(&uri) {
                args.SetCancel(true)?;
                return Ok(());
            }
            let allowed = super::permissions::javascript_allowed(&a, private, &origin_of(&uri).unwrap_or_default());
            if let Ok(s) = core.Settings() {
                if com::flag(|b| s.IsScriptEnabled(b)) != allowed {
                    let _ = s.SetIsScriptEnabled(allowed);
                }
            }
            super::permissions::cancel_for_tab(&a, tab_id);
            update(&a, tab_id, |t| {
                t.info.loading = true;
                t.info.reader_available = false;
                t.info.reader_active = false;
                t.info.crashed = None;
                t.browser_crashed = false;
            });
            Ok(())
        })),
        &mut token,
    )?;

    core.add_FrameNavigationStarting(
        &NavigationStartingEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                if is_blocked_navigation(&com::string(|p| args.Uri(p))) {
                    args.SetCancel(true)?;
                }
            }
            Ok(())
        })),
        &mut token,
    )?;

    let a = app.clone();
    core.add_SourceChanged(
        &SourceChangedEventHandler::create(Box::new(move |sender, args| {
            let Some(core) = sender else { return Ok(()) };
            let url = com::string(|p| core.Source(p));
            let new_document = args.map(|a| com::flag(|b| a.IsNewDocument(b))).unwrap_or(true);
            if !private && !new_document && is_web_url(&url) {
                record_visit(&a, url.clone(), com::string(|p| core.DocumentTitle(p)));
            }
            let app = a.clone();
            mutate(&a, move |b, _| {
                let Some((w, i)) = b.tab_mut(tab_id) else { return };
                let t = &mut w.tabs[i];
                let host_changed = host_of(&t.info.url) != host_of(&url);
                t.info.url = url.clone();
                t.info.reader_active = false;
                t.refresh_security();
                emit_tab(&app, &w.label, t);
                session_dirty(&app);
                if host_changed {
                    let app2 = app.clone();
                    defer(&app, move |_| apply_site_zoom(&app2, tab_id, private, &url));
                }
            });
            Ok(())
        })),
        &mut token,
    )?;

    let a = app.clone();
    core.add_HistoryChanged(
        &HistoryChangedEventHandler::create(Box::new(move |sender, _| {
            let Some(core) = sender else { return Ok(()) };
            let back = com::flag(|b| core.CanGoBack(b));
            let fwd = com::flag(|b| core.CanGoForward(b));
            update(&a, tab_id, move |t| {
                t.web_can_back = back;
                t.web_can_forward = fwd;
                t.refresh_nav();
            });
            Ok(())
        })),
        &mut token,
    )?;

    let a = app.clone();
    core.add_NavigationCompleted(
        &NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
            let (Some(core), Some(args)) = (sender, args) else { return Ok(()) };
            let ok = com::flag(|b| args.IsSuccess(b));
            let url = com::string(|p| core.Source(p));
            let title = com::string(|p| core.DocumentTitle(p));
            if ok && !private && is_web_url(&url) {
                record_visit(&a, url.clone(), title.clone());
            }
            update(&a, tab_id, move |t| {
                t.info.loading = false;
                if !title.is_empty() {
                    t.info.title = title;
                }
            });
            if ok && is_web_url(&url) {
                super::reader::detect(&a, tab_id, &core);
            }
            Ok(())
        })),
        &mut token,
    )?;

    let a = app.clone();
    core.add_DocumentTitleChanged(
        &DocumentTitleChangedEventHandler::create(Box::new(move |sender, _| {
            let Some(core) = sender else { return Ok(()) };
            let title = com::string(|p| core.DocumentTitle(p));
            let url = com::string(|p| core.Source(p));
            if !private && is_web_url(&url) && !title.is_empty() {
                let (app, t) = (a.clone(), title.clone());
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = app.state::<AppState>().db.history_set_title(&url, &t);
                });
            }
            update(&a, tab_id, move |t| t.info.title = title);
            session_dirty(&a);
            Ok(())
        })),
        &mut token,
    )?;

    if let Ok(core15) = core.cast::<ICoreWebView2_15>() {
        let a = app.clone();
        core15.add_FaviconChanged(
            &FaviconChangedEventHandler::create(Box::new(move |sender, _| {
                let Some(core) = sender else { return Ok(()) };
                let core15: ICoreWebView2_15 = core.cast()?;
                let page = com::string(|p| core.Source(p));
                if com::string(|p| core15.FaviconUri(p)).is_empty() {
                    update(&a, tab_id, |t| t.info.favicon = None);
                    return Ok(());
                }
                let a2 = a.clone();
                core15.GetFavicon(
                    COREWEBVIEW2_FAVICON_IMAGE_FORMAT_PNG,
                    &GetFaviconCompletedHandler::create(Box::new(move |hr, stream| {
                        let bytes = match (hr, stream) {
                            (Ok(()), Some(s)) => com::read_stream(&s, 256 * 1024),
                            _ => Vec::new(),
                        };
                        if bytes.is_empty() {
                            return Ok(());
                        }
                        let data = format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(&bytes)
                        );
                        if !private {
                            if let Some(host) = favicon_key(&page) {
                                let (app, d) = (a2.clone(), data.clone());
                                tauri::async_runtime::spawn_blocking(move || {
                                    let _ = app.state::<AppState>().db.favicon_set(&host, &d);
                                });
                            }
                        }
                        update(&a2, tab_id, move |t| t.info.favicon = Some(data));
                        Ok(())
                    })),
                )?;
                Ok(())
            })),
            &mut token,
        )?;
    }

    let (a, w) = (app.clone(), win_label.to_string());
    core.add_ContainsFullScreenElementChanged(
        &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
            let Some(core) = sender else { return Ok(()) };
            let full = com::flag(|b| core.ContainsFullScreenElement(b));
            let w = w.clone();
            defer(&a, move |app| super::window::set_html_fullscreen(app, &w, full));
            Ok(())
        })),
        &mut token,
    )?;

    let (a, w) = (app.clone(), win_label.to_string());
    core.add_ProcessFailed(
        &ProcessFailedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
            args.ProcessFailedKind(&mut kind)?;
            let (message, browser) = match kind {
                COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED => ("This page stopped working.", false),
                COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => ("This page isn't responding.", false),
                COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED => ("The web engine stopped unexpectedly.", true),
                _ => return Ok(()),
            };
            update(&a, tab_id, move |t| {
                t.info.crashed = Some(message.into());
                t.info.loading = false;
                t.browser_crashed |= browser;
            });
            let w = w.clone();
            defer(&a, move |app| layout(app, &w));
            Ok(())
        })),
        &mut token,
    )?;

    let a = app.clone();
    core.add_WebMessageReceived(
        &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                let json = com::string(|p| args.WebMessageAsJson(p));
                super::reader::on_message(&a, tab_id, &json);
            }
            Ok(())
        })),
        &mut token,
    )?;

    if let Ok(core14) = core.cast::<ICoreWebView2_14>() {
        let a = app.clone();
        core14.add_ServerCertificateErrorDetected(
            &ServerCertificateErrorDetectedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let uri = com::string(|p| args.RequestUri(p));
                if let Some(host) = Url::parse(&uri).ok().and_then(|u| u.host_str().map(str::to_string)) {
                    update(&a, tab_id, move |t| {
                        t.cert_errors.insert(host);
                        t.refresh_security();
                    });
                }
                Ok(())
            })),
            &mut token,
        )?;
    }

    if let Ok(core8) = core.cast::<ICoreWebView2_8>() {
        let a = app.clone();
        core8.add_IsDocumentPlayingAudioChanged(
            &IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |sender, _| {
                let Some(core) = sender else { return Ok(()) };
                let core8: ICoreWebView2_8 = core.cast()?;
                let audible = com::flag(|b| core8.IsDocumentPlayingAudio(b));
                update(&a, tab_id, move |t| t.info.audible = audible);
                Ok(())
            })),
            &mut token,
        )?;
    }

    let hint_label = label.clone();
    core.add_NewWindowRequested(
        &NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
            let user = args.map(|a| com::flag(|b| a.IsUserInitiated(b))).unwrap_or(false);
            let background = GetKeyState(VK_CONTROL.0 as i32) < 0 || GetKeyState(VK_MBUTTON.0 as i32) < 0;
            NEW_WINDOW_HINTS.with(|h| h.borrow_mut().insert(hint_label.clone(), (user, background)));
            Ok(())
        })),
        &mut token,
    )?;

    super::permissions::attach(app, win_label, Some(tab_id), private, &core)?;
    super::downloads::attach(app, win_label, Some(tab_id), private, &core)?;
    super::context_menu::attach(app, win_label, tab_id, private, &core)?;
    super::find::attach(app, win_label, tab_id, &core)?;
    super::shortcuts::attach(app, win_label, Some(tab_id), &controller)?;

    let a = app.clone();
    controller.add_ZoomFactorChanged(
        &ZoomFactorChangedEventHandler::create(Box::new(move |sender, _| {
            let Some(controller) = sender else { return Ok(()) };
            let mut z = 1.0;
            controller.ZoomFactor(&mut z)?;
            let app = a.clone();
            defer(&a, move |_| super::tabs::record_zoom(&app, tab_id, z));
            Ok(())
        })),
        &mut token,
    )?;

    Ok(())
}

pub fn navigate_core(tab_id: u32, url: &str) -> Result<()> {
    let c = com::tab(tab_id).ok_or(Error::NotFound("Tab"))?;
    unsafe { c.core.Navigate(&windows::core::HSTRING::from(url))? };
    Ok(())
}

