use serde::Deserialize;
use tauri::window::{Color, WindowBuilder};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Rect, Theme, WebviewBuilder, WebviewUrl, WindowEvent,
};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::NavigationStartingEventHandler;
use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
};

use super::com::{self, ViewCom};
use super::tabs::{self, Placement, TabSpec};
use super::{emit_overlay, emit_ui, layout, overlay_label, session_dirty, ui_label, with, Win};
use crate::error::{Error, Result};
use crate::settings::Theme as ThemeSetting;
use crate::state::AppState;

fn background(app: &AppHandle, private: bool) -> Color {
    let theme = app.state::<AppState>().settings().theme;
    let dark = private
        || match theme {
            ThemeSetting::Dark => true,
            ThemeSetting::Light => false,
            ThemeSetting::System => system_dark(),
        };
    if private {
        Color(0x1d, 0x1b, 0x22, 255)
    } else if dark {
        Color(0x1c, 0x1b, 0x19, 255)
    } else {
        Color(0xf6, 0xf4, 0xef, 255)
    }
}

fn system_dark() -> bool {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut value = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            windows::core::w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    ok.is_ok() && value == 0
}

pub fn hidden_mode() -> bool {
    std::env::var("SILB_HIDDEN").is_ok_and(|v| v == "1")
}

pub fn window_theme(app: &AppHandle) -> Option<Theme> {
    match app.state::<AppState>().settings().theme {
        ThemeSetting::Dark => Some(Theme::Dark),
        ThemeSetting::Light => Some(Theme::Light),
        ThemeSetting::System => None,
    }
}

pub fn create_window(app: &AppHandle, private: bool, specs: Vec<TabSpec>, active: usize) -> Result<String> {
    let n = with(|b| {
        b.next_win += 1;
        b.next_win
    })
    .ok_or_else(|| Error::invalid("Busy"))?;
    let label = format!("{}{}", if private { "p" } else { "w" }, n);
    let geometry = app
        .state::<AppState>()
        .db
        .kv_get("window")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<Geometry>(&s).ok())
        .unwrap_or_else(Geometry::default_size);
    let hidden = hidden_mode();
    let mut builder = WindowBuilder::new(app, &label);
    if hidden {
        builder = builder.skip_taskbar(true).position(-4000.0, -4000.0).focused(false);
    }
    let window = builder
        .title(if private { "Silb — Private" } else { "Silb" })
        .inner_size(if hidden { 1280.0 } else { geometry.width.max(640.0) }, if hidden { 820.0 } else { geometry.height.max(480.0) })
        .min_inner_size(560.0, 420.0)
        .decorations(false)
        .shadow(true)
        .visible(false)
        .theme(window_theme(app))
        .build()?;
    if geometry.maximized && !hidden {
        let _ = window.maximize();
    }
    let (ui, overlay) = match build_views(app, &window, &label, private) {
        Ok(views) => views,
        Err(e) => {
            let _ = window.destroy();
            return Err(e);
        }
    };

    let overlay_hwnd = std::sync::Arc::new(std::sync::atomic::AtomicIsize::new(0));
    for (wv, is_overlay) in [(&ui, false), (&overlay, true)] {
        let (a, w, l, hwnd) = (app.clone(), label.clone(), wv.label().to_string(), overlay_hwnd.clone());
        wv.with_webview(move |pw| unsafe {
            let controller = pw.controller();
            if is_overlay {
                hwnd.store(com::container_hwnd(&controller), std::sync::atomic::Ordering::Relaxed);
            }
            if let Err(e) = attach_trusted(&a, &w, &l, controller) {
                eprintln!("silb: failed to configure {l}: {e}");
            }
        })?;
    }

    let win = Win {
        label: label.clone(),
        window: window.clone(),
        ui,
        overlay,
        overlay_hwnd: overlay_hwnd.load(std::sync::atomic::Ordering::Relaxed),
        private,
        tabs: Vec::new(),
        active: None,
        chrome_top: 84.0,
        html_fullscreen: false,
        closed: Vec::new(),
        content_visible: true,
        force_close: false,
        shown: false,
        overlay_visible: false,
        overlay_focused: false,
    };
    with(|b| b.windows.push(win)).ok_or_else(|| Error::invalid("Busy"))?;

    let app2 = app.clone();
    let l2 = label.clone();
    window.on_window_event(move |event| on_window_event(&app2, &l2, event));

    let specs = if specs.is_empty() { vec![tabs::default_new_tab(app, private)] } else { specs };
    let active = active.min(specs.len() - 1);
    for (i, spec) in specs.into_iter().enumerate() {
        tabs::open_tab(app, &label, spec, i == active, Placement::End);
    }

    let (a, l) = (app.clone(), label.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let inner = a.clone();
        let _ = a.run_on_main_thread(move || show_window(&inner, &l));
    });
    Ok(label)
}

