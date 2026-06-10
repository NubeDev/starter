//! `POST /api/v1/nav` — create a nav node for the caller's tenant.
//!
//! A `dashboard` target is validated to exist **within the caller's tenant**
//! before insert (WS-13 §4): `nexus_dashboards.id` is a global PK, so a node
//! must not point at another tenant's page. Group/route targets need no check.
//! The create is recorded as a reversible `Change` (C6).

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::nav::{CreateNavNodeRequest, NavNodeDetail, NavTarget};
use nexus_store::nav_node::{self, NewNavNode};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_undo::ChangeDraft;

use super::convert::{context_to_json, target_to_json, to_detail, validate_target};
use crate::authz::KIND_NAV_NODE;
use crate::changelog::{actor_from, record};
use crate::middleware::tenant::caller;
use crate::reversible::nav_node_snapshot_json;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/nav",
    tag = "nav",
    operation_id = "create_nav",
    request_body = CreateNavNodeRequest,
    responses(
        (status = 200, description = "Created", body = NavNodeDetail),
        (status = 400, description = "Dashboard target not found in this tenant"),
    ),
)]
pub async fn create_nav(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateNavNodeRequest>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(e) = validate_target(&state.metadata, &tenant, &req.target).await {
        return IntoResponse(e).into_response();
    }
    // Context only travels with a dashboard mount; drop it for group/route so a
    // stray payload can't ride a non-dashboard node.
    let context = match req.target {
        NavTarget::Dashboard { .. } => req.context.as_ref(),
        _ => None,
    };
    let new = NewNavNode {
        parent_id: req.parent_id,
        title: req.title,
        sort_order: req.sort_order,
        target: target_to_json(&req.target),
        context: context_to_json(context),
        icon: req.icon,
        accent: req.accent,
    };
    match nav_node::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => {
            let draft = ChangeDraft {
                resource: ResourceRef::row(KIND_NAV_NODE, rec.id.to_string()).with_tenant(&tenant),
                op: Op::Create,
                before: None,
                after: Some(nav_node_snapshot_json(&rec)),
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
                tracing::warn!(error = %e, "failed to record nav node create");
            }
            Json(to_detail(&rec)).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
