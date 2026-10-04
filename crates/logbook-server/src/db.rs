//! SQLite pool and migrations (spec 6.5).

use std::{path::Path, str::FromStr, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

/// Opens (or creates) the database and runs the migrations.
///
/// WAL mode and a 5 s busy timeout. Litestream runs the checkpoints, so the
/// app turns off automatic checkpoints (spec 6.5).
///
/// # Errors
///
/// Fails if the file cannot be opened or a migration fails.
pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str("sqlite:")?
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true)
        .pragma("wal_autocheckpoint", "0");
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
