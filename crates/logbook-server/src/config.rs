//! Settings from environment variables.

use std::{env, net::SocketAddr, path::PathBuf};

use ipnet::IpNet;

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
        Ok(Self {
            addr,
            static_dir,
            db_path,
            origin,
            trusted_proxies,
            auth_rate_limit,
        })
    }
}
