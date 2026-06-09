//! Assemble the application `Router` from app state.
//!
//! Shared by `fn main` and the HTTP integration tests so both exercise the same
//! wiring. `starter_server::ServerBuilder` adds `/health`, `/metrics`, and
//! `/openapi.json`; the product routes merge on top.

use axum::Router;
use starter_server::ServerBuilder;

use crate::openapi::document;
use crate::routes::product_router;
use crate::state::AppState;

/// Build the full router: starter's baseline routes plus the nexus product
/// surface, with the OpenAPI document served at `/openapi.json`.
pub fn router(state: AppState) -> Router {
    ServerBuilder::<AppState>::new(state)
        .merge_router(product_router())
        .with_openapi(document())
        .build()
}
