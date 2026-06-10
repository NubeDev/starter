//! `GET /api/v1/nav` — the caller's navigation tree, access-filtered.
//!
//! The tree *is* the access surface (WS-13 §6): a node is returned only if the
//! principal holds `view` on it. So the sidebar a user sees is exactly the set
//! of nodes granted to them — granting "Building-1" but not "Building-2" returns
//! only Building-1 though both mount the same page.
//!
//! A node whose parent is filtered out is still returned (it re-roots in the
//! client tree) so a granted leaf under an ungranted group is not orphaned out
//! of view — the grant is per node, not inherited down the branch.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::nav::NavNodeDetail;
use nexus_store::nav_node;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_NAV_NODE};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/nav",
    tag = "nav",
    operation_id = "list_nav",
    responses((status = 200, description = "The caller's nav tree", body = [NavNodeDetail])),
)]
pub async fn list_nav(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rows = match nav_node::list(&state.metadata, &tenant).await {
        Ok(rows) => rows,
        Err(e) => return IntoResponse(e).into_response(),
    };
    let mut visible = Vec::with_capacity(rows.len());
    for row in &rows {
        if authz::can(
            state.engine.as_ref(),
            caller,
            ACTION_VIEW,
            KIND_NAV_NODE,
            &row.id.to_string(),
            &tenant,
        )
        .await
        {
            visible.push(to_detail(row));
        }
    }
    Json(visible).into_response()
}
