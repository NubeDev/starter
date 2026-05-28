//! `GET /api/v1/admin/registry/skills` (+ `/{id}`).

use axum::extract::{Path, Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;

use crate::admin::{paginate, skill_items, AdminState};
use crate::routes::{RouteMeta, RouteRegistrar};

use super::errors::{not_found, page_error_response};
use super::query::ListQuery;

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry/skills",
            get(list).with_state(state.clone()),
            RouteMeta::new()
                .describe("List skills (paginated).")
                .tag("admin"),
        )
        .mount(
            Method::GET,
            "/api/v1/admin/registry/skills/{id}",
            get(detail).with_state(state),
            RouteMeta::new()
                .describe("Fetch one skill envelope by id.")
                .tag("admin"),
        )
}

async fn list(State(_state): State<AdminState>, Query(q): Query<ListQuery>) -> Response {
    let decoded = match q.decode() {
        Ok(q) => q,
        Err(e) => return page_error_response(e),
    };
    let mut items = skill_items();
    if let Some(filter) = decoded.source.as_ref() {
        items.retain(|item| filter.matches(&item.source));
    }
    match paginate(items, decoded.cursor.as_ref(), decoded.limit) {
        Ok(page) => Json(page).into_response(),
        Err(e) => page_error_response(e),
    }
}

async fn detail(State(_state): State<AdminState>, Path(id): Path<String>) -> Response {
    match skill_items().into_iter().find(|item| item.id == id) {
        Some(item) => Json(item).into_response(),
        None => not_found("skill", &id),
    }
}
