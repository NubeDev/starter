//! `GET /api/v1/admin/registry/tools` (+ `/{id}`).

use axum::extract::{Path, Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;

use crate::admin::{paginate, tools as proj, AdminState};
use crate::routes::{RouteMeta, RouteRegistrar};

use super::errors::{not_found, page_error_response};
use super::query::ListQuery;

/// Build the tools sub-registrar.
pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry/tools",
            get(list).with_state(state.clone()),
            RouteMeta::new()
                .describe("List registered tools (paginated).")
                .tag("admin"),
        )
        .mount(
            Method::GET,
            "/api/v1/admin/registry/tools/{id}",
            get(detail).with_state(state),
            RouteMeta::new()
                .describe("Fetch one tool envelope by id.")
                .tag("admin"),
        )
}

async fn list(State(state): State<AdminState>, Query(q): Query<ListQuery>) -> Response {
    let decoded = match q.decode() {
        Ok(q) => q,
        Err(e) => return page_error_response(e),
    };
    let tools: Vec<_> = state.tools.values().cloned().collect();
    let mut items = proj::tool_items(&tools, state.extensions.as_ref());
    if let Some(filter) = decoded.source.as_ref() {
        items.retain(|item| filter.matches(&item.source));
    }
    match paginate(items, decoded.cursor.as_ref(), decoded.limit) {
        Ok(page) => (axum::http::StatusCode::OK, Json(page)).into_response(),
        Err(e) => page_error_response(e),
    }
}

async fn detail(State(state): State<AdminState>, Path(id): Path<String>) -> Response {
    let Some(tool) = state.tools.get(&id) else {
        return not_found("tool", &id);
    };
    let item = proj::tool_to_item(&**tool, state.extensions.as_ref());
    Json(item).into_response()
}
