//! Federated query dispatch: resolve a request's `sources` against the caller's
//! tenant and run them through the engine federation runner.
//!
//! This is the API half of RW-05. A request that names `sources` (a
//! cross-datasource or file join) is resolved here — every datasource authorised
//! and decrypted ([`resolve`]) — then executed ([`run`]) under the same caps and
//! result cache as the single-datasource path. A request with no `sources` never
//! reaches this module: the query handler keeps it on the push-down path, so
//! today's single-datasource behaviour is byte-identical.

mod resolve;
mod run;

use axum::response::IntoResponse as _;
use axum::Json;
use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use nexus_store::QueryIdentity;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::Error;

use crate::state::AppState;

/// Transport seam: run the federated request and map the result to an HTTP
/// response. Kept here (not in the route file) so the route handler stays thin —
/// it decides the path and delegates; this maps the one domain call to a DTO.
pub async fn respond(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    req: &QueryRequest,
) -> axum::response::Response {
    let identity = QueryIdentity {
        tenant_id: Some(tenant.to_string()),
        user_id: Some(principal.subject.clone()),
    };
    match run_cached(state, principal, tenant, req, &identity).await {
        Ok(out) => Json(out).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// True when `req` should run on the federation path rather than push-down: it
/// names one or more federated `sources`. A single-datasource request (empty
/// `sources`) returns false and stays on the existing path.
pub fn is_federated(req: &QueryRequest) -> bool {
    !req.sources.is_empty()
}

/// Resolve and run a federated request, serving from the result cache when
/// possible. The cache key already folds in `sources` (see `cache::key`), so a
/// federated result is cached under a key distinct from the same SQL on the
/// push-down path. The concurrency permit is acquired on a miss, like push-down.
pub async fn run_cached(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    req: &QueryRequest,
    identity: &QueryIdentity,
) -> Result<QueryResponse, Error> {
    // A stable scope so the cache key for a federated query never collides with a
    // single-datasource one; the per-source ids are folded in by `cache::key`.
    let key = crate::cache::cache_key(req, identity, "federation");
    state
        .query_cache
        .get_or_load(key, || async {
            let _guard = state.quotas.admit(tenant).await?;
            let sources =
                resolve::resolve_sources(state, principal, tenant, &principal.subject, &req.sources)
                    .await?;
            run::run_federated(req, sources, state.guards).await
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A request body with no `sources` and only `sql` set — the single-datasource
    /// shape a pre-RW-05 client sends.
    fn push_down_request() -> QueryRequest {
        QueryRequest {
            sql: "SELECT 1".to_string(),
            time_range: None,
            interval_secs: None,
            variables: Vec::new(),
            kind: None,
            params: None,
            sources: Vec::new(),
        }
    }

    #[test]
    fn empty_sources_stays_on_the_push_down_path() {
        // The dispatch seam: a request with no federated sources is byte-identical
        // to pre-RW-05 — it must never be routed through the federation runner.
        assert!(!is_federated(&push_down_request()));
    }

    #[test]
    fn naming_a_source_selects_the_federation_path() {
        let mut req = push_down_request();
        req.sources.push(nexus_spi::dto::query::FederatedSourceRef {
            alias: "a".to_string(),
            datasource: "00000000-0000-0000-0000-000000000000".to_string(),
            table: Some("t".to_string()),
        });
        assert!(is_federated(&req));
    }
}
