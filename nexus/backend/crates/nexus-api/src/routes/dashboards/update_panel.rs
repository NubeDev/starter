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

use super::convert::to_panel;
use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
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
    // Updating a panel mutates its dashboard — authorize `edit` on the owning
    // dashboard. A panel not visible to the tenant is a 404 (RLS hid it).
    let dashboard_id = match dashboard::panel::dashboard_id_of(&state.metadata, &tenant, id).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DASHBOARD,
        &dashboard_id.to_string(),
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
    };
    match dashboard::panel::update(&state.metadata, &tenant, id, &patch).await {
        Ok(Some(rec)) => Json(to_panel(&rec)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
