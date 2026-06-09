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

use super::convert::to_panel;
use crate::middleware::tenant::tenant_of;
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
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    let new = NewPanel {
        dashboard_id: dash.id,
        datasource_id: Some(req.datasource_id),
        title: req.title,
        sql: req.sql,
        viz: req.viz.unwrap_or_else(|| "table".into()),
        layout: req.layout.unwrap_or_else(|| json!({})),
    };
    match dashboard::panel::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_panel(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
