//! Pick the execution path for a `QueryRequest` — raw SQL or a query-kind — and
//! run it under the shared binder + guards.
//!
//! This is the one place that branches on `req.kind`, so both query handlers
//! (`POST /query` and `POST /datasources/:id/query`) stay thin: they resolve the
//! pool and identity, then call [`run`]. Kind-mode validates params against the
//! registry and binds the kind's SQL; sql-mode runs the request's `sql`. Both
//! flow through `nexus_store`'s one binder — kinds are a front door, not a
//! second engine.

use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use sqlx::PgPool;
use starter_spi::Error;

use super::Registry;
use crate::state::AppState;

/// Run `req` against `pool` for `identity`, dispatching on mode. A kind-mode
/// request whose kind is unknown or whose params fail validation is a 4xx
/// (`Error::Invalid`); the registry resolution happens before any database work.
pub async fn run(
    state: &AppState,
    pool: &PgPool,
    req: &QueryRequest,
    identity: &nexus_store::QueryIdentity,
) -> Result<QueryResponse, Error> {
    match &req.kind {
        Some(name) => run_kind(&state.kinds, pool, name, req, identity, state.guards).await,
        None => nexus_store::run_request(pool, req, identity, state.guards).await,
    }
}

/// Resolve and run a kind-mode request: validate params host-side, then hand the
/// kind's SQL + lowered params to the store binder.
async fn run_kind(
    registry: &Registry,
    pool: &PgPool,
    name: &str,
    req: &QueryRequest,
    identity: &nexus_store::QueryIdentity,
    guards: nexus_store::QueryGuards,
) -> Result<QueryResponse, Error> {
    // Absent params default to an empty object so the schema's declared defaults
    // still apply (a kind with all-defaulted params needs no body).
    let params = req
        .params
        .clone()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let bound = super::resolve(registry, name, &params).map_err(|e| Error::Invalid {
        message: e.to_string(),
    })?;
    nexus_store::run_kind_request(pool, &bound.sql, bound.params, req, identity, guards).await
}
