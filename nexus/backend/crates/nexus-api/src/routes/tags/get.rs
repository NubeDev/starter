//! `GET /api/v1/tags/{kind}/{id}` — the tags on one entity.

use axum::extract::{Path, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::tag::{Tag, TaggableKind};
use nexus_store::tag::{self, EntityRef};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::authorize::authorize_read;
use super::convert::{kind_to_stored, to_dto};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/tags/{kind}/{id}",
    tag = "tags",
    operation_id = "get_tags",
    params(
        ("kind" = TaggableKind, Path, description = "The kind of entity"),
        ("id" = String, Path, description = "The entity's id"),
    ),
    responses(
        (status = 200, description = "Tags on the entity", body = [Tag]),
        (status = 403, description = "Not allowed to view this entity"),
        (status = 404, description = "Entity not found in this tenant"),
    ),
)]
pub async fn get_tags(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path((kind, id)): Path<(TaggableKind, String)>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Reading tags requires `view` on the target entity (WS-13 §3); the entity
    // must exist in-tenant.
    if let Err(resp) = authorize_read(&state, caller, &tenant, kind, &id).await {
        return resp;
    }
    let entity = EntityRef {
        entity_type: kind_to_stored(kind).into(),
        entity_id: id,
    };
    match tag::list_for_entity(&state.metadata, &tenant, &entity).await {
        Ok(rows) => Json(rows.iter().map(to_dto).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
