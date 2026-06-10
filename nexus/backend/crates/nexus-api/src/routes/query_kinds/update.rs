//! `PUT /api/v1/query-kinds/:id` — edit a tenant-authored query-kind.

use axum::extract::{Path, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query_kind::{QueryKindDetail, UpdateQueryKindRequest};
use nexus_store::query_kind::{self, QueryKindPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_EDIT, KIND_QUERY_KIND};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::query_kind_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    put,
    path = "/api/v1/query-kinds/{id}",
    tag = "query-kinds",
    operation_id = "update_query_kind",
    params(("id" = Uuid, Path, description = "Query-kind id")),
    request_body = UpdateQueryKindRequest,
    responses(
        (status = 200, description = "Updated", body = QueryKindDetail),
        (status = 400, description = "The merged SQL failed a save-time lint"),
        (status = 403, description = "Not allowed to edit this query-kind"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_query_kind(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateQueryKindRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Must already exist (and be visible) before the edit gate, so a 404 isn't
    // masked as a 403. The fetched row is also the `before` snapshot the undo log
    // needs and the base the re-lint merges the request over.
    let before = match query_kind::get(&state.metadata, &tenant, id).await {
        Ok(r) => r,
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_QUERY_KIND,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    // Re-lint the *merged* result: apply the request's present fields over the
    // stored row, then lint, so an edit can't introduce unsafe SQL the way an
    // unlinted update could. `name` is immutable, so it carries over unchanged.
    let merged = crate::kinds::QueryKind {
        name: before.name.clone(),
        sql: req.sql.clone().unwrap_or_else(|| before.sql.clone()),
        params_schema: req
            .params_schema
            .clone()
            .unwrap_or_else(|| before.params_schema.clone()),
        datasource_kind: req
            .datasource_kind
            .clone()
            .unwrap_or_else(|| before.datasource_kind.clone()),
        tables: req.tables.clone().unwrap_or_else(|| before.tables.clone()),
        datasource_binding: req
            .datasource_binding
            .clone()
            .or_else(|| before.datasource_binding.clone()),
        description: req
            .description
            .clone()
            .or_else(|| before.description.clone()),
    };
    if let Err(e) = crate::kinds::lint(&merged) {
        return (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }
    // A present value sets the column; this verb does not express clearing
    // (mirrors agent update's `system_prompt: req.system_prompt.map(Some)`), so a
    // present optional field maps to `Some(Some(_))` and an absent one leaves it.
    let patch = QueryKindPatch {
        sql: req.sql,
        params_schema: req.params_schema,
        datasource_kind: req.datasource_kind,
        tables: req.tables,
        datasource_binding: req.datasource_binding.map(Some),
        description: req.description.map(Some),
    };
    match query_kind::update(&state.metadata, &tenant, id, &patch).await {
        Ok(rec) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_QUERY_KIND, rec.id.to_string())
                    .with_tenant(&tenant),
                op: Op::Update,
                before: Some(query_kind_snapshot_json(&before)),
                after: Some(query_kind_snapshot_json(&rec)),
                resource_version: None,
                correlation: None,
            };
            if let Err(e) = record(
                &state.changelog.registry,
                state.metadata.clone(),
                &tenant,
                actor_from(caller),
                draft,
            )
            .await
            {
                tracing::warn!(error = %e, "failed to record query-kind update");
            }
            Json(to_detail(&rec)).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
