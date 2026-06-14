//! `GET /api/v1/dashboards/:slug/export` — the portable dashboard JSON model.
//!
//! LAYER: transport (REST). Resolve → authorize (`view`) → gather → shape DTO.
//! Emits a self-contained [`DashboardExport`] (appearance + panels + variables)
//! that `POST /dashboards/import` can re-create from. See contract C1.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::{
    DashboardExport, PanelExport, VariableExport, DASHBOARD_SCHEMA_VERSION,
};
use nexus_store::{dashboard, variable};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use crate::authz::{self, ACTION_VIEW, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/dashboards/{slug}/export",
    tag = "dashboards",
    operation_id = "export_dashboard",
    params(("slug" = String, Path, description = "Dashboard slug")),
    responses(
        (status = 200, description = "Portable dashboard model", body = DashboardExport),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn export_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_VIEW,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    let panels = match dashboard::panel::list_for_dashboard(&state.metadata, &tenant, dash.id).await
    {
        Ok(p) => p,
        Err(e) => return IntoResponse(e).into_response(),
    };
    let variables = match variable::list_for_dashboard(&state.metadata, &tenant, dash.id).await {
        Ok(v) => v,
        Err(e) => return IntoResponse(e).into_response(),
    };
    let export = DashboardExport {
        schema_version: DASHBOARD_SCHEMA_VERSION,
        slug: dash.slug,
        name: dash.name,
        icon: dash.icon,
        accent: dash.accent,
        panels: panels
            .iter()
            .map(|p| PanelExport {
                title: p.title.clone(),
                datasource_id: p.datasource_id,
                sql: p.sql.clone(),
                viz: p.viz.clone(),
                layout: p.layout.clone(),
                insight_id: p.insight_id,
                insight_params: p.insight_params.clone(),
            })
            .collect(),
        variables: variables
            .iter()
            .map(|v| VariableExport {
                name: v.name.clone(),
                label: v.label.clone(),
                kind: v.kind.clone(),
                options_config: v.options_config.clone(),
                current: v.current.clone(),
                multi: v.multi,
                include_all: v.include_all,
                hidden: v.hidden,
                sort_order: v.sort_order,
            })
            .collect(),
    };
    Json(export).into_response()
}
