use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
};

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let opts = database_url
        .parse::<SqliteConnectOptions>()
        .context("Failed to parse sqlite database url")?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(opts)
        .await
        .context("Failed to new sqlite connection pool")?;
    Ok(pool)
}
