//! Tag routes. Tags are tenant-scoped labels (`key` + optional `value`) on any
//! entity, addressed by `{kind}/{id}`. Each handler resolves the caller's tenant
//! and delegates to the tenant-scoped store.

mod convert;
pub mod get;
pub mod keys;
pub mod list_entities;
pub mod set;

use axum::routing::{get as http_get, put};
use axum::Router;

use crate::state::AppState;

/// Tag collection and per-entity routes.
///
/// `keys` and `entities` are fixed segments placed *before* the `{kind}/{id}`
/// item route so axum matches them first rather than as a kind.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tags/keys", http_get(keys::list_tag_keys))
        .route(
            "/api/v1/tags/entities/{kind}",
            http_get(list_entities::list_entities_with_tag),
        )
        .route(
            "/api/v1/tags/{kind}/{id}",
            put(set::set_tags).get(get::get_tags),
        )
}
