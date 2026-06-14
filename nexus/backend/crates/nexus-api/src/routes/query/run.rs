//! `POST /api/v1/query` — run one SQL statement against the datasource.

use axum::extract::{Extension, State};
use axum::Json;
use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use crate::state::AppState;

/// Extract the request, bind its macros/variables, run it under the server
/// guards, return the rows.
///
/// Two modes, distinguished by whether a verified `Principal` reached this route
/// (it sits behind the principal layer, so a logged-in session arrives as
/// `Some`):
///
/// - **Principal-bearing kind-mode** (`Some(principal)` + `req.kind`). Used by
///   product/extension UIs that read a contributed query-kind over an
///   **extension-owned table in the nexus metadata DB** (e.g.
///   `com.acme.devices.devices_list` over `com_acme_devices__devices`). We build
///   a `QueryIdentity` from the verified principal so the host tokens
///   `$caller_tenant_id` / `$caller_team_ids` bind (un-spoofable, server-bound),
///   and run against `state.metadata` where those tables live. This is the read
///   path WS-17's owned-table demo needs — without it the kind's host tokens
///   have nothing to bind and the request 400s.
/// - **Dev single-datasource shortcut** (no principal, or raw-SQL mode). Keeps
///   today's behaviour exactly: `QueryIdentity::default()` against
///   `state.datasource`. Host tokens are absent, so a query needing them errors,
///   which is correct for the unauthenticated ad-hoc path.
///
/// The guards (read-only, timeout, caps) and the binder live in the store; this
/// handler only wires identity + pool selection.
#[utoipa::path(
    post,
    path = "/api/v1/query",
    tag = "query",
    operation_id = "run_query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse),
        (status = 400, description = "Invalid or rejected query", body = nexus_spi::Problem),
    ),
)]
pub async fn run_query(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, IntoResponse> {
    // A principal-bearing kind-mode read runs tenant-scoped against the metadata
    // DB; everything else stays on the unauthenticated dev datasource shortcut.
    let principal_kind_read = req.kind.is_some()
        && principal
            .as_ref()
            .is_some_and(|Extension(p)| p.tenant_id.is_some());

    let (pool, identity, scope, tenant) = if principal_kind_read {
        let Extension(p) = principal.as_ref().expect("checked above");
        let identity = nexus_store::QueryIdentity {
            tenant_id: p.tenant_id.clone(),
            user_id: Some(p.subject.clone()),
            teams: p.teams.clone(),
        };
        (&state.metadata, identity, "metadata", p.tenant_id.clone())
    } else {
        (
            &state.datasource,
            nexus_store::QueryIdentity::default(),
            "dev",
            None,
        )
    };

    let result = crate::cache::run_cached(&state, pool, &req, &identity, scope)
        .await
        .map_err(IntoResponse)?;
    // RW-06 insight seam: a tenant-scoped read may reference a stored insight;
    // the dev path (no tenant) still only accepts an inline script.
    let result = match &req.insight {
        Some(insight) => crate::insights::apply_insight(
            &state,
            &state.metadata,
            tenant.as_deref(),
            insight,
            result,
        )
        .await
        .map_err(IntoResponse)?,
        None => result,
    };
    Ok(Json(result))
}
