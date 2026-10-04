//! Settings from environment variables.

use std::{env, net::SocketAddr, path::PathBuf};

/// Server settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Listen address. `LOGBOOK_ADDR`, default `127.0.0.1:8080`.
    pub addr: SocketAddr,
    /// Folder with `index.html`, CSS, JS, and fonts. `LOGBOOK_STATIC_DIR`, default `static`.
    pub static_dir: PathBuf,
    /// SQLite file. `LOGBOOK_DB`, default `logbook.db`.
    pub db_path: PathBuf,
    /// Public origin for absolute URLs, no trailing slash. `LOGBOOK_ORIGIN`,
    /// default `http://127.0.0.1:8080`.
    pub origin: String,
}

impl Config {
    /// Reads the settings from the environment.
    ///
    /// # Errors
    ///
    /// Fails if `LOGBOOK_ADDR` is not a socket address, or `LOGBOOK_ORIGIN` is not
    /// an `http(s)://` origin.
    pub fn from_env() -> Result<Self, String> {
        let addr = env::var("LOGBOOK_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
        let addr = addr
            .parse()
            .map_err(|e| format!("LOGBOOK_ADDR={addr:?} is not a socket address: {e}"))?;
        let static_dir =
            env::var_os("LOGBOOK_STATIC_DIR").map_or_else(|| "static".into(), PathBuf::from);
        let db_path = env::var_os("LOGBOOK_DB").map_or_else(|| "logbook.db".into(), PathBuf::from);
        let origin = env::var("LOGBOOK_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
        let origin = origin.trim_end_matches('/').to_string();
        if !(origin.starts_with("https://") || origin.starts_with("http://"))
            || origin.contains(['"', '<', '>', ' '])
        {
            return Err(format!(
                "LOGBOOK_ORIGIN={origin:?} is not an http(s) origin"
            ));
        }
        Ok(Self {
            addr,
            static_dir,
            db_path,
            origin,
        })
    }
}
