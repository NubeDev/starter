//! `PATCH /api/v1/panels/:id` — update a panel's fields (canvas layout-save).
//!
//! The UI persists a drag/resize by sending the changed `layout`; the same route
//! also edits `title`/`sql`/`datasource_id`/`viz` so a panel is editable without
//! delete + re-add. The patch is partial — omitted fields are left unchanged.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::panel::{PanelDetail, UpdatePanelRequest};
use nexus_store::dashboard::{self, PanelPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::to_panel;
use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD, KIND_PANEL};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::panel_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/panels/{id}",
    tag = "panels",
    operation_id = "update_panel",
    params(("id" = Uuid, Path, description = "Panel id")),
    request_body = UpdatePanelRequest,
    responses(
        (status = 200, description = "Panel updated", body = PanelDetail),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_panel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePanelRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Read the full panel up front: it is both the authz target (via its owning
    // `dashboard_id`) and the `before` snapshot the undo log reverts to. Reading
    // it inside the tenant tx means RLS-hidden panels read as absent → a 404, and
    // the recorded `before` is guaranteed non-empty on a successful update.
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
    let patch = PanelPatch {
        title: req.title,
        datasource_id: req.datasource_id,
        sql: req.sql,
        viz: req.viz,
        layout: req.layout,
        // Both are already three-valued on the wire (Option<Option<_>>), mapping
        // 1:1 onto the store patch: None = leave, Some(None) = detach/clear,
        // Some(Some(_)) = set.
        insight_id: req.insight_id,
        insight_params: req.insight_params,
    };
    match dashboard::panel::update(&state.metadata, &tenant, id, &patch).await {
        Ok(Some(rec)) => {
            // `before` was read above (pre-mutation); `after` is what we wrote.
            // Recording both lets undo restore the prior panel state and redo
            // re-apply the edit. Logged-not-surfaced on failure.
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_PANEL, rec.id.to_string()).with_tenant(&tenant),
                op: Op::Update,
                before: Some(panel_snapshot_json(&before)),
                after: Some(panel_snapshot_json(&rec)),
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
                tracing::warn!(error = %e, "failed to record panel update");
            }
            Json(to_panel(&rec)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
