//! `PATCH /api/v1/nav/{id}` — retitle / reparent / reorder / retarget a node.
//!
//! Partial: omitted fields are unchanged. The wire's `clear_*` flags collapse
//! into the store's three-valued patch (clear wins, then an explicit value, else
//! leave). A new `dashboard` target is re-validated in-tenant. Authorized as
//! `edit` on the node; recorded as a reversible `Change` (C6).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::nav::{NavNodeDetail, NavTarget, UpdateNavNodeRequest};
use nexus_store::nav_node::{self, NavNodePatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;
use uuid::Uuid;

use super::convert::{context_to_json, target_to_json, to_detail, validate_target};
use crate::authz::{self, ACTION_EDIT, KIND_NAV_NODE};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::nav_node_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/nav/{id}",
    tag = "nav",
    operation_id = "update_nav",
    params(("id" = Uuid, Path, description = "Nav node id")),
    request_body = UpdateNavNodeRequest,
    responses(
        (status = 200, description = "Updated", body = NavNodeDetail),
        (status = 400, description = "Dashboard target not found in this tenant"),
        (status = 403, description = "Not allowed to edit this node"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_nav(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNavNodeRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Resolve under RLS (404 for absent/foreign), then authorize `edit` before
    // mutating. The pre-update row is the `before` snapshot the undo log needs.
    let before = match nav_node::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(n)) => n,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_NAV_NODE,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    if let Some(target) = &req.target {
        if let Err(e) = validate_target(&state.metadata, &tenant, target).await {
            return IntoResponse(e).into_response();
        }
    }

    // Collapse the JSON-friendly value+clear pairs into the store's three-valued
    // Option<Option<_>> patch: clear wins, then an explicit value, else leave.
    let parent_id = if req.clear_parent {
        Some(None)
    } else {
        req.parent_id.map(Some)
    };
    let context = if req.clear_context {
        Some(None)
    } else {
        req.context.as_ref().map(|c| context_to_json(Some(c)))
    };
    let icon = if req.clear_icon {
        Some(None)
    } else {
        req.icon.map(Some)
    };
    let accent = if req.clear_accent {
        Some(None)
    } else {
        req.accent.map(Some)
    };
    let patch = NavNodePatch {
        parent_id,
        title: req.title,
        sort_order: req.sort_order,
        target: req.target.as_ref().map(target_to_json),
        context,
        icon,
        accent,
    };

    match nav_node::update(&state.metadata, &tenant, id, &patch).await {
        Ok(Some(rec)) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_NAV_NODE, rec.id.to_string()).with_tenant(&tenant),
                op: Op::Update,
                before: Some(nav_node_snapshot_json(&before)),
                after: Some(nav_node_snapshot_json(&rec)),
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
                tracing::warn!(error = %e, "failed to record nav node update");
            }
            // A dashboard→group retarget that didn't also clear context would
            // leave a dangling payload; the UI sends clear_context, but guard the
            // returned shape so a `group` never reports a context.
            let mut detail = to_detail(&rec);
            if !matches!(detail.target, NavTarget::Dashboard { .. }) {
                detail.context = None;
            }
            Json(detail).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
