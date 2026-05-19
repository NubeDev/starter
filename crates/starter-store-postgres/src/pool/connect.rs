//! Connect a `Pool` from a Postgres URL.

use sqlx::{postgres::PgPoolOptions, PgPool};

use super::wrapper::Pool;

/// Connect to a Postgres database.
///
/// `url` is a standard `postgres://user:pass@host:5432/db` URL.
pub async fn connect(url: &str) -> Result<Pool, sqlx::Error> {
    let inner: PgPool = PgPoolOptions::new()
        .max_connections(16)
        .connect(url)
        .await?;
    Ok(Pool::from_sqlx(inner))
}
