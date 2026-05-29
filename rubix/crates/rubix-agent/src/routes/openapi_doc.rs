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

use axum::http::Method;
use axum::routing::get;
use axum::Json;
use utoipa::openapi::OpenApi;

use crate::routes::{RouteMeta, RouteRegistrar};

/// Build the `/openapi.json` registrar. The document is cloned
/// per request; cloning an `OpenApi` is cheap relative to a
/// network round-trip and avoids holding a long-lived borrow
/// across the service boundary.
pub fn openapi_registrar(doc: OpenApi) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/openapi.json",
        get(move || serve(doc.clone())).with_state(()),
        RouteMeta::new()
            .describe("utoipa-derived OpenAPI document (merges starter-auth-users).")
            .tag("system"),
    )
}

/// Backwards-compatible alias.
pub fn openapi_router(doc: OpenApi) -> axum::Router {
    openapi_registrar(doc).into_router()
}

async fn serve(doc: OpenApi) -> Json<OpenApi> {
    Json(doc)
}
