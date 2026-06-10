//! `GET /api/v1/audit/resources/{kind}/{id}` — one resource's history timeline.
//!
//! Powers a "History" tab on a dashboard/datasource: every change to one
//! resource, newest first, paged. A thin wrapper over [`ChangeLog::list`] with a
//! resource-scoped filter, sharing the audit-read gate and tenant pin with the
//! collection list.
//!
//! LAYER: transport (REST).

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use starter_changelog::{ChangeFilter, ChangeLog as _, ChangePage};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use nexus_store::changelog::NexusChangeLog;

use super::gate::require_audit_read;
use crate::state::AppState;

/// The page-control half of [`ChangeFilter`] a single-resource timeline accepts.
/// The resource is pinned by the path, so only paging and the time window are
/// caller-supplied here — separating them keeps the path from silently competing
/// with a `resource_kind` query param.
#[derive(Debug, Default, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct HistoryPage {
    /// Inclusive lower bound on `at` (RFC 3339).
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    /// Exclusive upper bound on `at` (RFC 3339).
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    /// Page size (backend-capped).
    pub limit: Option<u32>,
    /// Opaque cursor from a previous page.
    pub cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/resources/{kind}/{id}",
    tag = "audit",
    operation_id = "resource_history",
    params(
        ("kind" = String, Path, description = "Resource kind, e.g. nexus.dashboard"),
        ("id" = String, Path, description = "Resource id"),
        HistoryPage,
    ),
    responses(
        (status = 200, description = "Resource history (newest first)", body = ChangePage),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not an admin / no tenant binding"),
    ),
)]
pub async fn resource_history(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path((kind, id)): Path<(String, String)>,
    Query(page): Query<HistoryPage>,
) -> axum::response::Response {
    let tenant = match require_audit_read(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let filter = ChangeFilter {
        resource_kind: Some(kind),
        resource_id: Some(id),
        since: page.since,
        until: page.until,
        limit: page.limit,
        cursor: page.cursor,
        ..Default::default()
    };
    let log = NexusChangeLog::new(state.metadata.clone(), tenant);
    match log.list(&filter).await {
        Ok(page) => Json(page).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
