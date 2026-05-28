//! `GET /api/v1/admin/registry/templates` (+ `/{name}`).

use axum::extract::{Path, Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;

use crate::admin::{paginate, template_items, AdminState};
use crate::routes::{RouteMeta, RouteRegistrar};

use super::errors::{not_found, page_error_response};
use super::query::ListQuery;

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry/templates",
            get(list).with_state(state.clone()),
            RouteMeta::new()
                .describe("List warehouse templates (paginated).")
                .tag("admin"),
        )
        .mount(
            Method::GET,
            "/api/v1/admin/registry/templates/{name}",
            get(detail).with_state(state),
            RouteMeta::new()
                .describe("Fetch one warehouse template envelope by name.")
                .tag("admin"),
        )
}

async fn list(State(state): State<AdminState>, Query(q): Query<ListQuery>) -> Response {
    let decoded = match q.decode() {
        Ok(q) => q,
        Err(e) => return page_error_response(e),
    };
    let mut items = template_items(state.templates.as_ref(), state.extensions.as_ref());
    if let Some(filter) = decoded.source.as_ref() {
        items.retain(|item| filter.matches(&item.source));
    }
    match paginate(items, decoded.cursor.as_ref(), decoded.limit) {
        Ok(page) => Json(page).into_response(),
        Err(e) => page_error_response(e),
    }
}

async fn detail(State(state): State<AdminState>, Path(name): Path<String>) -> Response {
    let items = template_items(state.templates.as_ref(), state.extensions.as_ref());
    match items.into_iter().find(|item| item.id == name) {
        Some(item) => Json(item).into_response(),
        None => not_found("template", &name),
    }
}