fn build_views(
    app: &AppHandle,
    window: &tauri::Window,
    label: &str,
    private: bool,
) -> Result<(tauri::Webview, tauri::Webview)> {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.inner_size()?.to_logical::<f64>(scale);
    let ui = window.add_child(
        WebviewBuilder::new(ui_label(label), WebviewUrl::App("index.html".into()))
            .with_environment(com::env())
            .auto_resize()
            .background_color(background(app, private))
            .devtools(cfg!(debug_assertions)),
        LogicalPosition::new(0.0, 0.0),
        size,
    )?;
    let overlay = window.add_child(
        WebviewBuilder::new(overlay_label(label), WebviewUrl::App("overlay.html".into()))
            .with_environment(com::env())
            .transparent(true)
            .focused(false)
            .disable_drag_drop_handler()
            .devtools(cfg!(debug_assertions)),
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(1.0, 1.0),
    )?;
    let _ = overlay.hide();
    Ok((ui, overlay))
}

unsafe fn attach_trusted(
    app: &AppHandle,
    win: &str,
    label: &str,
    controller: ICoreWebView2Controller,
) -> windows::core::Result<()> {
    let core = controller.CoreWebView2()?;
    com::set_ui(label, ViewCom { controller: controller.clone(), core: core.clone() });
    let settings = core.Settings()?;
    if !cfg!(debug_assertions) {
        settings.SetAreDefaultContextMenusEnabled(false)?;
        if let Ok(s3) = settings.cast::<ICoreWebView2Settings3>() {
            s3.SetAreBrowserAcceleratorKeysEnabled(false)?;
        }
    }
    if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
        let _ = s4.SetIsGeneralAutofillEnabled(false);
        let _ = s4.SetIsPasswordAutosaveEnabled(false);
    }
    if let Ok(s8) = settings.cast::<ICoreWebView2Settings8>() {
        let _ = s8.SetIsReputationCheckingRequired(false);
    }
    let mut token = 0i64;
    core.add_NavigationStarting(
        &NavigationStartingEventHandler::create(Box::new(|_, args| {
            if let Some(args) = args {
                let uri = com::string(|p| args.Uri(p));
                let trusted = uri.starts_with("http://tauri.localhost/")
                    || uri.starts_with("tauri://localhost/")
                    || (cfg!(debug_assertions) && uri.starts_with("http://localhost:1420/"));
                if !trusted {
                    args.SetCancel(true)?;
                }
            }
            Ok(())
        })),
        &mut token,
    )?;
    core.add_NewWindowRequested(
        &webview2_com::NewWindowRequestedEventHandler::create(Box::new(|_, args| {
            if let Some(args) = args {
                args.SetHandled(true)?;
            }
            Ok(())
        })),
        &mut token,
    )?;
    super::shortcuts::attach(app, win, None, &controller)?;
    Ok(())
}

pub fn show_window(app: &AppHandle, label: &str) {
    let target = with(|b| {
        let w = b.win_mut(label)?;
        if w.shown {
            return None;
        }
        w.shown = true;
        Some(w.window.clone())
    })
    .flatten();
    if let Some(window) = target {
        let _ = window.show();
        if hidden_mode() {
            let _ = window.unminimize();
            let _ = window.set_position(tauri::LogicalPosition::new(-4000.0, -4000.0));
        } else {
            let _ = window.set_focus();
        }
        layout(app, label);
        tabs::focus_active(app, label);
    }
}

