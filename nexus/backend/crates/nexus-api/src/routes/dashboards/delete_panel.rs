//! `DELETE /api/v1/panels/:id` — remove a panel.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD, KIND_PANEL};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::panel_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/panels/{id}",
    tag = "panels",
    operation_id = "delete_panel",
    params(("id" = Uuid, Path, description = "Panel id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_panel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Read the full panel up front: it is both the authz target (via its owning
    // `dashboard_id`) and the `before` snapshot undo-of-delete resurrects from. A
    // panel not visible to the tenant reads as absent → a 404 (RLS hid it).
    let before = match dashboard::panel::get(&state.metadata, &tenant, id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DASHBOARD,
        &before.dashboard_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match dashboard::panel::delete(&state.metadata, &tenant, id).await {
        Ok(true) => {
            // A delete records the full `before` (no `after`) so undo can
            // resurrect the panel under its original id. Logged-not-surfaced on
            // failure — the delete is already committed.
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_PANEL, before.id.to_string()).with_tenant(&tenant),
                op: Op::Delete,
                before: Some(panel_snapshot_json(&before)),
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
                tracing::warn!(error = %e, "failed to record panel delete");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
