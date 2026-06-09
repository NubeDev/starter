//! Notification-channel handlers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::alert::{ChannelDetail, CreateChannelRequest};
use nexus_store::alert::{channel, NewChannel};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::channel_to_detail;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(get, path = "/api/v1/alerts/channels", tag = "alerts", operation_id = "list_channels",
    responses((status = 200, description = "Channels", body = [ChannelDetail])))]
pub async fn list_channels(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match channel::list(&state.metadata, &tenant).await {
        Ok(cs) => Json(cs.iter().map(channel_to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/alerts/channels", tag = "alerts", operation_id = "create_channel",
    request_body = CreateChannelRequest,
    responses((status = 200, description = "Created", body = ChannelDetail)))]
pub async fn create_channel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateChannelRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewChannel {
        name: req.name,
        kind: req.kind,
        config: req.config,
    };
    match channel::insert(&state.metadata, &tenant, &new).await {
        Ok(c) => Json(channel_to_detail(&c)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/alerts/channels/{id}", tag = "alerts", operation_id = "delete_channel",
    params(("id" = Uuid, Path, description = "Channel id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_channel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match channel::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
