//! `DELETE /api/v1/nav/{id}` — remove a nav node (children re-root).
//!
//! Authorized as `delete` on the node; recorded as a reversible `Change` (C6)
//! so a delete is undoable (the `Reversible` resurrects under the original id).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::nav_node;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;
use uuid::Uuid;

use crate::authz::{self, ACTION_DELETE, KIND_NAV_NODE};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::nav_node_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/nav/{id}",
    tag = "nav",
    operation_id = "delete_nav",
    params(("id" = Uuid, Path, description = "Nav node id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not allowed to delete this node"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_nav(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Resolve the `before` snapshot (also a 404 for absent/foreign), authorize
    // `delete`, then remove — recording the delete for undo.
    let before = match nav_node::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(n)) => n,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_NAV_NODE,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match nav_node::delete(&state.metadata, &tenant, id).await {
        Ok(true) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_NAV_NODE, id.to_string()).with_tenant(&tenant),
                op: Op::Delete,
                before: Some(nav_node_snapshot_json(&before)),
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
                tracing::warn!(error = %e, "failed to record nav node delete");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
