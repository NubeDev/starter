//! `POST /api/v1/datasources/:id/test` — probe a registered datasource's
//! connectivity without running user SQL.
//!
//! The datasource form's "Test connection" affordance calls this. It resolves
//! the caller's own datasource (same `view` gate as the read/query routes),
//! builds or reuses the pool through the audited decrypt boundary, and runs a
//! trivial `SELECT 1` to force a real round-trip. A connect/probe failure is
//! reported as `{ ok: false, message }` with a 200 — a failed probe is a normal
//! outcome of the form, not a request error — and the driver message is
//! sanitized so it never carries the connection secret.

use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::TestDatasourceResponse;
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
use crate::routes::datasources::probe_outcome::{elapsed_ms, failed};
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/datasources/{id}/test",
    tag = "datasources",
    operation_id = "test_datasource",
    params(("id" = Uuid, Path, description = "Datasource id")),
    responses(
        (status = 200, description = "Probe outcome (ok=false on a failed probe)", body = TestDatasourceResponse),
        (status = 403, description = "Not authorized to view this datasource"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn test_datasource(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller_principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match datasource::get(&state.metadata, &tenant, id).await {
        Ok(Some(rec)) => rec,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller_principal,
        ACTION_VIEW,
        KIND_DATASOURCE,
        &rec.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    // From here a failure is a *probe* result, not an HTTP error: the form wants
    // `{ ok: false, message }` back so it can show the user why the connection
    // didn't work. `message` is the short error label only — never the secret.
    let started = Instant::now();
    let pool = match state
        .datasource_pools
        .get_or_connect(
            &state.metadata,
            &state.envelope,
            &tenant,
            &caller_principal.subject,
            &rec,
        )
        .await
    {
        Ok(p) => p,
        Err(e) => return Json(failed(&e)).into_response(),
    };
    // `get_or_connect` may hand back a cached pool whose connections are already
    // established, so run a trivial statement to force a real round-trip rather
    // than trust pool construction alone.
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => Json(TestDatasourceResponse {
            ok: true,
            message: None,
            latency_ms: Some(elapsed_ms(started)),
        })
        .into_response(),
        Err(e) => Json(failed_probe(&e.to_string())).into_response(),
    }
}

/// A `SELECT 1` failure on an already-built pool: shape the sanitized driver
/// message into the wire outcome. (The `failed` helper takes a domain error; here
/// we already hold the driver string, so shape it directly through the same
/// sanitizer-backed outcome.)
fn failed_probe(driver_message: &str) -> TestDatasourceResponse {
    let wrapped = starter_spi::Error::Invalid {
        message: driver_message.to_string(),
    };
    failed(&wrapped)
}
