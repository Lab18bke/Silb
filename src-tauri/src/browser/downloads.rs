use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewBuilder, WebviewUrl};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    BytesReceivedChangedEventHandler, DownloadStartingEventHandler, NavigationCompletedEventHandler,
    StateChangedEventHandler,
};
use windows::core::{Interface, HSTRING};

use super::{com, emit_chrome, emit_ui, with};
use crate::db::DownloadRecord;
use crate::error::{Error, Result};
use crate::state::AppState;
use crate::util::{now_ms, unique_path};

struct Active {
    op: ICoreWebView2DownloadOperation,
    record: DownloadRecord,
    last_emit: Instant,
    last_saved: Instant,
}

struct Retry {
    id: i64,
    started: bool,
    helper: Option<tauri::Webview>,
}

thread_local! {
    static ACTIVE: RefCell<HashMap<i64, Active>> = RefCell::new(HashMap::new());
    static PRIVATE: RefCell<Vec<DownloadRecord>> = const { RefCell::new(Vec::new()) };
    static NEXT_PRIVATE: RefCell<i64> = const { RefCell::new(1 << 40) };
    static RETRIES: RefCell<HashMap<String, Retry>> = RefCell::new(HashMap::new());
    static ORPHANS: RefCell<Vec<tauri::Webview>> = const { RefCell::new(Vec::new()) };
}

fn downloading() -> bool {
    ACTIVE.with(|a| a.borrow().values().any(|d| d.record.state == "in_progress" || d.record.can_resume))
        || PENDING_SAVE.with(|p| !p.borrow().is_empty())
}

pub fn release_webview(wv: tauri::Webview) {
    if downloading() {
        let _ = wv.hide();
        ORPHANS.with(|o| o.borrow_mut().push(wv));
    } else {
        let _ = wv.close();
    }
}

fn close_orphans() {
    if downloading() {
        return;
    }
    for wv in ORPHANS.with(|o| std::mem::take(&mut *o.borrow_mut())) {
        let _ = wv.close();
    }
}

fn reserved_paths() -> Vec<PathBuf> {
    ACTIVE.with(|a| {
        a.borrow()
            .values()
            .filter(|d| d.record.state == "in_progress")
            .map(|d| PathBuf::from(&d.record.path))
            .collect()
    })
}

fn publish(app: &AppHandle, record: &DownloadRecord) {
    emit_chrome(app, "download-updated", record.clone(), Some(record.private));
}

fn persist(app: &AppHandle, record: &DownloadRecord) {
    if record.private {
        PRIVATE.with(|p| {
            let mut list = p.borrow_mut();
            match list.iter_mut().find(|r| r.id == record.id) {
                Some(r) => *r = record.clone(),
                None => list.insert(0, record.clone()),
            }
        });
    } else {
        let _ = app.state::<AppState>().db.download_update(record);
    }
}

pub unsafe fn attach(app: &AppHandle, win: &str, tab: Option<u32>, private: bool, core: &ICoreWebView2) -> windows::core::Result<()> {
    let core4: ICoreWebView2_4 = core.cast()?;
    let (a, w) = (app.clone(), win.to_string());
    let mut token = 0i64;
    core4.add_DownloadStarting(
        &DownloadStartingEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            if let Some(id) = tab {
                super::defer(&a, move |app| super::tabs::after_download(app, id));
            }
            on_starting(&a, &w, private, args)
        })),
        &mut token,
    )?;
    Ok(())
}

unsafe fn on_starting(
    app: &AppHandle,
    win: &str,
    private: bool,
    args: ICoreWebView2DownloadStartingEventArgs,
) -> windows::core::Result<()> {
    let op = args.DownloadOperation()?;
    let url = com::string(|p| op.Uri(p));
    let suggested = com::string(|p| args.ResultFilePath(p));
    let name = Path::new(&suggested)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "download".into());
    let retry_id = RETRIES.with(|r| {
        r.borrow_mut().get_mut(&url).map(|retry| {
            retry.started = true;
            retry.id
        })
    });
    let state = app.state::<AppState>();
    let dir = state.download_dir();
    let ask = state.settings().ask_download_location;
    args.SetHandled(true)?;

    if ask {
        let deferral = args.GetDeferral()?;
        let window = with(|b| b.win(win).map(|w| w.window.clone())).flatten();
        let key = now_ms();
        PENDING_SAVE.with(|p| p.borrow_mut().insert(key, (args, deferral, op)));
        let (a, w) = (app.clone(), win.to_string());
        tauri::async_runtime::spawn(async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_directory(&dir).set_file_name(&name).set_title("Save file");
            if let Some(window) = &window {
                dialog = dialog.set_parent(window);
            }
            let chosen = dialog.save_file().await.map(|h| h.path().to_path_buf());
            let inner = a.clone();
            let _ = a.run_on_main_thread(move || {
                let Some((args, deferral, op)) = PENDING_SAVE.with(|p| p.borrow_mut().remove(&key)) else { return };
                match chosen {
                    Some(path) => {
                        let _ = args.SetResultFilePath(&HSTRING::from(path.as_os_str()));
                        register(&inner, &w, private, op, path, retry_id);
                    }
                    None => {
                        let _ = args.SetCancel(true);
                    }
                }
                let _ = deferral.Complete();
                close_orphans();
            });
        });
        return Ok(());
    }

    let path = unique_path(&dir, &name, &reserved_paths());
    args.SetResultFilePath(&HSTRING::from(path.as_os_str()))?;
    register(app, win, private, op, path, retry_id);
    Ok(())
}

