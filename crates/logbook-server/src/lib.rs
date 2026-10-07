//! The Logbook server: a JSON API plus one `index.html` (spec 6).

pub mod assets;
pub mod auth;
pub mod checks;
pub mod config;
pub mod counter;
pub mod db;
pub mod export;
pub mod feed;
pub mod guard;
pub mod head;
pub mod headers;
pub mod media;
pub mod now;
pub mod pages;
pub mod posts;
pub mod routes;
pub mod seed;
pub mod seo;
pub mod site;
pub mod surf;
pub mod topic;

use std::{sync::Arc, time::Duration};

use axum::{Router, middleware};
use ipnet::IpNet;
use sqlx::SqlitePool;
use tokio::sync::Notify;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub use config::Config;

/// Shared state for all handlers.
#[derive(Clone)]
pub struct AppState {
    /// `static/index.html`, read once at startup, with versioned `/static/` URLs.
    pub index_html: Arc<str>,
    /// Version hashes of the static files.
    pub assets: Arc<assets::Assets>,
    pub pool: SqlitePool,
    /// Public origin, for absolute URLs and the CSRF check.
    pub origin: Arc<str>,
    /// Passkey ceremonies.
    pub auth: Arc<auth::Auth>,
    /// Rate limit for `/auth/*`.
    pub auth_limiter: Arc<guard::RateLimiter>,
    /// Proxies whose `X-Forwarded-For` header counts.
    pub trusted_proxies: Arc<[IpNet]>,
    /// Page loads not yet flushed.
    pub counter: Arc<counter::Counter>,
    /// Wakes the git export after a change to a public post. `None` without `EXPORT_REPO`.
    pub export_changed: Option<Arc<Notify>>,
    /// Image storage.
    pub media: media::Media,
    /// The latest surf data, if any.
    pub surf: Arc<tokio::sync::RwLock<Option<surf::Surf>>>,
    /// The latest replica check, for `/healthz`.
    pub replica: checks::ReplicaStatus,
}

impl AppState {
    /// Tells the git export that public content may have changed.
    pub fn content_changed(&self) {
        if let Some(n) = &self.export_changed {
            n.notify_one();
        }
    }
}

impl AppState {
    /// Reads `index.html`, checks its head markers, and versions its `/static/` URLs.
    ///
    /// # Errors
    ///
    /// Fails if a static file cannot be read, if `index.html` has no
    /// `<!--head-->...<!--/head-->` block, or if WebAuthn cannot use the origin.
    pub fn new(config: &Config, pool: SqlitePool) -> Result<Self, String> {
        let path = config.static_dir.join("index.html");
        let index_html = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        match (index_html.find(head::START), index_html.find(head::END)) {
            (Some(a), Some(b)) if a < b => {}
            _ => {
                return Err(format!(
                    "{} has no {}...{} block",
                    path.display(),
                    head::START,
                    head::END
                ));
            }
        }
        if let Some(m) = site::MARKERS.iter().find(|m| !index_html.contains(**m)) {
            return Err(format!("{} has no {m} marker", path.display()));
        }
        let assets = assets::Assets::load(&config.static_dir)?;
        Ok(Self {
            index_html: assets.version_html(&index_html).into(),
            assets: Arc::new(assets),
            pool,
            origin: config.origin.clone().into(),
            auth: auth::Auth::new(&config.origin)?,
            auth_limiter: Arc::new(guard::RateLimiter::new(
                config.auth_rate_limit,
                Duration::from_mins(1),
            )),
            trusted_proxies: config.trusted_proxies.clone().into(),
            counter: Arc::new(counter::Counter::default()),
            export_changed: config.git_export.as_ref().map(|_| Arc::new(Notify::new())),
            media: media_store(config)?,
            surf: Arc::default(),
            replica: Arc::default(),
        })
    }
}

/// The image store: the bucket if configured, else a local folder.
///
/// # Errors
///
/// Fails if the bucket settings are invalid or the folder cannot be made.
pub fn media_store(config: &Config) -> Result<media::Media, String> {
    use object_store::{aws::AmazonS3Builder, local::LocalFileSystem};
    let Some(s3) = &config.s3 else {
        std::fs::create_dir_all(&config.media_dir)
            .map_err(|e| format!("cannot make {}: {e}", config.media_dir.display()))?;
        let store =
            LocalFileSystem::new_with_prefix(&config.media_dir).map_err(|e| e.to_string())?;
        return Ok(media::Media {
            store: Arc::new(store),
            cache: None,
        });
    };
    // Virtual-host style puts the bucket in the host name (Hetzner needs this).
    let endpoint = if s3.path_style {
        s3.endpoint.clone()
    } else {
        let (scheme, host) = s3
            .endpoint
            .split_once("://")
            .ok_or("S3_ENDPOINT needs a scheme")?;
        format!("{scheme}://{}.{host}", s3.bucket)
    };
    let store = AmazonS3Builder::new()
        .with_endpoint(endpoint)
        .with_bucket_name(&s3.bucket)
        .with_access_key_id(&s3.access_key)
        .with_secret_access_key(&s3.secret_key)
        .with_region(&s3.region)
        .with_virtual_hosted_style_request(!s3.path_style)
        .with_allow_http(s3.endpoint.starts_with("http://"))
        .build()
        .map_err(|e| format!("bad S3 settings: {e}"))?;
    Ok(media::Media {
        store: Arc::new(store),
        cache: Some(config.media_cache.clone()),
    })
}

/// Builds the full app: the route table, static files, the 404 fallback, and the
/// security headers on every response.
pub fn app(config: &Config, state: AppState) -> Router {
    let statics = Router::new()
        .fallback_service(ServeDir::new(&config.static_dir))
        .layer(middleware::from_fn_with_state(
            state.assets.clone(),
            assets::serve,
        ));
    let router = routes::router(&state)
        .nest_service("/static", statics)
        .fallback(pages::not_found)
        .with_state(state);
    headers::apply(router).layer(TraceLayer::new_for_http())
}
