use logbook_server::{Config, app};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    if let Err(e) = run().await {
        tracing::error!("{e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let config = Config::from_env()?;
    let app = app(&config).map_err(|e| {
        format!(
            "cannot read {}: {e}",
            config.static_dir.join("index.html").display()
        )
    })?;
    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(|e| format!("cannot listen on {}: {e}", config.addr))?;
    tracing::info!("listening on http://{}", config.addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|e| format!("server error: {e}"))
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
