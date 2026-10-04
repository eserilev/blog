//! Settings from environment variables.

use std::{env, net::SocketAddr, path::PathBuf};

/// Server settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Listen address. `LOGBOOK_ADDR`, default `127.0.0.1:8080`.
    pub addr: SocketAddr,
    /// Folder with `index.html`, CSS, JS, and fonts. `LOGBOOK_STATIC_DIR`, default `static`.
    pub static_dir: PathBuf,
}

impl Config {
    /// Reads the settings from the environment.
    ///
    /// # Errors
    ///
    /// Fails if `LOGBOOK_ADDR` is not a valid socket address.
    pub fn from_env() -> Result<Self, String> {
        let addr = env::var("LOGBOOK_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
        let addr = addr
            .parse()
            .map_err(|e| format!("LOGBOOK_ADDR={addr:?} is not a socket address: {e}"))?;
        let static_dir =
            env::var_os("LOGBOOK_STATIC_DIR").map_or_else(|| "static".into(), PathBuf::from);
        Ok(Self { addr, static_dir })
    }
}
