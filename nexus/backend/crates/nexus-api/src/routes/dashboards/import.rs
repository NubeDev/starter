//! `POST /api/v1/dashboards/import` — re-create a dashboard from its portable
//! JSON model (contract C1, WS-05).
//!
//! LAYER: transport (REST). Validate the model version → create the dashboard,
//! its panels, and its variables under fresh ids in the caller's tenant → record
//! a Create change → return the new summary. A panel whose `datasource_id` is not
//! present in the importing tenant is filed with its datasource unset rather than
//! failing the whole import (the C1 cross-tenant rule).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::{DashboardExport, DashboardSummary, DASHBOARD_SCHEMA_VERSION};
use nexus_store::dashboard::{self, NewDashboard, NewPanel};
use nexus_store::variable::{self, NewVariable};
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::to_summary;
use crate::authz::KIND_DASHBOARD;
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::dashboard_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/dashboards/import",
    tag = "dashboards",
    operation_id = "import_dashboard",
    request_body = DashboardExport,
    responses(
        (status = 200, description = "Imported", body = DashboardSummary),
        (status = 400, description = "Unsupported schema version"),
        (status = 409, description = "Slug already used in this tenant"),
    ),
)]
pub async fn import_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(model): Json<DashboardExport>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Reject a model this server does not understand rather than importing it
    // partially under a mismatched shape.
    if model.schema_version != DASHBOARD_SCHEMA_VERSION {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "unsupported dashboard schema_version {} (this server understands {})",
                model.schema_version, DASHBOARD_SCHEMA_VERSION
            ),
        )
            .into_response();
    }

    // Create the dashboard shell first; a slug clash surfaces as 409 here before
    // any panels/variables are written.
    let new = NewDashboard {
        slug: model.slug.clone(),
        name: model.name.clone(),
        icon: model.icon.clone(),
        accent: model.accent.clone(),
        folder_id: None,
    };
    let dash = match dashboard::insert(&state.metadata, &tenant, &new).await {
        Ok(d) => d,
        Err(e) => return IntoResponse(e).into_response(),
    };

    // Panels, in export order. A datasource not present in this tenant is dropped
    // to NULL (C1) rather than tripping the panel insert's FK check.
    for p in &model.panels {
        let datasource_id = match p.datasource_id {
            Some(id) => match datasource::get(&state.metadata, &tenant, id).await {
                Ok(Some(_)) => Some(id),
                Ok(None) => None,
                Err(e) => return IntoResponse(e).into_response(),
            },
            None => None,
        };
        let panel = NewPanel {
            dashboard_id: dash.id,
            datasource_id,
            title: p.title.clone(),
            sql: p.sql.clone(),
            viz: p.viz.clone(),
            layout: p.layout.clone(),
        };
        if let Err(e) = dashboard::panel::insert(&state.metadata, &tenant, &panel).await {
            return IntoResponse(e).into_response();
        }
    }

    // Variables, preserving authoring order.
    for v in &model.variables {
        let var = NewVariable {
            dashboard_id: dash.id,
            name: v.name.clone(),
            label: v.label.clone(),
            kind: v.kind.clone(),
            options_config: v.options_config.clone(),
            current: v.current.clone(),
            multi: v.multi,
            include_all: v.include_all,
            hidden: v.hidden,
            sort_order: v.sort_order,
        };
        if let Err(e) = variable::insert(&state.metadata, &tenant, &var).await {
            return IntoResponse(e).into_response();
        }
    }

    record_create(&state, principal, &tenant, &dash).await;
    Json(to_summary(&dash)).into_response()
}

/// Record the imported dashboard's creation as a reversible `Change` (C6). The
/// panels/variables ride the dashboard's snapshot via the cascade delete, matching
/// the create handler's recording shape.
async fn record_create(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    rec: &dashboard::DashboardRecord,
) {
    let draft = ChangeDraft {
        resource: ResourceRef::row(KIND_DASHBOARD, rec.id.to_string()).with_tenant(tenant),
        op: Op::Create,
        before: None,
        after: Some(dashboard_snapshot_json(rec)),
        resource_version: None,
        correlation: None,
    };
    if let Err(e) = record(
        &state.changelog.registry,
        state.metadata.clone(),
        tenant,
        actor_from(principal),
        draft,
    )
    .await
    {
        tracing::warn!(error = %e, "failed to record dashboard import");
    }
}
