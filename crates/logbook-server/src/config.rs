//! Settings from environment variables.

use std::{env, net::SocketAddr, path::PathBuf};

use base64::Engine;
use ipnet::IpNet;

use crate::export::GitExport;

/// Server settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Listen address. `LOGBOOK_ADDR`, default `127.0.0.1:8080`.
    pub addr: SocketAddr,
    /// Folder with `index.html`, CSS, JS, and fonts. `LOGBOOK_STATIC_DIR`, default `static`.
    pub static_dir: PathBuf,
    /// SQLite file. `LOGBOOK_DB`, default `logbook.db`.
    pub db_path: PathBuf,
    /// Public origin, no trailing slash. `LOGBOOK_ORIGIN`, default `http://localhost:8080`.
    /// Its host is the WebAuthn RP ID, so it must be a domain name, not an IP address.
    pub origin: String,
    /// Proxies whose `X-Forwarded-For` header counts, as CIDRs.
    /// `LOGBOOK_TRUSTED_PROXIES`, comma-separated, default none.
    pub trusted_proxies: Vec<IpNet>,
    /// Requests per minute per IP on `/auth/*`. `LOGBOOK_AUTH_RATE_LIMIT`, default 20.
    pub auth_rate_limit: u32,
    /// The git export of public posts (spec 6.10). Off without `EXPORT_REPO`.
    /// `EXPORT_BRANCH` (default `main`), `EXPORT_DEPLOY_KEY` (base64 private key),
    /// `LOGBOOK_EXPORT_DIR` (default: `export` next to the database).
    pub git_export: Option<GitExport>,
    /// The bucket for images (spec 6.11). Off without `S3_BUCKET`: then images go to
    /// `LOGBOOK_MEDIA_DIR` (default: `media` next to the database).
    pub s3: Option<S3>,
    /// Local folder for images when there is no bucket.
    pub media_dir: PathBuf,
    /// Local image cache for the bucket. `LOGBOOK_MEDIA_CACHE`, default `media-cache`
    /// next to the database.
    pub media_cache: PathBuf,
    /// Fetch NOAA surf data (spec 4.5). `LOGBOOK_SURF=off` turns it off (tests).
    pub surf: bool,
    /// `User-Agent` for the NWS API, with contact info. `NWS_USER_AGENT`.
    pub nws_user_agent: String,
    /// The litestream config for the backup check. `LOGBOOK_LITESTREAM_CONFIG`,
    /// default `/etc/litestream.yml`.
    pub litestream_config: PathBuf,
    /// Pinged after each backup check (healthchecks.io style). `HEALTHCHECK_URL`.
    pub healthcheck_url: Option<String>,
}

/// S3 settings: `S3_ENDPOINT`, `S3_BUCKET`, `S3_ACCESS_KEY`, `S3_SECRET_KEY`,
/// `S3_REGION` (default `auto`), `S3_FORCE_PATH_STYLE` (default `false`; Hetzner
/// denies path-style writes).
#[derive(Debug, Clone)]
pub struct S3 {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub path_style: bool,
}