thread_local! {
    static PENDING_SAVE: RefCell<HashMap<i64, (ICoreWebView2DownloadStartingEventArgs, ICoreWebView2Deferral, ICoreWebView2DownloadOperation)>> =
        RefCell::new(HashMap::new());
}

unsafe fn register(
    app: &AppHandle,
    win: &str,
    private: bool,
    op: ICoreWebView2DownloadOperation,
    path: PathBuf,
    retry_id: Option<i64>,
) {
    let url = com::string(|p| op.Uri(p));
    let mime = com::string(|p| op.MimeType(p));
    let mut total = -1i64;
    let _ = op.TotalBytesToReceive(&mut total);
    let mut record = DownloadRecord {
        id: 0,
        url: url.clone(),
        path: path.to_string_lossy().into_owned(),
        file_name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        mime,
        total_bytes: (total > 0).then_some(total),
        received_bytes: 0,
        state: "in_progress".into(),
        error: None,
        started_at: now_ms(),
        finished_at: None,
        exists: true,
        private,
        can_resume: false,
    };
    record.id = match (private, retry_id) {
        (true, Some(id)) => id,
        (true, None) => NEXT_PRIVATE.with(|n| {
            let mut n = n.borrow_mut();
            *n += 1;
            *n
        }),
        (false, Some(id)) => {
            record.id = id;
            let _ = app.state::<AppState>().db.download_update(&record);
            id
        }
        (false, None) => match app.state::<AppState>().db.download_insert(&record) {
            Ok(id) => id,
            Err(_) => return,
        },
    };
    if private {
        persist(app, &record);
    }
    let id = record.id;
    let mut token = 0i64;
    let a = app.clone();
    let _ = op.add_BytesReceivedChanged(
        &BytesReceivedChangedEventHandler::create(Box::new(move |sender, _| {
            let Some(op) = sender else { return Ok(()) };
            let mut received = 0i64;
            op.BytesReceived(&mut received)?;
            let mut total = -1i64;
            let _ = op.TotalBytesToReceive(&mut total);
            let snapshot = ACTIVE.with(|m| {
                let mut m = m.borrow_mut();
                let d = m.get_mut(&id)?;
                d.record.received_bytes = received;
                if total > 0 {
                    d.record.total_bytes = Some(total);
                }
                let emit = d.last_emit.elapsed().as_millis() >= 200;
                let save = d.last_saved.elapsed().as_secs() >= 3;
                if emit {
                    d.last_emit = Instant::now();
                }
                if save {
                    d.last_saved = Instant::now();
                }
                Some((d.record.clone(), emit, save))
            });
            if let Some((rec, emit, save)) = snapshot {
                if save {
                    persist(&a, &rec);
                }
                if emit {
                    publish(&a, &rec);
                }
            }
            Ok(())
        })),
        &mut token,
    );
    let a = app.clone();
    let _ = op.add_StateChanged(
        &StateChangedEventHandler::create(Box::new(move |sender, _| {
            let Some(op) = sender else { return Ok(()) };
            on_state(&a, id, &op);
            Ok(())
        })),
        &mut token,
    );
    ACTIVE.with(|m| {
        m.borrow_mut().insert(
            id,
            Active { op, record: record.clone(), last_emit: Instant::now(), last_saved: Instant::now() },
        )
    });
    publish(app, &record);
    emit_ui(app, win, "download-started", record.clone());
}

