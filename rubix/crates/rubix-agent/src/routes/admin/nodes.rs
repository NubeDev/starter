//! `GET /api/v1/admin/registry/nodes` (+ `/{kind}`).

use axum::extract::{Path, Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;

use crate::admin::{nodes as proj, paginate, AdminState};
use crate::routes::{RouteMeta, RouteRegistrar};

use super::errors::{not_found, page_error_response};
use super::query::ListQuery;

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry/nodes",
            get(list).with_state(state.clone()),
            RouteMeta::new()
                .describe("List flow node kinds (paginated).")
                .tag("admin"),
        )
        .mount(
            Method::GET,
            "/api/v1/admin/registry/nodes/{kind}",
            get(detail).with_state(state),
            RouteMeta::new()
                .describe("Fetch one flow node-kind envelope by id.")
                .tag("admin"),
        )
}

async fn list(State(state): State<AdminState>, Query(q): Query<ListQuery>) -> Response {
    let decoded = match q.decode() {
        Ok(q) => q,
        Err(e) => return page_error_response(e),
    };
    let mut items = proj::node_items(&state.node_behaviors, state.extensions.as_ref());
    if let Some(filter) = decoded.source.as_ref() {
        items.retain(|item| filter.matches(&item.source));
    }
    match paginate(items, decoded.cursor.as_ref(), decoded.limit) {
        Ok(page) => Json(page).into_response(),
        Err(e) => page_error_response(e),
    }
}

async fn detail(State(state): State<AdminState>, Path(kind): Path<String>) -> Response {
    for behavior in state.node_behaviors.iter() {
        if behavior.kind_id().as_str() == kind {
            let item = proj::node_to_item(&**behavior, state.extensions.as_ref());
            return Json(item).into_response();
        }
    }
    not_found("node", &kind)
}
