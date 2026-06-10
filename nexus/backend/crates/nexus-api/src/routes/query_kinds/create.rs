//! `POST /api/v1/query-kinds` — promote a tenant-authored query-kind.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query_kind::{CreateQueryKindRequest, QueryKindDetail};
use nexus_store::query_kind::{self, NewQueryKind};
use serde_json::json;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::to_detail;
use crate::authz::KIND_QUERY_KIND;
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::query_kind_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/query-kinds",
    tag = "query-kinds",
    operation_id = "create_query_kind",
    request_body = CreateQueryKindRequest,
    responses(
        (status = 200, description = "Created", body = QueryKindDetail),
        (status = 400, description = "The SQL failed a save-time lint"),
        (status = 409, description = "Name already used in this tenant"),
    ),
)]
pub async fn create_query_kind(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateQueryKindRequest>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Reconstruct the kind the dispatcher would run and lint it *before* writing,
    // so a row is never persisted that would later fail (or leak) at dispatch.
    // `params_schema` defaults to an empty object — the same default the binder
    // applies to a schema-less kind.
    let kind = crate::kinds::QueryKind {
        name: req.name.clone(),
        sql: req.sql.clone(),
        params_schema: req.params_schema.clone().unwrap_or_else(|| json!({})),
        datasource_kind: req.datasource_kind.clone(),
        tables: req.tables.clone(),
        datasource_binding: req.datasource_binding.clone(),
        description: req.description.clone(),
    };
    if let Err(e) = crate::kinds::lint(&kind) {
        return (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    let new = NewQueryKind {
        name: kind.name,
        sql: kind.sql,
        params_schema: kind.params_schema,
        datasource_kind: kind.datasource_kind,
        tables: kind.tables,
        datasource_binding: kind.datasource_binding,
        description: kind.description,
    };
    match query_kind::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => {
            record_create(&state, principal, &tenant, &rec).await;
            Json(to_detail(&rec)).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// Record the create as a reversible `Change` (C6). A recording failure is logged
/// here, never surfaced — the query-kind is already committed.
async fn record_create(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    rec: &nexus_store::query_kind::QueryKindRecord,
) {
    let draft = ChangeDraft {
        resource: ResourceRef::row(KIND_QUERY_KIND, rec.id.to_string()).with_tenant(tenant),
        op: Op::Create,
        before: None,
        after: Some(query_kind_snapshot_json(rec)),
        resource_version: None,
        correlation: None,
    };
    if let Err(e) = record(
        &state.changelog.registry,
        state.metadata.clone(),
        tenant,
        actor_from(principal),
        draft,
    )
    .await
    {
        tracing::warn!(error = %e, "failed to record query-kind create");
    }
}
