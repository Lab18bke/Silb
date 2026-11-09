use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use url::Url;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn host_of(url: &str) -> Option<String> {
    let u = Url::parse(url).ok()?;
    let host = u.host_str()?.to_ascii_lowercase();
    Some(host.strip_prefix("www.").map(str::to_string).unwrap_or(host))
}

pub fn origin_of(url: &str) -> Option<String> {
    let u = Url::parse(url).ok()?;
    match u.origin() {
        url::Origin::Tuple(..) => Some(u.origin().ascii_serialization()),
        url::Origin::Opaque(_) => None,
    }
}

pub fn is_web_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    let stem = cleaned.split('.').next().unwrap_or("").to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if cleaned.is_empty() {
        "download".into()
    } else if RESERVED.contains(&stem.as_str()) {
        format!("_{cleaned}")
    } else {
        cleaned
    }
}

pub fn unique_path(dir: &Path, name: &str, reserved: &[PathBuf]) -> PathBuf {
    let name = sanitize_file_name(name);
    let candidate = dir.join(&name);
    let taken = |p: &Path| p.exists() || reserved.iter().any(|r| r.as_path() == p);
    if !taken(&candidate) {
        return candidate;
    }
    let (stem, ext) = split_extension(&name);
    for i in 1..10_000 {
        let p = dir.join(format!("{stem} ({i}){ext}"));
        if !taken(&p) {
            return p;
        }
    }
    dir.join(format!("{stem} ({}){ext}", now_ms()))
}

fn split_extension(name: &str) -> (&str, &str) {
    let lower = name.to_ascii_lowercase();
    for compound in [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"] {
        if lower.ends_with(compound) && name.len() > compound.len() {
            let at = name.len() - compound.len();
            return (&name[..at], &name[at..]);
        }
    }
    match name.rfind('.') {
        Some(at) if at > 0 => (&name[..at], &name[at..]),
        _ => (name, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_strips_www() {
        assert_eq!(host_of("https://www.Example.com/a").as_deref(), Some("example.com"));
        assert_eq!(host_of("about:blank"), None);
    }

    #[test]
    fn origin() {
        assert_eq!(origin_of("https://a.b.com:8443/x?y").as_deref(), Some("https://a.b.com:8443"));
        assert_eq!(origin_of("data:text/plain,hi"), None);
    }

    #[test]
    fn sanitizes() {
        assert_eq!(sanitize_file_name("a/b:c?.txt"), "a_b_c_.txt");
        assert_eq!(sanitize_file_name("CON.txt"), "_CON.txt");
        assert_eq!(sanitize_file_name("  "), "download");
    }

    #[test]
    fn unique_paths() {
        let dir = std::env::temp_dir().join(format!("silb-test-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("file.tar.gz"), b"x").unwrap();
        let p = unique_path(&dir, "file.tar.gz", &[]);
        assert_eq!(p.file_name().unwrap(), "file (1).tar.gz");
        let reserved = vec![dir.join("file (1).tar.gz")];
        let p = unique_path(&dir, "file.tar.gz", &reserved);
        assert_eq!(p.file_name().unwrap(), "file (2).tar.gz");
        assert_eq!(unique_path(&dir, "new.bin", &[]).file_name().unwrap(), "new.bin");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
