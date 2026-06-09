//! `GET /api/v1/tags/entities/{kind}?key=…&value=…` — entities of a kind that
//! carry a given tag. `value` is optional: omit it to match any value for the
//! key ("tagged with `key` at all"), supply it to pin the value exactly.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::tag::{TaggableKind, TaggedEntity};
use nexus_store::tag;
use serde::Deserialize;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::{kind_to_stored, to_tagged_entity};
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct TagFilter {
    key: String,
    value: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tags/entities/{kind}",
    tag = "tags",
    operation_id = "list_entities_with_tag",
    params(
        ("kind" = TaggableKind, Path, description = "The kind of entity to list"),
        ("key" = String, Query, description = "The tag key to match"),
        ("value" = Option<String>, Query, description = "Optional exact value; omit to match any"),
    ),
    responses((status = 200, description = "Entities carrying the tag", body = [TaggedEntity])),
)]
pub async fn list_entities_with_tag(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(kind): Path<TaggableKind>,
    Query(filter): Query<TagFilter>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match tag::entities_with_tag(
        &state.metadata,
        &tenant,
        kind_to_stored(kind),
        &filter.key,
        filter.value.as_deref(),
    )
    .await
    {
        Ok(rows) => {
            Json(rows.iter().filter_map(to_tagged_entity).collect::<Vec<_>>()).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
