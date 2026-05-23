//! Health endpoint.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.
//!
//! Serves `/healthz` with a minimal axum router. Used both as a
//! liveness probe and as the smallest possible startup smoke test.

use anyhow::Result;
use axum::{routing::get, Router};
use tracing::info;

/// Build the `/healthz` router and serve on `bind`.
pub async fn serve(bind: &str) -> Result<()> {
    let app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!(bind = %bind, "rubix-agent listening on /healthz");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Liveness probe. Returns 200 with a tiny JSON body — no DB, no
/// downstream calls. A reachable port is the entire signal.
async fn healthz() -> &'static str {
    r#"{"status":"ok"}"#
}
