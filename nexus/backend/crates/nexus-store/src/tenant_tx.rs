//! Open a transaction bound to a tenant for the duration of one request.
//!
//! Tenant isolation is enforced by Postgres RLS reading the `app.tenant_id` GUC.
//! `SET LOCAL` scopes that GUC to the current transaction, so it cannot leak into
//! the next checkout of a pooled connection — **but only if every tenant-scoped
//! query runs inside a transaction that set it first**. A bare pooled query with
//! no surrounding `SET LOCAL` would see the previous checkout's GUC (or none).
//! This module is the single place that binding happens; all tenant-scoped store
//! functions take the transaction it returns.

use sqlx::{Executor, PgPool, Postgres, Transaction};
use starter_spi::Error;

/// Begin a transaction and bind `tenant_id` to it via `SET LOCAL app.tenant_id`.
/// Every query run on the returned transaction is filtered by the RLS policies
/// to that tenant. The caller commits (or drops to roll back).
///
/// `tenant_id` is bound as a parameter through `set_config`, never interpolated
/// into SQL, so a tenant id can't carry an injection.
pub async fn begin<'a>(
    pool: &'a PgPool,
    tenant_id: &str,
) -> Result<Transaction<'a, Postgres>, Error> {
    let mut tx = pool.begin().await.map_err(internal)?;
    // `set_config(name, value, is_local=true)` is the function form of
    // `SET LOCAL`, and unlike the statement form it accepts a bind parameter.
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    Ok(tx)
}

/// Run `op` inside a tenant-bound transaction and commit it on success. A
/// convenience over [`begin`] for the common "do some work, commit" path.
pub async fn with<'a, F, Fut, T>(pool: &'a PgPool, tenant_id: &str, op: F) -> Result<T, Error>
where
    F: FnOnce(Transaction<'a, Postgres>) -> Fut,
    Fut: std::future::Future<Output = Result<(T, Transaction<'a, Postgres>), Error>>,
{
    let tx = begin(pool, tenant_id).await?;
    let (value, tx) = op(tx).await?;
    tx.commit().await.map_err(internal)?;
    Ok(value)
}

/// Set the GUC on an existing connection/transaction handle. Used where a caller
/// already holds a transaction (e.g. composing several store calls in one tx).
pub async fn bind<'c, E>(executor: E, tenant_id: &str) -> Result<(), Error>
where
    E: Executor<'c, Database = Postgres>,
{
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id)
        .execute(executor)
        .await
        .map_err(internal)?;
    Ok(())
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
