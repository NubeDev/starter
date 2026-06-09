//! `PUT /api/v1/tags/{kind}/{id}` — replace an entity's full tag set.

use axum::extract::{Path, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::tag::{SetTagsRequest, TaggableKind};
use nexus_store::tag::{self, EntityRef};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::{kind_to_stored, to_record};
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    put,
    path = "/api/v1/tags/{kind}/{id}",
    tag = "tags",
    operation_id = "set_tags",
    params(
        ("kind" = TaggableKind, Path, description = "The kind of entity being tagged"),
        ("id" = String, Path, description = "The entity's id"),
    ),
    request_body = SetTagsRequest,
    responses((status = 204, description = "Tags replaced")),
)]
pub async fn set_tags(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path((kind, id)): Path<(TaggableKind, String)>,
    Json(req): Json<SetTagsRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let entity = EntityRef {
        entity_type: kind_to_stored(kind).into(),
        entity_id: id,
    };
    let tags: Vec<_> = req.tags.iter().map(to_record).collect();
    match tag::set_for_entity(&state.metadata, &tenant, &entity, &tags).await {
        Ok(()) => axum::http::StatusCode::NO_CONTENT.into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
