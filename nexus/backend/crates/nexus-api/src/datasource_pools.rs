//! A per-datasource connection-pool cache, keyed on the immutable datasource id
//! within its tenant.
//!
//! Connecting to a datasource decrypts its secret and opens a pool — too costly
//! to repeat on every query. This cache holds one pool per datasource so repeat
//! queries (a dashboard refreshing its panels) reuse connections, while a first
//! touch builds the pool through the audited [`nexus_store::datasource::postgres`]
//! boundary. Single-node for v1, like live fan-out (R7): the map lives in this
//! process; a multi-node deployment gets one cache per node, which is correct —
//! each node holds its own connections.

use std::collections::HashMap;
use std::sync::Arc;

use nexus_store::datasource::{self, DatasourceRecord, Envelope};
use sqlx::PgPool;
use starter_spi::Error;
use tokio::sync::Mutex;

/// Cloneable handle to the shared pool cache. Cheap to clone (an `Arc`), so it
/// rides on `AppState` like every other handle.
#[derive(Clone, Default)]
pub struct DatasourcePools {
    inner: Arc<Mutex<HashMap<String, PgPool>>>,
}

impl DatasourcePools {
    /// Return the pool for `record`, building and caching it on first use. The
    /// build path connects via the datasource's own kind connector and is audited
    /// (`actor`). Subsequent calls for the same tenant+id return the cached pool.
    pub async fn get_or_connect(
        &self,
        metadata: &PgPool,
        envelope: &Envelope,
        tenant_id: &str,
        actor: &str,
        record: &DatasourceRecord,
    ) -> Result<PgPool, Error> {
        let key = datasource::pool_key(tenant_id, record.id);
        let mut map = self.inner.lock().await;
        if let Some(pool) = map.get(&key) {
            return Ok(pool.clone());
        }
        let pool = build(metadata, envelope, tenant_id, actor, record).await?;
        map.insert(key, pool.clone());
        Ok(pool)
    }

    /// Drop a datasource's cached pool — called when it is deleted or its secret
    /// rotates, so a stale connection is never reused. A no-op if nothing was
    /// cached.
    pub async fn evict(&self, tenant_id: &str, id: uuid::Uuid) {
        let key = datasource::pool_key(tenant_id, id);
        if let Some(pool) = self.inner.lock().await.remove(&key) {
            pool.close().await;
        }
    }
}

/// Dispatch on the datasource kind to the right connector. Postgres is the only
/// kind today; a new kind is a new arm here plus its `datasource/<kind>/` module,
/// with nothing else in this file changing.
async fn build(
    metadata: &PgPool,
    envelope: &Envelope,
    tenant_id: &str,
    actor: &str,
    record: &DatasourceRecord,
) -> Result<PgPool, Error> {
    match record.kind.as_str() {
        "postgres" => {
            datasource::postgres::connect(metadata, envelope, tenant_id, actor, record).await
        }
        other => Err(Error::Invalid {
            message: format!("datasource kind '{other}' is not queryable"),
        }),
    }
}