#[derive(serde::Serialize, Deserialize, Default)]
struct Geometry {
    width: f64,
    height: f64,
    maximized: bool,
}

impl Geometry {
    fn default_size() -> Self {
        Geometry { width: 1280.0, height: 840.0, maximized: false }
    }
}

fn save_geometry(app: &AppHandle, window: &tauri::Window) {
    if window.is_fullscreen().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let scale = window.scale_factor().unwrap_or(1.0);
    let mut g = Geometry::default_size();
    g.maximized = maximized;
    if !maximized {
        if let Ok(s) = window.inner_size() {
            let s = s.to_logical::<f64>(scale);
            g.width = s.width;
            g.height = s.height;
        }
    } else if let Ok(Some(prev)) = app.state::<AppState>().db.kv_get("window") {
        if let Ok(p) = serde_json::from_str::<Geometry>(&prev) {
            g.width = p.width;
            g.height = p.height;
        }
    }
    if let Ok(json) = serde_json::to_string(&g) {
        let _ = app.state::<AppState>().db.kv_set("window", &json);
    }
}

fn on_window_event(app: &AppHandle, label: &str, event: &WindowEvent) {
    match event {
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            layout(app, label);
            let state = with(|b| b.win(label).map(|w| (w.window.is_maximized().unwrap_or(false), w.window.is_fullscreen().unwrap_or(false))))
                .flatten();
            if let Some((maximized, fullscreen)) = state {
                emit_ui(app, label, "window-state", serde_json::json!({ "maximized": maximized, "fullscreen": fullscreen }));
            }
            hide_overlay(app, label);
        }
        WindowEvent::Moved(_) => hide_overlay(app, label),
        WindowEvent::CloseRequested { api, .. } => {
            let decision = with(|b| {
                let w = b.win(label)?;
                let confirm = app.state::<AppState>().settings().confirm_close_multiple;
                Some((!w.force_close && confirm && w.tabs.len() > 1, w.tabs.len(), w.window.clone(), w.private))
            })
            .flatten();
            if let Some((ask, count, window, private)) = decision {
                if ask {
                    api.prevent_close();
                    emit_ui(app, label, "confirm-close", count);
                    return;
                }
                let last_normal = with(|b| !private && b.windows.iter().filter(|w| !w.private).count() == 1).unwrap_or(false);
                if last_normal {
                    super::session::save_now(app);
                }
                if !hidden_mode() {
                    save_geometry(app, &window);
                }
            }
        }
        WindowEvent::Destroyed => {
            let removed = with(|b| {
                let i = b.windows.iter().position(|w| w.label == label)?;
                let w = b.windows.remove(i);
                Some((w.tabs.iter().map(|t| t.info.id).collect::<Vec<_>>(), w.private, b.has_private_windows()))
            })
            .flatten();
            com::remove_ui(&ui_label(label));
            com::remove_ui(&overlay_label(label));
            if let Some((ids, private, private_left)) = removed {
                for id in ids {
                    super::permissions::cancel_for_tab(app, id);
                    com::remove_tab(id);
                }
                if private && !private_left {
                    app.state::<AppState>().end_private_session();
                    super::downloads::end_private_session(app);
                }
                if !private {
                    session_dirty(app);
                }
            }
        }
        _ => {}
    }
}

pub fn close_window(_app: &AppHandle, label: &str, force: bool) {
    let window = with(|b| {
        let w = b.win_mut(label)?;
        w.force_close |= force;
        Some(w.window.clone())
    })
    .flatten();
    if let Some(w) = window {
        let _ = w.close();
    }
}

pub fn set_html_fullscreen(app: &AppHandle, label: &str, full: bool) {
    let window = with(|b| {
        let w = b.win_mut(label)?;
        w.html_fullscreen = full;
        Some(w.window.clone())
    })
    .flatten();
    if let Some(window) = window {
        let _ = window.set_fullscreen(full);
        layout(app, label);
        emit_ui(app, label, "html-fullscreen", full);
    }
}

