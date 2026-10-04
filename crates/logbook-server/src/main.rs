//! `logbook` — the server binary.
//!
//! - `logbook` or `logbook serve`: run the server.
//! - `logbook seed-sample`: add sample posts to an empty database.
//! - `logbook setup-link`: print a one-time link to register a passkey (spec 6.6).

use std::net::SocketAddr;

use logbook_server::{AppState, Config, app, auth, checks, db, seed};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let cmd = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    let result = match cmd.as_str() {
        "serve" => serve().await,
        "seed-sample" => seed_sample().await,
        "setup-link" => setup_link().await,
        other => Err(format!(
            "unknown command {other:?}; use `serve`, `seed-sample`, or `setup-link`"
        )),
    };
    if let Err(e) = result {
        tracing::error!("{e}");
        std::process::exit(1);
    }
}

async fn serve() -> Result<(), String> {
    let config = Config::from_env()?;
    let pool = db::connect(&config.db_path)
        .await
        .map_err(|e| format!("cannot open {}: {e}", config.db_path.display()))?;
    checks::beat(&pool)
        .await
        .map_err(|e| format!("cannot write the heartbeat: {e}"))?;
    checks::spawn_heartbeat(pool.clone());
    let app = app(&config, AppState::new(&config, pool)?);
    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|e| format!("cannot listen on {}: {e}", config.addr))?;
    tracing::info!("listening on http://{}", config.addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await
    .map_err(|e| format!("server error: {e}"))
}

async fn seed_sample() -> Result<(), String> {
    let config = Config::from_env()?;
    let pool = db::connect(&config.db_path)
        .await
        .map_err(|e| format!("cannot open {}: {e}", config.db_path.display()))?;
    let n = seed::seed_sample(&pool).await?;
    tracing::info!("added {n} sample posts to {}", config.db_path.display());
    Ok(())
}

/// Prints a setup link to stdout, never to the log. The token is in the URL
/// fragment, so the browser never sends it in a request line or `Referer`.
async fn setup_link() -> Result<(), String> {
    let config = Config::from_env()?;
    let pool = db::connect(&config.db_path)
        .await
        .map_err(|e| format!("cannot open {}: {e}", config.db_path.display()))?;
    let token = auth::create_setup_token(&pool)
        .await
        .map_err(|e| format!("cannot create a setup token: {e}"))?;
    println!("{}/setup#{token}", config.origin);
    println!(
        "Open it within {} minutes. It works once.",
        auth::SETUP_MINUTES
    );
    Ok(())
}

/// Resolves on Ctrl-C or SIGTERM (Docker stop).
async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { () = ctrl_c => {}, () = term => {} }
    tracing::info!("shutting down");
}
