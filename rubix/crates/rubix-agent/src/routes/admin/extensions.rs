//! `GET /api/v1/admin/registry/extensions` (+ `/{id}`).
//!
//! The detail page surfaces a row identical to the lifecycle
//! `/api/v1/extensions/{id}` API but in the canonical admin
//! envelope shape; consumers that already speak the lifecycle API
//! ignore this surface, but the test console hits it for a
//! uniform projection.

use axum::extract::{Path, Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;

use crate::admin::{extension_items, paginate, AdminState};
use crate::routes::{RouteMeta, RouteRegistrar};

use super::errors::{not_found, page_error_response};
use super::query::ListQuery;

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry/extensions",
            get(list).with_state(state.clone()),
            RouteMeta::new()
                .describe("List extensions (paginated).")
                .tag("admin"),
        )
        .mount(
            Method::GET,
            "/api/v1/admin/registry/extensions/{id}",
            get(detail).with_state(state),
            RouteMeta::new()
                .describe("Fetch one extension envelope by id.")
                .tag("admin"),
        )
}

async fn list(State(state): State<AdminState>, Query(q): Query<ListQuery>) -> Response {
    let decoded = match q.decode() {
        Ok(q) => q,
        Err(e) => return page_error_response(e),
    };
    let mut items = extension_items(state.extensions.as_ref());
    if let Some(filter) = decoded.source.as_ref() {
        items.retain(|item| filter.matches(&item.source));
    }
    match paginate(items, decoded.cursor.as_ref(), decoded.limit) {
        Ok(page) => Json(page).into_response(),
        Err(e) => page_error_response(e),
    }
}

async fn detail(State(state): State<AdminState>, Path(id): Path<String>) -> Response {
    match extension_items(state.extensions.as_ref())
        .into_iter()
        .find(|item| item.id == id)
    {
        Some(item) => Json(item).into_response(),
        None => not_found("extension", &id),
    }
}
