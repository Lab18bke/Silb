use std::cell::RefCell;
use std::collections::HashMap;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::PermissionRequestedEventHandler;
use windows::core::Interface;

use super::{com, emit_ui};
use crate::error::{Error, Result};
use crate::state::AppState;
use crate::util::origin_of;

pub const SITE_KINDS: [&str; 11] = [
    "location",
    "camera",
    "microphone",
    "notifications",
    "clipboard",
    "automatic-downloads",
    "file-editing",
    "local-fonts",
    "midi",
    "popups",
    "javascript",
];

struct Pending {
    args: ICoreWebView2PermissionRequestedEventArgs,
    deferral: ICoreWebView2Deferral,
    tab: Option<u32>,
    win: String,
    private: bool,
    origin: String,
    kind: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PermissionPrompt {
    pub id: u32,
    pub tab_id: Option<u32>,
    pub origin: String,
    pub kind: String,
    pub user_initiated: bool,
}

thread_local! {
    static PENDING: RefCell<HashMap<u32, Pending>> = RefCell::new(HashMap::new());
    static NEXT: RefCell<u32> = const { RefCell::new(0) };
}

fn kind_name(kind: COREWEBVIEW2_PERMISSION_KIND) -> Option<&'static str> {
    Some(match kind {
        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE => "microphone",
        COREWEBVIEW2_PERMISSION_KIND_CAMERA => "camera",
        COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION => "location",
        COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS => "notifications",
        COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ => "clipboard",
        COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS => "automatic-downloads",
        COREWEBVIEW2_PERMISSION_KIND_FILE_READ_WRITE => "file-editing",
        COREWEBVIEW2_PERMISSION_KIND_LOCAL_FONTS => "local-fonts",
        COREWEBVIEW2_PERMISSION_KIND_MIDI_SYSTEM_EXCLUSIVE_MESSAGES => "midi",
        _ => return None,
    })
}

pub fn popups_allowed(app: &AppHandle, private: bool, origin: &str) -> bool {
    let state = app.state::<AppState>();
    match state.site_value(private, origin, "popups").as_deref() {
        Some("allow") => true,
        Some("block") => false,
        _ => !state.settings().block_popups,
    }
}

pub fn javascript_allowed(app: &AppHandle, private: bool, origin: &str) -> bool {
    origin.is_empty() || app.state::<AppState>().site_value(private, origin, "javascript").as_deref() != Some("block")
}

pub unsafe fn attach(
    app: &AppHandle,
    win: &str,
    tab: Option<u32>,
    private: bool,
    core: &ICoreWebView2,
) -> windows::core::Result<()> {
    let (a, w) = (app.clone(), win.to_string());
    let mut token = 0i64;
    core.add_PermissionRequested(
        &PermissionRequestedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            on_request(&a, &w, tab, private, args)
        })),
        &mut token,
    )?;
    Ok(())
}

unsafe fn on_request(
    app: &AppHandle,
    win: &str,
    tab: Option<u32>,
    private: bool,
    args: ICoreWebView2PermissionRequestedEventArgs,
) -> windows::core::Result<()> {
    let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
    args.PermissionKind(&mut kind)?;
    if kind == COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY {
        args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
        return Ok(());
    }
    if let Ok(a3) = args.cast::<ICoreWebView2PermissionRequestedEventArgs3>() {
        let _ = a3.SetSavesInProfile(false);
    }
    let Some(name) = kind_name(kind) else {
        args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
        return Ok(());
    };
    let uri = com::string(|p| args.Uri(p));
    let Some(origin) = origin_of(&uri) else {
        args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
        return Ok(());
    };
    match app.state::<AppState>().site_value(private, &origin, name).as_deref() {
        Some("allow") => return args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW),
        Some("block") => return args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY),
        _ => {}
    }
    if let Ok(a2) = args.cast::<ICoreWebView2PermissionRequestedEventArgs2>() {
        let _ = a2.SetHandled(true);
    }
    let user_initiated = com::flag(|b| args.IsUserInitiated(b));
    let deferral = args.GetDeferral()?;
    let id = NEXT.with(|n| {
        let mut n = n.borrow_mut();
        *n += 1;
        *n
    });
    PENDING.with(|p| {
        p.borrow_mut().insert(
            id,
            Pending { args, deferral, tab, win: win.to_string(), private, origin: origin.clone(), kind: name.into() },
        )
    });
    emit_ui(app, win, "permission-request", PermissionPrompt { id, tab_id: tab, origin, kind: name.into(), user_initiated });
    Ok(())
}

pub fn pending_for_window(win: &str) -> Vec<PermissionPrompt> {
    PENDING.with(|p| {
        p.borrow()
            .iter()
            .filter(|(_, v)| v.win == win)
            .map(|(id, v)| PermissionPrompt {
                id: *id,
                tab_id: v.tab,
                origin: v.origin.clone(),
                kind: v.kind.clone(),
                user_initiated: false,
            })
            .collect()
    })
}

pub fn resolve(app: &AppHandle, id: u32, allow: bool, remember: bool) -> Result<()> {
    let pending = PENDING.with(|p| p.borrow_mut().remove(&id)).ok_or(Error::NotFound("Permission request"))?;
    let state = if allow { COREWEBVIEW2_PERMISSION_STATE_ALLOW } else { COREWEBVIEW2_PERMISSION_STATE_DENY };
    unsafe {
        let _ = pending.args.SetState(state);
        let _ = pending.deferral.Complete();
    }
    if remember {
        app.state::<AppState>().set_site_value(
            pending.private,
            &pending.origin,
            &pending.kind,
            Some(if allow { "allow" } else { "block" }),
        )?;
    }
    emit_ui(app, &pending.win, "permission-cancel", id);
    Ok(())
}

pub fn cancel_for_tab(app: &AppHandle, tab: u32) {
    let removed: Vec<(u32, Pending)> = PENDING.with(|p| {
        let mut map = p.borrow_mut();
        let ids: Vec<u32> = map.iter().filter(|(_, v)| v.tab == Some(tab)).map(|(id, _)| *id).collect();
        ids.into_iter().filter_map(|id| map.remove(&id).map(|v| (id, v))).collect()
    });
    for (id, pending) in removed {
        unsafe {
            let _ = pending.args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY);
            let _ = pending.deferral.Complete();
        }
        emit_ui(app, &pending.win, "permission-cancel", id);
    }
}
