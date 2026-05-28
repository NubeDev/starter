//! Health endpoint + transport-layer entry point.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See [docs/design/tools/](../../docs/design/tools/README.md) for
//! the dispatch-only handler rule that applies to every route file
//! in this crate.
//!
//! Owns `/healthz` (a minimal liveness probe), `/readyz` (a DB
//! readiness probe with a hard 1 s timeout — added during the
//! "agent stops responding" investigation: if the pool is
//! exhausted, `/readyz` returns 503 fast instead of timing out
//! along with every other auth-gated request), and the
//! [`serve`] entry point that binds a listener and runs an
//! [`axum::Router`] the binary composed from the per-verb sub-
//! routers under [`crate::routes`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::{routing::get, Router};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::routes::{RouteMeta, RouteRegistrar};

/// Hard ceiling on how long `/readyz` waits for a pool
/// acquire + `SELECT 1`. Picked to be shorter than every
/// realistic external probe timeout (curl default 0 = infinite,
/// k8s liveness 1 s, our investigation script 5 s) so the route
/// fails *loudly* the moment the pool starves.
const READYZ_PROBE_BUDGET: Duration = Duration::from_millis(1_000);

/// The single liveness route — exported so `main.rs` can merge it
/// alongside the tool routes without re-stating the endpoint here.
pub fn healthz_registrar() -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/healthz",
        get(healthz).with_state(()),
        RouteMeta::new()
            .describe("Process liveness canary; always 200 if the binary serves traffic.")
            .tag("system"),
    )
}

pub fn healthz_router() -> Router {
    healthz_registrar().into_router()
}

/// Runtime-canary route. Mounts `/livez`, which reports the
/// staleness of the [`boot::runtime_canary`] atomic. Returns 200
/// when the runtime ticked within
/// [`crate::boot::runtime_canary::STALENESS_BUDGET`], 503
/// otherwise — explicitly distinguishes "tokio runtime wedged"
/// (the canary atomic is stale) from "HTTP layer wedged" (the
/// canary is fresh but external probes still hang).
///
/// Cheap: no allocations beyond the small response body, no
/// futures::time::sleep, no I/O. Safe even when every other
/// route is jammed.
pub fn livez_registrar(canary: crate::boot::runtime_canary::Canary) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/livez",
        get(livez).with_state(Arc::new(canary)),
        RouteMeta::new()
            .describe("tokio runtime liveness; 503 when the per-second canary stops advancing.")
            .tag("system"),
    )
}

pub fn livez_router(canary: crate::boot::runtime_canary::Canary) -> Router {
    livez_registrar(canary).into_router()
}

async fn livez(
    State(canary): State<Arc<crate::boot::runtime_canary::Canary>>,
) -> (StatusCode, String) {
    let stale = canary.staleness().unwrap_or(Duration::ZERO);
    if stale <= crate::boot::runtime_canary::STALENESS_BUDGET {
        (
            StatusCode::OK,
            format!(r#"{{"status":"live","stale_secs":{}}}"#, stale.as_secs()),
        )
    } else {
        warn!(
            target: "rubix.livez",
            stale_secs = stale.as_secs(),
            budget_secs = crate::boot::runtime_canary::STALENESS_BUDGET.as_secs(),
            "runtime canary stale — tokio runtime may be wedged",
        );
        (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                r#"{{"status":"runtime-wedged","stale_secs":{}}}"#,
                stale.as_secs()
            ),
        )
    }
}

/// Readiness router. Mounted only when a Postgres pool is wired
/// in (laptop / no-DSN paths skip it). Returns:
///
///   - 200 `{"status":"ready","ms":<elapsed>}` when the probe
///     completes inside [`READYZ_PROBE_BUDGET`].
///   - 503 `{"status":"unready","reason":"…"}` on pool acquire
///     timeout or query error. Body carries the underlying
///     reason so an operator's `curl` shows the cause without
///     consulting the agent log.
///
/// Either outcome is logged at WARN with the same fields the
/// pool-telemetry task emits, so an /readyz miss correlates with
/// pool-stats lines from `boot::pool_telemetry`.
pub fn readyz_registrar(pool: PgPool) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/readyz",
        get(readyz).with_state(Arc::new(pool)),
        RouteMeta::new()
            .describe("DB readiness probe; 503 if the pool cannot serve SELECT 1 within 1s.")
            .tag("system"),
    )
}

pub fn readyz_router(pool: PgPool) -> Router {
    readyz_registrar(pool).into_router()
}

async fn readyz(State(pool): State<Arc<PgPool>>) -> (StatusCode, String) {
    let started = Instant::now();
    let probe = async {
        let mut conn = pool.acquire().await?;
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&mut *conn)
            .await
    };
    match tokio::time::timeout(READYZ_PROBE_BUDGET, probe).await {
        Ok(Ok(_)) => {
            let ms = started.elapsed().as_millis();
            (StatusCode::OK, format!(r#"{{"status":"ready","ms":{ms}}}"#))
        }
        Ok(Err(e)) => {
            warn!(
                target: "rubix.readyz",
                error = %e,
                elapsed_ms = started.elapsed().as_millis() as u64,
                pool_size = pool.size(),
                pool_idle = pool.num_idle(),
                "readiness probe sql error",
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                format!(
                    r#"{{"status":"unready","reason":"sql error: {}"}}"#,
                    escape_json(&e.to_string())
                ),
            )
        }
        Err(_) => {
            warn!(
                target: "rubix.readyz",
                budget_ms = READYZ_PROBE_BUDGET.as_millis() as u64,
                pool_size = pool.size(),
                pool_idle = pool.num_idle(),
                "readiness probe exceeded budget — pool likely saturated",
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                format!(
                    r#"{{"status":"unready","reason":"probe exceeded {} ms — pool saturated?"}}"#,
                    READYZ_PROBE_BUDGET.as_millis()
                ),
            )
        }
    }
}

/// Minimal JSON-safe escaper for the reason string. We only need
/// to escape `"` and `\` to keep the inline body parseable; the
/// rest of the payload is hard-coded ASCII.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Bind and serve `router` on `bind`. The router is whatever
/// `main.rs` composed (typically `healthz_router().merge(...)`).
pub async fn serve(bind: &str, router: Router) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    // Log the bound `local_addr` alongside the requested `bind` so
    // callers driving an ephemeral port (`RUBIX_BIND=127.0.0.1:0`,
    // used by `rubix/scripts/snapshot-openapi.sh` to capture
    // `rubix/openapi.json`) can discover the real port from a
    // line-buffered log stream. The original `bind` field is
    // preserved for back-compat with operators grep'ing logs.
    let local_addr = listener.local_addr()?;
    info!(bind = %bind, local_addr = %local_addr, "rubix-agent listening");
    axum::serve(listener, router).await?;
    Ok(())
}

/// Liveness probe. Returns 200 with a tiny JSON body — no DB, no
/// downstream calls. A reachable port is the entire signal.
#[utoipa::path(
    get,
    path = "/healthz",
    tag = "system",
    responses(
        (status = 200, description = "Agent is alive; body is `{\"status\":\"ok\"}`"),
    ),
)]
pub(crate) async fn healthz() -> &'static str {
    r#"{"status":"ok"}"#
}
