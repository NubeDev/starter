//! `PUT /api/v1/tags/{kind}/{id}` — replace an entity's full tag set.

use axum::extract::{Path, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::tag::{SetTagsRequest, TaggableKind};
use nexus_store::tag::{self, EntityRef};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::authorize::authorize_write;
use super::convert::{kind_to_stored, to_record};
use crate::middleware::tenant::caller;
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
    responses(
        (status = 204, description = "Tags replaced"),
        (status = 403, description = "Not allowed to edit this entity"),
        (status = 404, description = "Entity not found in this tenant"),
    ),
)]
pub async fn set_tags(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path((kind, id)): Path<(TaggableKind, String)>,
    Json(req): Json<SetTagsRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Tags are query-affecting inputs (WS-13 §3): writing them requires `edit`
    // on the target, which must exist in-tenant. Closes the old tenant-only hole.
    if let Err(resp) = authorize_write(&state, caller, &tenant, kind, &id).await {
        return resp;
    }
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
