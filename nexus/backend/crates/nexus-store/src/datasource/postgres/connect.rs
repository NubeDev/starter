//! Open a connection pool to a Postgres datasource's *own* database.
//!
//! A Postgres datasource is stored as host/port/database/user plus a sealed
//! secret; querying it (R4) needs a live pool against that database. This is the
//! one place that pool is built: it recovers the plaintext secret through the
//! audited [`crate::datasource::open_secret`] boundary, assembles the connect
//! options, and opens a small bounded pool. The plaintext password is moved into
//! the connect options and dropped immediately — never held, logged, or returned.
//!
//! The pool is intentionally small: a datasource query is short-lived and runs
//! under the R4 statement timeout, so a handful of connections absorbs concurrent
//! panels without holding a large fan-out open against the customer's database.

use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use starter_spi::Error;

use crate::datasource::secret::Envelope;
use crate::datasource::DatasourceRecord;

/// Max connections nexus opens against a single datasource Postgres. Small on
/// purpose: queries are short and timeout-bounded, so this caps our footprint in
/// the customer's database without throttling normal dashboard refresh.
const MAX_CONNECTIONS: u32 = 4;

/// Open a bounded pool to the Postgres described by `record`, authenticating with
/// the secret recovered through the audited decrypt boundary. `actor` is recorded
/// on that decrypt. Returns `NotFound` when the datasource is not visible to the
/// tenant and `Internal` on a connection failure.
pub async fn connect(
    pool: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    actor: &str,
    record: &DatasourceRecord,
) -> Result<PgPool, Error> {
    let secret = crate::datasource::open_secret(pool, envelope, tenant_id, actor, record.id).await?;

    let opts = PgConnectOptions::new()
        .host(&record.host)
        .port(u16::try_from(record.port).map_err(|_| Error::Invalid {
            message: format!("datasource port {} out of range", record.port),
        })?)
        .database(&record.database)
        .username(&record.db_user)
        .password(&secret);
    drop(secret);

    PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })
}
