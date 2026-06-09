//! `GET /api/v1/tags/{kind}/{id}` — the tags on one entity.

use axum::extract::{Path, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::tag::{Tag, TaggableKind};
use nexus_store::tag::{self, EntityRef};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::{kind_to_stored, to_dto};
use crate::middleware::tenant::tenant_of;
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
    responses((status = 200, description = "Tags on the entity", body = [Tag])),
)]
pub async fn get_tags(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path((kind, id)): Path<(TaggableKind, String)>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let entity = EntityRef {
        entity_type: kind_to_stored(kind).into(),
        entity_id: id,
    };
    match tag::list_for_entity(&state.metadata, &tenant, &entity).await {
        Ok(rows) => Json(rows.iter().map(to_dto).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
