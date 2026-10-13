//! `gameasset://` serves NCSOFT's game font and icons from a local cache,
//! fetching each from the public CDN the first time it is asked for.
//!
//! The files are never shipped with the app. Serving them through our own
//! scheme also gives the font the CORS header the CDN does not send, and keeps
//! icons available offline once seen.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

pub const SCHEME: &str = "gameasset";

const CDN: &str = "https://assets.playnccdn.com";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const USER_AGENT: &str = concat!("Aether/", env!("CARGO_PKG_VERSION"));

/// The only things the scheme will fetch.
#[derive(Debug, PartialEq)]
enum Asset {
    Font(&'static str),
    Icon(String),
    Emblem(&'static str),
}

impl Asset {
    /// `font/Medium`, `icon/ICON_GL_SKILL_001.png`, `emblem/cleric.png`.
    fn parse(path: &str) -> Option<Self> {
        let (kind, name) = path.trim_start_matches('/').split_once('/')?;
        match kind {
            "font" => ["Bold", "Medium", "Light"]
                .into_iter()
                .find(|w| *w == name)
                .map(Asset::Font),
            "icon" => {
                let stem = name.strip_suffix(".png")?;
                let valid = (1..=96).contains(&stem.len())
                    && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                valid.then(|| Asset::Icon(stem.to_string()))
            }
            "emblem" => {
                let class = name.strip_suffix(".png")?;
                // The CDN calls the Brawler "fighter".
                let file = match class {
                    "brawler" => "fighter",
                    c => c,
                };
                [
                    "gladiator",
                    "templar",
                    "ranger",
                    "assassin",
                    "sorcerer",
                    "elementalist",
                    "cleric",
                    "chanter",
                    "fighter",
                ]
                .into_iter()
                .find(|c| *c == file)
                .map(Asset::Emblem)
            }
            _ => None,
        }
    }

    fn url(&self) -> String {
        match self {
            Self::Font(weight) => format!("{CDN}/uikit/preorder/client/fonts/aion2_{weight}.woff2"),
            Self::Icon(stem) => format!("{CDN}/static-aion2-gamedata/resources/{stem}.png"),
            Self::Emblem(class) => {
                format!("{CDN}/static-aion2/characters/img/daevanion/board_icon_start_{class}.png")
            }
        }
    }

    fn cache_name(&self) -> String {
        match self {
            Self::Font(weight) => format!("font_aion2_{weight}.woff2"),
            Self::Icon(stem) => format!("icon_{stem}.png"),
            Self::Emblem(class) => format!("emblem_{class}.png"),
        }
    }

    fn content_type(&self) -> &'static str {
        match self {
            Self::Font(_) => "font/woff2",
            Self::Icon(_) | Self::Emblem(_) => "image/png",
        }
    }
}

pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let cache_dir = ctx
        .app_handle()
        .path()
        .app_cache_dir()
        .map(|d| d.join("gameassets"));
    let path = percent_decode(request.uri().path());
    tauri::async_runtime::spawn_blocking(move || {
        let response = match (Asset::parse(&path), cache_dir) {
            (Some(asset), Ok(dir)) => match load(&asset, &dir) {
                Ok(bytes) => respond(StatusCode::OK, asset.content_type(), bytes),
                Err(e) => {
                    tracing::debug!("{}: {e}", asset.url());
                    respond(StatusCode::NOT_FOUND, "text/plain", e.into_bytes())
                }
            },
            _ => respond(
                StatusCode::NOT_FOUND,
                "text/plain",
                b"unknown asset".to_vec(),
            ),
        };
        responder.respond(response);
    });
}

fn load(asset: &Asset, dir: &Path) -> Result<Vec<u8>, String> {
    let cached: PathBuf = dir.join(asset.cache_name());
    if let Ok(bytes) = std::fs::read(&cached) {
        return Ok(bytes);
    }
    let bytes = download(&asset.url())?;
    if std::fs::create_dir_all(dir).is_ok() {
        let tmp = cached.with_extension("tmp");
        if std::fs::write(&tmp, &bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &cached);
        }
    }
    Ok(bytes)
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

fn respond(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(
            header::CACHE_CONTROL,
            if status == StatusCode::OK {
                "max-age=31536000, immutable"
            } else {
                "no-store"
            },
        )
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

/// `convertFileSrc` percent-encodes the path; only `%XX` needs undoing.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_assets_are_served() {
        assert_eq!(Asset::parse("/font/Medium"), Some(Asset::Font("Medium")));
        assert_eq!(
            Asset::parse("/icon/ICON_GL_SKILL_001.png"),
            Some(Asset::Icon("ICON_GL_SKILL_001".into()))
        );
        assert_eq!(
            Asset::parse("/emblem/brawler.png"),
            Some(Asset::Emblem("fighter"))
        );
        assert_eq!(Asset::parse("/font/Heavy"), None);
        assert_eq!(Asset::parse("/icon/../../etc/passwd.png"), None);
        assert_eq!(Asset::parse("/icon/a.b.png"), None);
        assert_eq!(Asset::parse("/emblem/necromancer.png"), None);
        assert_eq!(Asset::parse("/other/x"), None);
    }

    #[test]
    fn encoded_paths_are_decoded() {
        assert_eq!(percent_decode("/icon%2FICON_A.png"), "/icon/ICON_A.png");
        assert_eq!(percent_decode("/a%zz"), "/a%zz");
    }
}
