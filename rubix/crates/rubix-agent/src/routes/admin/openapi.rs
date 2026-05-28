//! `GET /api/v1/admin/openapi.json` — projection of the live
//! route catalog (every endpoint mounted through
//! [`crate::routes::RouteRegistrar`]).
//!
//! The doc is built once at boot from the final catalog and
//! served by value per request — same posture as the utoipa-
//! derived `/openapi.json`. The projection does *not* include
//! the `/api/v1/admin/openapi.json` route itself: the doc Arc is
//! snapshot before this registrar is merged into the app, so the
//! catalog used for projection has not yet seen this entry. This
//! is intentional — operators consuming the doc do not need a
//! self-referential entry, and it keeps the projection a pure
//! function of registrar state.
//!
//! Distinct from `/openapi.json` (utoipa, merges
//! `starter-auth-users`): the admin variant is strictly the
//! projection of rubix-agent-owned routes that flowed through
//! the registrar. See [docs/design/admin/](../../../../docs/design/admin/README.md)
//! §"Route catalog".

use std::sync::Arc;

use axum::extract::State;
use axum::http::Method;
use axum::routing::get;
use axum::Json;
use serde_json::Value;

use crate::routes::{RouteMeta, RouteRegistrar};

/// Build the admin openapi registrar.
pub fn admin_openapi_registrar(doc: Arc<Value>) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/api/v1/admin/openapi.json",
        get(handler).with_state(doc),
        RouteMeta::new()
            .describe("OpenAPI 3.0.3 doc projected from the rubix-agent route catalog.")
            .tag("admin"),
    )
}

async fn handler(State(doc): State<Arc<Value>>) -> Json<Value> {
    Json((*doc).clone())
}
