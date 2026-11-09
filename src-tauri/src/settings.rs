use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Startup {
    NewTab,
    Restore,
    Homepage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NewTabPage {
    Start,
    Blank,
    Homepage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TrackingPrevention {
    Off,
    Basic,
    Balanced,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomEngine {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub startup: Startup,
    pub homepage: String,
    pub new_tab_page: NewTabPage,
    pub website_language: String,

    pub search_engine: String,
    pub custom_engines: Vec<CustomEngine>,

    pub theme: Theme,
    pub accent: String,
    pub density: String,
    pub show_bookmarks_bar: bool,
    pub show_top_sites: bool,

    pub open_links_in_new_tab: bool,
    pub switch_to_new_tabs: bool,
    pub confirm_close_multiple: bool,
    pub suspend_background_tabs: bool,

    pub download_dir: String,
    pub ask_download_location: bool,
    pub show_downloads_popup: bool,

    pub tracking_prevention: TrackingPrevention,
    pub block_popups: bool,

    pub reader_font_size: u32,
    pub reader_width: u32,
    pub reader_theme: String,
    pub reader_font: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            startup: Startup::NewTab,
            homepage: "https://www.google.com/".into(),
            new_tab_page: NewTabPage::Start,
            website_language: String::new(),
            search_engine: "google".into(),
            custom_engines: Vec::new(),
            theme: Theme::System,
            accent: "juniper".into(),
            density: "comfortable".into(),
            show_bookmarks_bar: false,
            show_top_sites: true,
            open_links_in_new_tab: true,
            switch_to_new_tabs: false,
            confirm_close_multiple: true,
            suspend_background_tabs: true,
            download_dir: String::new(),
            ask_download_location: false,
            show_downloads_popup: true,
            tracking_prevention: TrackingPrevention::Balanced,
            block_popups: true,
            reader_font_size: 19,
            reader_width: 680,
            reader_theme: "auto".into(),
            reader_font: "serif".into(),
        }
    }
}

pub const BUILTIN_ENGINES: [(&str, &str, &str); 4] = [
    ("google", "Google", "https://www.google.com/search?q=%s"),
    ("bing", "Bing", "https://www.bing.com/search?q=%s"),
    ("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q=%s"),
    ("brave", "Brave Search", "https://search.brave.com/search?q=%s"),
];

pub const ACCENTS: [&str; 5] = ["juniper", "ink", "clay", "plum", "graphite"];
pub const READER_THEMES: [&str; 4] = ["auto", "light", "sepia", "dark"];

impl Settings {
    pub fn engine(&self) -> (String, String) {
        if let Some(c) = self.custom_engines.iter().find(|c| c.id == self.search_engine) {
            return (c.name.clone(), c.url.clone());
        }
        let (_, name, url) = BUILTIN_ENGINES
            .iter()
            .find(|(id, _, _)| *id == self.search_engine)
            .unwrap_or(&BUILTIN_ENGINES[0]);
        (name.to_string(), url.to_string())
    }

    pub fn validated(mut self) -> crate::error::Result<Self> {
        use crate::error::Error;
        self.homepage = self.homepage.trim().to_string();
        if !self.homepage.is_empty() && !crate::util::is_web_url(&self.homepage) {
            return Err(Error::invalid("The homepage must be an http:// or https:// address."));
        }
        for engine in &self.custom_engines {
            let ok = crate::util::is_web_url(&engine.url) && engine.url.contains("%s");
            if !ok || engine.name.trim().is_empty() || engine.id.is_empty() {
                return Err(Error::invalid(format!(
                    "Search engine \"{}\" needs a name and an http(s) URL containing %s.",
                    engine.name
                )));
            }
        }
        let known = BUILTIN_ENGINES.iter().any(|(id, _, _)| *id == self.search_engine)
            || self.custom_engines.iter().any(|c| c.id == self.search_engine);
        if !known {
            self.search_engine = "google".into();
        }
        if !ACCENTS.contains(&self.accent.as_str()) {
            self.accent = "juniper".into();
        }
        if self.density != "compact" {
            self.density = "comfortable".into();
        }
        if !READER_THEMES.contains(&self.reader_theme.as_str()) {
            self.reader_theme = "auto".into();
        }
        if self.reader_font != "sans" {
            self.reader_font = "serif".into();
        }
        self.reader_font_size = self.reader_font_size.clamp(14, 30);
        self.reader_width = self.reader_width.clamp(520, 960);
        let lang_ok = self.website_language.is_empty()
            || (self.website_language.len() <= 12
                && self.website_language.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
        if !lang_ok {
            self.website_language.clear();
        }
        Ok(self)
    }
}
