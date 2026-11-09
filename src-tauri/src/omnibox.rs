use std::net::Ipv4Addr;

use url::Url;

pub const INTERNAL_PAGES: [&str; 6] = ["newtab", "history", "bookmarks", "downloads", "settings", "private"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Web(String),
    Internal(String),
}

const NOT_TLDS: [&str; 34] = [
    "js", "ts", "tsx", "jsx", "json", "txt", "exe", "dll", "html", "htm", "css", "cpp", "java", "png",
    "jpg", "jpeg", "gif", "pdf", "docx", "xlsx", "pptx", "mp3", "mp4", "csv", "log", "ini", "cfg", "yaml",
    "yml", "toml", "lock", "svelte", "vue", "bat",
];

pub fn internal_page(url: &str) -> Option<String> {
    let rest = url.strip_prefix("silb://")?;
    let page = rest.trim_end_matches('/');
    let base = page.split('/').next().unwrap_or("");
    let valid_tail = page[base.len()..].chars().all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '-');
    (INTERNAL_PAGES.contains(&base) && valid_tail).then(|| page.to_string())
}

pub fn search_url(template: &str, query: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    template.replacen("%s", &encoded, 1)
}

pub fn resolve(input: &str, search_template: &str) -> Option<Target> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some(q) = input.strip_prefix('?') {
        let q = q.trim();
        return (!q.is_empty()).then(|| Target::Web(search_url(search_template, q)));
    }
    if let Some(page) = internal_page(&input.to_ascii_lowercase()) {
        return Some(Target::Internal(page));
    }
    if let Some(url) = explicit_url(input) {
        return Some(Target::Web(url));
    }
    if !input.contains(char::is_whitespace) {
        if let Some(url) = implicit_url(input) {
            return Some(Target::Web(url));
        }
    }
    Some(Target::Web(search_url(search_template, input)))
}

fn explicit_url(input: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    let allowed = ["http://", "https://", "file:///", "about:blank", "view-source:http"];
    if !allowed.iter().any(|p| lower.starts_with(p)) {
        return None;
    }
    let url = Url::parse(input).ok()?;
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_none_or(str::is_empty) {
        return None;
    }
    Some(url.to_string())
}

fn implicit_url(input: &str) -> Option<String> {
    let authority_end = input.find(['/', '?', '#']).unwrap_or(input.len());
    let authority = &input[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let (host, port) = split_port(authority)?;
    let host_lower = host.to_ascii_lowercase();

    let scheme = if host_lower == "localhost" || host_lower.ends_with(".localhost") {
        "http"
    } else if host.parse::<Ipv4Addr>().is_ok() || (host.starts_with('[') && host.ends_with(']')) {
        "http"
    } else if looks_like_domain(&host_lower) {
        if port.is_some() { "http" } else { "https" }
    } else {
        return None;
    };
    let url = Url::parse(&format!("{scheme}://{input}")).ok()?;
    url.host_str()?;
    Some(url.to_string())
}

fn split_port(authority: &str) -> Option<(&str, Option<u16>)> {
    if authority.starts_with('[') {
        let close = authority.find(']')?;
        let rest = &authority[close + 1..];
        let port = match rest.strip_prefix(':') {
            Some(p) => Some(p.parse().ok()?),
            None if rest.is_empty() => None,
            None => return None,
        };
        return Some((&authority[..=close], port));
    }
    match authority.rsplit_once(':') {
        Some((h, p)) => Some((h, Some(p.parse().ok()?))),
        None => Some((authority, None)),
    }
}

fn looks_like_domain(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let label_ok = |l: &&str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_alphanumeric() || c == '-')
    };
    if !labels.iter().all(label_ok) {
        return false;
    }
    let tld = labels[labels.len() - 1];
    let tld_ok = tld.starts_with("xn--") || (tld.chars().count() >= 2 && tld.chars().all(char::is_alphabetic));
    tld_ok && !NOT_TLDS.contains(&tld)
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: &str = "https://www.google.com/search?q=%s";

    fn web(input: &str) -> String {
        match resolve(input, G) {
            Some(Target::Web(u)) => u,
            other => panic!("{input:?} resolved to {other:?}"),
        }
    }

    #[test]
    fn urls() {
        assert_eq!(web("example.com"), "https://example.com/");
        assert_eq!(web("docs.rs/tauri"), "https://docs.rs/tauri");
        assert_eq!(web("https://example.com/a b"), "https://example.com/a%20b");
        assert_eq!(web("http://example.com"), "http://example.com/");
        assert_eq!(web("localhost:3000/x"), "http://localhost:3000/x");
        assert_eq!(web("192.168.1.1"), "http://192.168.1.1/");
        assert_eq!(web("[::1]:8080"), "http://[::1]:8080/");
        assert_eq!(web("example.com:8080"), "http://example.com:8080/");
        assert_eq!(web("bücher.de"), "https://xn--bcher-kva.de/");
        assert_eq!(web("about:blank"), "about:blank");
        assert_eq!(web("file:///C:/x.txt"), "file:///C:/x.txt");
    }

    #[test]
    fn searches() {
        assert_eq!(web("rust lang"), "https://www.google.com/search?q=rust+lang");
        assert_eq!(web("node.js"), "https://www.google.com/search?q=node.js");
        assert_eq!(web("hello"), "https://www.google.com/search?q=hello");
        assert_eq!(web("?example.com"), "https://www.google.com/search?q=example.com");
        assert_eq!(web("javascript:alert(1)"), "https://www.google.com/search?q=javascript%3Aalert%281%29");
        assert_eq!(web("http://"), "https://www.google.com/search?q=http%3A%2F%2F");
        assert_eq!(web("a&b=c"), "https://www.google.com/search?q=a%26b%3Dc");
        assert_eq!(web("user@example.com"), "https://www.google.com/search?q=user%40example.com");
        assert_eq!(web("3.14"), "https://www.google.com/search?q=3.14");
    }

    #[test]
    fn internal() {
        assert_eq!(resolve("silb://settings", G), Some(Target::Internal("settings".into())));
        assert_eq!(resolve("SILB://Settings/privacy", G), Some(Target::Internal("settings/privacy".into())));
        assert_eq!(resolve("silb://evil<x>", G), Some(Target::Web(search_url(G, "silb://evil<x>"))));
        assert_eq!(resolve("   ", G), None);
    }
}