pub fn toggle_fullscreen(app: &AppHandle, label: &str) -> Result<bool> {
    let window = with(|b| b.win(label).map(|w| w.window.clone())).flatten().ok_or(Error::NotFound("Window"))?;
    let full = !window.is_fullscreen()?;
    window.set_fullscreen(full)?;
    if !full {
        with(|b| {
            if let Some(w) = b.win_mut(label) {
                w.html_fullscreen = false;
            }
        });
    }
    layout(app, label);
    Ok(full)
}

pub fn set_chrome_top(app: &AppHandle, label: &str, top: f64) {
    with(|b| {
        if let Some(w) = b.win_mut(label) {
            w.chrome_top = top.clamp(0.0, 400.0);
        }
    });
    layout(app, label);
}

pub fn set_content_visible(app: &AppHandle, label: &str, visible: bool) {
    with(|b| {
        if let Some(w) = b.win_mut(label) {
            w.content_visible = visible;
        }
    });
    layout(app, label);
}

#[derive(Debug, Deserialize)]
pub struct OverlayRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn show_overlay(app: &AppHandle, label: &str, rect: OverlayRect, payload: serde_json::Value, focus: bool) -> Result<()> {
    let (overlay, hwnd, window) = with(|b| {
        b.win_mut(label).map(|w| {
            w.overlay_visible = true;
            w.overlay_focused = focus;
            (w.overlay.clone(), w.overlay_hwnd, w.window.clone())
        })
    })
    .flatten()
    .ok_or(Error::NotFound("Window"))?;
    let scale = window.scale_factor()?;
    let size = window.inner_size()?.to_logical::<f64>(scale);
    let width = rect.width.clamp(1.0, size.width);
    let height = rect.height.clamp(1.0, size.height);
    let x = rect.x.clamp(0.0, (size.width - width).max(0.0));
    let y = rect.y.clamp(0.0, (size.height - height).max(0.0));
    emit_overlay(app, label, "overlay-show", payload);
    overlay.set_bounds(Rect { position: LogicalPosition::new(x, y).into(), size: LogicalSize::new(width, height).into() })?;
    if hwnd != 0 {
        unsafe {
            let _ = SetWindowPos(
                HWND(hwnd as *mut _),
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
    overlay.show()?;
    if focus {
        overlay.set_focus()?;
    }
    Ok(())
}

pub fn resize_overlay(label: &str, height: f64) -> Result<()> {
    let (overlay, window) = with(|b| b.win(label).map(|w| (w.overlay.clone(), w.window.clone())))
        .flatten()
        .ok_or(Error::NotFound("Window"))?;
    let scale = window.scale_factor()?;
    let pos = overlay.position()?.to_logical::<f64>(scale);
    let size = overlay.size()?.to_logical::<f64>(scale);
    let max = window.inner_size()?.to_logical::<f64>(scale).height - pos.y;
    overlay.set_size(LogicalSize::new(size.width, height.clamp(1.0, max.max(1.0))))?;
    Ok(())
}

pub fn hide_overlay(app: &AppHandle, label: &str) {
    let views = with(|b| {
        b.win_mut(label).and_then(|w| {
            let was = (w.overlay_visible, w.overlay_focused);
            w.overlay_visible = false;
            w.overlay_focused = false;
            was.0.then(|| (w.overlay.clone(), w.ui.clone(), was.1))
        })
    })
    .flatten();
    if let Some((overlay, ui, had_focus)) = views {
        {
            let _ = overlay.hide();
            emit_overlay(app, label, "overlay-hidden", ());
            emit_ui(app, label, "overlay-hidden", ());
            if had_focus {
                let _ = ui.set_focus();
            }
        }
    }
}

pub fn snapshot(label: &str) -> Option<super::WindowSnapshot> {
    with(|b| b.win(label).map(Win::snapshot)).flatten()
}

