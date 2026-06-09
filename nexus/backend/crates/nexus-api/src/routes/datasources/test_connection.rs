//! `POST /api/v1/datasources/test` — probe connectivity for a *raw* config,
//! before any datasource is saved.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.
//!
//! The saved-datasource probe (`:id/test`) only works after a row exists, which
//! left the form unable to validate a connection the user is still typing. This
//! route closes that gap: it takes the connection fields directly, probes through
//! the store, and reports `{ ok, message }` — a failed probe is a normal form
//! outcome (200, `ok=false`), and the driver message is sanitized so it never
//! carries the connection secret. The tenant gate is the authenticated principal
//! itself: probing an arbitrary host needs no per-resource grant because nothing
//! is read, written, or persisted.

use std::time::Instant;

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::{DatasourceKind, TestConnectionRequest, TestDatasourceResponse};
use nexus_store::datasource::postgres::{self, ProbeParams};
use starter_spi::auth::Principal;

use crate::middleware::tenant::tenant_of;
use crate::routes::datasources::probe_outcome::{elapsed_ms, failed};
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/datasources/test",
    tag = "datasources",
    operation_id = "test_connection",
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Probe outcome (ok=false on a failed probe)", body = TestDatasourceResponse),
        (status = 400, description = "Connector kind cannot be probed pre-save"),
    ),
)]
pub async fn test_connection(
    State(_state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<TestConnectionRequest>,
) -> axum::response::Response {
    if let Err(resp) = tenant_of(&principal) {
        return resp;
    }
    let outcome = match req.kind {
        DatasourceKind::Postgres => probe_postgres(&req).await,
    };
    Json(outcome).into_response()
}

/// Probe a Postgres config and shape the store result into the wire outcome. A
/// connect failure is a `{ ok: false, message }` body, not an HTTP error.
async fn probe_postgres(req: &TestConnectionRequest) -> TestDatasourceResponse {
    let started = Instant::now();
    let params = ProbeParams {
        host: &req.host,
        port: req.port,
        database: &req.database,
        user: &req.user,
        secret: &req.password,
    };
    match postgres::probe(params).await {
        Ok(()) => TestDatasourceResponse {
            ok: true,
            message: None,
            latency_ms: Some(elapsed_ms(started)),
        },
        Err(e) => failed(&e),
    }
}
