//! Health endpoint + transport-layer entry point.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See [docs/design/tools/](../../docs/design/tools/README.md) for
//! the dispatch-only handler rule that applies to every route file
//! in this crate.
//!
//! Owns `/healthz` (a minimal liveness probe) and the
//! [`serve`] entry point that binds a listener and runs an
//! [`axum::Router`] the binary composed from the per-verb sub-
//! routers under [`crate::routes`].

use anyhow::Result;
use axum::{routing::get, Router};
use tracing::info;

/// The single liveness route — exported so `main.rs` can merge it
/// alongside the tool routes without re-stating the endpoint here.
pub fn healthz_router() -> Router {
    Router::new().route("/healthz", get(healthz))
}

/// Bind and serve `router` on `bind`. The router is whatever
/// `main.rs` composed (typically `healthz_router().merge(...)`).
pub async fn serve(bind: &str, router: Router) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!(bind = %bind, "rubix-agent listening");
    axum::serve(listener, router).await?;
    Ok(())
}

/// Liveness probe. Returns 200 with a tiny JSON body — no DB, no
/// downstream calls. A reachable port is the entire signal.
async fn healthz() -> &'static str {
    r#"{"status":"ok"}"#
}
