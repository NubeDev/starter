//! `GET /api/v1/query-kinds` — the caller's tenant-authored query-kinds, in full.
//!
//! The admin management list: it carries each kind's `sql`, unlike the picker
//! catalogue at `/api/v1/query/kinds` which hides it. Filtered to the kinds the
//! caller may view, mirroring the agents list gate.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query_kind::QueryKindDetail;
use nexus_store::query_kind;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_QUERY_KIND};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/query-kinds",
    tag = "query-kinds",
    operation_id = "list_query_kinds_admin",
    responses((status = 200, description = "Query-kinds", body = [QueryKindDetail])),
)]
pub async fn list_query_kinds_admin(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let recs = match query_kind::list(&state.metadata, &tenant).await {
        Ok(r) => r,
        Err(e) => return IntoResponse(e).into_response(),
    };
    // Drop kinds the caller may not view, mirroring the agents/dashboards gate.
    let mut out = Vec::with_capacity(recs.len());
    for rec in &recs {
        if authz::can(
            state.engine.as_ref(),
            caller,
            ACTION_VIEW,
            KIND_QUERY_KIND,
            &rec.id.to_string(),
            &tenant,
        )
        .await
        {
            out.push(to_detail(rec));
        }
    }
    Json(out).into_response()
}
