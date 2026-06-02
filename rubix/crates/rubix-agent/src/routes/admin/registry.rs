//! `GET /api/v1/admin/registry` — multiplexed snapshot.
//!
//! Returns one [`Page`](rubix_spi::starter::paging::Page) per
//! requested kind, keyed by [`RegistryKind`]. When `?kinds=` is
//! absent every kind is included. Per-kind pagination semantics
//! match the dedicated sugar routes; the multiplexed cursor is
//! deliberately *not* supported (each kind would need its own
//! cursor in a compound envelope — instead, callers paginate per
//! kind via the sugar routes once they have selected one to drill
//! into).

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use rubix_spi::dto::admin::{RegistryItem, RegistryKind, RegistrySnapshot};
use rubix_spi::starter::paging::Page;
use starter_ext_host::ExtensionRegistry;

use crate::admin::{
    extension_items, node_items, paginate, rule_items, skill_items, table_items, template_items,
    tool_items, AdminState,
};
use crate::routes::{RouteMeta, RouteRegistrar};

use super::errors::page_error_response;
use super::query::{DecodedQuery, ListQuery};
use super::{
    cache, extensions, nodes, overview, rules, skills, supervisor, tables, templates, tools,
};

/// Build the full admin registrar — the multiplexed `/registry`
/// route plus every per-kind sugar registrar plus `/overview`.
pub fn admin_registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/registry",
            get(snapshot).with_state(state.clone()),
            RouteMeta::new()
                .describe("Multiplexed registry snapshot keyed by kind.")
                .tag("admin"),
        )
        .merge(overview::registrar(state.clone()))
        .merge(tools::registrar(state.clone()))
        .merge(nodes::registrar(state.clone()))
        .merge(rules::registrar(state.clone()))
        .merge(templates::registrar(state.clone()))
        .merge(tables::registrar(state.clone()))
        .merge(skills::registrar(state.clone()))
        .merge(cache::registrar(state.clone()))
        .merge(supervisor::registrar(state.clone()))
        .merge(extensions::registrar(state))
}

async fn snapshot(State(state): State<AdminState>, Query(q): Query<ListQuery>) -> Response {
    let decoded = match q.decode() {
        Ok(q) => q,
        Err(e) => return page_error_response(e),
    };
    let kinds = match resolve_kinds(decoded.kinds.as_deref()) {
        Ok(k) => k,
        Err(unknown) => return unknown_kind_response(&unknown),
    };
    let mut snapshot = RegistrySnapshot::new();
    for kind in kinds {
        let page = match build_page(kind, &state, &decoded) {
            Ok(p) => p,
            Err(e) => return page_error_response(e),
        };
        snapshot.insert(kind, page);
    }
    Json(snapshot).into_response()
}

fn unknown_kind_response(token: &str) -> Response {
    use axum::http::StatusCode;
    use serde_json::json;
    let body = json!({
        "error": "bad_request",
        "message": format!("unknown registry kind: {token}"),
    });
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}

fn resolve_kinds(raw: Option<&[String]>) -> Result<Vec<RegistryKind>, String> {
    let Some(raw) = raw else {
        return Ok(RegistryKind::ALL.to_vec());
    };
    if raw.is_empty() {
        return Ok(RegistryKind::ALL.to_vec());
    }
    let mut out = Vec::with_capacity(raw.len());
    for token in raw {
        match RegistryKind::from_str(token) {
            Ok(k) => out.push(k),
            Err(e) => return Err(e.0),
        }
    }
    Ok(out)
}

fn build_page(
    kind: RegistryKind,
    state: &AdminState,
    decoded: &DecodedQuery,
) -> Result<Page<RegistryItem>, crate::admin::paging::PageError> {
    let extensions: Option<&Arc<ExtensionRegistry>> = state.extensions.as_ref();
    let mut items = match kind {
        RegistryKind::Tool => {
            let tools: Vec<_> = state.tools.values().cloned().collect();
            tool_items(&tools, extensions)
        }
        RegistryKind::Node => node_items(&state.node_behaviors, extensions),
        RegistryKind::Rule => rule_items(state.rules.as_ref(), extensions),
        RegistryKind::Template => template_items(state.templates.as_ref(), extensions),
        RegistryKind::Table => table_items(extensions),
        RegistryKind::Skill => skill_items(),
        RegistryKind::Extension => extension_items(extensions),
    };
    if let Some(filter) = decoded.source.as_ref() {
        items.retain(|item| filter.matches(&item.source));
    }
    paginate(items, decoded.cursor.as_ref(), decoded.limit)
}
