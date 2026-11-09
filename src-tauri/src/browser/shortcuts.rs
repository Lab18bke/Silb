use tauri::AppHandle;
use webview2_com::AcceleratorKeyPressedEventHandler;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};

use super::{emit_ui, with};

#[derive(Clone, Copy)]
pub struct Keys {
    pub vk: u32,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub repeat: bool,
}

pub fn action(k: Keys, in_page: bool, page_loading: bool) -> Option<&'static str> {
    let Keys { vk, ctrl, shift, alt, repeat } = k;
    let letter = |c: u8| vk == c as u32;
    let once = |a: &'static str| (!repeat).then_some(a);
    match (ctrl, shift, alt) {
        (true, false, false) => match vk {
            _ if letter(b'T') => once("new-tab"),
            _ if letter(b'W') => once("close-tab"),
            0x73 => once("close-tab"),
            0x09 | 0x22 => Some("next-tab"),
            0x21 => Some("prev-tab"),
            _ if letter(b'L') => once("focus-address"),
            _ if letter(b'R') => once("reload"),
            0x74 => once("hard-reload"),
            _ if letter(b'D') => once("bookmark"),
            _ if letter(b'H') => once("history"),
            _ if letter(b'J') => once("downloads"),
            _ if letter(b'N') => once("new-window"),
            _ if letter(b'F') => once("find"),
            _ if letter(b'G') => Some("find-next"),
            _ if letter(b'U') => once("view-source"),
            _ if letter(b'P') && !in_page => once("print"),
            0xBB | 0x6B => Some("zoom-in"),
            0xBD | 0x6D => Some("zoom-out"),
            0x30 | 0x60 => once("zoom-reset"),
            0xBC => once("settings"),
            0x31..=0x38 => once(["tab-1", "tab-2", "tab-3", "tab-4", "tab-5", "tab-6", "tab-7", "tab-8"][(vk - 0x31) as usize]),
            0x39 => once("tab-last"),
            _ => None,
        },
        (true, true, false) => match vk {
            _ if letter(b'T') => once("reopen-tab"),
            0x09 => Some("prev-tab"),
            _ if letter(b'N') => once("new-private-window"),
            _ if letter(b'R') => once("hard-reload"),
            _ if letter(b'B') => once("toggle-bookmarks-bar"),
            _ if letter(b'O') => once("bookmarks"),
            _ if letter(b'G') => Some("find-prev"),
            _ if letter(b'I') && !in_page => once("devtools"),
            0x2E => once("clear-data"),
            0xBB => Some("zoom-in"),
            _ => None,
        },
        (false, false, true) => match vk {
            0x25 => Some("back"),
            0x27 => Some("forward"),
            _ if letter(b'D') => once("focus-address"),
            0x24 => once("home"),
            _ => None,
        },
        (false, false, false) => match vk {
            0x74 => once("reload"),
            0x75 => once("focus-address"),
            0x72 => Some("find-next"),
            0x7A => once("fullscreen"),
            0x7B if !in_page => once("devtools"),
            0x1B if in_page && page_loading => once("stop"),
            _ => None,
        },
        (false, true, false) => match vk {
            0x74 => once("hard-reload"),
            0x72 => Some("find-prev"),
            _ => None,
        },
        _ => None,
    }
}

pub unsafe fn attach(
    app: &AppHandle,
    win: &str,
    tab: Option<u32>,
    controller: &ICoreWebView2Controller,
) -> windows::core::Result<()> {
    let (a, w) = (app.clone(), win.to_string());
    let mut token = 0i64;
    controller.add_AcceleratorKeyPressed(
        &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
            args.KeyEventKind(&mut kind)?;
            let system = kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN;
            if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && !system {
                return Ok(());
            }
            let mut vk = 0u32;
            args.VirtualKey(&mut vk)?;
            let mut status = COREWEBVIEW2_PHYSICAL_KEY_STATUS::default();
            args.PhysicalKeyStatus(&mut status)?;
            let keys = Keys {
                vk,
                ctrl: GetKeyState(VK_CONTROL.0 as i32) < 0,
                shift: GetKeyState(VK_SHIFT.0 as i32) < 0,
                alt: system || GetKeyState(VK_MENU.0 as i32) < 0,
                repeat: status.WasKeyDown.as_bool(),
            };
            let loading = tab
                .and_then(|id| with(|b| b.window_of_tab(id).and_then(|w| w.tab(id)).map(|t| t.info.loading)).flatten())
                .unwrap_or(false);
            let matched = action(keys, tab.is_some(), loading);
            let swallow = tab.is_none() && keys.ctrl && !keys.alt && matches!(vk, 0x50 | 0x53 | 0x55 | 0x47 | 0x4F)
                || tab.is_none() && matches!(vk, 0x7E | 0x76);
            if let Some(name) = matched {
                args.SetHandled(true)?;
                emit_ui(&a, &w, "shortcut", name);
            } else if swallow {
                args.SetHandled(true)?;
            }
            Ok(())
        })),
        &mut token,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(vk: u32, ctrl: bool, shift: bool, alt: bool) -> Keys {
        Keys { vk, ctrl, shift, alt, repeat: false }
    }

    #[test]
    fn mapping() {
        assert_eq!(action(k(b'T' as u32, true, false, false), true, false), Some("new-tab"));
        assert_eq!(action(k(b'T' as u32, true, true, false), true, false), Some("reopen-tab"));
        assert_eq!(action(k(0x09, true, true, false), true, false), Some("prev-tab"));
        assert_eq!(action(k(0x25, false, false, true), true, false), Some("back"));
        assert_eq!(action(k(0x1B, false, false, false), true, false), None);
        assert_eq!(action(k(0x1B, false, false, false), true, true), Some("stop"));
        assert_eq!(action(k(0x1B, false, false, false), false, true), None);
        assert_eq!(action(k(b'C' as u32, true, false, false), true, false), None);
        assert_eq!(action(k(b'P' as u32, true, false, false), true, false), None);
        assert_eq!(action(k(0x7B, false, false, false), true, false), None);
        let mut repeat = k(b'T' as u32, true, false, false);
        repeat.repeat = true;
        assert_eq!(action(repeat, true, false), None);
        repeat.vk = 0xBB;
        assert_eq!(action(repeat, true, false), Some("zoom-in"));
    }
}
