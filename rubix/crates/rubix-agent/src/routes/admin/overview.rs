//! `GET /api/v1/admin/overview` — per-kind counts.

use axum::extract::State;
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;

use crate::admin::{overview as proj, AdminState};
use crate::routes::{RouteMeta, RouteRegistrar};

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/api/v1/admin/overview",
        get(handler).with_state(state),
        RouteMeta::new()
            .describe("Cheap per-kind item counts for the admin sidebar.")
            .tag("admin"),
    )
}

async fn handler(State(state): State<AdminState>) -> Response {
    Json(proj(&state)).into_response()
}
