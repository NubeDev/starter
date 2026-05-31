//! `GET /v1/authz/resources/:kind/instances` — enumerate the
//! instances of a registered resource kind, scoped to the
//! caller's tenant, with effective-ACL summaries the admin UI
//! renders without needing to talk to the rules table directly.

use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use starter_spi::auth::Principal;

use crate::instances::{InstancesError, InstancesQuery};

use super::state::AuthzRoutesState;

pub(super) async fn list_instances(
    Extension(state): Extension<Arc<AuthzRoutesState>>,
    Extension(principal): Extension<Principal>,
    Path(kind): Path<String>,
    Query(query): Query<InstancesQuery>,
) -> Response {
    let Some(registry) = state.instances.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(provider) = registry.get(&kind) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Tenant scope: a super-admin (`tenant_id == "*"`) can pass
    // `?tenant=` to inspect another tenant; everyone else is
    // pinned to their own binding. Bare principals with no tenant
    // and no override get 400 — there's nothing meaningful to
    // list.
    let is_super = principal.tenant_id.as_deref() == Some("*");
    let tenant_id = if is_super {
        match query.tenant.clone() {
            Some(t) => t,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "super-admin must pass ?tenant=<slug>"
                    })),
                )
                    .into_response();
            }
        }
    } else {
        match principal.tenant_id.clone() {
            Some(t) => t,
            None => return StatusCode::FORBIDDEN.into_response(),
        }
    };

    match provider.list(&principal, &tenant_id, query).await {
        Ok(page) => Json(page).into_response(),
        Err(InstancesError::Forbidden) => StatusCode::FORBIDDEN.into_response(),
        Err(InstancesError::Backend(msg)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response(),
    }
}
