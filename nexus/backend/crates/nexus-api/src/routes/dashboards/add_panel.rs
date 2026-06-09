//! `POST /api/v1/dashboards/:slug/panels` — add a panel to a dashboard.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::panel::{CreatePanelRequest, PanelDetail};
use nexus_store::dashboard::{self, NewPanel};
use serde_json::json;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

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
    post,
    path = "/api/v1/dashboards/{slug}/panels",
    tag = "panels",
    operation_id = "add_panel",
    params(("slug" = String, Path, description = "Dashboard slug")),
    request_body = CreatePanelRequest,
    responses(
        (status = 200, description = "Panel added", body = PanelDetail),
        (status = 404, description = "Dashboard not found in this tenant"),
    ),
)]
pub async fn add_panel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
    Json(req): Json<CreatePanelRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    // Adding a panel mutates the dashboard, so it is an `edit` on the dashboard.
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
    let new = NewPanel {
        dashboard_id: dash.id,
        datasource_id: Some(req.datasource_id),
        title: req.title,
        sql: req.sql,
        viz: req.viz.unwrap_or_else(|| "table".into()),
        layout: req.layout.unwrap_or_else(|| json!({})),
    };
    match dashboard::panel::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => {
            // Record the create so undo reverts *this panel*, not the dashboard's
            // creation (the bug this kind was added to fix). A create has no
            // `before`; `after` is the full panel snapshot. A recording failure is
            // logged, never surfaced — the panel is already committed.
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_PANEL, rec.id.to_string()).with_tenant(&tenant),
                op: Op::Create,
                before: None,
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
                tracing::warn!(error = %e, "failed to record panel create");
            }
            Json(to_panel(&rec)).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
