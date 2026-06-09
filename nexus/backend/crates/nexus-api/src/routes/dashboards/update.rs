//! `PATCH /api/v1/dashboards/:slug` — rename or re-slug a dashboard.
//!
//! The patch is partial — omitted fields are left unchanged. Re-slugging changes
//! only the route alias; grants and panel refs key on the immutable id, so
//! nothing is orphaned. Authorized as `edit` on the dashboard, like a panel edit.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::{DashboardSummary, UpdateDashboardRequest};
use nexus_store::dashboard::{self, DashboardPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::to_summary;
use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::dashboard_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/dashboards/{slug}",
    tag = "dashboards",
    operation_id = "update_dashboard",
    params(("slug" = String, Path, description = "Dashboard slug")),
    request_body = UpdateDashboardRequest,
    responses(
        (status = 200, description = "Dashboard updated", body = DashboardSummary),
        (status = 404, description = "Not found in this tenant"),
        (status = 409, description = "New slug already used in this tenant"),
    ),
)]
pub async fn update_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
    Json(req): Json<UpdateDashboardRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Resolve the slug to the immutable id (a 404 if RLS hides it), then
    // authorize `edit` on that dashboard before mutating it.
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    // Collapse the JSON-friendly `folder_id` + `clear_folder` pair into the
    // store's three-valued patch: clear wins, then an explicit folder, else leave.
    let folder_id = if req.clear_folder {
        Some(None)
    } else {
        req.folder_id.map(Some)
    };
    let patch = DashboardPatch {
        name: req.name,
        slug: req.slug,
        icon: req.icon,
        accent: req.accent,
        folder_id,
        starred: req.starred,
    };
    // The pre-update row is the `before` snapshot the undo log needs; `dash` was
    // read by id above for the authz check, so reuse it.
    let before = dash.clone();
    match dashboard::update(&state.metadata, &tenant, dash.id, &patch).await {
        Ok(Some(rec)) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_DASHBOARD, rec.id.to_string()).with_tenant(&tenant),
                op: Op::Update,
                before: Some(dashboard_snapshot_json(&before)),
                after: Some(dashboard_snapshot_json(&rec)),
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
                tracing::warn!(error = %e, "failed to record dashboard update");
            }
            Json(to_summary(&rec)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
