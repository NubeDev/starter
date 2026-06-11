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
use nexus_store::datasource::{mqtt, zenoh};
use starter_spi::auth::Principal;
use starter_spi::Error;

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
        DatasourceKind::Mqtt => probe_mqtt(&req).await,
        DatasourceKind::Zenoh => probe_zenoh(&req).await,
    };
    Json(outcome).into_response()
}

/// Probe a Postgres config and shape the store result into the wire outcome. A
/// connect failure is a `{ ok: false, message }` body, not an HTTP error.
async fn probe_postgres(req: &TestConnectionRequest) -> TestDatasourceResponse {
    let started = Instant::now();
    let params = ProbeParams {
        host: req.host.as_deref().unwrap_or_default(),
        port: req.port.unwrap_or_default(),
        database: req.database.as_deref().unwrap_or_default(),
        user: req.user.as_deref().unwrap_or_default(),
        secret: req.password.as_deref().unwrap_or_default(),
    };
    shape(started, postgres::probe(params).await)
}

/// Probe an MQTT broker described by the stream connector's `config` block.
async fn probe_mqtt(req: &TestConnectionRequest) -> TestDatasourceResponse {
    let started = Instant::now();
    let cfg = match req.config.as_ref() {
        Some(c) => c,
        None => return failed(&missing("mqtt", "config")),
    };
    let host = match cfg.get("host").and_then(|v| v.as_str()) {
        Some(h) => h,
        None => return failed(&missing("mqtt", "config.host")),
    };
    let port = cfg.get("port").and_then(|v| v.as_u64()).unwrap_or(1883) as u16;
    let params = mqtt::ProbeParams {
        host,
        port,
        client_id: cfg
            .get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("nexus-probe"),
        user: cfg.get("username").and_then(|v| v.as_str()),
        password: cfg.get("password").and_then(|v| v.as_str()),
    };
    shape(started, mqtt::probe(params).await)
}

/// Probe a Zenoh fabric described by the stream connector's `config` block.
async fn probe_zenoh(req: &TestConnectionRequest) -> TestDatasourceResponse {
    let started = Instant::now();
    let cfg = match req.config.as_ref() {
        Some(c) => c,
        None => return failed(&missing("zenoh", "config")),
    };
    let endpoints: Vec<String> = cfg
        .get("endpoints")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mode = cfg.get("mode").and_then(|v| v.as_str()).unwrap_or("client");
    let params = zenoh::ProbeParams {
        endpoints: &endpoints,
        mode,
    };
    shape(started, zenoh::probe(params).await)
}

/// Shape a store probe result into the wire outcome: `ok` on success (with
/// latency), or a redacted `{ ok: false, message }` on failure.
fn shape(started: Instant, result: Result<(), Error>) -> TestDatasourceResponse {
    match result {
        Ok(()) => TestDatasourceResponse {
            ok: true,
            message: None,
            latency_ms: Some(elapsed_ms(started)),
        },
        Err(e) => failed(&e),
    }
}

/// A required probe field was absent — a malformed form submission, surfaced as a
/// failed probe rather than an HTTP error so the form shows it inline.
fn missing(kind: &str, field: &str) -> Error {
    Error::Invalid {
        message: format!("{kind} probe requires {field}"),
    }
}
