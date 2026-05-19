//! Connect a `Pool` from a SQLite URL.

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

use super::wrapper::Pool;

/// Connect to a SQLite database.
///
/// `url` follows sqlx's convention: `sqlite::memory:` for in-memory,
/// `sqlite:./data.db` for a file. Returns a ready-to-use [`Pool`].
pub async fn connect(url: &str) -> Result<Pool, sqlx::Error> {
    let inner: SqlitePool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect(url)
        .await?;
    Ok(Pool::from_sqlx(inner))
}