impl Config {
    /// Reads the settings from the environment.
    ///
    /// # Errors
    ///
    /// Fails if a setting does not parse.
    pub fn from_env() -> Result<Self, String> {
        let addr = env::var("LOGBOOK_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
        let addr = addr
            .parse()
            .map_err(|e| format!("LOGBOOK_ADDR={addr:?} is not a socket address: {e}"))?;
        let static_dir =
            env::var_os("LOGBOOK_STATIC_DIR").map_or_else(|| "static".into(), PathBuf::from);
        let db_path = env::var_os("LOGBOOK_DB").map_or_else(|| "logbook.db".into(), PathBuf::from);
        let origin = env::var("LOGBOOK_ORIGIN").unwrap_or_else(|_| "http://localhost:8080".into());
        let origin = origin.trim_end_matches('/').to_string();
        if !(origin.starts_with("https://") || origin.starts_with("http://"))
            || origin.contains(['"', '<', '>', ' '])
        {
            return Err(format!(
                "LOGBOOK_ORIGIN={origin:?} is not an http(s) origin"
            ));
        }
        let trusted_proxies = env::var("LOGBOOK_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse()
                    .map_err(|e| format!("LOGBOOK_TRUSTED_PROXIES: {s:?} is not a CIDR: {e}"))
            })
            .collect::<Result<Vec<IpNet>, String>>()?;
        let auth_rate_limit = match env::var("LOGBOOK_AUTH_RATE_LIMIT") {
            Ok(v) => v
                .parse()
                .map_err(|e| format!("LOGBOOK_AUTH_RATE_LIMIT={v:?}: {e}"))?,
            Err(_) => 20,
        };
        let git_export = git_export_from_env(&db_path)?;
        let s3 = s3_from_env()?;
        let beside_db = |name: &str| {
            db_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(name)
        };
        let media_dir =
            env::var_os("LOGBOOK_MEDIA_DIR").map_or_else(|| beside_db("media"), PathBuf::from);
        let media_cache = env::var_os("LOGBOOK_MEDIA_CACHE")
            .map_or_else(|| beside_db("media-cache"), PathBuf::from);
        let surf = env::var("LOGBOOK_SURF").map_or(true, |v| v != "off");
        let nws_user_agent = env::var("NWS_USER_AGENT")
            .unwrap_or_else(|_| "logbook (https://github.com/eserilev/blog)".into());
        let litestream_config = env::var_os("LOGBOOK_LITESTREAM_CONFIG")
            .map_or_else(|| "/etc/litestream.yml".into(), PathBuf::from);
        let healthcheck_url = env::var("HEALTHCHECK_URL")
            .ok()
            .filter(|u| !u.trim().is_empty());
        Ok(Self {
            addr,
            static_dir,
            db_path,
            origin,
            trusted_proxies,
            auth_rate_limit,
            git_export,
            s3,
            media_dir,
            media_cache,
            surf,
            nws_user_agent,
            litestream_config,
            healthcheck_url,
        })
    }
}

/// The git export settings, if `EXPORT_REPO` is set.
fn git_export_from_env(db_path: &std::path::Path) -> Result<Option<GitExport>, String> {
    Ok(match env::var("EXPORT_REPO") {
        Ok(repo) if !repo.trim().is_empty() => {
            let deploy_key = match env::var("EXPORT_DEPLOY_KEY") {
                Ok(k) if !k.trim().is_empty() => Some(
                    base64::engine::general_purpose::STANDARD
                        .decode(k.trim())
                        .map_err(|e| format!("EXPORT_DEPLOY_KEY is not base64: {e}"))?,
                ),
                _ => None,
            };
            let dir = env::var_os("LOGBOOK_EXPORT_DIR").map_or_else(
                || {
                    db_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join("export")
                },
                PathBuf::from,
            );
            Some(GitExport {
                repo,
                branch: env::var("EXPORT_BRANCH").unwrap_or_else(|_| "main".into()),
                dir,
                deploy_key,
            })
        }
        _ => None,
    })
}

/// The bucket settings, if `S3_BUCKET` is set.
fn s3_from_env() -> Result<Option<S3>, String> {
    Ok(match env::var("S3_BUCKET") {
        Ok(bucket) if !bucket.trim().is_empty() => {
            let need =
                |k: &str| env::var(k).map_err(|_| format!("{k} must be set when S3_BUCKET is set"));
            Some(S3 {
                endpoint: need("S3_ENDPOINT")?,
                bucket,
                access_key: need("S3_ACCESS_KEY")?,
                secret_key: need("S3_SECRET_KEY")?,
                region: env::var("S3_REGION").unwrap_or_else(|_| "auto".into()),
                path_style: env::var("S3_FORCE_PATH_STYLE").is_ok_and(|v| v == "true"),
            })
        }
        _ => None,
    })
}
