//! Probe a Postgres datasource from *raw* connection params, before it is saved.
//!
//! [`super::connect`] opens a pool for a stored datasource, recovering the secret
//! through the audited decrypt boundary. The "Test connection" form needs the
//! opposite: validate connectivity for a config the user is still typing, which
//! has no row and no sealed secret yet. This builds a one-shot connection from
//! the supplied plaintext, runs a trivial round-trip, and drops it — so the form
//! can report success before the datasource is persisted.
//!
//! The plaintext secret is moved into the connect options and dropped
//! immediately; it is never logged, cached, or returned. Unlike [`super::connect`]
//! there is nothing to audit — no stored secret is decrypted.

use std::time::Duration;

use sqlx::postgres::PgConnectOptions;
use sqlx::{ConnectOptions, Connection};
use starter_spi::Error;

/// Raw Postgres connection parameters for a pre-save probe. Mirrors the fields a
/// create request carries, but lives in the store layer because connecting is a
/// store concern; the route maps its DTO onto this.
pub struct ProbeParams<'a> {
    pub host: &'a str,
    pub port: u16,
    pub database: &'a str,
    pub user: &'a str,
    pub secret: &'a str,
}

/// Open a single short-lived connection to the described Postgres and force a
/// real round-trip (`SELECT 1`). Returns `Ok(())` when the credentials connect,
/// and an `Internal` error carrying the driver's reason otherwise — the route
/// sanitizes that reason before it reaches the client. The connection is closed
/// before returning so a probe never holds a connection against the customer DB.
pub async fn probe(params: ProbeParams<'_>) -> Result<(), Error> {
    let opts = PgConnectOptions::new()
        .host(params.host)
        .port(params.port)
        .database(params.database)
        .username(params.user)
        .password(params.secret);

    let mut conn = opts
        .connect()
        .await
        .map_err(|e| Error::Internal { source: Box::new(e) })?;

    // Bound the probe so a black-holed host can't hang the form indefinitely; the
    // round-trip itself is what proves the credentials, not pool construction.
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        sqlx::query("SELECT 1").execute(&mut conn).await
    })
    .await;

    let _ = conn.close().await;

    match result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(Error::Internal { source: Box::new(e) }),
        Err(_) => Err(Error::Invalid {
            message: "connection probe timed out".into(),
        }),
    }
}
