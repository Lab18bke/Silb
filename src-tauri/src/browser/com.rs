use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{
    take_pwstr, CallDevToolsProtocolMethodCompletedHandler, CoreWebView2EnvironmentOptions,
    CreateCoreWebView2EnvironmentCompletedHandler, ExecuteScriptCompletedHandler,
};
use windows::core::{Interface, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{E_POINTER, HWND};
use windows::Win32::System::Com::{CoInitializeEx, IStream, COINIT_APARTMENTTHREADED, STREAM_SEEK_SET};

use crate::error::{Error, Result};

pub struct SendEnv(pub ICoreWebView2Environment);
unsafe impl Send for SendEnv {}
unsafe impl Sync for SendEnv {}

static ENV: OnceLock<SendEnv> = OnceLock::new();

pub fn env() -> ICoreWebView2Environment {
    ENV.get().expect("WebView2 environment not initialised").0.clone()
}

pub fn create_environment(data_dir: &Path, language: &str) -> Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let options = CoreWebView2EnvironmentOptions::default();
    unsafe {
        options.set_additional_browser_arguments("--disable-features=msWebOOUI,msPdfOOUI".into());
        options.set_enable_tracking_prevention(true);
        options.set_are_browser_extensions_enabled(false);
        options.set_scroll_bar_style(COREWEBVIEW2_SCROLLBAR_STYLE_FLUENT_OVERLAY);
        if !language.is_empty() {
            options.set_language(language.to_string());
        }
    }
    std::fs::create_dir_all(data_dir)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(move |hr, env| {
        let _ = tx.send(hr.and_then(|_| env.ok_or_else(|| windows::core::Error::from(E_POINTER))));
        Ok(())
    }));
    unsafe {
        CreateCoreWebView2EnvironmentWithOptions(
            PCWSTR::null(),
            &HSTRING::from(data_dir.as_os_str()),
            &ICoreWebView2EnvironmentOptions::from(options),
            &handler,
        )?;
    }
    let env = webview2_com::wait_with_pump(rx)
        .map_err(|e| Error::invalid(format!("WebView2 environment: {e:?}")))??;
    let _ = ENV.set(SendEnv(env));
    Ok(())
}

pub fn browser_version() -> String {
    unsafe {
        let mut p = PWSTR::null();
        match env().BrowserVersionString(&mut p) {
            Ok(()) => take_pwstr(p),
            Err(_) => String::new(),
        }
    }
}

pub fn string(f: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>) -> String {
    let mut p = PWSTR::null();
    match f(&mut p) {
        Ok(()) if !p.is_null() => take_pwstr(p),
        _ => String::new(),
    }
}

pub fn flag(f: impl FnOnce(*mut BOOL) -> windows::core::Result<()>) -> bool {
    let mut b = BOOL::default();
    f(&mut b).is_ok() && b.as_bool()
}

pub fn read_stream(stream: &IStream, max: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    unsafe {
        let _ = stream.Seek(0, STREAM_SEEK_SET, None);
        loop {
            let mut read = 0u32;
            let hr = stream.Read(buf.as_mut_ptr().cast(), buf.len() as u32, Some(&mut read));
            if hr.is_err() || read == 0 {
                break;
            }
            out.extend_from_slice(&buf[..read as usize]);
            if out.len() > max {
                return Vec::new();
            }
        }
    }
    out
}

pub fn exec_script(core: &ICoreWebView2, script: &str, done: impl FnOnce(Option<String>) + 'static) {
    let handler = ExecuteScriptCompletedHandler::create(Box::new(move |hr, json| {
        done(hr.ok().map(|_| json));
        Ok(())
    }));
    unsafe {
        let _ = core.ExecuteScript(&HSTRING::from(script), &handler);
    }
}

pub fn devtools_call(core: &ICoreWebView2, method: &str, params: &str) {
    let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_, _| Ok(())));
    unsafe {
        let _ = core.CallDevToolsProtocolMethod(&HSTRING::from(method), &HSTRING::from(params), &handler);
    }
}

pub fn container_hwnd(controller: &ICoreWebView2Controller) -> isize {
    let mut hwnd = HWND::default();
    unsafe {
        let _ = controller.ParentWindow(&mut hwnd);
    }
    hwnd.0 as isize
}

pub fn profile(core: &ICoreWebView2) -> Option<ICoreWebView2Profile> {
    unsafe { core.cast::<ICoreWebView2_13>().ok()?.Profile().ok() }
}

#[derive(Clone)]
pub struct ViewCom {
    pub controller: ICoreWebView2Controller,
    pub core: ICoreWebView2,
}

thread_local! {
    static TABS: RefCell<HashMap<u32, ViewCom>> = RefCell::new(HashMap::new());
    static UIS: RefCell<HashMap<String, ViewCom>> = RefCell::new(HashMap::new());
}

pub fn set_tab(id: u32, com: ViewCom) {
    TABS.with(|t| t.borrow_mut().insert(id, com));
}

pub fn remove_tab(id: u32) {
    TABS.with(|t| t.borrow_mut().remove(&id));
}

pub fn tab(id: u32) -> Option<ViewCom> {
    TABS.with(|t| t.borrow().get(&id).cloned())
}

pub fn tab_cores() -> Vec<(u32, ICoreWebView2)> {
    TABS.with(|t| t.borrow().iter().map(|(id, c)| (*id, c.core.clone())).collect())
}

pub fn set_ui(label: &str, com: ViewCom) {
    UIS.with(|t| t.borrow_mut().insert(label.to_string(), com));
}

pub fn remove_ui(label: &str) {
    UIS.with(|t| t.borrow_mut().remove(label));
}

pub fn ui_cores() -> Vec<ICoreWebView2> {
    UIS.with(|t| t.borrow().values().map(|c| c.core.clone()).collect())
}
