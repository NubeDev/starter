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

use super::QueryKind;
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
        Some(name) => run_kind(state, pool, name, req, identity, state.guards).await,
        None => nexus_store::run_request(pool, req, identity, state.guards).await,
    }
}

/// Resolve and run a kind-mode request: validate params host-side, then hand the
/// kind's SQL + lowered params to the store binder.
///
/// The registry is **three-source** (§4.5c + WS-14): the built-in file pack is
/// consulted first (global, all tenants), then the **extension-contributed**
/// kinds (also global — installed once per deployment, materialised into the
/// in-memory `extension_kinds` registry at boot, WS-14 §5), and only on a miss in
/// both global sources are the caller's tenant-authored kinds in the metadata DB
/// tried. Global precedes tenant for the same reason the file pack does:
/// admin-curated kinds win over a tenant's same-named overlay. All three paths
/// reconstruct the same [`QueryKind`] and run through the identical validate/lower
/// path, so an extension or DB kind is bound under the exact same schema and
/// host-token rules as a file kind. A name found in no source is a 4xx
/// (`Error::Invalid`); resolution happens before any data-side database work.
async fn run_kind(
    state: &AppState,
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

    // File pack first; then extension-contributed (global); then a tenant kind.
    let bound = match super::resolve(&state.kinds, name, &params) {
        Ok(bound) => bound,
        Err(super::KindError::Unknown(_)) => {
            match super::resolve(&state.extension_kinds, name, &params) {
                Ok(bound) => bound,
                Err(super::KindError::Unknown(_)) => {
                    let kind = load_tenant_kind(state, identity, name).await?;
                    super::resolve_kind(&kind, &params).map_err(invalid)?
                }
                Err(e) => return Err(invalid(e)),
            }
        }
        Err(e) => return Err(invalid(e)),
    };
    nexus_store::run_kind_request(pool, &bound.sql, bound.params, req, identity, guards).await
}

/// Look up a tenant-authored kind by name in the metadata DB, reconstructing it
/// into a [`QueryKind`]. The row was lint-validated at save time, so it carries
/// the same guarantees as a file kind. Returns `Error::Invalid` if the caller has
/// no tenant context or the name is unknown to this tenant — a kind-mode request
/// naming nothing the caller can see is a bad request, not a server error.
async fn load_tenant_kind(
    state: &AppState,
    identity: &nexus_store::QueryIdentity,
    name: &str,
) -> Result<QueryKind, Error> {
    let tenant = identity.tenant_id.as_deref().ok_or_else(|| Error::Invalid {
        message: format!("query-kind `{name}` is not a known kind"),
    })?;
    let record = nexus_store::query_kind::get_by_name(&state.metadata, tenant, name)
        .await?
        .ok_or_else(|| Error::Invalid {
            message: format!("query-kind `{name}` is not a known kind"),
        })?;
    Ok(QueryKind {
        name: record.name,
        sql: record.sql,
        params_schema: record.params_schema,
        datasource_kind: record.datasource_kind,
        tables: record.tables,
        datasource_binding: record.datasource_binding,
        description: record.description,
    })
}

/// Map a kind resolution failure (unknown name, bad params) to a 4xx.
fn invalid(e: super::KindError) -> Error {
    Error::Invalid {
        message: e.to_string(),
    }
}
