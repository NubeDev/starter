//! Run a query through the result cache, falling back to the live engine.
//!
//! This is the seam both query handlers call instead of `kinds::run` directly:
//! it builds the C3 cache key, then asks the cache to serve a live entry or run
//! the backing query (coalescing concurrent misses). Caching is transparent to
//! the dispatch layer below — a cache miss runs exactly the same `kinds::run`
//! path as before.
//!
//! A per-request concurrency permit is acquired *before* the cache lookup is
//! resolved into a backing load, so a tenant at its cap is rejected fast and a
//! cache *hit* still costs no permit (the closure that needs one only runs on a
//! miss). The permit is held for the query's duration via the guard's RAII.

use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use nexus_store::QueryIdentity;
use sqlx::PgPool;
use starter_spi::Error;

use super::cache_key;
use crate::state::AppState;

/// Run `req` against `pool` for `identity`, serving from cache when possible.
/// `datasource_scope` distinguishes the datasource the query targets in the key
/// (`"dev"` for the single-datasource shortcut, the datasource id otherwise) so
/// identical SQL against different datasources never shares an entry.
///
/// A tenant query (one with a tenant in `identity`) is admitted through the
/// per-tenant concurrency cap; the dev shortcut (no tenant) is unmetered, as it
/// is not a multi-tenant path.
pub async fn run_cached(
    state: &AppState,
    pool: &PgPool,
    req: &QueryRequest,
    identity: &QueryIdentity,
    datasource_scope: &str,
) -> Result<QueryResponse, Error> {
    let key = cache_key(req, identity, datasource_scope);
    state
        .query_cache
        .get_or_load(key, || async {
            // The concurrency permit is acquired only on a miss (a cache hit
            // never reaches here), and held until the backing query returns.
            let _guard = match identity.tenant_id.as_deref() {
                Some(tenant) => Some(state.quotas.admit(tenant).await?),
                None => None,
            };
            crate::kinds::run(state, pool, req, identity).await
        })
        .await
}