fn interrupt_message(reason: COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON) -> &'static str {
    match reason {
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_NO_SPACE => "Not enough disk space",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_ACCESS_DENIED => "Permission denied — choose another folder",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_NAME_TOO_LONG => "The file name is too long",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_TOO_LARGE => "The file is too large for this drive",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_MALICIOUS
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_SECURITY_CHECK_FAILED => "Blocked by a security check",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_FILE_BLOCKED_BY_POLICY => "Blocked by system policy",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_FAILED
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_TIMEOUT
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_DISCONNECTED => "Network error",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NETWORK_SERVER_DOWN => "The server is unavailable",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_FAILED
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_BAD_CONTENT
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_NO_RANGE => "Server error",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_UNAUTHORIZED
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_FORBIDDEN
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_CERTIFICATE_PROBLEM => "The server refused the download",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_UNEXPECTED_RESPONSE
        | COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_SERVER_CROSS_ORIGIN_REDIRECT => "The file is no longer available",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_DOWNLOAD_PROCESS_CRASHED => "The download process stopped",
        COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_SHUTDOWN => "Silb was closed during the download",
        _ => "Download failed",
    }
}

fn on_state(app: &AppHandle, id: i64, op: &ICoreWebView2DownloadOperation) {
    let mut state = COREWEBVIEW2_DOWNLOAD_STATE::default();
    let mut reason = COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON::default();
    let mut received = 0i64;
    unsafe {
        let _ = op.State(&mut state);
        let _ = op.InterruptReason(&mut reason);
        let _ = op.BytesReceived(&mut received);
    }
    let can_resume = com::flag(|b| unsafe { op.CanResume(b) });
    let Some(mut rec) = ACTIVE.with(|m| m.borrow().get(&id).map(|d| d.record.clone())) else { return };
    rec.received_bytes = received;
    rec.can_resume = false;
    let done = match state {
        COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED => {
            rec.state = "completed".into();
            rec.error = None;
            rec.total_bytes = Some(received.max(rec.total_bytes.unwrap_or(0)));
            rec.finished_at = Some(now_ms());
            true
        }
        COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED => {
            rec.state = "cancelled".into();
            rec.error = None;
            rec.finished_at = Some(now_ms());
            true
        }
        COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED => {
            rec.state = "interrupted".into();
            rec.error = Some(interrupt_message(reason).into());
            rec.finished_at = Some(now_ms());
            rec.can_resume = can_resume;
            !can_resume
        }
        _ => {
            rec.state = "in_progress".into();
            rec.error = None;
            false
        }
    };
    ACTIVE.with(|m| {
        let mut m = m.borrow_mut();
        if done {
            m.remove(&id);
        } else if let Some(d) = m.get_mut(&id) {
            d.record = rec.clone();
        }
    });
    rec.exists = rec.state != "completed" || Path::new(&rec.path).exists();
    persist(app, &rec);
    publish(app, &rec);
    if done {
        let a = app.clone();
        super::defer(&a, |_| close_orphans());
    }
}

fn live(mut rec: DownloadRecord) -> DownloadRecord {
    ACTIVE.with(|m| {
        if let Some(d) = m.borrow().get(&rec.id) {
            rec.received_bytes = d.record.received_bytes;
            rec.total_bytes = d.record.total_bytes;
            rec.state = d.record.state.clone();
            rec.can_resume = d.record.can_resume;
            rec.error = d.record.error.clone();
        } else if rec.state == "in_progress" {
            rec.state = "interrupted".into();
            rec.error = Some("Download interrupted".into());
        }
    });
    rec
}

pub fn list(app: &AppHandle, private: bool) -> Result<Vec<DownloadRecord>> {
    if private {
        return Ok(PRIVATE.with(|p| {
            p.borrow()
                .iter()
                .cloned()
                .map(|mut r| {
                    r.exists = r.state != "completed" || Path::new(&r.path).exists();
                    live(r)
                })
                .collect()
        }));
    }
    Ok(app.state::<AppState>().db.downloads_list()?.into_iter().map(live).collect())
}

fn record(app: &AppHandle, id: i64) -> Result<DownloadRecord> {
    if let Some(r) = ACTIVE.with(|m| m.borrow().get(&id).map(|d| d.record.clone())) {
        return Ok(r);
    }
    if let Some(r) = PRIVATE.with(|p| p.borrow().iter().find(|r| r.id == id).cloned()) {
        return Ok(r);
    }
    app.state::<AppState>().db.download_get(id).map_err(|_| Error::NotFound("Download"))
}

pub fn cancel(id: i64) -> Result<()> {
    let op = ACTIVE.with(|m| m.borrow().get(&id).map(|d| d.op.clone())).ok_or(Error::NotFound("Download"))?;
    unsafe { op.Cancel()? };
    Ok(())
}

