//! `DELETE /api/v1/query-kinds/:id` — remove a tenant-authored query-kind.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::query_kind;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;
use uuid::Uuid;

use crate::authz::{self, ACTION_DELETE, KIND_QUERY_KIND};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::query_kind_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/query-kinds/{id}",
    tag = "query-kinds",
    operation_id = "delete_query_kind",
    params(("id" = Uuid, Path, description = "Query-kind id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not allowed to delete this query-kind"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_query_kind(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Existence-check before the gate (404 not masked as 403); the fetched row is
    // also the `before` snapshot the undo log needs to restore on undo.
    let before = match query_kind::get(&state.metadata, &tenant, id).await {
        Ok(r) => r,
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_QUERY_KIND,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match query_kind::delete(&state.metadata, &tenant, id).await {
        Ok(()) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_QUERY_KIND, before.id.to_string())
                    .with_tenant(&tenant),
                op: Op::Delete,
                before: Some(query_kind_snapshot_json(&before)),
                after: None,
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
                tracing::warn!(error = %e, "failed to record query-kind delete");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
