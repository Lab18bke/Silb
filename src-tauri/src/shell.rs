use std::path::Path;

use windows::core::{w, HSTRING};
use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::error::{Error, Result};

pub fn open_path(path: &Path) -> Result<()> {
    let result = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(path.as_os_str()), None, None, SW_SHOWNORMAL) };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(Error::invalid("Windows couldn't open this file."))
    }
}

pub fn reveal_path(path: &Path) -> Result<()> {
    if !path.exists() {
        let dir = path.parent().filter(|d| d.is_dir()).ok_or_else(|| Error::invalid("The folder no longer exists."))?;
        return open_path(dir);
    }
    unsafe {
        let pidl = ILCreateFromPathW(&HSTRING::from(path.as_os_str()));
        if pidl.is_null() {
            return Err(Error::invalid("Couldn't locate the file."));
        }
        let result = SHOpenFolderAndSelectItems(pidl, None, 0);
        ILFree(Some(pidl));
        result.map_err(Error::from)
    }
}
