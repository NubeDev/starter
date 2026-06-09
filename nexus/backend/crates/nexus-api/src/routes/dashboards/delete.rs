//! `DELETE /api/v1/dashboards/:slug` — remove a dashboard and its panels.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use crate::authz::{self, ACTION_DELETE, KIND_DASHBOARD};
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::dashboard_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/dashboards/{slug}",
    tag = "dashboards",
    operation_id = "delete_dashboard",
    params(("slug" = String, Path, description = "Dashboard slug")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
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
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match dashboard::delete(&state.metadata, &tenant, dash.id).await {
        Ok(_) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_DASHBOARD, dash.id.to_string())
                    .with_tenant(&tenant),
                op: Op::Delete,
                before: Some(dashboard_snapshot_json(&dash)),
                after: None,
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
                tracing::warn!(error = %e, "failed to record dashboard delete");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
