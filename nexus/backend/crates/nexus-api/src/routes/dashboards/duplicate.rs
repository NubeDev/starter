//! `POST /api/v1/dashboards/:slug/duplicate` — copy a dashboard with its panels
//! and variables under a fresh id (WS-05).
//!
//! LAYER: transport (REST). Resolve → authorize (`view` on the source) → copy the
//! dashboard, its panels, and its variables under new ids → record a Create.
//! Duplication runs here, not through the changelog `clone_with` path, because a
//! bare row clone would orphan the panels; this copies the whole tree.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::DashboardSummary;
use nexus_store::dashboard::{self, NewDashboard, NewPanel};
use nexus_store::variable::{self, NewVariable};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::to_summary;
use crate::authz::{self, ACTION_VIEW, KIND_DASHBOARD};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::dashboard_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/dashboards/{slug}/duplicate",
    tag = "dashboards",
    operation_id = "duplicate_dashboard",
    params(("slug" = String, Path, description = "Source dashboard slug")),
    responses(
        (status = 200, description = "Duplicated", body = DashboardSummary),
        (status = 404, description = "Source not found in this tenant"),
        (status = 409, description = "Derived slug already used in this tenant"),
    ),
)]
pub async fn duplicate_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let src = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_VIEW,
        KIND_DASHBOARD,
        &src.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    // The copy keeps the source's folder and appearance but takes a derived slug
    // and name so it does not collide with the original. A clash on the derived
    // slug surfaces as 409 — the caller renames and retries.
    let new = NewDashboard {
        slug: format!("{}-copy", src.slug),
        name: format!("{} (copy)", src.name),
        icon: src.icon.clone(),
        accent: src.accent.clone(),
        folder_id: src.folder_id,
    };
    let copy = match dashboard::insert(&state.metadata, &tenant, &new).await {
        Ok(d) => d,
        Err(e) => return IntoResponse(e).into_response(),
    };

    // Copy panels in canvas order under the new dashboard id.
    let panels = match dashboard::panel::list_for_dashboard(&state.metadata, &tenant, src.id).await
    {
        Ok(p) => p,
        Err(e) => return IntoResponse(e).into_response(),
    };
    for p in &panels {
        let panel = NewPanel {
            dashboard_id: copy.id,
            datasource_id: p.datasource_id,
            title: p.title.clone(),
            sql: p.sql.clone(),
            viz: p.viz.clone(),
            layout: p.layout.clone(),
            // Same tenant, so the insight id stays valid on the copy.
            insight_id: p.insight_id,
            insight_params: p.insight_params.clone(),
        };
        if let Err(e) = dashboard::panel::insert(&state.metadata, &tenant, &panel).await {
            return IntoResponse(e).into_response();
        }
    }

    // Copy variables, preserving authoring order.
    let variables = match variable::list_for_dashboard(&state.metadata, &tenant, src.id).await {
        Ok(v) => v,
        Err(e) => return IntoResponse(e).into_response(),
    };
    for v in &variables {
        let var = NewVariable {
            dashboard_id: copy.id,
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

    let draft = ChangeDraft {
        resource: ResourceRef::row(KIND_DASHBOARD, copy.id.to_string()).with_tenant(&tenant),
        op: Op::Create,
        before: None,
        after: Some(dashboard_snapshot_json(&copy)),
        resource_version: None,
        correlation: None,
    };
    if let Err(e) = record(
        &state.changelog.registry,
        state.metadata.clone(),
        &tenant,
        actor_from(principal),
        draft,
    )
    .await
    {
        tracing::warn!(error = %e, "failed to record dashboard duplicate");
    }
    Json(to_summary(&copy)).into_response()
}
