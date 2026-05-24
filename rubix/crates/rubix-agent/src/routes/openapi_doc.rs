//! `GET /openapi.json` — serves the rubix-agent OpenAPI document.
//!
//! Mirrors `crates/starter-server/src/routes/openapi_doc.rs`: the
//! document is captured at boot (via [`crate::openapi::rubix_openapi`])
//! and served by value with no rebuild on request. The route is
//! mounted **unauthenticated** so the codegen pipeline
//! (`pnpm --filter @nube/rubix-client-ts run codegen`) and any
//! future API explorer can pull it without provisioning credentials,
//! matching the starter-server precedent. See
//! `rubix/docs/design/agent/README.md` for the boot wiring.

use axum::{routing::get, Json, Router};
use utoipa::openapi::OpenApi;

/// Build the `/openapi.json` router. The document is cloned per
/// request; cloning an `OpenApi` is cheap relative to a network
/// round-trip and avoids holding a long-lived borrow across the
/// service boundary.
pub fn openapi_router(doc: OpenApi) -> Router {
    Router::new().route("/openapi.json", get(move || serve(doc.clone())))
}

async fn serve(doc: OpenApi) -> Json<OpenApi> {
    Json(doc)
}