pub fn retry(app: &AppHandle, win: &str, id: i64) -> Result<()> {
    let resumable = ACTIVE.with(|m| m.borrow().get(&id).filter(|d| d.record.can_resume).map(|d| d.op.clone()));
    if let Some(op) = resumable {
        unsafe { op.Resume()? };
        return Ok(());
    }
    let rec = record(app, id)?;
    if !crate::util::is_web_url(&rec.url) {
        return Err(Error::invalid("This download can't be retried because its source is no longer available."));
    }
    let (window, private) = with(|b| b.win(win).map(|w| (w.window.clone(), w.private)))
        .flatten()
        .ok_or(Error::NotFound("Window"))?;
    let label = format!("{win}-dl{}", now_ms());
    let a = app.clone();
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(rec.url.parse().map_err(|_| Error::invalid("Invalid URL"))?))
        .with_environment(com::env())
        .incognito(private)
        .disable_drag_drop_handler()
        .on_navigation(|u| !super::webview::is_blocked_navigation(u.as_str()));
    RETRIES.with(|r| r.borrow_mut().insert(rec.url.clone(), Retry { id, started: false, helper: None }));
    let helper = window.add_child(builder, LogicalPosition::new(-10.0, -10.0), LogicalSize::new(1.0, 1.0))?;
    let _ = helper.hide();
    RETRIES.with(|r| {
        if let Some(retry) = r.borrow_mut().get_mut(&rec.url) {
            retry.helper = Some(helper.clone());
        }
    });
    let (w, url) = (win.to_string(), rec.url.clone());
    helper.with_webview(move |pw| unsafe {
        let Ok(core) = pw.controller().CoreWebView2() else { return };
        let _ = attach(&a, &w, None, private, &core);
        let a2 = a.clone();
        let mut token = 0i64;
        let _ = core.add_NavigationCompleted(
            &NavigationCompletedEventHandler::create(Box::new(move |_, _| {
                let url = url.clone();
                let a3 = a2.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    let a4 = a3.clone();
                    let _ = a3.run_on_main_thread(move || finish_retry(&a4, &url));
                });
                Ok(())
            })),
            &mut token,
        );
    })?;
    Ok(())
}

fn finish_retry(app: &AppHandle, url: &str) {
    let Some(retry) = RETRIES.with(|r| r.borrow_mut().remove(url)) else { return };
    if let Some(h) = retry.helper {
        let _ = h.close();
    }
    if !retry.started {
        if let Ok(mut rec) = record(app, retry.id) {
            rec.state = "interrupted".into();
            rec.error = Some("The server didn't return a file. The link may have expired.".into());
            persist(app, &rec);
            publish(app, &rec);
        }
    }
}

pub fn open(app: &AppHandle, id: i64) -> Result<()> {
    let rec = record(app, id)?;
    if !Path::new(&rec.path).exists() {
        return Err(Error::invalid("The file was moved or deleted."));
    }
    crate::shell::open_path(Path::new(&rec.path))
}

pub fn reveal(app: &AppHandle, id: i64) -> Result<()> {
    let rec = record(app, id)?;
    crate::shell::reveal_path(Path::new(&rec.path))
}

pub fn remove(app: &AppHandle, id: i64) -> Result<()> {
    if ACTIVE.with(|m| m.borrow().get(&id).is_some_and(|d| d.record.state == "in_progress")) {
        return Err(Error::invalid("Cancel the download before removing it."));
    }
    ACTIVE.with(|m| m.borrow_mut().remove(&id));
    let was_private = PRIVATE.with(|p| {
        let mut list = p.borrow_mut();
        let before = list.len();
        list.retain(|r| r.id != id);
        before != list.len()
    });
    if !was_private {
        app.state::<AppState>().db.download_delete(id)?;
    }
    Ok(())
}

pub fn clear(app: &AppHandle, private: bool, from: i64, to: i64) -> Result<()> {
    let active: Vec<i64> = ACTIVE.with(|m| {
        let mut m = m.borrow_mut();
        m.retain(|_, d| d.record.state == "in_progress");
        m.keys().copied().collect()
    });
    if private {
        PRIVATE.with(|p| p.borrow_mut().retain(|r| active.contains(&r.id) || r.started_at < from || r.started_at > to));
    } else {
        app.state::<AppState>().db.downloads_clear_range(from, to)?;
    }
    emit_chrome(app, "downloads-cleared", (), Some(private));
    Ok(())
}

pub fn end_private_session(app: &AppHandle) {
    PRIVATE.with(|p| p.borrow_mut().clear());
    emit_chrome(app, "downloads-cleared", (), Some(true));
}

