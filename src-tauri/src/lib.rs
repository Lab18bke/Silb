mod browser;
mod commands;
mod db;
mod error;
mod omnibox;
mod settings;
mod shell;
mod state;
mod util;

use tauri::Manager;

use crate::db::Db;
use crate::state::AppState;

pub fn run() {
    let test_mode = browser::window::hidden_mode();
    let mut builder = tauri::Builder::default();
    if !test_mode {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let urls: Vec<String> = args.iter().skip(1).filter(|a| crate::util::is_web_url(a)).cloned().collect();
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                let specs = urls.iter().map(|u| browser::tabs::TabSpec::web(u.clone())).collect();
                let _ = browser::window::create_window(&a, false, specs, 0);
            });
        }));
    }
    builder
        .setup(move |app| {
            browser::mark_main_thread();
            let handle = app.handle().clone();
            let base_dir = app.path().app_local_data_dir()?;
            let data_dir = if test_mode { base_dir.join("test-profile") } else { base_dir };
            let db = Db::open(&data_dir.join("silb.db"))?;
            let downloads = app.path().download_dir().unwrap_or_else(|_| data_dir.join("Downloads"));
            let state = AppState::load(db, downloads)?;
            let language = state.language_at_start.clone();
            app.manage(state);
            if let Err(e) = browser::com::create_environment(&data_dir.join("EBWebView"), &language) {
                browser::session::fatal(&format!("Silb couldn't start the Microsoft Edge WebView2 runtime.\n\n{e}"));
            }
            let urls: Vec<String> = std::env::args().skip(1).filter(|a| crate::util::is_web_url(a)).collect();
            if urls.is_empty() {
                browser::session::restore(&handle);
            } else {
                let specs = urls.into_iter().map(browser::tabs::TabSpec::web).collect();
                browser::session::open_first_window(&handle, specs);
            }
            browser::session::spawn_saver(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ui_ready,
            commands::window_snapshot,
            commands::window_control,
            commands::window_new,
            commands::set_chrome_top,
            commands::set_content_visible,
            commands::focus_content,
            commands::tab_new,
            commands::tab_close,
            commands::tab_close_others,
            commands::tab_activate,
            commands::tab_move,
            commands::tab_back,
            commands::tab_forward,
            commands::tab_reload,
            commands::tab_stop,
            commands::tab_mute,
            commands::tab_reader,
            commands::tab_devtools,
            commands::tab_print,
            commands::tab_navigate,
            commands::tab_zoom,
            commands::tab_reopen,
            commands::tab_duplicate,
            commands::tab_open_popup,
            commands::find_start,
            commands::find_step,
            commands::find_stop,
            commands::overlay_show,
            commands::overlay_hide,
            commands::overlay_event,
            commands::overlay_resize,
            commands::suggest,
            commands::history_query,
            commands::history_delete,
            commands::top_sites,
            commands::bookmarks_all,
            commands::bookmark_create,
            commands::bookmark_update,
            commands::bookmark_move,
            commands::bookmark_delete,
            commands::bookmarks_for_url,
            commands::bookmarks_search,
            commands::downloads_list,
            commands::download_cancel,
            commands::download_retry,
            commands::download_open,
            commands::download_reveal,
            commands::download_remove,
            commands::downloads_clear,
            commands::settings_get,
            commands::settings_set,
            commands::pick_download_dir,
            commands::clear_browsing_data,
            commands::site_data_list,
            commands::site_data_clear,
            commands::site_settings_list,
            commands::site_setting_set,
            commands::site_info,
            commands::permission_resolve,
            commands::permissions_pending,
            commands::app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Silb Browser");
}
