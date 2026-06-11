//! `POST /api/v1/dashboards` — create a dashboard for the caller's tenant.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::{CreateDashboardRequest, DashboardSummary};
use nexus_store::dashboard::{self, NewDashboard};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::to_summary;
use crate::authz::{self, KIND_DASHBOARD};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::dashboard_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/dashboards",
    tag = "dashboards",
    operation_id = "create_dashboard",
    request_body = CreateDashboardRequest,
    responses(
        (status = 200, description = "Created", body = DashboardSummary),
        (status = 409, description = "Slug already used in this tenant"),
    ),
)]
pub async fn create_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateDashboardRequest>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Kind-wide create gate: admins may, non-admins may not (an instance grant on
    // one dashboard never confers the ability to mint new ones).
    if let Err(resp) =
        authz::require_create(state.engine.as_ref(), principal, KIND_DASHBOARD, &tenant).await
    {
        return resp;
    }
    // Appearance is optional on the wire; fall back to the same defaults the
    // DB column carries so a name/slug-only client still gets a valid record.
    let new = NewDashboard {
        slug: req.slug,
        name: req.name,
        icon: req.icon.unwrap_or_else(|| "Activity".to_string()),
        accent: req.accent.unwrap_or_else(|| "152 76% 44%".to_string()),
        folder_id: req.folder_id,
    };
    match dashboard::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => {
            record_create(&state, principal, &tenant, &rec).await;
            Json(to_summary(&rec)).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// Record the create as a reversible `Change` (C6). A recording failure is logged
/// inside `record`'s caller, never surfaced — the dashboard is already committed.
async fn record_create(
    state: &AppState,
    principal: &Principal,
    tenant: &str,
    rec: &nexus_store::dashboard::DashboardRecord,
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
        tracing::warn!(error = %e, "failed to record dashboard create");
    }
}
